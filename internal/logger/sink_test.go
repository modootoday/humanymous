package logger

import (
	"context"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"
)

func TestFileSinkAppendsOnReopen(t *testing.T) {
	path := filepath.Join(t.TempDir(), "runtime.jsonl")
	for _, message := range []string{"first", "second"} {
		runtime := mustOpen(t, Config{
			Service: "gate",
			Level:   "info",
			Sinks:   []SinkConfig{{Name: "file", Format: FormatJSONL, Path: path}},
		})
		runtime.Logger().Info(message, "event", "runtime.append")
		closeRuntime(t, runtime)
	}
	content, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	lines := strings.Split(strings.TrimSpace(string(content)), "\n")
	if len(lines) != 2 || !strings.Contains(lines[0], "first") || !strings.Contains(lines[1], "second") {
		t.Fatalf("append content = %q", content)
	}
}

func TestFileSinkRejectsSamePathAndNonRegularTargets(t *testing.T) {
	temp := t.TempDir()
	path := filepath.Join(temp, "same.log")
	if _, err := Open(Config{
		Service: "gate",
		Sinks: []SinkConfig{
			{Name: "plain", Format: FormatPlain, Path: path},
			{Name: "jsonl", Format: FormatJSONL, Path: filepath.Join(temp, ".", "same.log")},
		},
	}); err == nil {
		t.Fatal("Open accepted the same path for two sinks")
	}
	if _, err := Open(Config{
		Service: "gate",
		Sinks:   []SinkConfig{{Format: FormatPlain, Path: temp}},
	}); err == nil {
		t.Fatal("Open accepted a directory sink")
	}
	if _, err := Open(Config{
		Service: "gate",
		Sinks:   []SinkConfig{{Format: FormatPlain, Path: filepath.Join(temp, "missing", "runtime.log")}},
	}); err == nil {
		t.Fatal("Open accepted an unopenable path")
	}
}

func TestFileSinkRejectsSymlink(t *testing.T) {
	temp := t.TempDir()
	target := filepath.Join(temp, "target.log")
	if err := os.WriteFile(target, nil, 0o640); err != nil {
		t.Fatal(err)
	}
	link := filepath.Join(temp, "linked.log")
	if err := os.Symlink(target, link); err != nil {
		if runtime.GOOS == "windows" {
			t.Skipf("symlink creation not permitted: %v", err)
		}
		t.Fatal(err)
	}
	if _, err := Open(Config{
		Service: "gate",
		Sinks:   []SinkConfig{{Format: FormatPlain, Path: link}},
	}); err == nil {
		t.Fatal("Open accepted a symlink sink")
	}
}

func TestFlushHonorsContextWhenSinkIsBlocked(t *testing.T) {
	writer := &blockingWriter{
		entered: make(chan struct{}),
		release: make(chan struct{}),
	}
	runtime := mustOpen(t, Config{
		Service: "gate",
		Sinks:   []SinkConfig{{Format: FormatPlain, Writer: writer}},
	})
	runtime.Logger().Info("blocked", "event", "runtime.blocked")
	select {
	case <-writer.entered:
	case <-time.After(time.Second):
		t.Fatal("worker did not enter blocking writer")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Millisecond)
	defer cancel()
	if err := runtime.Flush(ctx); err == nil {
		t.Fatal("Flush ignored its deadline")
	}
	close(writer.release)
}
