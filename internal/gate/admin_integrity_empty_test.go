package gate

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/modootoday/humanymous/internal/audit"
)

// SoT-38 P0-5 / wargame F6: adminIntegrity (the shipped SelfVerify surface) must
// report empty-chain when the log has zero records.
//
// Note: the authenticated HTTP path meta-audits *before* the handler runs
// (SoT-28 §9), so a first GET /integrity always seals admin.access first and is
// never empty. This test drives the real adminIntegrity entry point on a still-
// empty log — the same function the route invokes after meta-audit.
func TestAdminIntegrityEmptyChain(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Write([]byte("ok"))
	}))
	defer up.Close()
	srv, alog := banStack(t, up.URL, 1000)
	if alog.Len() != 0 {
		t.Fatalf("fixture must start empty, len=%d", alog.Len())
	}

	w := httptest.NewRecorder()
	srv.adminIntegrity(w)
	if w.Code != http.StatusOK {
		t.Fatalf("adminIntegrity status %d body=%s", w.Code, w.Body.String())
	}
	var out struct {
		OK      bool   `json:"ok"`
		Class   string `json:"class"`
		Records int    `json:"records"`
		Detail  string `json:"detail"`
	}
	if err := json.Unmarshal(w.Body.Bytes(), &out); err != nil {
		t.Fatalf("json: %v body=%s", err, w.Body.String())
	}
	if out.OK {
		t.Fatalf("empty chain must not report ok=true: %+v", out)
	}
	if out.Class != string(audit.ClassEmptyChain) {
		t.Fatalf("class=%q want %q detail=%q", out.Class, audit.ClassEmptyChain, out.Detail)
	}
	if out.Records != 0 {
		t.Fatalf("records=%d want 0", out.Records)
	}

	// After a sealed decision, the same shipped function must pass.
	srv.sink.Emit(audit.Record{EventType: "gate.decision", Verdict: "allow", Mode: "enforce", KeyID: "k1"})
	w2 := httptest.NewRecorder()
	srv.adminIntegrity(w2)
	if err := json.Unmarshal(w2.Body.Bytes(), &out); err != nil {
		t.Fatalf("json after emit: %v", err)
	}
	if !out.OK || out.Records == 0 {
		t.Fatalf("non-empty chain must SelfVerify: %+v body=%s", out, w2.Body.String())
	}

	// Authenticated HTTP path meta-audits first: first GET never sees a zero-record
	// chain (by design). After emit above, authenticated integrity still OK.
	toks := seedAdmins(t, srv)
	hw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/integrity", "")
	if hw.Code != http.StatusOK {
		t.Fatalf("HTTP integrity: %d %s", hw.Code, hw.Body.String())
	}
	if err := json.Unmarshal(hw.Body.Bytes(), &out); err != nil {
		t.Fatalf("HTTP json: %v", err)
	}
	if !out.OK {
		t.Fatalf("HTTP integrity after records should pass: %+v", out)
	}
}
