package gate

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/modootoday/humanymous/internal/audit"
)

// upstream_error_test.go verifies the PLAN-07 R16 ErrorHandler: an origin transport
// failure is now recorded in the tamper-evident log (with status + error class +
// correlation id) while the client still receives the exact default response — a 502
// with an empty body.
func TestUpstreamErrorAudited(t *testing.T) {
	// Stand up an upstream then close it: its URL now points at a closed port, so the
	// proxy's dial fails with connection-refused — a deterministic upstream outage.
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {}))
	deadURL := up.URL
	up.Close()

	srv, alog := banStack(t, deadURL, 100)

	r := httptest.NewRequest("GET", "http://p/data", nil)
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)

	if w.Code != http.StatusBadGateway {
		t.Fatalf("upstream failure should yield 502, got %d", w.Code)
	}
	if w.Body.Len() != 0 {
		t.Errorf("502 body must be empty (identical to httputil's default), got %q", w.Body.String())
	}

	var found *audit.Record
	for _, rec := range alog.Records() {
		if rec.EventType == audit.EventUpstreamError {
			r := rec
			found = &r
			break
		}
	}
	if found == nil {
		t.Fatal("no upstream.error audit record emitted for the outage")
	}
	if found.Upstream == nil || found.Upstream.Status != http.StatusBadGateway {
		t.Errorf("upstream record missing status 502: %+v", found.Upstream)
	}
	if found.Upstream.ErrorClass == "" {
		t.Error("upstream record missing error class")
	}
	if found.Correlation == "" {
		t.Error("upstream record has no correlation id (PLAN-07 R15 threading)")
	}
	if found.EventID == "" {
		t.Error("upstream record has no event id (PLAN-07 R15 stamping)")
	}
}
