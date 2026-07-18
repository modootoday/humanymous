package sentinel

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
)

// WS9: the audit stream supports server-side verdict/host/rule/minRisk filters
// and cursor pagination.
func TestAuditSearchAndPagination(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	// Seed a mix of records.
	for i := 0; i < 6; i++ {
		srv.sink.Emit(audit.Record{EventType: "enforcement.deny", Verdict: "deny", Host: "checkout.acme", RiskScore: 80, Rules: []string{"HR-7"}})
		srv.sink.Emit(audit.Record{EventType: "enforcement.allow", Verdict: "allow", Host: "blog.acme", RiskScore: 5})
	}

	// verdict + host + minRisk filter → only the checkout denies.
	dw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/audit?verdict=deny&host=checkout&minRisk=50&rule=HR-7", "")
	var res struct {
		Records []struct {
			Verdict string `json:"verdict"`
			Host    string `json:"host"`
		} `json:"records"`
		NextBefore uint64 `json:"nextBefore"`
	}
	json.Unmarshal(dw.Body.Bytes(), &res)
	if len(res.Records) == 0 {
		t.Fatal("filtered query returned nothing")
	}
	for _, r := range res.Records {
		if r.Verdict != "deny" || !strings.Contains(r.Host, "checkout") {
			t.Fatalf("filter leaked a non-matching record: %+v", r)
		}
	}

	// Pagination: a small limit returns a cursor; the next page is older.
	p1 := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/audit?limit=3", "")
	var pg struct {
		NextBefore uint64 `json:"nextBefore"`
		Count      int    `json:"count"`
	}
	json.Unmarshal(p1.Body.Bytes(), &pg)
	if pg.Count != 3 || pg.NextBefore == 0 {
		t.Fatalf("pagination page 1 wrong: count=%d cursor=%d", pg.Count, pg.NextBefore)
	}
	p2 := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/audit?limit=3&before="+itoaInt(int(pg.NextBefore)), "")
	if !strings.Contains(p2.Body.String(), "\"records\"") {
		t.Fatal("pagination page 2 failed")
	}
}

// WS9: bulk ban applies many temporary keys at once and rejects
// permanent/broad keys (those need dual-control).
func TestBulkBan(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	body := `{"Keys":["ip:203.0.113.1","ip:203.0.113.2","fp:abc","cidr:10.0.0.0/8"],"DurationSec":3600,"Reason":"scraper fleet"}`
	w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans/bulk", body)
	if w.Code != http.StatusOK {
		t.Fatalf("bulk ban failed: %d %s", w.Code, w.Body.String())
	}
	if !strings.Contains(w.Body.String(), "\"applied\":3") || !strings.Contains(w.Body.String(), "\"skipped\":1") {
		t.Fatalf("bulk ban counts wrong (3 applied, cidr skipped): %s", w.Body.String())
	}
	if _, banned := srv.bans.Check("ip:203.0.113.1"); !banned {
		t.Fatal("bulk-banned key not in force")
	}
	// Permanent bulk is rejected.
	if pw := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans/bulk", `{"Keys":["ip:1.1.1.1"],"DurationSec":0}`); pw.Code == http.StatusOK {
		t.Fatal("permanent bulk ban must be rejected")
	}
	_ = time.Second
}
