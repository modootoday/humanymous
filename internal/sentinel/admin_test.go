package sentinel

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// the read-only admin APIs back the console: integrity self-verify, the audit
// stream, and incident lookup (SoT-26 §3-5).
func TestAdminReadAPIs(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html")
		w.Write([]byte("<html><head></head><body>x</body></html>"))
	}))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	// generate some audited decisions
	for i := 0; i < 5; i++ {
		r := httptest.NewRequest("GET", "http://p/", nil)
		r.RemoteAddr = "203.0.113.20:1"
		r.Header.Set("User-Agent", "Chrome/126")
		srv.ServeHTTP(httptest.NewRecorder(), r)
	}

	// WS1: unauthenticated admin read is 404 (deny-by-default, non-discoverable).
	if w := adminDo(srv, "", "GET", "/__hmn/admin/integrity", ""); w.Code != http.StatusNotFound {
		t.Fatalf("unauthenticated admin read must 404, got %d", w.Code)
	}

	// integrity (authenticated auditor): chain must self-verify.
	iw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/integrity", "")
	var integ struct {
		OK      bool `json:"ok"`
		Records int  `json:"records"`
	}
	if err := json.Unmarshal(iw.Body.Bytes(), &integ); err != nil {
		t.Fatalf("integrity json: %v — %s", err, iw.Body.String())
	}
	if !integ.OK || integ.Records == 0 {
		t.Fatalf("chain should self-verify with records: %+v", integ)
	}

	// audit stream: returns recent records, newest-first, pseudonymized.
	aw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/audit?limit=10", "")
	var stream struct {
		Records []audit0 `json:"records"`
		Count   int      `json:"count"`
	}
	if err := json.Unmarshal(aw.Body.Bytes(), &stream); err != nil {
		t.Fatalf("audit json: %v", err)
	}
	if stream.Count == 0 || len(stream.Records) == 0 {
		t.Fatal("audit stream empty")
	}
	if strings.Contains(aw.Body.String(), "203.0.113.20") {
		t.Fatal("raw IP leaked into the audit stream")
	}

	// incident lookup by OPAQUE handle (WS4): the raw seq is not accepted.
	handle := stream.Records[0].Incident
	if handle == "" {
		t.Fatal("stream records must carry an opaque incident handle")
	}
	cw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/incidents/"+handle, "")
	if cw.Code != http.StatusOK || !strings.Contains(cw.Body.String(), "\"seq\":") {
		t.Fatalf("incident lookup by handle failed: %d %s", cw.Code, cw.Body.String())
	}
	// A raw seq must NOT resolve (non-enumerable).
	if raw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/incidents/"+itoaInt(int(stream.Records[0].Seq)), ""); raw.Code == http.StatusOK {
		t.Fatal("raw seq lookup must fail — incidents are non-enumerable")
	}

	// meta-audit: the admin reads above must themselves be recorded (SoT-28 §9).
	if !hasEvent(srv.sink.Log(), "admin.access") {
		t.Fatal("admin reads must emit admin.access meta-audit")
	}
}

type audit0 struct {
	Seq      uint64 `json:"seq"`
	Verdict  string `json:"verdict"`
	Incident string `json:"incident"`
}

// the console SPA is served and the policy/erasure endpoints back its views.
func TestAdminConsoleAndFeatures(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	// console HTML serves without auth (top-level nav can't send a bearer).
	cw := adminDo(srv, "", "GET", "/__hmn/admin/console", "")
	if cw.Code != http.StatusOK || !strings.Contains(cw.Body.String(), "Audit Console") {
		t.Fatalf("console did not serve: %d", cw.Code)
	}
	if !strings.Contains(cw.Header().Get("Content-Type"), "text/html") {
		t.Fatal("console must be text/html")
	}

	// policy view data (authenticated).
	pw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/policy", "")
	if !strings.Contains(pw.Body.String(), "\"routes\"") || !strings.Contains(pw.Body.String(), "rateLimit") {
		t.Fatalf("policy payload missing fields: %s", pw.Body.String())
	}

	// WS2: erasure is a two-phase dual-control action. Requesting it creates a
	// PENDING action (never shreds on the first request) and needs a DPO approver.
	er := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/erasure", `{"Subject":"sess-1","LegalBasis":"GDPR Art.17"}`)
	if er.Code != http.StatusOK || !strings.Contains(er.Body.String(), "approvalId") {
		t.Fatalf("erasure request should return a pending approvalId: %d %s", er.Code, er.Body.String())
	}
	var pend struct {
		ApprovalID string `json:"approvalId"`
	}
	json.Unmarshal(er.Body.Bytes(), &pend)

	// The same operator cannot self-approve (dual-control).
	self := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/approvals/"+pend.ApprovalID, "")
	if self.Code == http.StatusOK {
		t.Fatal("operator must not approve their own erasure request")
	}
	// A non-DPO approver cannot approve an erasure (DPO-gated).
	appr := adminDo(srv, toks[RoleApprover], "POST", "/__hmn/admin/approvals/"+pend.ApprovalID, "")
	if appr.Code == http.StatusOK {
		t.Fatal("erasure needs the DPO role to approve")
	}
	// A distinct DPO commits it.
	dpo := adminDo(srv, toks[RoleDPO], "POST", "/__hmn/admin/approvals/"+pend.ApprovalID, "")
	if dpo.Code != http.StatusOK || !strings.Contains(dpo.Body.String(), "\"committed\":\"erasure\"") {
		t.Fatalf("DPO erasure approval failed: %d %s", dpo.Code, dpo.Body.String())
	}
}
