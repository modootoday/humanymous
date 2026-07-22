package main

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

// ops_test.go locks the PLAN-07 R14/R17 ops surface: /healthz is always-on and
// unauthenticated; /api/explain + /api/counters are absent unless an operator token
// is configured, and then require that exact bearer (a wrong/missing one → 404).

func TestHealthzAlwaysOn(t *testing.T) {
	a := newTestApp(t, false) // no ops token configured
	rr := httptest.NewRecorder()
	a.routes().ServeHTTP(rr, httptest.NewRequest(http.MethodGet, "/healthz", nil))
	if rr.Code != http.StatusOK {
		t.Fatalf("/healthz must be 200 without auth, got %d", rr.Code)
	}
}

func TestOpsEndpointsAbsentWithoutToken(t *testing.T) {
	a := newTestApp(t, false)
	a.configureOps("") // disabled
	for _, path := range []string{"/api/explain/abc", "/api/counters"} {
		rr := httptest.NewRecorder()
		a.routes().ServeHTTP(rr, httptest.NewRequest(http.MethodGet, path, nil))
		if rr.Code != http.StatusNotFound {
			t.Errorf("%s must be 404 when no ops token is set, got %d", path, rr.Code)
		}
	}
}

func TestOpsEndpointsGated(t *testing.T) {
	a := newTestApp(t, false)
	a.configureOps("s3cret")
	handler := a.routes()

	// Wrong/missing token → 404 (non-discoverable).
	rr := httptest.NewRecorder()
	handler.ServeHTTP(rr, httptest.NewRequest(http.MethodGet, "/api/counters", nil))
	if rr.Code != http.StatusNotFound {
		t.Errorf("counters without token must be 404, got %d", rr.Code)
	}

	// Correct token → 200.
	rr = httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/api/counters", nil)
	req.Header.Set("Authorization", "Bearer s3cret")
	handler.ServeHTTP(rr, req)
	if rr.Code != http.StatusOK {
		t.Errorf("counters with correct token must be 200, got %d", rr.Code)
	}
}
