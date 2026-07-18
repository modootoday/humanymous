package audit

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"sync"
	"time"
)

// sink_clickhouse.go is a ClickHouse PROJECTION sink (SoT-32 Tier 2: durable
// columnar store for range/verify/analytics/retention). It uses ClickHouse's HTTP
// interface — NO third-party client — so the core stays dependency-free. Records
// are buffered and inserted in batches (ClickHouse wants large, infrequent inserts,
// not row-by-row), with async_insert + wait_for_async_insert for a durable ack.
// Best-effort and non-blocking: it never blocks or fails the seal (the WAL is the
// authority); on a full buffer or a down server, records drop from this projection
// and are counted (the durable copy is in the WAL).
//
// Expected table (the operator creates it), keyed on the chain order so insert
// order is irrelevant and retention drops whole partitions:
//
//	CREATE TABLE audit_log ( ... , node_id String, seq UInt64, ts DateTime, ... )
//	ENGINE = MergeTree ORDER BY (node_id, seq)
//	PARTITION BY toYYYYMMDD(ts) TTL ts + INTERVAL 12 MONTH DELETE;

type CHSink struct {
	insertURL string // full HTTP URL incl. ?query=INSERT...FORMAT JSONEachRow&settings
	client    *http.Client
	maxRows   int
	maxWait   time.Duration
	bufCap    int

	mu      sync.Mutex
	buf     []Record
	dropped uint64

	trigger chan struct{}
	done    chan struct{}
	closed  bool
}

// NewCHSink starts a background batch flusher. base is the ClickHouse HTTP endpoint
// (e.g. "http://ch:8123"); table is the target table; maxRows/maxWait bound a batch;
// bufCap bounds the in-process backlog before drop-and-count.
func NewCHSink(base, table string, maxRows int, maxWait time.Duration, bufCap int) *CHSink {
	if maxRows <= 0 {
		maxRows = 10000
	}
	if maxWait <= 0 {
		maxWait = time.Second
	}
	if bufCap <= 0 {
		bufCap = 100000
	}
	q := "INSERT INTO " + table + " FORMAT JSONEachRow"
	u := base + "/?query=" + url.QueryEscape(q) + "&async_insert=1&wait_for_async_insert=1"
	s := &CHSink{
		insertURL: u,
		client:    &http.Client{Timeout: 10 * time.Second},
		maxRows:   maxRows, maxWait: maxWait, bufCap: bufCap,
		trigger: make(chan struct{}, 1),
		done:    make(chan struct{}),
	}
	go s.run()
	return s
}

// AppendRecord buffers non-blocking; a full buffer drops + counts (WAL is durable).
func (s *CHSink) AppendRecord(_ context.Context, r Record) error {
	s.mu.Lock()
	if len(s.buf) >= s.bufCap {
		s.dropped++
		s.mu.Unlock()
		return nil
	}
	s.buf = append(s.buf, r)
	full := len(s.buf) >= s.maxRows
	s.mu.Unlock()
	if full {
		select {
		case s.trigger <- struct{}{}:
		default:
		}
	}
	return nil
}

// Flush forces the current buffer to ClickHouse and waits for the insert.
func (s *CHSink) Flush(ctx context.Context) error { return s.flush(ctx) }

// Close flushes and stops the flusher.
func (s *CHSink) Close() error {
	s.mu.Lock()
	if s.closed {
		s.mu.Unlock()
		return nil
	}
	s.closed = true
	s.mu.Unlock()
	_ = s.flush(context.Background())
	close(s.done)
	return nil
}

// Dropped reports records dropped from this projection under load.
func (s *CHSink) Dropped() uint64 {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.dropped
}

func (s *CHSink) run() {
	t := time.NewTicker(s.maxWait)
	defer t.Stop()
	for {
		select {
		case <-s.done:
			return
		case <-t.C:
			_ = s.flush(context.Background())
		case <-s.trigger:
			_ = s.flush(context.Background())
		}
	}
}

// flush swaps out the buffer and POSTs it as one JSONEachRow batch.
func (s *CHSink) flush(ctx context.Context) error {
	s.mu.Lock()
	if len(s.buf) == 0 {
		s.mu.Unlock()
		return nil
	}
	batch := s.buf
	s.buf = nil
	s.mu.Unlock()

	body := chBatchBody(batch)
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, s.insertURL, bytes.NewReader(body))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/x-ndjson")
	resp, err := s.client.Do(req)
	if err != nil {
		// server down: this projection loses the batch (durable copy is in the WAL).
		s.mu.Lock()
		s.dropped += uint64(len(batch))
		s.mu.Unlock()
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 300 {
		msg, _ := io.ReadAll(io.LimitReader(resp.Body, 512))
		s.mu.Lock()
		s.dropped += uint64(len(batch))
		s.mu.Unlock()
		return fmt.Errorf("clickhouse insert status %d: %s", resp.StatusCode, msg)
	}
	_, _ = io.Copy(io.Discard, resp.Body)
	return nil
}

// chBatchBody encodes records as ClickHouse JSONEachRow (one JSON object per line).
func chBatchBody(batch []Record) []byte {
	var b bytes.Buffer
	enc := json.NewEncoder(&b) // Encode writes a trailing newline per record
	for i := range batch {
		_ = enc.Encode(batch[i])
	}
	return b.Bytes()
}
