package main

import (
	"bytes"
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestEnvOrDefault(t *testing.T) {
	t.Setenv("HMN_TEST_LOG_SETTING", "jsonl")
	if got := envOrDefault("HMN_TEST_LOG_SETTING", "plain"); got != "jsonl" {
		t.Fatalf("environment value = %q, want jsonl", got)
	}
	if got := envOrDefault("HMN_TEST_LOG_SETTING_MISSING", "plain"); got != "plain" {
		t.Fatalf("fallback value = %q, want plain", got)
	}
}

func TestOpenGateLoggerWritesConsolePlainAndJSONLFile(t *testing.T) {
	var stdout, stderr bytes.Buffer
	dir := t.TempDir()
	plainPath := filepath.Join(dir, "gate.log")
	jsonlPath := filepath.Join(dir, "gate.jsonl")
	runtime, err := openGateLogger(gateLoggingConfig{
		level:         "info",
		consoleFormat: "plain",
		consoleStream: "stderr",
		plainFile:     plainPath,
		jsonlFile:     jsonlPath,
	}, "gate-test", &stdout, &stderr)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		if err := runtime.Close(ctx); err != nil {
			t.Errorf("close logger: %v", err)
		}
	})

	runtime.Logger().Info("Gate logging test.",
		"component", "gate.test",
		"event", "runtime.test",
		"enabled", true)
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := runtime.Flush(ctx); err != nil {
		t.Fatal(err)
	}

	if stdout.Len() != 0 {
		t.Fatalf("stdout unexpectedly received %q", stdout.String())
	}
	for name, text := range map[string]string{
		"stderr":     stderr.String(),
		"plain file": mustReadText(t, plainPath),
	} {
		if !strings.Contains(text, `service="gate"`) ||
			!strings.Contains(text, `component="gate.test"`) ||
			!strings.Contains(text, `event="runtime.test"`) {
			t.Fatalf("%s is not a canonical plain record: %q", name, text)
		}
	}

	line := strings.TrimSpace(mustReadText(t, jsonlPath))
	var record map[string]any
	if err := json.Unmarshal([]byte(line), &record); err != nil {
		t.Fatalf("JSONL record is invalid: %v\n%s", err, line)
	}
	if record["service"] != "gate" || record["component"] != "gate.test" || record["event"] != "runtime.test" {
		t.Fatalf("unexpected JSONL envelope: %#v", record)
	}
}

func TestOpenGateLoggerRejectsInvalidConfigBeforeUse(t *testing.T) {
	tests := []struct {
		name string
		cfg  gateLoggingConfig
	}{
		{
			name: "level",
			cfg: gateLoggingConfig{
				level: "verbose", consoleFormat: "plain", consoleStream: "stderr",
			},
		},
		{
			name: "console format",
			cfg: gateLoggingConfig{
				level: "info", consoleFormat: "yaml", consoleStream: "stderr",
			},
		},
		{
			name: "console stream",
			cfg: gateLoggingConfig{
				level: "info", consoleFormat: "plain", consoleStream: "file",
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if runtime, err := openGateLogger(tt.cfg, "gate-test", &bytes.Buffer{}, &bytes.Buffer{}); err == nil {
				closeGateLogger(runtime)
				t.Fatal("configuration unexpectedly accepted")
			}
		})
	}
}

func TestOpenGateLoggerRejectsFileOpenFailure(t *testing.T) {
	missingParent := filepath.Join(t.TempDir(), "missing", "gate.log")
	runtime, err := openGateLogger(gateLoggingConfig{
		level:         "info",
		consoleFormat: "off",
		consoleStream: "stderr",
		plainFile:     missingParent,
	}, "gate-test", &bytes.Buffer{}, &bytes.Buffer{})
	if err == nil {
		closeGateLogger(runtime)
		t.Fatal("missing parent directory unexpectedly accepted")
	}
}

func TestGateFatalClassDoesNotReflectDynamicErrorText(t *testing.T) {
	if got := gateFatalClass("proxy: %v"); got != "origin_proxy" {
		t.Fatalf("fatal class = %q, want origin_proxy", got)
	}
	if got := gateFatalClass("unknown failure %v"); got != "startup" {
		t.Fatalf("unknown fatal class = %q, want startup", got)
	}
}

func mustReadText(t *testing.T, path string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return string(data)
}
