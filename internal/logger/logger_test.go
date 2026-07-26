package logger

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"io"
	"strings"
	"sync"
	"testing"
	"time"
)

var fixedTime = time.Date(2026, 7, 26, 12, 34, 56, 123456789, time.FixedZone("KST", 9*60*60))

func TestPlainAndJSONLShareCanonicalRecord(t *testing.T) {
	var plain bytes.Buffer
	var jsonl bytes.Buffer
	runtime := mustOpen(t, Config{
		Service: "gate",
		Version: "v1.2.3",
		Node:    "gate-1",
		Level:   "debug",
		Now:     func() time.Time { return fixedTime },
		Sinks: []SinkConfig{
			{Name: "plain", Format: FormatPlain, Writer: &plain},
			{Name: "jsonl", Format: FormatJSONL, Writer: &jsonl},
		},
	})

	runtime.Logger().Info(
		"Gate listener started.",
		"event", "runtime.started",
		"component", "gate.runtime",
		"z", "last",
		"a", 1,
	)
	flush(t, runtime)

	const wantPlain = "2026-07-26T03:34:56.123456789Z INFO service=\"gate\" version=\"v1.2.3\" node=\"gate-1\" component=\"gate.runtime\" event=\"runtime.started\" sequence=1 message=\"Gate listener started.\" field.a=1 field.z=\"last\"\n"
	if got := plain.String(); got != wantPlain {
		t.Fatalf("plain record mismatch\n got: %s\nwant: %s", got, wantPlain)
	}
	const wantJSONL = "{\"schema_version\":\"1.0.0\",\"ts\":\"2026-07-26T03:34:56.123456789Z\",\"level\":\"info\",\"service\":\"gate\",\"version\":\"v1.2.3\",\"node\":\"gate-1\",\"component\":\"gate.runtime\",\"event\":\"runtime.started\",\"message\":\"Gate listener started.\",\"sequence\":1,\"fields\":{\"a\":1,\"z\":\"last\"}}\n"
	if got := jsonl.String(); got != wantJSONL {
		t.Fatalf("JSONL record mismatch\n got: %s\nwant: %s", got, wantJSONL)
	}
}

func TestLevelValidationAndFiltering(t *testing.T) {
	for _, level := range []string{"off", "debug", "info", "warn", "error"} {
		runtime, err := Open(Config{Service: "core", Level: level})
		if err != nil {
			t.Fatalf("Open(level=%q): %v", level, err)
		}
		closeRuntime(t, runtime)
	}
	if _, err := Open(Config{Service: "core", Level: "verbose"}); err == nil {
		t.Fatal("Open accepted an invalid level")
	}
	if _, err := Open(Config{
		Service: "core",
		Sinks:   []SinkConfig{{Format: Format("xml"), Writer: io.Discard}},
	}); err == nil {
		t.Fatal("Open accepted an invalid format")
	}

	var output bytes.Buffer
	runtime := mustOpen(t, Config{
		Service: "core",
		Level:   "warn",
		Now:     func() time.Time { return fixedTime },
		Sinks:   []SinkConfig{{Format: FormatJSONL, Writer: &output}},
	})
	runtime.Logger().Info("hidden", "event", "runtime.hidden")
	runtime.Logger().Warn("visible", "event", "runtime.visible")
	flush(t, runtime)
	if strings.Contains(output.String(), "hidden") || !strings.Contains(output.String(), "visible") {
		t.Fatalf("level filter output = %q", output.String())
	}
}

func TestSanitizesHostileTextAndCapsPhysicalLines(t *testing.T) {
	var plain bytes.Buffer
	var jsonl bytes.Buffer
	runtime := mustOpen(t, Config{
		Service: "gate",
		Level:   "info",
		Now:     func() time.Time { return fixedTime },
		Sinks: []SinkConfig{
			{Format: FormatPlain, Writer: &plain},
			{Format: FormatJSONL, Writer: &jsonl},
		},
	})
	hostile := "first\r\n\x00\x1b[31mred\x1b[0m\u202esecond" + string([]byte{0xff}) + strings.Repeat("x", 9000)
	runtime.Logger().Info(hostile,
		"event", "runtime.hostile",
		"field\nforged", hostile,
		"error", errors.New("secret error text"),
	)
	flush(t, runtime)

	for name, line := range map[string]string{"plain": plain.String(), "jsonl": jsonl.String()} {
		if len(line) > MaxRecordBytes {
			t.Fatalf("%s line is %d bytes, exceeds %d", name, len(line), MaxRecordBytes)
		}
		if strings.Count(line, "\n") != 1 {
			t.Fatalf("%s produced a forged physical line: %q", name, line)
		}
		for _, forbidden := range []string{"\r", "\x00", "\x1b", "\u202e", "[31m", "secret error text"} {
			if strings.Contains(line, forbidden) {
				t.Fatalf("%s retained forbidden text %q: %q", name, forbidden, line)
			}
		}
		if !strings.Contains(line, "truncated") {
			t.Fatalf("%s did not mark normalization/truncation: %q", name, line)
		}
	}
	var decoded map[string]any
	if err := json.Unmarshal(bytes.TrimSpace(jsonl.Bytes()), &decoded); err != nil {
		t.Fatalf("JSONL did not parse: %v", err)
	}
}

func TestConcurrentProducersProduceIntactJSONLines(t *testing.T) {
	var output lockedBuffer
	runtime := mustOpen(t, Config{
		Service:   "core",
		Level:     "debug",
		QueueSize: 2048,
		Sinks:     []SinkConfig{{Format: FormatJSONL, Writer: &output}},
	})
	const producers = 8
	const perProducer = 100
	var group sync.WaitGroup
	for producer := 0; producer < producers; producer++ {
		group.Add(1)
		go func(id int) {
			defer group.Done()
			for index := 0; index < perProducer; index++ {
				runtime.Logger().Info("record", "event", "runtime.concurrent", "producer", id, "index", index)
			}
		}(producer)
	}
	group.Wait()
	flush(t, runtime)
	lines := strings.Split(strings.TrimSpace(output.String()), "\n")
	if len(lines) != producers*perProducer {
		t.Fatalf("got %d lines, want %d (stats=%+v)", len(lines), producers*perProducer, runtime.Stats())
	}
	for index, line := range lines {
		var decoded map[string]any
		if err := json.Unmarshal([]byte(line), &decoded); err != nil {
			t.Fatalf("line %d is not intact JSON: %v", index, err)
		}
		if decoded["sequence"] != float64(index+1) {
			t.Fatalf("line %d sequence = %v", index, decoded["sequence"])
		}
	}
}

func TestQueueSaturationDoesNotBlockProducer(t *testing.T) {
	writer := &blockingWriter{
		entered: make(chan struct{}),
		release: make(chan struct{}),
	}
	runtime := mustOpen(t, Config{
		Service:   "gate",
		Level:     "info",
		QueueSize: 1,
		Sinks:     []SinkConfig{{Format: FormatJSONL, Writer: writer}},
	})
	runtime.Logger().Info("first", "event", "runtime.first")
	select {
	case <-writer.entered:
	case <-time.After(time.Second):
		t.Fatal("worker did not enter blocking writer")
	}
	runtime.Logger().Info("queued", "event", "runtime.queued")

	start := time.Now()
	for index := 0; index < 1000; index++ {
		runtime.Logger().Info("drop", "event", "runtime.drop")
	}
	if elapsed := time.Since(start); elapsed > 250*time.Millisecond {
		t.Fatalf("producer blocked for %v", elapsed)
	}
	if dropped := runtime.Stats().Dropped.QueueFull.Info; dropped == 0 {
		t.Fatal("queue saturation did not increment drops")
	}
	close(writer.release)
	closeRuntime(t, runtime)
}

func TestOneFailedSinkDoesNotSuppressAnother(t *testing.T) {
	var good bytes.Buffer
	runtime := mustOpen(t, Config{
		Service: "core",
		Level:   "info",
		Sinks: []SinkConfig{
			{Name: "failed", Format: FormatJSONL, Writer: failingWriter{}},
			{Name: "good", Format: FormatJSONL, Writer: &good},
		},
	})
	runtime.Logger().Info("still delivered", "event", "runtime.delivered")
	flush(t, runtime)
	if !strings.Contains(good.String(), "still delivered") {
		t.Fatalf("healthy sink output = %q", good.String())
	}
	stats := runtime.Stats()
	if stats.WriteErrors != 1 || stats.Sinks[0].Healthy || !stats.Sinks[1].Healthy {
		t.Fatalf("unexpected sink stats: %+v", stats)
	}
}

func TestFlushAndCloseAreBoundedAndIdempotent(t *testing.T) {
	var output bytes.Buffer
	runtime := mustOpen(t, Config{
		Service: "gate",
		Sinks:   []SinkConfig{{Format: FormatPlain, Writer: &output}},
	})
	runtime.Logger().Info("before close", "event", "runtime.before_close")
	flush(t, runtime)
	closeRuntime(t, runtime)
	closeRuntime(t, runtime)

	runtime.Logger().Info("after close", "event", "runtime.after_close")
	if got := runtime.Stats().Dropped.Closed.Info; got != 1 {
		t.Fatalf("closed drop count = %d, want 1", got)
	}
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := runtime.Flush(ctx); err != nil {
		t.Fatalf("Flush after Close: %v", err)
	}
}

func mustOpen(t *testing.T, cfg Config) *Runtime {
	t.Helper()
	runtime, err := Open(cfg)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	t.Cleanup(func() {
		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		_ = runtime.Close(ctx)
	})
	return runtime
}

func flush(t *testing.T, runtime *Runtime) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := runtime.Flush(ctx); err != nil {
		t.Fatalf("Flush: %v", err)
	}
}

func closeRuntime(t *testing.T, runtime *Runtime) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := runtime.Close(ctx); err != nil {
		t.Fatalf("Close: %v", err)
	}
}

type lockedBuffer struct {
	mu sync.Mutex
	b  bytes.Buffer
}

func (b *lockedBuffer) Write(value []byte) (int, error) {
	b.mu.Lock()
	defer b.mu.Unlock()
	return b.b.Write(value)
}

func (b *lockedBuffer) String() string {
	b.mu.Lock()
	defer b.mu.Unlock()
	return b.b.String()
}

type blockingWriter struct {
	once    sync.Once
	entered chan struct{}
	release chan struct{}
}

func (w *blockingWriter) Write(value []byte) (int, error) {
	w.once.Do(func() { close(w.entered) })
	<-w.release
	return len(value), nil
}

type failingWriter struct{}

func (failingWriter) Write([]byte) (int, error) {
	return 0, errors.New("write failure")
}
