package gate

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

// r81–r120: composition attacks — path, inject, token domain separation, step-up
// receipt confusion, smuggle unicode-adjacent, GC, rate.

func TestWargameR081_PathDotDotAdminStaysStrict(t *testing.T) {
	c := Config{Routes: map[string]string{"/admin": "strict"}}
	if got := c.resolve("/foo/../admin/secret"); got.name != "strict" {
		t.Fatalf("got %q", got.name)
	}
}
func TestWargameR082_PathEncodedTraversalNormalized(t *testing.T) {
	// resolve uses path.Clean — double-encoded residual is out of scope of Go URL path;
	// pin that //admin still strict.
	c := Config{Routes: map[string]string{"/admin": "strict"}}
	if got := c.resolve("//admin"); got.name != "strict" {
		t.Fatalf("got %q", got.name)
	}
}
func TestWargameR083_PathPrefixNotConfusedWithOffRoute(t *testing.T) {
	c := Config{Routes: map[string]string{"/health": "off"}}
	if got := c.resolve("/healthz"); got.name == "off" {
		t.Fatal("/healthz must not inherit /health off")
	}
}
func TestWargameR084_PathCaseSensitiveStrict(t *testing.T) {
	c := Config{Routes: map[string]string{"/Admin": "strict", "/admin": "balanced"}}
	if got := c.resolve("/admin"); got.name != "balanced" {
		t.Fatalf("case must not bleed: got %q", got.name)
	}
}
func TestWargameR085_DefaultBalancedForUnknown(t *testing.T) {
	c := Config{Routes: map[string]string{"/login": "strict"}}
	if got := c.resolve("/api/v2/widget"); got.name != "balanced" {
		t.Fatalf("got %q", got.name)
	}
}

func TestWargameR086_VerdictNotValidAsStepUp(t *testing.T) {
	key := []byte("wargame-r86-key-material!!!!!")
	now := time.Unix(1_700_000_000, 0)
	vt := issueVerdictToken(key, "sid", "bind", "e1", now.Add(time.Hour))
	if r := verifyStepUpToken(key, vt, "bind", "sid", now, "e1"); r == tokenOK {
		t.Fatal("verdict token must not verify as step-up (domain separation)")
	}
}
func TestWargameR087_StepUpNotValidAsVerdict(t *testing.T) {
	key := []byte("wargame-r87-key-material!!!!!")
	now := time.Unix(1_700_000_000, 0)
	su := issueStepUpToken(key, "sid", "bind", "e1", now.Add(time.Hour))
	if r := verifyVerdictToken(key, su, "bind", "sid", now, "e1"); r == tokenOK {
		t.Fatal("step-up token must not verify as verdict")
	}
}
func TestWargameR088_ReceiptNotValidAsStepUpCookie(t *testing.T) {
	key := []byte("wargame-r88-key-material!!!!!")
	now := time.Unix(1_700_000_000, 0)
	rc := IssueStepUpReceipt(key, "sid", now.Add(time.Hour))
	if r := verifyStepUpToken(key, rc, "bind", "sid", now, "e1"); r == tokenOK {
		t.Fatal("receipt must not mint/verify as hmn_su cookie token")
	}
}
func TestWargameR089_StepUpCookieNotValidAsReceipt(t *testing.T) {
	key := []byte("wargame-r89-key-material!!!!!")
	now := time.Unix(1_700_000_000, 0)
	su := issueStepUpToken(key, "sid", "bind", "e1", now.Add(time.Hour))
	if r := verifyStepUpReceipt(key, su, "sid", now); r == receiptOK {
		t.Fatal("step-up cookie must not verify as cross-plane receipt")
	}
}
func TestWargameR090_ReceiptSIDMismatchTyped(t *testing.T) {
	key := []byte("wargame-r90-key-material!!!!!")
	now := time.Unix(1_700_000_000, 0)
	rc := IssueStepUpReceipt(key, "sid-A", now.Add(time.Hour))
	if r := verifyStepUpReceipt(key, rc, "sid-B", now); r != receiptSIDMismatch {
		t.Fatalf("want sid_mismatch got %q", r)
	}
}
func TestWargameR091_ReceiptExpiredTyped(t *testing.T) {
	key := []byte("wargame-r91-key-material!!!!!")
	now := time.Unix(1_700_000_000, 0)
	rc := IssueStepUpReceipt(key, "sid", now.Add(time.Minute))
	if r := verifyStepUpReceipt(key, rc, "sid", now.Add(10*time.Minute)); r != receiptExpired {
		t.Fatalf("want expired got %q", r)
	}
}
func TestWargameR092_ReceiptSkewLeewayAllowsSlightDrift(t *testing.T) {
	key := []byte("wargame-r92-key-material!!!!!")
	now := time.Unix(1_700_000_000, 0)
	rc := IssueStepUpReceipt(key, "sid", now)
	// within receiptSkew (120s) past exp should still OK
	if r := verifyStepUpReceipt(key, rc, "sid", now.Add(60*time.Second)); r != receiptOK {
		t.Fatalf("skew leeway should allow, got %q", r)
	}
}
func TestWargameR093_EmptyBindKeySkipsCollapse(t *testing.T) {
	r := httptest.NewRequest("GET", "http://p/", nil)
	// no UA
	if bk := bindKey(r); bk != "" {
		t.Fatalf("headerless client must not share bind bucket, got %q", bk)
	}
}
func TestWargameR094_TokenBindIncludesSubnet(t *testing.T) {
	mk := func(addr, ua string) string {
		r := httptest.NewRequest("GET", "http://p/", nil)
		r.RemoteAddr = addr
		r.Header.Set("User-Agent", ua)
		return tokenBind(r)
	}
	if mk("203.0.113.1:1", "Chrome/126") == mk("198.51.100.1:1", "Chrome/126") {
		t.Fatal("different subnets must change tokenBind")
	}
}
func TestWargameR095_TokenBindIncludesUA(t *testing.T) {
	mk := func(ua string) string {
		r := httptest.NewRequest("GET", "http://p/", nil)
		r.RemoteAddr = "203.0.113.9:1"
		r.Header.Set("User-Agent", ua)
		return tokenBind(r)
	}
	if mk("Chrome/126") == mk("HeadlessChrome/126") {
		t.Fatal("UA change must change tokenBind")
	}
}

func TestWargameR096_SmuggleTEIdentity(t *testing.T) {
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["Transfer-Encoding"] = []string{"identity"}
	if smuggleScan(r) != smuggleBadTE {
		t.Fatal("TE identity must be bad TE")
	}
}
func TestWargameR097_SmuggleTEGzipChunkedList(t *testing.T) {
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["Transfer-Encoding"] = []string{"gzip, chunked"}
	if smuggleScan(r) != smuggleBadTE {
		t.Fatal("TE list must be bad TE")
	}
}
func TestWargameR098_SmuggleObsFoldLFOnly(t *testing.T) {
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["X-Foo"] = []string{"a\nb"}
	if smuggleScan(r) != smuggleObsFold {
		t.Fatal("LF-only obs-fold must trip")
	}
}
func TestWargameR099_SmuggleCLIdenticalDup(t *testing.T) {
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["Content-Length"] = []string{"5", "5"}
	if smuggleScan(r) != smuggleDupCL {
		t.Fatal("identical CL dups must reject")
	}
}
func TestWargameR100_SmuggleCLTEOrderIndependent(t *testing.T) {
	r := httptest.NewRequest("POST", "http://p/", nil)
	r.Header["Transfer-Encoding"] = []string{"chunked"}
	r.Header["Content-Length"] = []string{"4"}
	if smuggleScan(r) != smuggleTECL {
		t.Fatal("TE+CL must conflict regardless of map iteration")
	}
}

func TestWargameR101_SweepDoesNotFlagSingleSession(t *testing.T) {
	d := NewSweepDetector(time.Minute, 8)
	now := time.Unix(3000, 0)
	if d.Observe("bind", "sid1", now) {
		t.Fatal("single session must not flag")
	}
}
func TestWargameR102_SweepFlagsBurst(t *testing.T) {
	d := NewSweepDetector(time.Minute, 3)
	now := time.Unix(3000, 0)
	flagged := false
	for i := 0; i < 5; i++ {
		if d.Observe("bind", "s"+string(rune('a'+i)), now) {
			flagged = true
		}
	}
	if !flagged {
		t.Fatal("burst must flag")
	}
}
func TestWargameR103_SweepWindowResets(t *testing.T) {
	d := NewSweepDetector(5*time.Second, 3)
	base := time.Unix(4000, 0)
	for i := 0; i < 3; i++ {
		d.Observe("b", "s"+string(rune('a'+i)), base.Add(time.Duration(i)*6*time.Second))
	}
	if d.Observe("b", "sZ", base.Add(30*time.Second)) {
		t.Fatal("spread sessions must not flag")
	}
}

func TestWargameR104_TempBanNotPermanent(t *testing.T) {
	bs, _ := fixedBanStore()
	e := bs.Add("ip:203.0.113.104", "tmp", "op", "inc", time.Second)
	if e.Permanent() {
		t.Fatal("temp ban must not be permanent")
	}
	if e.Until.IsZero() {
		t.Fatal("expected until set")
	}
}

func TestWargameR105_PermanentBanHasZeroUntil(t *testing.T) {
	bs, _ := fixedBanStore()
	e := bs.Add("ip:203.0.113.105", "perm", "op", "inc", 0)
	if !e.Permanent() {
		t.Fatal("duration 0 must be permanent")
	}
}

func TestWargameR106_LiftRemovesBan(t *testing.T) {
	bs, _ := fixedBanStore()
	bs.Add("ip:203.0.113.106", "x", "op", "", time.Hour)
	bs.Lift("ip:203.0.113.106")
	if _, ok := bs.Check("ip:203.0.113.106"); ok {
		t.Fatal("lifted ban must not check active")
	}
}

func TestWargameR107_AutoBanEscalatesStrike(t *testing.T) {
	if escalation(1) != time.Hour {
		t.Fatal("strike1")
	}
	if escalation(2) != 6*time.Hour {
		t.Fatal("strike2")
	}
	if escalation(3) != 24*time.Hour {
		t.Fatal("strike3")
	}
	if escalation(4) != 0 {
		t.Fatal("strike4 permanent")
	}
}

func TestWargameR108_InjectIdempotentDouble(t *testing.T) {
	html := "<html><head></head><body>x</body></html>"
	once, _ := runInject(t, html, 64)
	// Second pass over already-injected HTML must not add another loader.
	twice, _ := runInject(t, once, 64)
	if strings.Count(twice, "loader.js") != 1 {
		t.Fatalf("double inject count=%d body=%s", strings.Count(twice, "loader.js"), twice)
	}
}

func TestWargameR109_NoHTMLNoInject(t *testing.T) {
	out, res := runInject(t, `{"json":true}`, 64)
	if strings.Contains(out, "loader.js") || res.Injected {
		t.Fatal("JSON body must not receive loader inject")
	}
}

func TestWargameR110_OversizeHeadGivesUp(t *testing.T) {
	// Oversize headless body: inject must not panic.
	big := strings.Repeat("a", 1<<20)
	_, _ = runInject(t, big, 4096)
}

// R111–120: admin auth edge / metrics / integrity honesty
func TestWargameR111_UnauthAdminWhoami404(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	w := adminDo(srv, "", "GET", "/__hmn/admin/whoami", "")
	if w.Code != http.StatusNotFound {
		t.Fatalf("unauth whoami want 404 got %d", w.Code)
	}
}
func TestWargameR112_AuditorWhoamiOK(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	w := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/whoami", "")
	if w.Code != http.StatusOK || !strings.Contains(w.Body.String(), "auditor") {
		t.Fatalf("got %d %s", w.Code, w.Body.String())
	}
}
func TestWargameR113_KeysOmitHMACSecret(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	w := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/keys", "")
	body := w.Body.String()
	if strings.Contains(strings.ToLower(body), "hmac") && strings.Contains(body, "hmac_key") {
		t.Fatal("keys response must not publish hmac_key")
	}
	if !strings.Contains(body, "sth_public_key") {
		t.Fatal("must publish sth_public_key")
	}
}
func TestWargameR114_CheckpointsAuthRequired(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	w := adminDo(srv, "", "GET", "/__hmn/admin/checkpoints", "")
	if w.Code != http.StatusNotFound {
		t.Fatalf("want 404 got %d", w.Code)
	}
}
func TestWargameR115_MetricsAuthRequired(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	w := adminDo(srv, "", "GET", "/__hmn/admin/metrics", "")
	if w.Code != http.StatusNotFound {
		t.Fatalf("want 404 got %d", w.Code)
	}
}
func TestWargameR116_OperatorCannotListAsUnscoped(t *testing.T) {
	// bogus token
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	w := adminDo(srv, "not-a-real-token", "GET", "/__hmn/admin/bans", "")
	if w.Code != http.StatusNotFound {
		t.Fatalf("bad token want 404 got %d", w.Code)
	}
}
func TestWargameR117_IntegrityEmptyClass(t *testing.T) {
	// Empty adminIntegrity entry is covered by admin_integrity_empty_test; here pin
	// that empty IP ban key rejection stays independent of integrity surface.
	assertBanKeyRejected(t, "ip:")
}

func TestWargameR118_BanKeyIPv4LoopbackAllowed(t *testing.T) {
	// loopback is a valid IP; policy may still ban it (ops choice)
	if !validBanKey("ip:127.0.0.1") {
		t.Fatal("loopback IP is syntactically valid")
	}
}
func TestWargameR119_BanKeyRejectsTabChar(t *testing.T) {
	assertBanKeyRejected(t, "ip:1.2.3.4\t")
}
func TestWargameR120_FPWithSpacesRejected(t *testing.T) {
	assertBanKeyRejected(t, "fp:abcd efgh")
}
