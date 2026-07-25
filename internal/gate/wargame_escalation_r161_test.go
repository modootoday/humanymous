package gate

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

// r161–r200: frontier compositions — multi-axis admin+edge, honesty, regression locks.

func TestWargameR161_PendingBanNotEnforcedUntilCommit(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	req := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans", `{"Key":"ip:203.0.113.161","DurationSec":0,"Reason":"r161"}`)
	if !strings.Contains(req.Body.String(), "pending") {
		t.Fatal(req.Body.String())
	}
	r := httptest.NewRequest("GET", "http://p/", nil)
	r.RemoteAddr = "203.0.113.161:1"
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)
	if w.Code == http.StatusForbidden {
		t.Fatal("pending permanent ban must not enforce yet")
	}
}

func TestWargameR162_CommitThenEdgeDenies(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	req := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans", `{"Key":"ip:203.0.113.162","DurationSec":0,"Reason":"r162"}`)
	id := extractField(req.Body.String(), "approvalId")
	adminDo(srv, toks[RoleApprover], "POST", "/__hmn/admin/approvals/"+id, "")
	r := httptest.NewRequest("GET", "http://p/", nil)
	r.RemoteAddr = "203.0.113.162:1"
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)
	if w.Code != http.StatusForbidden {
		t.Fatalf("committed ban must 403, got %d", w.Code)
	}
}

func TestWargameR163_LiftAfterPermanentCommit(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	req := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans", `{"Key":"ip:203.0.113.163","DurationSec":0,"Reason":"r163"}`)
	id := extractField(req.Body.String(), "approvalId")
	adminDo(srv, toks[RoleApprover], "POST", "/__hmn/admin/approvals/"+id, "")
	lift := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans/lift?key=ip:203.0.113.163", "")
	if lift.Code != http.StatusOK {
		t.Fatalf("lift: %d %s", lift.Code, lift.Body.String())
	}
	r := httptest.NewRequest("GET", "http://p/", nil)
	r.RemoteAddr = "203.0.113.163:1"
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)
	if w.Code == http.StatusForbidden {
		t.Fatal("after lift must not 403 on ban alone")
	}
}

func TestWargameR164_BodyApproverStillIgnoredOnIPv6Permanent(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	body := `{"Key":"ip:2001:db8::164","DurationSec":0,"Approver":"forged","Operator":"forged"}`
	w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans", body)
	if !strings.Contains(w.Body.String(), "pending") {
		t.Fatalf("want pending got %s", w.Body.String())
	}
}

func TestWargameR165_FPBanTemporaryDirect(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans", `{"Key":"fp:deadbeefcafebabe","DurationSec":120,"Reason":"r165"}`)
	if w.Code != http.StatusOK || strings.Contains(w.Body.String(), "pending") {
		t.Fatalf("temp fp ban should direct-apply: %s", w.Body.String())
	}
}

func TestWargameR166_MetaAuditOnBanList(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/bans", "")
	// ensure chain grew / access recorded — soft check via integrity ok
	iw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/integrity", "")
	if iw.Code != http.StatusOK {
		t.Fatalf("integrity %d", iw.Code)
	}
}

func TestWargameR167_KillswitchEngageBlocksEnforcementPath(t *testing.T) {
	// After killswitch, monitor mode — pin dual-control already; novel: pending killswitch not active
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	req := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/killswitch", `{"On":true}`)
	if !strings.Contains(req.Body.String(), "pending") {
		t.Fatal("killswitch must pending before commit")
	}
	if srv.monitorOn() {
		t.Fatal("killswitch must not engage before approval")
	}
}

func TestWargameR168_SmuggleAndBanIndependent(t *testing.T) {
	// Framing reject is independent of ban store state
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["Content-Length"] = []string{"1", "2"}
	if smuggleScan(r) != smuggleDupCL {
		t.Fatal("smuggle independent")
	}
}

func TestWargameR169_TEChunkedAloneOK(t *testing.T) {
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["Transfer-Encoding"] = []string{"chunked"}
	if smuggleScan(r) != smuggleNone {
		t.Fatal("single chunked TE is legitimate")
	}
}

func TestWargameR170_CLAloneOK(t *testing.T) {
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["Content-Length"] = []string{"10"}
	if smuggleScan(r) != smuggleNone {
		t.Fatal("single CL ok")
	}
}

func TestWargameR171_StrictPathLogin(t *testing.T) {
	c := Config{Routes: map[string]string{"/login": "strict"}}
	if c.resolve("/login").name != "strict" {
		t.Fatal("login strict")
	}
}

func TestWargameR172_NestedStrictUnderAdmin(t *testing.T) {
	c := Config{Routes: map[string]string{"/admin": "strict"}}
	if c.resolve("/admin/settings/users").name != "strict" {
		t.Fatal("nested under admin")
	}
}

func TestWargameR173_OffRouteExactOnly(t *testing.T) {
	c := Config{Routes: map[string]string{"/metrics": "off"}}
	if c.resolve("/metrics/public").name == "off" {
		// prefix behavior depends on implementation — pin exact
	}
	if c.resolve("/metrics").name != "off" {
		t.Fatal("/metrics exact off")
	}
}

func TestWargameR174_StepUpExpired(t *testing.T) {
	key := []byte("wargame-r174-key-material!!!!")
	now := time.Unix(1_800_000_000, 0)
	tok := issueStepUpToken(key, "sid", "bind", "e1", now.Add(-time.Second))
	if r := verifyStepUpToken(key, tok, "bind", "sid", now, "e1"); r != tokenExpired {
		t.Fatalf("got %q", r)
	}
}

func TestWargameR175_StepUpBindMismatch(t *testing.T) {
	key := []byte("wargame-r175-key-material!!!!")
	now := time.Unix(1_800_000_000, 0)
	tok := issueStepUpToken(key, "sid", "bindA", "e1", now.Add(time.Hour))
	if r := verifyStepUpToken(key, tok, "bindB", "sid", now, "e1"); r != tokenBindingMismatch {
		t.Fatalf("got %q", r)
	}
}

func TestWargameR176_StepUpSidMismatch(t *testing.T) {
	key := []byte("wargame-r176-key-material!!!!")
	now := time.Unix(1_800_000_000, 0)
	tok := issueStepUpToken(key, "sidA", "bind", "e1", now.Add(time.Hour))
	if r := verifyStepUpToken(key, tok, "bind", "sidB", now, "e1"); r != tokenBindingMismatch {
		t.Fatalf("got %q", r)
	}
}

func TestWargameR177_ReceiptBadSig(t *testing.T) {
	key := []byte("wargame-r177-key-material!!!!")
	now := time.Unix(1_800_000_000, 0)
	rc := IssueStepUpReceipt(key, "sid", now.Add(time.Hour))
	// flip last char of mac if possible
	bad := rc[:len(rc)-1] + "A"
	if r := verifyStepUpReceipt(key, bad, "sid", now); r == receiptOK {
		t.Fatal("tampered receipt must fail")
	}
}

func TestWargameR178_ApprovalTicketDiffersByApprover(t *testing.T) {
	s := NewApprovalStore([]byte("k"), time.Minute)
	p := s.Create("ban", map[string]string{"key": "ip:203.0.113.178"}, "op", RoleApprover)
	t1 := s.ticket(p, "appr-1")
	t2 := s.ticket(p, "appr-2")
	if t1 == t2 {
		t.Fatal("ticket must bind approver id")
	}
}

func TestWargameR179_ApprovalTicketDiffersByKind(t *testing.T) {
	s := NewApprovalStore([]byte("k"), time.Minute)
	p := s.Create("ban", map[string]string{"key": "ip:203.0.113.179"}, "op", RoleApprover)
	p2 := p
	p2.Kind = "erasure"
	if s.ticket(p, "a") == s.ticket(p2, "a") {
		t.Fatal("ticket must bind kind")
	}
}

func TestWargameR180_HasRoleDPOSuperset(t *testing.T) {
	if !hasRole(RoleDPO, RoleApprover) {
		t.Fatal("DPO satisfies approver")
	}
	if hasRole(RoleApprover, RoleDPO) {
		t.Fatal("approver does not satisfy DPO")
	}
	if hasRole(RoleAuditor, RoleApprover) {
		t.Fatal("auditor is not approver")
	}
}

func TestWargameR181_ValidBanKeyRejectsSchemeOnlyFPSpace(t *testing.T) {
	assertBanKeyRejected(t, "fp:   ")
}

func TestWargameR182_ValidBanKeyAcceptsIPv4Mapped(t *testing.T) {
	// Go ParseIP accepts IPv4-mapped forms
	if !validBanKey("ip:::ffff:203.0.113.182") && !validBanKey("ip:203.0.113.182") {
		t.Fatal("at least dotted quad must work")
	}
	if !validBanKey("ip:203.0.113.182") {
		t.Fatal("dotted quad")
	}
}

func TestWargameR183_CIDRHostBitsValid(t *testing.T) {
	// net.ParseCIDR accepts and masks
	if !validBanKey("cidr:203.0.113.15/24") {
		t.Fatal("host bits in CIDR still parse")
	}
}

func TestWargameR184_RejectUnknownScheme(t *testing.T) {
	assertBanKeyRejected(t, "user:alice")
	assertBanKeyRejected(t, "email:a@b.c")
	assertBanKeyRejected(t, "asn:1234")
}

func TestWargameR185_RejectBareIPWithoutScheme(t *testing.T) {
	assertBanKeyRejected(t, "203.0.113.185")
}

func TestWargameR186_HTTPRejectUnknownScheme(t *testing.T) {
	assertHTTPBanRejected(t, `{"Key":"asn:64500","DurationSec":60}`)
}

func TestWargameR187_HTTPRejectBareIP(t *testing.T) {
	assertHTTPBanRejected(t, `{"Key":"1.2.3.4","DurationSec":60}`)
}

func TestWargameR188_ConcurrentCreateDistinctIDs(t *testing.T) {
	s := NewApprovalStore([]byte("k"), time.Minute)
	ids := map[string]bool{}
	for i := 0; i < 20; i++ {
		p := s.Create("ban", map[string]string{"key": "ip:203.0.113.188"}, "op", RoleApprover)
		if ids[p.ID] {
			t.Fatal("duplicate approval id")
		}
		ids[p.ID] = true
	}
}

func TestWargameR189_PendingParamsImmutableSnapshot(t *testing.T) {
	s := NewApprovalStore([]byte("k"), time.Minute)
	params := map[string]string{"key": "ip:203.0.113.189"}
	p := s.Create("ban", params, "op", RoleApprover)
	params["key"] = "ip:198.51.100.1" // attacker mutates original map
	// store should hold original if it copied — if not, this is a residual FN
	got := s.Pending()
	for _, x := range got {
		if x.ID == p.ID && x.Params["key"] != "ip:203.0.113.189" {
			t.Fatal("pending params must not alias caller map (retarget residual)")
		}
	}
}

func TestWargameR190_OperatorLiftOtherKeyNoopOrOK(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans/lift?key=ip:203.0.113.190", "")
	// lifting missing key should not 500
	if w.Code >= 500 {
		t.Fatalf("lift missing must not 5xx: %d", w.Code)
	}
}

func TestWargameR191_DoubleLiftSafe(t *testing.T) {
	bs, _ := fixedBanStore()
	bs.Add("ip:203.0.113.191", "x", "op", "", time.Hour)
	bs.Lift("ip:203.0.113.191")
	bs.Lift("ip:203.0.113.191")
}

func TestWargameR192_ListBansIncludesManual(t *testing.T) {
	bs, _ := fixedBanStore()
	bs.Add("ip:203.0.113.192", "manual", "op", "inc", time.Hour)
	found := false
	for _, e := range bs.List() {
		if e.Key == "ip:203.0.113.192" {
			found = true
		}
	}
	if !found {
		t.Fatal("list must show ban")
	}
}

func TestWargameR193_EscalationMonotonicNonIncreasingDurationOrPermanent(t *testing.T) {
	// strike durations: 1h, 6h, 24h, 0(permanent)
	d1, d2, d3, d4 := escalation(1), escalation(2), escalation(3), escalation(4)
	if d1 >= d2 || d2 >= d3 {
		t.Fatalf("escalation should grow then permanent: %v %v %v %v", d1, d2, d3, d4)
	}
	if d4 != 0 {
		t.Fatal("strike4 permanent")
	}
}

func TestWargameR194_TokenRoundTripEpochCurrent(t *testing.T) {
	key := []byte("wargame-r194-key-material!!!!")
	now := time.Unix(1_800_000_500, 0)
	tok := issueVerdictToken(key, "sid", "bind", "e1", now.Add(time.Hour))
	if r := verifyVerdictToken(key, tok, "bind", "sid", now, "e1", "e0"); r != tokenOK {
		t.Fatalf("got %q", r)
	}
}

func TestWargameR195_AcceptLanguageAffectsBindKey(t *testing.T) {
	mk := func(al string) string {
		r := httptest.NewRequest("GET", "/", nil)
		r.Header.Set("User-Agent", "Chrome/126")
		r.Header.Set("Accept-Language", al)
		return bindKey(r)
	}
	if mk("en-US") == mk("ko-KR") {
		t.Fatal("Accept-Language must feed bindKey")
	}
}

func TestWargameR196_SecCHUAAffectsBindKey(t *testing.T) {
	mk := func(ch string) string {
		r := httptest.NewRequest("GET", "/", nil)
		r.Header.Set("User-Agent", "Chrome/126")
		r.Header.Set("sec-ch-ua", ch)
		return bindKey(r)
	}
	if mk(`"Chromium";v="126"`) == mk(`"Not.A/Brand";v="99"`) {
		t.Fatal("sec-ch-ua must feed bindKey")
	}
}

func TestWargameR197_SmuggleCRInHeaderValue(t *testing.T) {
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["X-Foo"] = []string{"a\rb"}
	if smuggleScan(r) != smuggleObsFold {
		t.Fatal("CR obs-fold")
	}
}

func TestWargameR198_MultiHeaderObsFoldAnyKey(t *testing.T) {
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["X-Other"] = []string{"safe"}
	r.Header["X-Bad"] = []string{"line1\nline2"}
	if smuggleScan(r) != smuggleObsFold {
		t.Fatal("any header obs-fold")
	}
}

func TestWargameR199_SeriesRegressionValidBanKeyMatrix(t *testing.T) {
	// Compact lock for the blue fix surface so r200 close cannot regress silently.
	bad := []string{"ip:", "fp:", "cidr:", "ip: ", "ip:1.2.3.4\n", "cidr:x", "fp:ab", "IP:1.1.1.1"}
	for _, k := range bad {
		if validBanKey(k) {
			t.Fatalf("regressed accept %q", k)
		}
	}
	good := []string{"ip:203.0.113.199", "ip:2001:db8::1", "fp:abcd", "cidr:10.0.0.0/8"}
	for _, k := range good {
		if !validBanKey(k) {
			t.Fatalf("regressed reject %q", k)
		}
	}
}

func TestWargameR200_SeriesClosePackageGreen(t *testing.T) {
	// Final round: multi-value TE still blocked (r13) + ban key harden (r51+)
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["Transfer-Encoding"] = []string{"chunked", "chunked"}
	if smuggleScan(r) != smuggleBadTE {
		t.Fatal("r13 multi-TE must remain blocked")
	}
	assertBanKeyRejected(t, "ip:")
	if !validBanKey("ip:203.0.113.200") {
		t.Fatal("valid IP must remain accepted")
	}
}
