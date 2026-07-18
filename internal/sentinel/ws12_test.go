package sentinel

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
)

// WS1: the admin plane is NOT reachable on the public proxied listener — any
// /__hmn/admin/* there is 404, so origin-served (cross-origin) JS cannot reach it.
func TestAdminOffPublicListener(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	seedAdmins(t, srv)

	for _, p := range []string{"/__hmn/admin/audit", "/__hmn/admin/bans", "/__hmn/admin/console", "/__hmn/admin/integrity"} {
		w := httptest.NewRecorder()
		srv.ServeHTTP(w, httptest.NewRequest("GET", "http://public"+p, nil))
		if w.Code != http.StatusNotFound {
			t.Fatalf("%s on public listener must 404, got %d", p, w.Code)
		}
	}
}

// WS1: deny-by-default — unauthenticated or bad-token admin requests get 404.
func TestAdminDenyByDefault(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	seedAdmins(t, srv)

	if w := adminDo(srv, "", "GET", "/__hmn/admin/audit", ""); w.Code != http.StatusNotFound {
		t.Fatalf("no token must 404, got %d", w.Code)
	}
	if w := adminDo(srv, "wrong-token", "GET", "/__hmn/admin/audit", ""); w.Code != http.StatusNotFound {
		t.Fatalf("bad token must 404, got %d", w.Code)
	}
	// A failed auth is meta-audited.
	if !hasEvent(srv.sink.Log(), audit.EventAdminAuthFail) {
		t.Fatal("failed admin auth must be recorded")
	}
}

// WS2: RBAC — Auditor is read-only; only Operator/DPO may mutate.
func TestAdminRBAC(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	// Auditor CAN read.
	if w := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/bans", ""); w.Code != http.StatusOK {
		t.Fatalf("auditor read denied: %d", w.Code)
	}
	// Auditor CANNOT ban (403 — authenticated but not permitted).
	if w := adminDo(srv, toks[RoleAuditor], "POST", "/__hmn/admin/bans", `{"Key":"ip:1.2.3.4","DurationSec":3600}`); w.Code != http.StatusForbidden {
		t.Fatalf("auditor ban must be 403, got %d", w.Code)
	}
	// Operator CAN ban (temp).
	if w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans", `{"Key":"ip:1.2.3.4","DurationSec":3600}`); w.Code != http.StatusOK {
		t.Fatalf("operator temp ban denied: %d %s", w.Code, w.Body.String())
	}
}

// WS2: self-lift is closed — an unauthenticated (e.g. auto-banned) caller cannot
// lift a ban; only an authenticated Operator can.
func TestSelfLiftClosed(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	srv.bans.Add("ip:9.9.9.9", "auto", "sys", "", time.Hour)

	// Unauthenticated lift attempt (what an auto-banned attacker would try) → 404.
	if w := adminDo(srv, "", "POST", "/__hmn/admin/bans/lift?key=ip:9.9.9.9", ""); w.Code != http.StatusNotFound {
		t.Fatalf("unauthenticated self-lift must 404, got %d", w.Code)
	}
	if _, banned := srv.bans.Check("ip:9.9.9.9"); !banned {
		t.Fatal("ban must survive the unauthenticated lift attempt")
	}
	// Authenticated Operator can lift.
	if w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans/lift?key=ip:9.9.9.9", ""); w.Code != http.StatusOK {
		t.Fatalf("operator lift failed: %d", w.Code)
	}
}

// WS2: real two-phase dual-control for a permanent ban — request creates a
// pending action; a DISTINCT approver commits; the requester cannot self-approve.
func TestDualControlPermanentBan(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	// Operator requests a PERMANENT ban → pending, NOT applied yet.
	req := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans", `{"Key":"ip:5.5.5.5","DurationSec":0,"Reason":"abuse"}`)
	if req.Code != http.StatusOK || !strings.Contains(req.Body.String(), "approvalId") {
		t.Fatalf("permanent ban should be pending: %d %s", req.Code, req.Body.String())
	}
	if _, banned := srv.bans.Check("ip:5.5.5.5"); banned {
		t.Fatal("permanent ban must NOT be applied before approval")
	}
	id := extractField(req.Body.String(), "approvalId")

	// Requester self-approval is rejected.
	if w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/approvals/"+id, ""); w.Code == http.StatusOK {
		t.Fatal("requester must not self-approve")
	}
	// A distinct Approver commits it.
	if w := adminDo(srv, toks[RoleApprover], "POST", "/__hmn/admin/approvals/"+id, ""); w.Code != http.StatusOK {
		t.Fatalf("approver commit failed: %d %s", w.Code, w.Body.String())
	}
	if _, banned := srv.bans.Check("ip:5.5.5.5"); !banned {
		t.Fatal("ban must be applied after distinct approval")
	}
}

// The ApprovalStore enforces distinct-approver, role, and expiry directly.
func TestApprovalStoreRules(t *testing.T) {
	s := NewApprovalStore([]byte("k"), time.Minute)
	now := time.Unix(1000, 0)
	s.nowFn = func() time.Time { return now }
	p := s.Create("ban", map[string]string{"key": "ip:1.1.1.1"}, "op-1", RoleApprover)

	if _, _, err := s.Approve(p.ID, Operator{ID: "op-1", Role: RoleApprover}); err != errSelfApproval {
		t.Fatalf("self-approval must be rejected, got %v", err)
	}
	if _, _, err := s.Approve(p.ID, Operator{ID: "op-2", Role: RoleAuditor}); err != errWrongRole {
		t.Fatalf("wrong role must be rejected, got %v", err)
	}
	if _, tk, err := s.Approve(p.ID, Operator{ID: "op-2", Role: RoleApprover}); err != nil || tk == "" {
		t.Fatalf("distinct approver should succeed with a ticket, got %v", err)
	}
	if _, _, err := s.Approve(p.ID, Operator{ID: "op-2", Role: RoleApprover}); err != errNoPending {
		t.Fatalf("a committed action must be gone, got %v", err)
	}
}

// extractField pulls a "field":"value" string out of a small JSON body (test util).
func extractField(body, field string) string {
	i := strings.Index(body, "\""+field+"\":\"")
	if i < 0 {
		return ""
	}
	rest := body[i+len(field)+4:]
	j := strings.IndexByte(rest, '"')
	if j < 0 {
		return ""
	}
	return rest[:j]
}
