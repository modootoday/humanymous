// Package logger provides bounded, best-effort operational logging.
//
// Operational logs are diagnostics only. They are deliberately independent
// from the tamper-evident audit stream and must never affect enforcement.
package logger

import (
	"context"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"sync"
	"sync/atomic"
	"time"
)

const (
	SchemaVersion    = "1.0.0"
	DefaultQueueSize = 1024
	MaxRecordBytes   = 4096
	maxQueueSize     = 65536
)

// Format identifies the physical line encoding used by a sink.
type Format string

const (
	FormatPlain Format = "plain"
	FormatJSONL Format = "jsonl"
)

// SinkConfig describes one output. Exactly one of Path or Writer must be set.
// Path outputs are opened append-only and are owned by Runtime. Writer outputs
// remain owned by the caller.
type SinkConfig struct {
	Name   string
	Format Format
	Path   string
	Writer io.Writer
}

// Config describes one process-wide operational logger.
type Config struct {
	Service   string
	Version   string
	Node      string
	Level     string
	QueueSize int
	Sinks     []SinkConfig

	// Now exists for deterministic tests. Production callers should leave it nil.
	Now func() time.Time
}

// LevelCounts reports dropped records without unbounded label values.
type LevelCounts struct {
	Debug uint64
	Info  uint64
	Warn  uint64
	Error uint64
}

// DropStats divides drops into the two fixed producer-side reasons.
type DropStats struct {
	QueueFull LevelCounts
	Closed    LevelCounts
}

// SinkStats is a bounded snapshot of one configured sink.
type SinkStats struct {
	Name        string
	Format      Format
	Healthy     bool
	WriteErrors uint64
}

// Stats is a race-safe snapshot. It contains no dynamic error text or labels.
type Stats struct {
	QueueDepth  int
	Dropped     DropStats
	WriteErrors uint64
	Sinks       []SinkStats
}

type levelCounters struct {
	debug atomic.Uint64
	info  atomic.Uint64
	warn  atomic.Uint64
	err   atomic.Uint64
}

func (c *levelCounters) add(level slog.Level) {
	switch {
	case level >= slog.LevelError:
		c.err.Add(1)
	case level >= slog.LevelWarn:
		c.warn.Add(1)
	case level >= slog.LevelInfo:
		c.info.Add(1)
	default:
		c.debug.Add(1)
	}
}

func (c *levelCounters) snapshot() LevelCounts {
	return LevelCounts{
		Debug: c.debug.Load(),
		Info:  c.info.Load(),
		Warn:  c.warn.Load(),
		Error: c.err.Load(),
	}
}

type counters struct {
	queueFull   levelCounters
	closed      levelCounters
	writeErrors atomic.Uint64
}

type queueItem struct {
	record *record
	flush  chan struct{}
}

// Runtime owns a bounded producer queue and a single sink worker.
type Runtime struct {
	service string
	version string
	node    string
	level   slog.Level
	off     bool
	now     func() time.Time

	queue chan queueItem
	sinks []*sink
	stats counters

	stateMu   sync.RWMutex
	closed    bool
	closeOnce sync.Once
	done      chan struct{}
	handler   *handler
}

// Open validates config and opens every file before starting the worker.
func Open(cfg Config) (*Runtime, error) {
	level, off, err := parseLevel(cfg.Level)
	if err != nil {
		return nil, err
	}
	if cfg.QueueSize == 0 {
		cfg.QueueSize = DefaultQueueSize
	}
	if cfg.QueueSize < 0 || cfg.QueueSize > maxQueueSize {
		return nil, fmt.Errorf("logger: queue size must be between 1 and %d", maxQueueSize)
	}

	service := sanitizeString(cfg.Service, maxIdentityBytes)
	if service == "" {
		return nil, errors.New("logger: service is required")
	}
	version := sanitizeString(cfg.Version, maxIdentityBytes)
	if version == "" {
		version = "unknown"
	}
	node := sanitizeString(cfg.Node, maxIdentityBytes)
	if node == "" {
		node = "unknown"
	}

	sinks, err := openSinks(cfg.Sinks)
	if err != nil {
		return nil, err
	}
	now := cfg.Now
	if now == nil {
		now = time.Now
	}
	r := &Runtime{
		service: service,
		version: version,
		node:    node,
		level:   level,
		off:     off,
		now:     now,
		queue:   make(chan queueItem, cfg.QueueSize),
		sinks:   sinks,
		done:    make(chan struct{}),
	}
	r.handler = &handler{runtime: r}
	go r.run()
	return r, nil
}

// Discard returns an off Runtime with no sinks.
func Discard(service string) *Runtime {
	r, err := Open(Config{Service: service, Level: "off"})
	if err == nil {
		return r
	}
	// Keep this helper total even for an empty or hostile service string.
	r, _ = Open(Config{Service: "unknown", Level: "off"})
	return r
}

// Logger returns the slog facade for this runtime.
func (r *Runtime) Logger() *slog.Logger {
	return slog.New(r.handler)
}

// Handler exposes the shared handler for slog and standard-library bridges.
func (r *Runtime) Handler() slog.Handler {
	return r.handler
}

// Stats returns a race-safe bounded snapshot.
func (r *Runtime) Stats() Stats {
	out := Stats{
		QueueDepth: len(r.queue),
		Dropped: DropStats{
			QueueFull: r.stats.queueFull.snapshot(),
			Closed:    r.stats.closed.snapshot(),
		},
		WriteErrors: r.stats.writeErrors.Load(),
		Sinks:       make([]SinkStats, 0, len(r.sinks)),
	}
	for _, s := range r.sinks {
		n := s.writeErrors.Load()
		out.Sinks = append(out.Sinks, SinkStats{
			Name:        s.name,
			Format:      s.format,
			Healthy:     n == 0,
			WriteErrors: n,
		})
	}
	return out
}

// Flush waits until all records accepted before the call have reached every
// sink and synchronizable sinks have been synced.
func (r *Runtime) Flush(ctx context.Context) error {
	if ctx == nil {
		return errors.New("logger: nil flush context")
	}
	r.stateMu.RLock()
	if r.closed {
		r.stateMu.RUnlock()
		select {
		case <-r.done:
			return nil
		case <-ctx.Done():
			return ctx.Err()
		}
	}
	ack := make(chan struct{})
	item := queueItem{flush: ack}
	select {
	case r.queue <- item:
		r.stateMu.RUnlock()
	case <-ctx.Done():
		r.stateMu.RUnlock()
		return ctx.Err()
	}
	select {
	case <-ack:
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

// Close stops accepting records and waits for the queue to drain. It is safe
// to call more than once.
func (r *Runtime) Close(ctx context.Context) error {
	if ctx == nil {
		return errors.New("logger: nil close context")
	}
	r.closeOnce.Do(func() {
		r.stateMu.Lock()
		r.closed = true
		close(r.queue)
		r.stateMu.Unlock()
	})
	select {
	case <-r.done:
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (r *Runtime) run() {
	defer close(r.done)
	var sequence uint64
	for item := range r.queue {
		if item.flush != nil {
			r.syncSinks()
			close(item.flush)
			continue
		}
		sequence++
		item.record.Sequence = sequence
		item.record.fit()
		for _, s := range r.sinks {
			var line []byte
			if s.format == FormatJSONL {
				line = item.record.jsonLine()
			} else {
				line = item.record.plainLine()
			}
			if err := s.write(line); err != nil {
				s.writeErrors.Add(1)
				r.stats.writeErrors.Add(1)
			}
		}
	}
	r.syncSinks()
	for _, s := range r.sinks {
		if err := s.close(); err != nil {
			s.writeErrors.Add(1)
			r.stats.writeErrors.Add(1)
		}
	}
}

func (r *Runtime) syncSinks() {
	for _, s := range r.sinks {
		if err := s.sync(); err != nil {
			s.writeErrors.Add(1)
			r.stats.writeErrors.Add(1)
		}
	}
}

func parseLevel(raw string) (slog.Level, bool, error) {
	switch raw {
	case "", "info":
		return slog.LevelInfo, false, nil
	case "off":
		return slog.LevelError, true, nil
	case "debug":
		return slog.LevelDebug, false, nil
	case "warn":
		return slog.LevelWarn, false, nil
	case "error":
		return slog.LevelError, false, nil
	default:
		return 0, false, fmt.Errorf("logger: invalid level %q", sanitizeString(raw, 64))
	}
}

func validateFormat(format Format) error {
	switch format {
	case FormatPlain, FormatJSONL:
		return nil
	default:
		return fmt.Errorf("logger: invalid format %q", sanitizeString(string(format), 64))
	}
}
