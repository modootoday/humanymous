package gate

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/gate/settings"
)

func attachSettingsOverlay(t *testing.T, srv *Server, ov *settings.Overlay) *settings.FileStore {
	t.Helper()
	st, err := settings.NewFileStore(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	if ov != nil {
		ov.SchemaVersion = "1.0.0"
		ov.Status = "active"
		if ov.OverlayID == "" {
			ov.OverlayID = "ovl_test"
		}
		if err := st.SetActive(ov); err != nil {
			t.Fatal(err)
		}
	}
	if err := srv.SetSettingsStore(st); err != nil {
		t.Fatal(err)
	}
	return st
}

func TestSettingsRateHotApplyPreservesCounters(t *testing.T) {
	srv, _ := testSettingsServer(t)
	srv.cfg.RateWindow = time.Minute
	srv.cfg.RateSoft = 5
	srv.cfg.RateHard = 10
	srv.bans.(RateConfigurableBanLedger).ConfigureRate(time.Minute, 5, 10)

	const key = "ip:198.51.100.39"
	for i := 0; i < 4; i++ {
		if _, banned, _ := srv.bans.Observe(key); banned {
			t.Fatal("pre-apply count unexpectedly banned")
		}
	}
	eff := srv.SettingsEffective()
	body, _ := json.Marshal(map[string]any{
		"parentConfigVersion": eff.ConfigVersion,
		"rateLimit":           map[string]any{"hard": 5},
	})
	req := httptest.NewRequest(http.MethodPost, "/settings/overlays", bytes.NewReader(body))
	w := httptest.NewRecorder()
	srv.adminSettingsPropose(w, req, Operator{ID: "op-1", Role: RoleOperator})
	if w.Code != http.StatusOK {
		t.Fatalf("apply status=%d body=%s", w.Code, w.Body.String())
	}
	if _, banned, level := srv.bans.Observe(key); !banned || level != 2 {
		t.Fatalf("hot apply reset hits or missed hard=5: banned=%v level=%d", banned, level)
	}
}

func TestSettingsGateBanMonitorIsAuditOnly(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_, _ = w.Write([]byte("origin"))
	}))
	defer up.Close()
	srv, _ := buildStackWith(t, up.URL, Config{RateWindow: time.Minute, RateSoft: 1, RateHard: 2})
	attachSettingsOverlay(t, srv, &settings.Overlay{
		Gates: map[string]settings.Mode{"gate.ban": settings.ModeMonitor},
	})

	for i := 0; i < 4; i++ {
		r := httptest.NewRequest(http.MethodGet, "http://proxy/data", nil)
		r.RemoteAddr = "198.51.100.44:1234"
		w := httptest.NewRecorder()
		srv.ServeHTTP(w, r)
		if w.Code != http.StatusOK {
			t.Fatalf("monitor request %d blocked: %d", i, w.Code)
		}
	}
	if got := len(srv.bans.List()); got != 0 {
		t.Fatalf("monitor mode created ban state: %d", got)
	}
	found := false
	for _, rec := range srv.sink.Log().Records() {
		if rec.EventType == audit.EventRateHardExceeded && rec.Mode == "monitor" && rec.Action == "would_auto_ban" {
			found = true
		}
	}
	if !found {
		t.Fatal("missing audit-only hard-rate record")
	}
}

func TestSettingsVerdictTokenMonitorDoesNotFastPath(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_, _ = w.Write([]byte("origin"))
	}))
	defer up.Close()
	key := []byte("test-token-key-32-bytes-long!!!!")
	srv, _ := buildStackWith(t, up.URL, Config{TokenKey: key})
	attachSettingsOverlay(t, srv, &settings.Overlay{
		Gates: map[string]settings.Mode{"gate.verdict_token": settings.ModeMonitor},
	})

	sid := "token-monitor"
	now := srv.nowFn()
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictDeny, updated: now})
	r := aReq(http.MethodGet, "/", "198.51.100.45:1234", "Chrome/126")
	token := issueVerdictToken(key, sid, tokenBind(r), srv.tokenEpochs.Current(), now.Add(time.Minute))
	r.Header.Set("Cookie", "hsid="+sid+"; "+verdictCookie+"="+token)
	w := serve(srv, r)
	if w.Code != http.StatusForbidden {
		t.Fatalf("monitor token must fall through to sticky verdict, got %d", w.Code)
	}
	found := false
	for _, rec := range srv.sink.Log().Records() {
		if rec.EventType == audit.EventEnfAllow && rec.Mode == "monitor" && rec.Action == "would_fast_path" {
			found = true
		}
	}
	if !found {
		t.Fatal("missing verdict-token would-fast-path audit")
	}
}

func TestSettingsReconMonitorAuditsWithoutBlocking(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_, _ = w.Write([]byte("origin"))
	}))
	defer up.Close()
	srv, _ := buildStackWith(t, up.URL, Config{})
	attachSettingsOverlay(t, srv, &settings.Overlay{
		Gates: map[string]settings.Mode{"gate.recon_sweep": settings.ModeMonitor},
	})

	for i := 0; i < 10; i++ {
		r := aReq(http.MethodGet, "/", "198.51.100.46:1234", "Chrome/126", "hsid=s"+itoaInt(i))
		w := serve(srv, r)
		if w.Code != http.StatusOK {
			t.Fatalf("monitor sweep request %d blocked: %d", i, w.Code)
		}
	}
	found := false
	for _, rec := range srv.sink.Log().Records() {
		if rec.EventType == audit.EventReconProbing && rec.Mode == "monitor" && rec.Action == "would_block" {
			found = true
		}
	}
	if !found {
		t.Fatal("missing recon would-block audit")
	}
}

func TestSettingsInjectMonitorLeavesHTMLUntouched(t *testing.T) {
	up := htmlUpstream(t)
	defer up.Close()
	srv, _ := buildStackWith(t, up.URL, Config{})
	attachSettingsOverlay(t, srv, &settings.Overlay{
		Gates: map[string]settings.Mode{"gate.inject": settings.ModeMonitor},
	})

	w := do(srv, http.MethodGet, "/", "", "", nil)
	if w.Code != http.StatusOK || strings.Contains(w.Body.String(), injectMarker) {
		t.Fatalf("inject monitor changed response: status=%d body=%s", w.Code, w.Body.String())
	}
	found := false
	for _, rec := range srv.sink.Log().Records() {
		if rec.EventType == audit.EventInjectSkipped && rec.Mode == "monitor" &&
			strings.Contains(rec.FailReason, "would inject") {
			found = true
		}
	}
	if !found {
		t.Fatal("missing inject would-have audit")
	}
}

func TestSettingsAttestationFloorMonitorAuditsWithoutChallenging(t *testing.T) {
	srv, hits, _, _, _, _ := buildAttestStack(t, 0)
	attachSettingsOverlay(t, srv, &settings.Overlay{
		Gates: map[string]settings.Mode{"gate.attest_floor": settings.ModeMonitor},
	})

	sid := "floor-monitor"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictAllow, updated: srv.nowFn()})
	before := *hits
	w := serve(srv, aReq(http.MethodGet, "/transfer", "198.51.100.47:1234", "Chrome/126", "hsid="+sid))
	if w.Code != http.StatusOK || *hits != before+1 {
		t.Fatalf("monitor floor changed request outcome: status=%d hits+%d", w.Code, *hits-before)
	}
	found := false
	for _, rec := range srv.sink.Log().Records() {
		if rec.EventType == audit.EventEnfChallengeIssued && rec.Mode == "monitor" && rec.Action == "would_challenge" {
			found = true
		}
	}
	if !found {
		t.Fatal("missing attestation-floor would-challenge audit")
	}
}

func TestSettingsAttestationFloorMonitorAllowsVerdictTokenFastPath(t *testing.T) {
	srv, hits, key, epochs, _, _ := buildAttestStack(t, 0)
	attachSettingsOverlay(t, srv, &settings.Overlay{
		Gates: map[string]settings.Mode{"gate.attest_floor": settings.ModeMonitor},
	})

	sid := "floor-token-monitor"
	now := srv.nowFn()
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictDeny, updated: now})
	r := aReq(http.MethodGet, "/transfer", "198.51.100.48:1234", "Chrome/126")
	token := issueVerdictToken(key, sid, tokenBind(r), epochs.Current(), now.Add(time.Minute))
	r.Header.Set("Cookie", "hsid="+sid+"; "+verdictCookie+"="+token)
	before := *hits
	w := serve(srv, r)
	if w.Code != http.StatusOK || *hits != before+1 {
		t.Fatalf("monitor floor blocked verdict-token fast path: status=%d hits+%d", w.Code, *hits-before)
	}
}

func TestSettingsMetricsReflectApply(t *testing.T) {
	srv, _ := testSettingsServer(t)
	eff := srv.SettingsEffective()
	body, _ := json.Marshal(map[string]any{
		"parentConfigVersion": eff.ConfigVersion,
		"scoring":             map[string]any{"challengeAt": 20},
	})
	req := httptest.NewRequest(http.MethodPost, "/settings/overlays", bytes.NewReader(body))
	w := httptest.NewRecorder()
	srv.adminSettingsPropose(w, req, Operator{ID: "op-1", Role: RoleOperator})
	if w.Code != http.StatusOK {
		t.Fatalf("apply status=%d body=%s", w.Code, w.Body.String())
	}

	metrics := httptest.NewRecorder()
	srv.adminMetrics(metrics)
	text := metrics.Body.String()
	for _, want := range []string{
		"hmn_gate_settings_overlay_active 1",
		`hmn_gate_settings_apply_total{result="applied"} 1`,
		`hmn_gate_config_version{version="cfg-`,
	} {
		if !strings.Contains(text, want) {
			t.Fatalf("metrics missing %q\n%s", want, text)
		}
	}
}
