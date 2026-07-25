package gate

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/gate/settings"
	"github.com/modootoday/humanymous/internal/scoring"
)

func TestSettingsProposeClassAApplies(t *testing.T) {
	dir := t.TempDir()
	st, err := settings.NewFileStore(dir)
	if err != nil {
		t.Fatal(err)
	}
	key := []byte("test-token-key-32-bytes-long!!!!")
	alog := audit.NewLog(audit.Config{NodeID: "n1", HMACKey: key, CheckpointEvery: 32})
	sink := audit.NewSink(alog)
	store := collector.NewStore(0)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(0)
	vault := audit.NewVault()
	ctrl := NewControlPlane(store, engine, verdicts, sink, vault).WithTokenKey(key)
	srv, err := NewServer(Config{
		Upstream: "http://127.0.0.1:9", NodeID: "n1", TokenKey: key,
		Routes: map[string]string{"/": "balanced"},
	}, sink, vault, verdicts, ctrl.Handler())
	if err != nil {
		t.Fatal(err)
	}
	srv.SetSettingsStore(st)

	eff := srv.SettingsEffective()
	body := map[string]any{
		"parentConfigVersion": eff.ConfigVersion,
		"scoring":             map[string]any{"challengeAt": 20}, // tighten = class A
	}
	b, _ := json.Marshal(body)
	req := httptest.NewRequest(http.MethodPost, "/settings/overlays", bytes.NewReader(b))
	req.Header.Set("Content-Type", "application/json")
	rr := httptest.NewRecorder()
	srv.adminSettingsPropose(rr, req, Operator{ID: "op-1", Role: RoleOperator})
	if rr.Code != 200 {
		t.Fatalf("status %d body %s", rr.Code, rr.Body.String())
	}
	var out map[string]any
	_ = json.Unmarshal(rr.Body.Bytes(), &out)
	if out["applied"] != true {
		t.Fatalf("expected applied class A: %v", out)
	}
	if st.Active() == nil {
		t.Fatal("expected active overlay on disk")
	}
}

func TestSettingsProposeClassBPending(t *testing.T) {
	dir := t.TempDir()
	st, _ := settings.NewFileStore(dir)
	key := []byte("test-token-key-32-bytes-long!!!!")
	alog := audit.NewLog(audit.Config{NodeID: "n1", HMACKey: key, CheckpointEvery: 32})
	sink := audit.NewSink(alog)
	store := collector.NewStore(0)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(0)
	vault := audit.NewVault()
	ctrl := NewControlPlane(store, engine, verdicts, sink, vault).WithTokenKey(key)
	srv, err := NewServer(Config{
		Upstream: "http://127.0.0.1:9", NodeID: "n1", TokenKey: key,
		Routes: map[string]string{"/": "balanced"},
	}, sink, vault, verdicts, ctrl.Handler())
	if err != nil {
		t.Fatal(err)
	}
	srv.SetSettingsStore(st)

	eff := srv.SettingsEffective()
	body := map[string]any{
		"parentConfigVersion": eff.ConfigVersion,
		"hardRules":           map[string]string{"HR-12": "monitor"},
	}
	b, _ := json.Marshal(body)
	req := httptest.NewRequest(http.MethodPost, "/settings/overlays", bytes.NewReader(b))
	rr := httptest.NewRecorder()
	srv.adminSettingsPropose(rr, req, Operator{ID: "op-1", Role: RoleOperator})
	if rr.Code != 200 {
		t.Fatalf("status %d %s", rr.Code, rr.Body.String())
	}
	var out map[string]any
	_ = json.Unmarshal(rr.Body.Bytes(), &out)
	if out["pending"] != true {
		t.Fatalf("want pending B: %v", out)
	}
	if st.Active() != nil {
		t.Fatal("B must not apply before approval")
	}
	// dual-control commit
	aid, _ := out["approvalId"].(string)
	appr := Operator{ID: "appr-1", Role: RoleApprover}
	rr2 := httptest.NewRecorder()
	srv.adminApprove(rr2, aid, appr)
	if rr2.Code != 200 {
		t.Fatalf("approve %d %s", rr2.Code, rr2.Body.String())
	}
	if st.Active() == nil || st.Active().HardRules["HR-12"] != settings.ModeMonitor {
		t.Fatalf("expected active HR-12 monitor: %+v", st.Active())
	}
}

func TestSettingsCASStale(t *testing.T) {
	dir := t.TempDir()
	st, _ := settings.NewFileStore(dir)
	key := []byte("test-token-key-32-bytes-long!!!!")
	alog := audit.NewLog(audit.Config{NodeID: "n1", HMACKey: key, CheckpointEvery: 32})
	sink := audit.NewSink(alog)
	store := collector.NewStore(0)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(0)
	vault := audit.NewVault()
	ctrl := NewControlPlane(store, engine, verdicts, sink, vault).WithTokenKey(key)
	srv, _ := NewServer(Config{
		Upstream: "http://127.0.0.1:9", NodeID: "n1", TokenKey: key,
		Routes: map[string]string{"/": "balanced"},
	}, sink, vault, verdicts, ctrl.Handler())
	srv.SetSettingsStore(st)

	body := map[string]any{
		"parentConfigVersion": "cfg-stale",
		"scoring":             map[string]any{"challengeAt": 20},
	}
	b, _ := json.Marshal(body)
	req := httptest.NewRequest(http.MethodPost, "/settings/overlays", bytes.NewReader(b))
	rr := httptest.NewRecorder()
	srv.adminSettingsPropose(rr, req, Operator{ID: "op-1", Role: RoleOperator})
	if rr.Code != http.StatusConflict {
		t.Fatalf("want 409 stale CAS, got %d %s", rr.Code, rr.Body.String())
	}
}

func TestSettingsClassCRequiresConfirm(t *testing.T) {
	dir := t.TempDir()
	st, _ := settings.NewFileStore(dir)
	key := []byte("test-token-key-32-bytes-long!!!!")
	alog := audit.NewLog(audit.Config{NodeID: "n1", HMACKey: key, CheckpointEvery: 32})
	sink := audit.NewSink(alog)
	store := collector.NewStore(0)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(0)
	vault := audit.NewVault()
	ctrl := NewControlPlane(store, engine, verdicts, sink, vault).WithTokenKey(key)
	srv, _ := NewServer(Config{
		Upstream: "http://127.0.0.1:9", NodeID: "n1", TokenKey: key,
		Routes: map[string]string{"/": "balanced"},
	}, sink, vault, verdicts, ctrl.Handler())
	srv.SetSettingsStore(st)

	eff := srv.SettingsEffective()
	body := map[string]any{
		"parentConfigVersion": eff.ConfigVersion,
		"hardRules":           map[string]string{"HR-18": "monitor"},
	}
	b, _ := json.Marshal(body)
	req := httptest.NewRequest(http.MethodPost, "/settings/overlays", bytes.NewReader(b))
	rr := httptest.NewRecorder()
	srv.adminSettingsPropose(rr, req, Operator{ID: "op-1", Role: RoleOperator})
	if rr.Code != http.StatusBadRequest {
		t.Fatalf("want 400 missing DISABLE-INTEGRITY, got %d %s", rr.Code, rr.Body.String())
	}
}

func testSettingsServer(t *testing.T) (*Server, *settings.FileStore) {
	t.Helper()
	dir := t.TempDir()
	st, err := settings.NewFileStore(dir)
	if err != nil {
		t.Fatal(err)
	}
	key := []byte("test-token-key-32-bytes-long!!!!")
	alog := audit.NewLog(audit.Config{NodeID: "n1", HMACKey: key, CheckpointEvery: 32})
	sink := audit.NewSink(alog)
	store := collector.NewStore(0)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(0)
	vault := audit.NewVault()
	ctrl := NewControlPlane(store, engine, verdicts, sink, vault).WithTokenKey(key)
	srv, err := NewServer(Config{
		Upstream: "http://127.0.0.1:9", NodeID: "n1", TokenKey: key,
		Routes: map[string]string{"/": "balanced", "/checkout": "strict"},
	}, sink, vault, verdicts, ctrl.Handler())
	if err != nil {
		t.Fatal(err)
	}
	srv.SetSettingsStore(st)
	return srv, st
}

func TestSettingsDryRunLowNUnusable(t *testing.T) {
	srv, _ := testSettingsServer(t)
	eff := srv.SettingsEffective()
	body := map[string]any{
		"parentConfigVersion": eff.ConfigVersion,
		"scoring":             map[string]any{"challengeAt": 40},
	}
	b, _ := json.Marshal(body)
	req := httptest.NewRequest(http.MethodPost, "/settings/dry-run", bytes.NewReader(b))
	rr := httptest.NewRecorder()
	srv.adminSettingsDryRun(rr, req, Operator{ID: "op-1", Role: RoleOperator})
	if rr.Code != 200 {
		t.Fatalf("status %d %s", rr.Code, rr.Body.String())
	}
	var out map[string]any
	_ = json.Unmarshal(rr.Body.Bytes(), &out)
	if out["usable"] != false {
		t.Fatalf("want usable=false with empty audit, got %v", out)
	}
	if _, ok := out["allowDelta"]; ok {
		t.Fatal("must not claim numeric deltas when n < minN")
	}
}

func TestSettingsRollbackClearBClassOverlayIsA(t *testing.T) {
	srv, st := testSettingsServer(t)
	// Apply class A first so there is something to roll back without dual-control.
	eff := srv.SettingsEffective()
	body := map[string]any{
		"parentConfigVersion": eff.ConfigVersion,
		"scoring":             map[string]any{"challengeAt": 20},
	}
	b, _ := json.Marshal(body)
	req := httptest.NewRequest(http.MethodPost, "/settings/overlays", bytes.NewReader(b))
	rr := httptest.NewRecorder()
	srv.adminSettingsPropose(rr, req, Operator{ID: "op-1", Role: RoleOperator})
	if rr.Code != 200 || st.Active() == nil {
		t.Fatalf("setup apply failed: %d %s", rr.Code, rr.Body.String())
	}
	// Class A active → rollback undoes tighten = class B path (dual-control pending).
	// Force MutationClass on disk-active to A (propose already set it).
	if st.Active().MutationClass != settings.ClassA {
		t.Fatalf("setup want class A got %s", st.Active().MutationClass)
	}
	eff2 := srv.SettingsEffective()
	rb, _ := json.Marshal(map[string]any{"parentConfigVersion": eff2.ConfigVersion})
	req2 := httptest.NewRequest(http.MethodPost, "/settings/rollback", bytes.NewReader(rb))
	rr2 := httptest.NewRecorder()
	srv.adminSettingsRollback(rr2, req2, Operator{ID: "op-1", Role: RoleOperator})
	if rr2.Code != 200 {
		t.Fatalf("rollback %d %s", rr2.Code, rr2.Body.String())
	}
	var out map[string]any
	_ = json.Unmarshal(rr2.Body.Bytes(), &out)
	if out["pending"] != true {
		t.Fatalf("undo tighten should need Approver: %v", out)
	}
	if st.Active() == nil {
		t.Fatal("must not clear before Approver")
	}
	// Approver commits
	aid, _ := out["approvalId"].(string)
	rr3 := httptest.NewRecorder()
	srv.adminApprove(rr3, aid, Operator{ID: "appr-1", Role: RoleApprover})
	if rr3.Code != 200 {
		t.Fatalf("approve rollback %d %s", rr3.Code, rr3.Body.String())
	}
	if st.Active() != nil {
		t.Fatal("expected empty after rollback commit")
	}
}

func TestSettingsRollbackDemotionIsImmediate(t *testing.T) {
	srv, st := testSettingsServer(t)
	// Land a class B demotion via dual-control.
	eff := srv.SettingsEffective()
	body := map[string]any{
		"parentConfigVersion": eff.ConfigVersion,
		"hardRules":           map[string]string{"HR-12": "monitor"},
	}
	b, _ := json.Marshal(body)
	req := httptest.NewRequest(http.MethodPost, "/settings/overlays", bytes.NewReader(b))
	rr := httptest.NewRecorder()
	srv.adminSettingsPropose(rr, req, Operator{ID: "op-1", Role: RoleOperator})
	var out map[string]any
	_ = json.Unmarshal(rr.Body.Bytes(), &out)
	aid, _ := out["approvalId"].(string)
	rrA := httptest.NewRecorder()
	srv.adminApprove(rrA, aid, Operator{ID: "appr-1", Role: RoleApprover})
	if st.Active() == nil {
		t.Fatal("B not active")
	}
	// Undo demotion = tighten → Operator alone.
	eff2 := srv.SettingsEffective()
	rb, _ := json.Marshal(map[string]any{"parentConfigVersion": eff2.ConfigVersion})
	req2 := httptest.NewRequest(http.MethodPost, "/settings/rollback", bytes.NewReader(rb))
	rr2 := httptest.NewRecorder()
	srv.adminSettingsRollback(rr2, req2, Operator{ID: "op-1", Role: RoleOperator})
	if rr2.Code != 200 {
		t.Fatalf("rollback %d %s", rr2.Code, rr2.Body.String())
	}
	var out2 map[string]any
	_ = json.Unmarshal(rr2.Body.Bytes(), &out2)
	if out2["rolledBack"] != true {
		t.Fatalf("want immediate rollBack: %v", out2)
	}
	if st.Active() != nil {
		t.Fatal("expected cleared")
	}
}

func TestSettingsProposeNetPolicyAndRoutes(t *testing.T) {
	srv, st := testSettingsServer(t)
	eff := srv.SettingsEffective()
	// net.proxy.hop monitor from code default enforce = class B (pending)
	// Actually empty overlay code path: net missing → for classify, cur=ModeMonitor for non-correlation
	// demoting monitor→monitor is noop class A; enforcing hop from monitor is A.
	// net.correlation monitor from enforce = class C
	body := map[string]any{
		"parentConfigVersion": eff.ConfigVersion,
		"netPolicy":           map[string]string{"net.proxy.hop": "enforce"},
		"routes":              map[string]string{"/checkout": "attested"},
	}
	b, _ := json.Marshal(body)
	req := httptest.NewRequest(http.MethodPost, "/settings/overlays", bytes.NewReader(b))
	rr := httptest.NewRecorder()
	srv.adminSettingsPropose(rr, req, Operator{ID: "op-1", Role: RoleOperator})
	if rr.Code != 200 {
		t.Fatalf("status %d %s", rr.Code, rr.Body.String())
	}
	var out map[string]any
	_ = json.Unmarshal(rr.Body.Bytes(), &out)
	// tighten routes + net enforce = class A → applied
	if out["applied"] != true {
		t.Fatalf("want class A apply: %v", out)
	}
	if st.Active() == nil || st.Active().Routes["/checkout"] != "attested" {
		t.Fatalf("routes not stored: %+v", st.Active())
	}
	// Runtime resolve must see overlay route
	rp := srv.resolvePath("/checkout")
	if rp.name != "attested" {
		t.Fatalf("resolvePath want attested got %s", rp.name)
	}
}

func TestSettingsProposeRateLoosenIsB(t *testing.T) {
	srv, _ := testSettingsServer(t)
	eff := srv.SettingsEffective()
	hard := eff.RateHard + 50
	body := map[string]any{
		"parentConfigVersion": eff.ConfigVersion,
		"rateLimit":           map[string]any{"hard": hard},
	}
	b, _ := json.Marshal(body)
	req := httptest.NewRequest(http.MethodPost, "/settings/overlays", bytes.NewReader(b))
	rr := httptest.NewRecorder()
	srv.adminSettingsPropose(rr, req, Operator{ID: "op-1", Role: RoleOperator})
	if rr.Code != 200 {
		t.Fatalf("%d %s", rr.Code, rr.Body.String())
	}
	var out map[string]any
	_ = json.Unmarshal(rr.Body.Bytes(), &out)
	if out["pending"] != true || out["class"] != "B" {
		t.Fatalf("rate loosen want pending B: %v", out)
	}
}

func TestConfigureEngineFromEffectiveNetPolicy(t *testing.T) {
	e := scoring.NewEngine()
	eff := settings.Resolve(settings.BootInput{
		HMACKey: []byte("k"), RateWindowSec: 10, RateSoft: 60, RateHard: 120,
	}, &settings.Overlay{
		SchemaVersion: "1.0.0", OverlayID: "ovl_n", Status: "active",
		NetPolicy: map[string]settings.Mode{"net.proxy.hop": settings.ModeMonitor},
		HardRules: map[string]settings.Mode{"HR-12": settings.ModeMonitor},
	})
	ConfigureEngineFromEffective(e, eff)
	if e.RuleModes["HR-12"] != "monitor" {
		t.Fatalf("rule modes: %+v", e.RuleModes)
	}
	if e.NetPolicy["net.proxy.hop"] != "monitor" {
		t.Fatalf("net policy: %+v", e.NetPolicy)
	}
	if e.Policy.Version != "1.0.0+overlay" {
		t.Fatalf("policy version: %s", e.Policy.Version)
	}
}
