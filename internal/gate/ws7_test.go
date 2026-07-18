package gate

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/scoring"
)

// WS7: flooding the control plane is dropped with 429 BEFORE any scoring, and the
// flood is metered (a single sampled audit record), not chained per request.
func TestControlPlaneFloodMetered(t *testing.T) {
	alog := audit.NewLog(audit.Config{NodeID: "t", HMACKey: []byte("k"), CheckpointEvery: 100})
	sink := audit.NewSink(alog)
	store := collector.NewStore(time.Minute)
	cp := NewControlPlane(store, scoring.NewEngine(), NewVerdictStore(time.Minute), sink, audit.NewVault()).
		WithControlLimiter(time.Minute, 3, 5) // tiny thresholds for the test
	h := cp.Handler()

	body := `{"userAgent":"x","signals":[],"behavior":{}}`
	blockedAt := -1
	for i := 0; i < 12; i++ {
		r := httptest.NewRequest("POST", "http://c/collect", strings.NewReader(body))
		r.RemoteAddr = "203.0.113.50:1"
		r.Header.Set("Content-Type", "application/json")
		w := httptest.NewRecorder()
		h.ServeHTTP(w, r)
		if w.Code == http.StatusTooManyRequests && blockedAt < 0 {
			blockedAt = i
		}
	}
	if blockedAt < 0 {
		t.Fatal("control-plane flood was never rate-limited")
	}
	if blockedAt > 6 {
		t.Fatalf("flood should be dropped near the hard threshold (5), got first-429 at %d", blockedAt)
	}
	// The flood must NOT chain a record per request — only a sampled breach.
	breaches := 0
	for _, rec := range alog.Records() {
		if rec.EventType == audit.EventRateHardExceeded {
			breaches++
		}
	}
	if breaches != 1 {
		t.Fatalf("flood must be sampled (1 breach record), got %d — chain would bloat", breaches)
	}

	// A different IP is unaffected (the drop is per-source).
	r := httptest.NewRequest("POST", "http://c/collect", strings.NewReader(body))
	r.RemoteAddr = "198.51.100.9:1"
	r.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	h.ServeHTTP(w, r)
	if w.Code == http.StatusTooManyRequests {
		t.Fatal("an unrelated IP was collaterally rate-limited")
	}
}
