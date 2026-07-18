package main

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// newDemoApp builds an app whose webDir points at the real repo web/ folder so
// ServeFile can find demo.html (tests run from cmd/server).
func newDemoApp(t *testing.T) *app {
	t.Helper()
	t.Setenv("HMN_PLAYGROUND", "0")
	return newApp("../../web", make([]byte, 32), false /*ritOn*/)
}

// The public demo page is served without the dev playground flag: it is a
// read-only, self-scoring surface and must be reachable in the shipped image.
func TestDemoPageServed(t *testing.T) {
	a := newDemoApp(t)
	rec := httptest.NewRecorder()
	req := httptest.NewRequest("GET", "/demo", nil)
	a.routes().ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("GET /demo status = %d, want 200", rec.Code)
	}
	body := rec.Body.String()
	if !strings.Contains(body, "Score this browser") {
		t.Errorf("demo page missing the score CTA")
	}
	if !strings.Contains(body, "/static/js/demo.js") {
		t.Errorf("demo page must load the demo module")
	}
}

// The demo route issues a session cookie so the WASM client + /api/collect +
// /api/report/{sid} flow has a session to attach to.
func TestDemoIssuesSession(t *testing.T) {
	a := newDemoApp(t)
	rec := httptest.NewRecorder()
	req := httptest.NewRequest("GET", "/demo", nil)
	a.routes().ServeHTTP(rec, req)

	var found bool
	for _, c := range rec.Result().Cookies() {
		if c.Name == sessionCookie && c.Value != "" {
			found = true
		}
	}
	if !found {
		t.Fatalf("GET /demo did not set the %q session cookie", sessionCookie)
	}
}

// The demo must NOT depend on the dev playground: even with the gate off, no
// /playground* route is registered, but /demo still works.
func TestDemoIndependentOfPlayground(t *testing.T) {
	a := newDemoApp(t)
	// playground off → its routes 404, proving /demo is a separate public surface.
	rec := httptest.NewRecorder()
	req := httptest.NewRequest("GET", "/playground", nil)
	req.Host = "127.0.0.1:8443"
	a.routes().ServeHTTP(rec, req)
	if rec.Code != http.StatusNotFound {
		t.Fatalf("playground route should be absent (404) with gate off, got %d", rec.Code)
	}
}
