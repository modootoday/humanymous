package audit

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"
)

// sink_wal.go is the zero-dependency, authoritative durable sink (SoT-32 Tier 0):
// an append-only, fsync'd, segment-rotated write-ahead log of sealed records. It
// is the ordering + durability authority — the audit chain is durable exactly
// when a record is fsync'd here, so a restart replays it and enforcement never
// depends on a remote store. Records are one JSON object per line.

const (
	walSegPrefix          = "audit-"
	walSegSuffix          = ".log"
	walCPFile             = "checkpoints.jsonl"
	walDefaultRotateBytes = 16 << 20 // 16 MiB per segment
)

// WALSink is a local write-ahead log of sealed records + a checkpoint sidecar.
type WALSink struct {
	mu          sync.Mutex
	dir         string
	seg         *os.File
	bw          *bufio.Writer
	written     int64
	segIndex    int
	rotateBytes int64
}

// NewWALSink opens (creating if needed) a WAL directory and its latest segment.
func NewWALSink(dir string) (*WALSink, error) {
	if err := os.MkdirAll(dir, 0o750); err != nil {
		return nil, err
	}
	w := &WALSink{dir: dir, rotateBytes: walDefaultRotateBytes}
	idx, size, err := latestSegment(dir)
	if err != nil {
		return nil, err
	}
	w.segIndex = idx
	if idx == 0 {
		// no segment yet; open the first
		if err := w.openSegmentLocked(1); err != nil {
			return nil, err
		}
	} else {
		f, err := os.OpenFile(segPath(dir, idx), os.O_APPEND|os.O_WRONLY, 0o640)
		if err != nil {
			return nil, err
		}
		w.seg, w.bw, w.written = f, bufio.NewWriter(f), size
	}
	return w, nil
}

// AppendRecord writes one sealed record and fsyncs before returning (durable +
// ordered). A single writer under mu keeps segment offset == chain order.
func (w *WALSink) AppendRecord(_ context.Context, r Record) error {
	line, err := json.Marshal(r)
	if err != nil {
		return err
	}
	w.mu.Lock()
	defer w.mu.Unlock()
	if w.written >= w.rotateBytes {
		if err := w.rotateLocked(); err != nil {
			return err
		}
	}
	n, err := w.bw.Write(append(line, '\n'))
	if err != nil {
		return err
	}
	w.written += int64(n)
	// Durability boundary: flush the buffer and fsync so "sealed" == "on disk"
	// before the enforcement side effect is released.
	if err := w.bw.Flush(); err != nil {
		return err
	}
	return w.seg.Sync()
}

// AppendCheckpoint persists a Signed Tree Head to the sidecar so the checkpoint
// chain survives a restart (records live in the segments).
func (w *WALSink) AppendCheckpoint(cp Checkpoint) error {
	line, err := json.Marshal(cp)
	if err != nil {
		return err
	}
	w.mu.Lock()
	defer w.mu.Unlock()
	f, err := os.OpenFile(filepath.Join(w.dir, walCPFile), os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o640)
	if err != nil {
		return err
	}
	if _, err := f.Write(append(line, '\n')); err != nil {
		f.Close()
		return err
	}
	if err := f.Sync(); err != nil {
		f.Close()
		return err
	}
	return f.Close() // a writable file's Close can surface a deferred flush error — return it (no data loss goes unreported)
}

// ReadAll reads every sealed record from all segments in seq order.
func (w *WALSink) ReadAll() ([]Record, error) {
	w.mu.Lock()
	if w.bw != nil {
		_ = w.bw.Flush()
	}
	w.mu.Unlock()
	segs, err := listSegments(w.dir)
	if err != nil {
		return nil, err
	}
	var out []Record
	for i, s := range segs {
		recs, err := readRecords(s, i == len(segs)-1) // torn-tail recovery only on the last segment
		if err != nil {
			return nil, err
		}
		out = append(out, recs...)
	}
	return out, nil
}

// ReadCheckpoints reads the persisted Signed Tree Heads in order.
func (w *WALSink) ReadCheckpoints() ([]Checkpoint, error) {
	b, err := os.ReadFile(filepath.Join(w.dir, walCPFile))
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, err
	}
	var out []Checkpoint
	for _, ln := range strings.Split(strings.TrimSpace(string(b)), "\n") {
		if ln == "" {
			continue
		}
		var cp Checkpoint
		if err := json.Unmarshal([]byte(ln), &cp); err != nil {
			return nil, err
		}
		out = append(out, cp)
	}
	return out, nil
}

// Flush flushes the buffered segment writer.
func (w *WALSink) Flush(_ context.Context) error {
	w.mu.Lock()
	defer w.mu.Unlock()
	if w.bw == nil {
		return nil
	}
	if err := w.bw.Flush(); err != nil {
		return err
	}
	return w.seg.Sync()
}

// Close flushes and closes the current segment.
func (w *WALSink) Close() error {
	w.mu.Lock()
	defer w.mu.Unlock()
	if w.bw != nil {
		_ = w.bw.Flush()
	}
	if w.seg != nil {
		_ = w.seg.Sync()
		return w.seg.Close()
	}
	return nil
}

// rotateLocked closes the current segment and opens the next. Caller holds mu.
func (w *WALSink) rotateLocked() error {
	if w.bw != nil {
		if err := w.bw.Flush(); err != nil {
			return err
		}
		_ = w.seg.Sync()
		_ = w.seg.Close()
	}
	return w.openSegmentLocked(w.segIndex + 1)
}

func (w *WALSink) openSegmentLocked(idx int) error {
	f, err := os.OpenFile(segPath(w.dir, idx), os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o640)
	if err != nil {
		return err
	}
	w.seg, w.bw, w.written, w.segIndex = f, bufio.NewWriter(f), 0, idx
	return nil
}

func segPath(dir string, idx int) string {
	return filepath.Join(dir, fmt.Sprintf("%s%010d%s", walSegPrefix, idx, walSegSuffix))
}

// latestSegment returns the highest segment index present + its size (0,0 if none).
func latestSegment(dir string) (int, int64, error) {
	segs, err := listSegments(dir)
	if err != nil || len(segs) == 0 {
		return 0, 0, err
	}
	last := segs[len(segs)-1]
	fi, err := os.Stat(last)
	if err != nil {
		return 0, 0, err
	}
	return segIndexOf(last), fi.Size(), nil
}

func listSegments(dir string) ([]string, error) {
	ents, err := os.ReadDir(dir)
	if err != nil {
		return nil, err
	}
	var segs []string
	for _, e := range ents {
		n := e.Name()
		if strings.HasPrefix(n, walSegPrefix) && strings.HasSuffix(n, walSegSuffix) {
			segs = append(segs, filepath.Join(dir, n))
		}
	}
	sort.Strings(segs) // zero-padded index => lexical == numeric order
	return segs, nil
}

func segIndexOf(path string) int {
	base := filepath.Base(path)
	base = strings.TrimPrefix(base, walSegPrefix)
	base = strings.TrimSuffix(base, walSegSuffix)
	idx := 0
	fmt.Sscanf(base, "%d", &idx)
	return idx
}

// readRecords parses one segment. A record is one JSON object per newline-terminated line.
// A fully-framed line (ends in '\n') that fails to parse is genuine corruption and is fatal
// (fail-closed) wherever it occurs. A PARTIAL final line with NO trailing newline is a torn
// tail — a crash/power-loss mid-write or a not-yet-Sync'd final line. On the LAST segment a
// torn tail is TRUNCATED to the last good record and recovery proceeds, so one partial write
// cannot brick the node into an unrecoverable boot panic loop (deep-review); anywhere else a
// torn record is still fatal.
func readRecords(path string, lastSeg bool) ([]Record, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	br := bufio.NewReaderSize(f, 64*1024)
	var out []Record
	var good int64 // byte offset past the last fully-parsed, newline-terminated record
	for {
		line, rerr := br.ReadBytes('\n')
		framed := len(line) > 0 && line[len(line)-1] == '\n'
		trimmed := strings.TrimRight(string(line), "\r\n")
		if framed {
			if trimmed != "" {
				var r Record
				if e := json.Unmarshal([]byte(trimmed), &r); e != nil {
					f.Close()
					return nil, fmt.Errorf("wal %s: corrupt framed record: %w", path, e)
				}
				out = append(out, r)
			}
			good += int64(len(line))
		} else if trimmed != "" {
			// A non-empty chunk with no terminating newline = a torn/partial final write.
			if !lastSeg {
				f.Close()
				return nil, fmt.Errorf("wal %s: torn record in a non-terminal segment", path)
			}
			f.Close()
			if terr := os.Truncate(path, good); terr != nil {
				return nil, fmt.Errorf("wal %s: torn tail, truncate failed: %w", path, terr)
			}
			// Operational recovery notice only; the WAL bytes and audit
			// verification contract remain independent of the best-effort logger.
			slog.Warn("Recovered a partial trailing audit write.",
				"component", "gate.audit",
				"event", "audit.wal_recovered",
				"offset", good)
			return out, nil
		}
		if rerr != nil { // io.EOF (clean end) or a read error
			f.Close()
			if rerr == io.EOF {
				return out, nil
			}
			return out, rerr
		}
	}
}
