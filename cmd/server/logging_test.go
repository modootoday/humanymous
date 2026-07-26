package main

import (
	"context"
	"encoding/json"
	"log"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"
)

func preserveGlobalLogging(t *testing.T) {
	t.Helper()
	previousDefault := slog.Default()
	previousWriter := log.Writer()
	previousFlags := log.Flags()
	previousPrefix := log.Prefix()
	t.Cleanup(func() {
		slog.SetDefault(previousDefault)
		log.SetOutput(previousWriter)
		log.SetFlags(previousFlags)
		log.SetPrefix(previousPrefix)
	})
}

func openTestCoreLogger(t *testing.T, a *app, cfg coreLoggingConfig) {
	t.Helper()
	preserveGlobalLogging(t)
	runtime, err := a.configureLogging(cfg)
	if err != nil {
		t.Fatalf("configure logging: %v", err)
	}
	t.Cleanup(func() { closeOperationalLogger(runtime) })
}

func TestLoggingOffByDefault(t *testing.T) {
	a := newTestApp(t, false)
	openTestCoreLogger(t, a, coreLoggingConfig{
		Level:         "off",
		ConsoleFormat: "plain",
		ConsoleStream: "stderr",
	})
	if a.logEnabled {
		t.Fatal("logging must default to off")
	}
	h := http.HandlerFunc(func(http.ResponseWriter, *http.Request) {})
	got := a.withObservability(h)
	if reflect.ValueOf(got).Pointer() != reflect.ValueOf(h).Pointer() {
		t.Error("withObservability must return the handler unchanged when logging is off")
	}
}

func TestLoggingRejectsInvalidExplicitConfig(t *testing.T) {
	tests := []coreLoggingConfig{
		{Level: "", ConsoleFormat: "plain", ConsoleStream: "stderr"},
		{Level: "verbose", ConsoleFormat: "plain", ConsoleStream: "stderr"},
		{Level: "info", ConsoleFormat: "text", ConsoleStream: "stderr"},
		{Level: "info", ConsoleFormat: "plain", ConsoleStream: "console"},
	}
	for _, cfg := range tests {
		a := newTestApp(t, false)
		if runtime, err := a.configureLogging(cfg); err == nil {
			closeOperationalLogger(runtime)
			t.Errorf("configuration should fail: %+v", cfg)
		}
	}
}

func TestLogSettingPrecedence(t *testing.T) {
	t.Setenv("HMN_TEST_LOG_SETTING", "jsonl")
	if got := resolveLogSetting("plain", true, "HMN_TEST_LOG_SETTING", "off"); got != "plain" {
		t.Fatalf("explicit flag must win, got %q", got)
	}
	if got := resolveLogSetting("", false, "HMN_TEST_LOG_SETTING", "off"); got != "jsonl" {
		t.Fatalf("environment must win over fallback, got %q", got)
	}
	os.Unsetenv("HMN_TEST_LOG_SETTING")
	if got := resolveLogSetting("", false, "HMN_TEST_LOG_SETTING", "off"); got != "off" {
		t.Fatalf("fallback mismatch: %q", got)
	}
}

func TestLoggingAccumulatesPlainAndJSONLFiles(t *testing.T) {
	dir := t.TempDir()
	plainPath := filepath.Join(dir, "core.log")
	jsonlPath := filepath.Join(dir, "core.jsonl")
	a := newTestApp(t, false)
	preserveGlobalLogging(t)
	runtime, err := a.configureLogging(coreLoggingConfig{
		Level:         "info",
		ConsoleFormat: "off",
		ConsoleStream: "stderr",
		PlainFile:     plainPath,
		JSONLFile:     jsonlPath,
	})
	if err != nil {
		t.Fatal(err)
	}
	a.log.Info("Core test event.",
		"component", "core.test",
		"event", "test.dual_sink")
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := runtime.Flush(ctx); err != nil {
		t.Fatal(err)
	}
	closeOperationalLogger(runtime)

	plain, err := os.ReadFile(plainPath)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(plain), `component="core.test" event="test.dual_sink"`) {
		t.Fatalf("plain event missing: %s", plain)
	}
	jsonLine, err := os.ReadFile(jsonlPath)
	if err != nil {
		t.Fatal(err)
	}
	var record map[string]any
	if err := json.Unmarshal(jsonLine, &record); err != nil {
		t.Fatalf("JSONL record: %v", err)
	}
	if record["event"] != "test.dual_sink" || record["service"] != "core" {
		t.Fatalf("unexpected JSONL envelope: %#v", record)
	}
}

func TestObservabilityOmitsPathAndPanicMaterial(t *testing.T) {
	dir := t.TempDir()
	jsonlPath := filepath.Join(dir, "core.jsonl")
	a := newTestApp(t, false)
	preserveGlobalLogging(t)
	runtime, err := a.configureLogging(coreLoggingConfig{
		Level:         "debug",
		ConsoleFormat: "off",
		ConsoleStream: "stderr",
		JSONLFile:     jsonlPath,
	})
	if err != nil {
		t.Fatal(err)
	}

	const pathSecret = "path-secret"
	const panicSecret = "panic-secret"
	panicking := http.HandlerFunc(func(http.ResponseWriter, *http.Request) { panic(panicSecret) })
	rr := httptest.NewRecorder()
	a.withObservability(panicking).ServeHTTP(
		rr,
		httptest.NewRequest(http.MethodGet, "/"+pathSecret, nil),
	)
	if rr.Code != http.StatusInternalServerError {
		t.Errorf("recovered panic should yield 500, got %d", rr.Code)
	}
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := runtime.Flush(ctx); err != nil {
		t.Fatal(err)
	}
	closeOperationalLogger(runtime)

	content, err := os.ReadFile(jsonlPath)
	if err != nil {
		t.Fatal(err)
	}
	text := string(content)
	if strings.Contains(text, pathSecret) || strings.Contains(text, panicSecret) ||
		strings.Contains(text, "goroutine") {
		t.Fatalf("forbidden request or panic material reached operational log: %s", text)
	}
	if !strings.Contains(text, `"event":"http.panic_recovered"`) {
		t.Fatalf("panic event missing: %s", text)
	}
}
