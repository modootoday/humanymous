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

func banStack(t *testing.T, upstream string, hard int) (*Server, *audit.Log) {
	t.Helper()
	alog := audit.NewLog(audit.Config{NodeID: "test", HMACKey: []byte("k"), CheckpointEvery: 8})
	sink := audit.NewSink(alog)
	vault := audit.NewVault()
	store := collector.NewStore(time.Minute)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(time.Minute)
	control := NewControlPlane(store, engine, verdicts, sink, vault)
	srv, err := NewServer(Config{Upstream: upstream, NodeID: "test", ControlPath: "/__hmn/",
		RateWindow: time.Minute, RateSoft: hard - 2, RateHard: hard}, sink, vault, verdicts, control.Handler())
	if err != nil {
		t.Fatal(err)
	}
	return srv, alog
}

// a flood from one IP past the hard threshold auto-bans it; subsequent requests
// are dropped at the edge without contacting the origin (SoT-27 §2-3).
func TestAutoBanEndToEnd(t *testing.T) {
	originHits := 0
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		originHits++
		w.Write([]byte("ok"))
	}))
	defer up.Close()
	srv, alog := banStack(t, up.URL, 10)

	blockedAt := -1
	for i := 0; i < 20; i++ {
		r := httptest.NewRequest("GET", "http://p/data", nil)
		r.RemoteAddr = "203.0.113.9:5000"
		w := httptest.NewRecorder()
		srv.ServeHTTP(w, r)
		if w.Code == http.StatusForbidden && blockedAt < 0 {
			blockedAt = i
		}
	}
	if blockedAt < 0 {
		t.Fatal("flood was never auto-banned")
	}
	// After the ban, the origin must not be hit again.
	hitsAfterBan := originHits
	r := httptest.NewRequest("GET", "http://p/data", nil)
	r.RemoteAddr = "203.0.113.9:5000"
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)
	if w.Code != http.StatusForbidden {
		t.Fatalf("banned IP not blocked: %d", w.Code)
	}
	if originHits != hitsAfterBan {
		t.Fatal("origin contacted for a banned IP")
	}
	// A ban.applied event must be in the audit chain, and it must verify.
	if !hasEvent(alog, audit.EventBanApplied) {
		t.Fatal("no ban.applied event recorded")
	}
	if res := audit.Verify(alog.Records(), alog.Checkpoints(), []byte("k"), alog.PublicKey()); !res.OK {
		t.Fatalf("audit chain broken: %s", res.Class)
	}

	// A different IP is unaffected.
	r2 := httptest.NewRequest("GET", "http://p/data", nil)
	r2.RemoteAddr = "198.51.100.2:5000"
	w2 := httptest.NewRecorder()
	srv.ServeHTTP(w2, r2)
	if w2.Code == http.StatusForbidden {
		t.Fatal("an unrelated IP was collaterally banned")
	}
}

// a flood that ROTATES its IP but keeps one fingerprint is auto-banned by the
// fingerprint key, and a fresh IP carrying the same fingerprint is then blocked
// (SoT-27 §1, defeats residential-proxy rotation).
func TestFingerprintAutoBan(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 10)

	bh := map[string]string{
		"User-Agent":      "Mozilla/5.0 Chrome/126.0 Safari/537.36",
		"Accept-Language": "en-US,en;q=0.9",
		"sec-ch-ua":       `"Chromium";v="126"`,
	}
	// Flood from many different IPs, one shared fingerprint.
	for i := 0; i < 20; i++ {
		r := httptest.NewRequest("GET", "http://p/data", nil)
		r.RemoteAddr = "10.0." + itoaInt(i/250) + "." + itoaInt(i%250) + ":4000" // rotating IP
		for k, v := range bh {
			r.Header.Set(k, v)
		}
		w := httptest.NewRecorder()
		srv.ServeHTTP(w, r)
	}
	// A brand-new IP with the same fingerprint must be blocked by the fp ban.
	r := httptest.NewRequest("GET", "http://p/data", nil)
	r.RemoteAddr = "172.16.9.9:4000" // never seen before
	for k, v := range bh {
		r.Header.Set(k, v)
	}
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)
	if w.Code != http.StatusForbidden {
		t.Fatalf("rotated-IP flood not caught by fingerprint ban: %d", w.Code)
	}
	// Confirm a fp: ban exists.
	var fpBan bool
	for _, b := range srv.bans.List() {
		if strings.HasPrefix(b.Key, "fp:") {
			fpBan = true
		}
	}
	if !fpBan {
		t.Fatal("expected a fingerprint-keyed ban")
	}
}

// the console-driven manual ban flow (authenticated): add temp -> enforced ->
// list -> lift -> clear (SoT-28 WS2).
func TestConsoleBanManagement(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	// An authenticated Operator adds a temporary ban directly.
	aw := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans",
		`{"Key":"ip:192.0.2.44","Reason":"manual investigation","DurationSec":3600,"Incident":"INC-9"}`)
	if aw.Code != http.StatusOK {
		t.Fatalf("admin add temp ban failed: %d %s", aw.Code, aw.Body.String())
	}

	// That IP is now blocked on proxied traffic.
	r := httptest.NewRequest("GET", "http://p/", nil)
	r.RemoteAddr = "192.0.2.44:3000"
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)
	if w.Code != http.StatusForbidden {
		t.Fatalf("manually-banned IP not blocked: %d", w.Code)
	}
	if ra := w.Header().Get("Retry-After"); ra == "" {
		t.Fatal("temporary ban should carry Retry-After")
	}

	// Console lists it.
	lw := adminDo(srv, toks[RoleOperator], "GET", "/__hmn/admin/bans", "")
	if !strings.Contains(lw.Body.String(), "192.0.2.44") {
		t.Fatalf("ban not listed: %s", lw.Body.String())
	}

	// Operator lifts it (authenticated — an unauthenticated caller 404s).
	sw := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans/lift?key=ip:192.0.2.44", "")
	if sw.Code != http.StatusOK {
		t.Fatalf("lift failed: %d %s", sw.Code, sw.Body.String())
	}

	// Now the IP passes again.
	r2 := httptest.NewRequest("GET", "http://p/", nil)
	r2.RemoteAddr = "192.0.2.44:3000"
	w2 := httptest.NewRecorder()
	srv.ServeHTTP(w2, r2)
	if w2.Code == http.StatusForbidden {
		t.Fatal("lifted ban still blocking")
	}
}

func hasEvent(l *audit.Log, evt string) bool {
	for _, r := range l.Records() {
		if r.EventType == evt {
			return true
		}
	}
	return false
}
