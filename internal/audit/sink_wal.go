package audit

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
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
	defer f.Close()
	if _, err := f.Write(append(line, '\n')); err != nil {
		return err
	}
	return f.Sync()
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
	for _, s := range segs {
		recs, err := readRecords(s)
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

func readRecords(path string) ([]Record, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	var out []Record
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 0, 64*1024), 8<<20) // allow large records
	for sc.Scan() {
		ln := sc.Bytes()
		if len(ln) == 0 {
			continue
		}
		var r Record
		if err := json.Unmarshal(ln, &r); err != nil {
			return nil, err
		}
		out = append(out, r)
	}
	return out, sc.Err()
}
