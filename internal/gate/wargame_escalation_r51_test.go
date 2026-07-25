package gate

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
	"time"
)

// Escalation wargame r51+ : each TestWargameR### is a UNIQUE attack class not used
// in r01–r50 ledger rows. Red-first: assert defense property; blue fixes land in
// product code when a case fails (see validBanKey hardening for r51–r62).

func TestWargameR051_EmptyIPBanKeyRejected(t *testing.T) {
	assertBanKeyRejected(t, "ip:")
}
func TestWargameR052_EmptyFPBanKeyRejected(t *testing.T) {
	assertBanKeyRejected(t, "fp:")
}
func TestWargameR053_EmptyCIDRBanKeyRejected(t *testing.T) {
	assertBanKeyRejected(t, "cidr:")
}
func TestWargameR054_WhitespaceOnlyIPRejected(t *testing.T) {
	assertBanKeyRejected(t, "ip: ")
}
func TestWargameR055_InvalidIPv4OctetsRejected(t *testing.T) {
	assertBanKeyRejected(t, "ip:999.999.999.999")
}
func TestWargameR056_IPKeyWithEmbeddedNewlineRejected(t *testing.T) {
	assertBanKeyRejected(t, "ip:1.2.3.4\n")
}
func TestWargameR057_IPKeyWithSemicolonPayloadRejected(t *testing.T) {
	// semicolon is printable; still must fail ParseIP
	assertBanKeyRejected(t, "ip:1.2.3.4;drop")
}
func TestWargameR058_WrongCaseIPPrefixRejected(t *testing.T) {
	assertBanKeyRejected(t, "IP:1.2.3.4")
}
func TestWargameR059_IPWithCIDRSuffixRejected(t *testing.T) {
	// ranges must use cidr: scheme, not ip:addr/mask
	assertBanKeyRejected(t, "ip:1.2.3.4/24")
}
func TestWargameR060_GarbageCIDRRejected(t *testing.T) {
	assertBanKeyRejected(t, "cidr:not-a-cidr")
}
func TestWargameR061_IPWithPercentNullStyleRejected(t *testing.T) {
	// literal %00 is not a valid IP address
	assertBanKeyRejected(t, "ip:1.1.1.1%00")
}
func TestWargameR062_ShortFingerprintRejected(t *testing.T) {
	assertBanKeyRejected(t, "fp:ab")
}
func TestWargameR063_ValidIPv6BanKeyAccepted(t *testing.T) {
	if !validBanKey("ip:2001:db8::1") {
		t.Fatal("valid IPv6 must be accepted")
	}
}
func TestWargameR064_ValidCIDRBanKeyAccepted(t *testing.T) {
	if !validBanKey("cidr:198.51.100.0/24") {
		t.Fatal("valid CIDR must be accepted")
	}
}
func TestWargameR065_WorldCIDRStillBroad(t *testing.T) {
	k := "cidr:0.0.0.0/0"
	if !validBanKey(k) {
		t.Fatal("0.0.0.0/0 is syntactically valid CIDR")
	}
	if !isBroadKey(k) {
		t.Fatal("world CIDR must remain dual-control broad")
	}
}

func TestWargameR066_HTTPEmptyIPBan400(t *testing.T) {
	assertHTTPBanRejected(t, `{"Key":"ip:","DurationSec":60}`)
}
func TestWargameR067_HTTPNewlineIPBan400(t *testing.T) {
	assertHTTPBanRejected(t, `{"Key":"ip:1.2.3.4\n","DurationSec":60}`)
}
func TestWargameR068_HTTPGarbageCIDRBan400(t *testing.T) {
	assertHTTPBanRejected(t, `{"Key":"cidr:nope","DurationSec":0}`)
}
func TestWargameR069_HTTPShortFPBan400(t *testing.T) {
	assertHTTPBanRejected(t, `{"Key":"fp:x","DurationSec":60}`)
}
func TestWargameR070_BulkSkipsInvalidKeys(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	body := `{"Keys":["ip:","ip:203.0.113.50","fp:ab","fp:abcd1234"],"DurationSec":3600,"Reason":"r70"}`
	w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans/bulk", body)
	if w.Code != http.StatusOK {
		t.Fatalf("bulk: %d %s", w.Code, w.Body.String())
	}
	// Only the two valid keys apply (invalid skipped, not 500).
	if _, ok := srv.bans.Check("ip:203.0.113.50"); !ok {
		t.Fatal("valid bulk key must apply")
	}
	if _, ok := srv.bans.Check("ip:"); ok {
		t.Fatal("empty ip must not enter ban store")
	}
}

func TestWargameR071_ApprovalExpiredCannotCommit(t *testing.T) {
	s := NewApprovalStore([]byte("k"), 10*time.Second)
	now := time.Unix(1000, 0)
	s.nowFn = func() time.Time { return now }
	p := s.Create("ban", map[string]string{"key": "ip:203.0.113.71"}, "op-1", RoleApprover)
	now = now.Add(11 * time.Second)
	if _, _, err := s.Approve(p.ID, Operator{ID: "op-2", Role: RoleApprover}); err != errExpired {
		t.Fatalf("want expired, got %v", err)
	}
}

func TestWargameR072_ApprovalUnknownIDRejected(t *testing.T) {
	s := NewApprovalStore([]byte("k"), time.Minute)
	if _, _, err := s.Approve("no-such-id", Operator{ID: "op-2", Role: RoleApprover}); err != errNoPending {
		t.Fatalf("want no-pending, got %v", err)
	}
}

func TestWargameR073_ConcurrentApproveExactlyOnce(t *testing.T) {
	s := NewApprovalStore([]byte("k"), time.Minute)
	p := s.Create("ban", map[string]string{"key": "ip:203.0.113.73"}, "op-1", RoleApprover)
	var okCount int
	var mu sync.Mutex
	var wg sync.WaitGroup
	for i := 0; i < 16; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			id := "appr-" + string(rune('A'+i%26)) + string(rune('0'+i/26))
			_, _, err := s.Approve(p.ID, Operator{ID: id, Role: RoleApprover})
			if err == nil {
				mu.Lock()
				okCount++
				mu.Unlock()
			}
		}(i)
	}
	wg.Wait()
	if okCount != 1 {
		t.Fatalf("exactly one concurrent approve must win, got %d", okCount)
	}
}

func TestWargameR074_DoubleCommitAfterSuccessFails(t *testing.T) {
	s := NewApprovalStore([]byte("k"), time.Minute)
	p := s.Create("ban", map[string]string{"key": "ip:203.0.113.74"}, "op-1", RoleApprover)
	if _, _, err := s.Approve(p.ID, Operator{ID: "op-2", Role: RoleApprover}); err != nil {
		t.Fatal(err)
	}
	if _, _, err := s.Approve(p.ID, Operator{ID: "op-3", Role: RoleApprover}); err != errNoPending {
		t.Fatalf("second commit must fail, got %v", err)
	}
}

func TestWargameR075_DPOCanApproveBanRole(t *testing.T) {
	// hasRole: DPO satisfies Approver requirement (SoD still needs distinct principal).
	s := NewApprovalStore([]byte("k"), time.Minute)
	p := s.Create("ban", map[string]string{"key": "ip:203.0.113.75"}, "op-1", RoleApprover)
	if _, _, err := s.Approve(p.ID, Operator{ID: "dpo-1", Role: RoleDPO}); err != nil {
		t.Fatalf("DPO should satisfy approver role for ban: %v", err)
	}
}

func TestWargameR076_ApproverCannotSelfRequestBanThenApprove(t *testing.T) {
	// Same principal as requester rejected even if they hold Approver role.
	s := NewApprovalStore([]byte("k"), time.Minute)
	p := s.Create("ban", map[string]string{"key": "ip:203.0.113.76"}, "dual-hat", RoleApprover)
	if _, _, err := s.Approve(p.ID, Operator{ID: "dual-hat", Role: RoleApprover}); err != errSelfApproval {
		t.Fatalf("want self-approval reject, got %v", err)
	}
}

func TestWargameR077_PendingListHidesExpired(t *testing.T) {
	s := NewApprovalStore([]byte("k"), 5*time.Second)
	now := time.Unix(2000, 0)
	s.nowFn = func() time.Time { return now }
	s.Create("ban", map[string]string{"key": "ip:203.0.113.77"}, "op-1", RoleApprover)
	now = now.Add(6 * time.Second)
	if n := len(s.Pending()); n != 0 {
		t.Fatalf("expired pending must be GC'd from list, got %d", n)
	}
}

func TestWargameR078_TicketBindsParams(t *testing.T) {
	s := NewApprovalStore([]byte("secret-ticket-key"), time.Minute)
	p1 := s.Create("ban", map[string]string{"key": "ip:203.0.113.78", "durationSec": "0"}, "op-1", RoleApprover)
	p2 := PendingAction{ID: p1.ID, Kind: p1.Kind, Params: map[string]string{"key": "ip:198.51.100.1", "durationSec": "0"}, Requester: p1.Requester}
	t1 := s.ticket(p1, "appr")
	t2 := s.ticket(p2, "appr")
	if t1 == t2 {
		t.Fatal("ticket must change when ban key param is swapped (anti retarget)")
	}
}

func TestWargameR079_ErasureRequiresDPONotApprover(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	req := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/erasure", `{"Subject":"subj-r79","LegalBasis":"test"}`)
	if req.Code != http.StatusOK || !strings.Contains(req.Body.String(), "approvalId") {
		t.Fatalf("erasure pending: %d %s", req.Code, req.Body.String())
	}
	id := extractField(req.Body.String(), "approvalId")
	// Approver role alone must fail erasure (DPO required) — either at Approve role or adminApprove.
	w := adminDo(srv, toks[RoleApprover], "POST", "/__hmn/admin/approvals/"+id, "")
	if w.Code == http.StatusOK {
		t.Fatal("non-DPO approver must not commit erasure")
	}
}

func TestWargameR080_KillswitchRequiresDistinctApprover(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	req := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/killswitch", `{"On":true}`)
	id := extractField(req.Body.String(), "approvalId")
	if id == "" {
		t.Fatalf("killswitch pending: %s", req.Body.String())
	}
	if w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/approvals/"+id, ""); w.Code == http.StatusOK {
		t.Fatal("operator must not self-commit killswitch")
	}
}

// --- helpers ---

func assertBanKeyRejected(t *testing.T, key string) {
	t.Helper()
	if validBanKey(key) {
		t.Fatalf("validBanKey(%q) must be false", key)
	}
}

func assertHTTPBanRejected(t *testing.T, body string) {
	t.Helper()
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	w := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/bans", body)
	if w.Code == http.StatusOK {
		// Must not be a successful direct apply or a pending dual-control for garbage keys.
		var resp map[string]any
		_ = json.Unmarshal(w.Body.Bytes(), &resp)
		if resp["ok"] == true || resp["pending"] == true {
			t.Fatalf("garbage ban key must be 400, got 200 %s", w.Body.String())
		}
	}
	if w.Code != http.StatusBadRequest && w.Code != http.StatusOK {
		// 400 is ideal; if handler returns other 4xx still ok as long as not applied.
	}
	if w.Code != http.StatusBadRequest {
		t.Fatalf("want 400 bad request, got %d %s", w.Code, w.Body.String())
	}
}
