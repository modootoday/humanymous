package main

import (
	"net/http"
	"net/http/httptest"
	"reflect"
	"testing"
)

// logging_test.go guards the PLAN-07 R11 opt-in invariants: logging is OFF by
// default and adds neither cost nor behavior change when off, and when ON it
// recovers a panicking handler into a clean 500 instead of a dropped connection.

func TestLoggingOffByDefault(t *testing.T) {
	a := newTestApp(t, false)
	a.configureLogging("") // no flag, no env → off
	if a.logEnabled {
		t.Fatal("logging must default to OFF")
	}
	// withObservability must be a no-op wrapper when off: it returns the very handler
	// it was given, so there is zero added cost and net/http's panic handling is intact.
	h := http.HandlerFunc(func(http.ResponseWriter, *http.Request) {})
	got := a.withObservability(h)
	// Func values aren't ==-comparable, so compare underlying code pointers: an
	// unchanged return has the same pointer, a wrapper closure has a different one.
	if reflect.ValueOf(got).Pointer() != reflect.ValueOf(h).Pointer() {
		t.Error("withObservability must return the handler unchanged when logging is off")
	}
}

func TestLogLevelParsing(t *testing.T) {
	on := []string{"debug", "info", "warn", "warning", "error"}
	for _, v := range on {
		if _, ok := parseLogLevel(v); !ok {
			t.Errorf("level %q should enable logging", v)
		}
	}
	for _, v := range []string{"", "off", "none", "bogus"} {
		if _, ok := parseLogLevel(v); ok {
			t.Errorf("level %q should NOT enable logging", v)
		}
	}
}

func TestObservabilityRecoversPanic(t *testing.T) {
	a := newTestApp(t, false)
	a.configureLogging("error") // enabled → wrapping + recover active
	if !a.logEnabled {
		t.Fatal("expected logging enabled at level=error")
	}
	panicking := http.HandlerFunc(func(http.ResponseWriter, *http.Request) { panic("boom") })
	rr := httptest.NewRecorder()
	// Must NOT propagate the panic; must convert it to a 500.
	a.withObservability(panicking).ServeHTTP(rr, httptest.NewRequest(http.MethodGet, "/x", nil))
	if rr.Code != http.StatusInternalServerError {
		t.Errorf("recovered panic should yield 500, got %d", rr.Code)
	}
}
