package gate

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// Wargame r01 (blue): permanent ban request ignores body Approver/Operator fields —
// dual-control commit is only via Approvals with a distinct Approver bearer.
func TestPermanentBanIgnoresBodyApprover(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	// Spoof body claims Approver is already present — must still only PENDING.
	body := `{"Key":"ip:198.51.100.77","Reason":"wargame r01","DurationSec":0,"Operator":"evil","Approver":"already-approved"}`
	aw := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans", body)
	if aw.Code != http.StatusOK {
		t.Fatalf("permanent ban request: %d %s", aw.Code, aw.Body.String())
	}
	var resp struct {
		Pending    bool   `json:"pending"`
		ApprovalID string `json:"approvalId"`
		NeedsRole  string `json:"needsRole"`
	}
	if err := json.Unmarshal(aw.Body.Bytes(), &resp); err != nil {
		t.Fatalf("json: %v", err)
	}
	if !resp.Pending || resp.ApprovalID == "" {
		t.Fatalf("must be pending dual-control, got %s", aw.Body.String())
	}
	if resp.NeedsRole != string(RoleApprover) {
		t.Fatalf("needsRole=%q want approver", resp.NeedsRole)
	}
	// Key must NOT yet be banned (pending only).
	r := httptest.NewRequest("GET", "http://p/", nil)
	r.RemoteAddr = "198.51.100.77:1"
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)
	if w.Code == http.StatusForbidden {
		t.Fatal("pending permanent ban must not enforce until Approver commits")
	}
	// Operator self-approve must fail.
	self := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/approvals/"+resp.ApprovalID, "")
	if self.Code == http.StatusOK {
		t.Fatal("operator must not self-approve permanent ban")
	}
	// Distinct Approver commits.
	ok := adminDo(srv, toks[RoleApprover], "POST", "/__hmn/admin/approvals/"+resp.ApprovalID, "")
	if ok.Code != http.StatusOK {
		t.Fatalf("approver commit: %d %s", ok.Code, ok.Body.String())
	}
	w2 := httptest.NewRecorder()
	srv.ServeHTTP(w2, r)
	if w2.Code != http.StatusForbidden {
		t.Fatalf("after Approver commit ban must enforce, status=%d", w2.Code)
	}
	if !strings.Contains(ok.Body.String(), "committed") && !strings.Contains(ok.Body.String(), "ban") && ok.Code != http.StatusOK {
		t.Fatalf("unexpected commit body: %s", ok.Body.String())
	}
}
