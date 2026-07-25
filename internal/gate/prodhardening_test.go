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

// buildStackWith wires a full proxy in front of an httptest upstream with a
// caller-supplied Config (Upstream/ControlPath filled in), returning the server
// and its control plane for the production-hardening tests.
func buildStackWith(t *testing.T, upstream string, cfg Config) (*Server, *ControlPlane) {
	t.Helper()
	alog := audit.NewLog(audit.Config{NodeID: "test", HMACKey: []byte("k"), CheckpointEvery: 4})
	sink := audit.NewSink(alog)
	vault := audit.NewVault()
	store := collector.NewStore(time.Minute)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(time.Minute)
	control := NewControlPlane(store, engine, verdicts, sink, vault)
	cfg.Upstream = upstream
	cfg.NodeID = "test"
	cfg.ControlPath = "/__hmn/"
	srv, err := NewServer(cfg, sink, vault, verdicts, control.Handler())
	if err != nil {
		t.Fatal(err)
	}
	return srv, control
}

func htmlUpstream(t *testing.T) *httptest.Server {
	t.Helper()
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html")
		_, _ = w.Write([]byte("<html><head></head><body>ORIGIN</body></html>"))
	}))
}

// Add-only security headers appear on every proxied response; HSTS only when opted in.
func TestSecurityHeadersAddOnly(t *testing.T) {
	up := htmlUpstream(t)
	defer up.Close()

	srv, _ := buildStackWith(t, up.URL, Config{})
	h := do(srv, "GET", "/", "", "", nil).Result().Header
	for _, k := range []string{"X-Content-Type-Options", "Referrer-Policy", "X-Frame-Options"} {
		if h.Get(k) == "" {
			t.Errorf("expected security header %s to be set", k)
		}
	}
	if h.Get("Strict-Transport-Security") != "" {
		t.Errorf("HSTS must be off by default, got %q", h.Get("Strict-Transport-Security"))
	}

	srvH, _ := buildStackWith(t, up.URL, Config{HSTS: true})
	if hs := do(srvH, "GET", "/", "", "", nil).Result().Header.Get("Strict-Transport-Security"); hs == "" {
		t.Error("expected HSTS header when -hsts is enabled")
	}
}

// A declared body over -max-body is rejected with 413 before it reaches origin;
// a body under the cap proxies through.
func TestMaxBodyCap(t *testing.T) {
	originHits := 0
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		originHits++
		w.Write([]byte("ok"))
	}))
	defer up.Close()

	srv, _ := buildStackWith(t, up.URL, Config{MaxBodyBytes: 16})

	// Over the cap (Content-Length 100 > 16) -> 413, origin untouched.
	big := do(srv, "POST", "/api/x", "", strings.Repeat("a", 100), nil)
	if big.Code != http.StatusRequestEntityTooLarge {
		t.Fatalf("over-cap body: want 413, got %d", big.Code)
	}
	if originHits != 0 {
		t.Fatalf("origin must not be hit for an over-cap body, got %d hits", originHits)
	}

	// Under the cap -> the body-size guard does NOT fire (413); the request then
	// follows the normal gate flow (a bare POST is fail-closed to a challenge, so we
	// only assert the cap itself did not reject it).
	small := do(srv, "POST", "/api/x", "", "hello", nil)
	if small.Code == http.StatusRequestEntityTooLarge {
		t.Fatalf("under-cap body wrongly rejected with 413 (code=%d)", small.Code)
	}
	_ = originHits
}

// The readiness probe flips to 503 after SetReady(false) so an LB drains the node.
func TestReadinessProbe(t *testing.T) {
	up := htmlUpstream(t)
	defer up.Close()
	srv, control := buildStackWith(t, up.URL, Config{})

	if w := do(srv, "GET", "/__hmn/readyz", "", "", nil); w.Code != http.StatusOK ||
		!strings.Contains(w.Body.String(), "ready") {
		t.Fatalf("ready: want 200+ready, got %d %s", w.Code, w.Body.String())
	}
	control.SetReady(false)
	if w := do(srv, "GET", "/__hmn/readyz", "", "", nil); w.Code != http.StatusServiceUnavailable ||
		!strings.Contains(w.Body.String(), "draining") {
		t.Fatalf("draining: want 503+draining, got %d %s", w.Code, w.Body.String())
	}
	// Liveness stays ok regardless of readiness.
	if w := do(srv, "GET", "/__hmn/healthz", "", "", nil); w.Code != http.StatusOK {
		t.Fatalf("healthz should stay 200 while draining, got %d", w.Code)
	}
}

// SoT-38 P0-5: NewServer must not cache integrity_ok=1 before any SelfVerify /
// RefreshIntegrityMetrics — a fresh empty chain is not green.
func TestIntegrityOKDefaultsFalseOnEmptyServer(t *testing.T) {
	up := htmlUpstream(t)
	defer up.Close()
	srv, _ := buildStackWith(t, up.URL, Config{})
	// Do NOT call RefreshIntegrityMetrics — read the construction-time cache.
	w := httptest.NewRecorder()
	srv.adminMetrics(w)
	body := w.Body.String()
	if !strings.Contains(body, "hmn_gate_audit_integrity_ok 0") {
		t.Fatalf("fresh server must report integrity_ok=0 before first refresh\n%s", body)
	}
}

// The metrics endpoint emits Prometheus text with the expected gate gauges.
func TestAdminMetricsExposition(t *testing.T) {
	up := htmlUpstream(t)
	defer up.Close()
	srv, _ := buildStackWith(t, up.URL, Config{})

	srv.RefreshIntegrityMetrics() // exercise the real (off-request-path) verify + cache
	w := httptest.NewRecorder()
	srv.adminMetrics(w)
	body := w.Body.String()
	if ct := w.Result().Header.Get("Content-Type"); !strings.HasPrefix(ct, "text/plain") {
		t.Errorf("metrics content-type: want text/plain, got %q", ct)
	}
	for _, want := range []string{
		"hmn_gate_uptime_seconds",
		"hmn_gate_audit_records_total",
		"hmn_gate_bans_active",
		"hmn_gate_killswitch",
		"hmn_gate_goroutines",
		"hmn_gate_audit_projection_dropped_total",
		"hmn_gate_audit_integrity_ok",
		"hmn_gate_audit_witnessed",
	} {
		if !strings.Contains(body, want) {
			t.Errorf("metrics missing %s\n%s", want, body)
		}
	}
	// SoT-38 P0-5: an empty chain is NOT a green integrity pass (vacuous green trap).
	if !strings.Contains(body, "hmn_gate_audit_integrity_ok 0") {
		t.Errorf("expected integrity ok=0 on an empty chain\n%s", body)
	}
	// After real records are sealed, RefreshIntegrityMetrics must flip to ok=1.
	srv.sink.Emit(audit.Record{EventType: "gate.decision", Verdict: "allow", Mode: "enforce", KeyID: "k1"})
	srv.sink.Emit(audit.Record{EventType: "gate.decision", Verdict: "allow", Mode: "enforce", KeyID: "k1"})
	srv.RefreshIntegrityMetrics()
	w2 := httptest.NewRecorder()
	srv.adminMetrics(w2)
	body2 := w2.Body.String()
	if !strings.Contains(body2, "hmn_gate_audit_integrity_ok 1") {
		t.Errorf("expected integrity ok=1 after records sealed\n%s", body2)
	}
}

// The audit query can be scoped to one pseudonymous subject (GDPR Art. 15 access request),
// matching session_pseudonym or id_pseudonym.
func TestAuditSubjectFilter(t *testing.T) {
	up := htmlUpstream(t)
	defer up.Close()
	srv, _ := buildStackWith(t, up.URL, Config{})

	srv.sink.Emit(audit.Record{EventType: "gate.decision", SessionPsn: "subjA", Verdict: "allow"})
	srv.sink.Emit(audit.Record{EventType: "gate.decision", SessionPsn: "subjB", Verdict: "deny"})
	srv.sink.Emit(audit.Record{EventType: "gate.decision", Actor: audit.Actor{Kind: "operator", IDPsn: "subjA"}, Verdict: "challenge"})

	w := httptest.NewRecorder()
	r := httptest.NewRequest("GET", "http://admin/__hmn/admin/audit?subject=subjA", nil)
	srv.adminAudit(w, r)
	body := w.Body.String()
	if !strings.Contains(body, "subjA") {
		t.Fatalf("subject filter should return subjA records: %s", body)
	}
	if strings.Contains(body, "subjB") {
		t.Errorf("subject filter must exclude other subjects: %s", body)
	}
	// Two records reference subjA (one by session, one by id pseudonym).
	if n := strings.Count(body, "subjA"); n < 2 {
		t.Errorf("expected both subjA records (session + id), got %d", n)
	}
}
