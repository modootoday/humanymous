package sentinel

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// WS9: the kill switch is dual-controlled and, once committed, globally
// suppresses enforcement to monitor mode — a previously-DENY session passes.
func TestKillSwitchDualControlAndEffect(t *testing.T) {
	origin := 0
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { origin++; w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	// A session with a scored DENY verdict is denied at the edge verdict gate.
	sid := "denysess"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictDeny, risk: 90, rule: "HR-7", updated: srv.nowFn()})
	mkReq := func() *http.Request {
		r := httptest.NewRequest("GET", "http://p/", nil)
		r.RemoteAddr = "198.51.100.5:1"
		r.Header.Set("Cookie", "hsid="+sid)
		r.Header.Set("User-Agent", "Chrome/126")
		return r
	}
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, mkReq())
	if w.Code != http.StatusForbidden {
		t.Fatalf("scored-DENY session should be denied pre-kill-switch: %d", w.Code)
	}

	// Operator requests the kill switch → pending (not yet engaged).
	req := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/killswitch", `{"On":true}`)
	if req.Code != http.StatusOK || !strings.Contains(req.Body.String(), "approvalId") {
		t.Fatalf("kill-switch request should be pending: %d %s", req.Code, req.Body.String())
	}
	id := extractField(req.Body.String(), "approvalId")
	if srv.monitorOn() {
		t.Fatal("kill switch must NOT engage before approval")
	}
	// Requester cannot self-approve.
	if w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/approvals/"+id, ""); w.Code == http.StatusOK {
		t.Fatal("requester must not self-approve the kill switch")
	}
	// Distinct approver commits → kill switch engaged.
	if w := adminDo(srv, toks[RoleApprover], "POST", "/__hmn/admin/approvals/"+id, ""); w.Code != http.StatusOK {
		t.Fatalf("approver commit failed: %d %s", w.Code, w.Body.String())
	}
	if !srv.monitorOn() {
		t.Fatal("kill switch must be engaged after approval")
	}

	// Now enforcement is globally suppressed: the scored-DENY session passes.
	w2 := httptest.NewRecorder()
	srv.ServeHTTP(w2, mkReq())
	if w2.Code == http.StatusForbidden {
		t.Fatal("with the kill switch engaged, the verdict gate must not enforce (monitor mode)")
	}
}
