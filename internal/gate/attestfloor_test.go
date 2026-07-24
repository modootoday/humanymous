package gate

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/signals"
)

// attestfloor_test.go verifies the ceiling-guard #1 attestation floor + the
// ceiling-guard #2 WebAuthn fan-out cap + the /stepup receipt→proof exchange
// (Blue-team ceiling-guard meeting). The floor's contract: on an operator-marked
// high-value route a scoring-ALLOW no longer buys the fast-path; the session must
// present possession OR a step-up proof — CHALLENGE→Pass, never DENY, and public
// routes are untouched.

const attestTokenKey = "ceiling-guard-test-token-key-32b!"

// buildAttestStack wires a proxy with a shared token key + epochs, a WebAuthn
// verifier, and an attestFloor route (/transfer) beside a balanced default. It
// returns the server, a pointer to the origin hit counter, the shared epoch/key so
// tests can mint tokens exactly as the control plane would, and a registered
// WebAuthn credential (priv key + id) for the fan-out test.
func buildAttestStack(t *testing.T, fanoutCap int) (srv *Server, originHits *int, key []byte, epochs *EpochManager, priv *ecdsa.PrivateKey, credID string) {
	t.Helper()
	hits := 0
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		hits++
		_, _ = w.Write([]byte("ORIGIN-OK"))
	}))
	t.Cleanup(up.Close)

	alog := audit.NewLog(audit.Config{NodeID: "test", HMACKey: []byte("k"), CheckpointEvery: 4})
	sink := audit.NewSink(alog)
	vault := audit.NewVault()
	store := collector.NewStore(time.Minute)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(time.Minute)
	em := NewEpochManager()
	tk := []byte(attestTokenKey)

	priv, _ = ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	credID = "cred-ceiling"
	reg := webauthnDir(t, credID, &priv.PublicKey)

	control := NewControlPlane(store, engine, verdicts, sink, vault).WithTokenKey(tk).WithTokenEpochs(em).WithCohortShadow()
	cfg := Config{
		Upstream: up.URL, NodeID: "test", ControlPath: "/__hmn/",
		TokenKey: tk, TokenEpochs: em, WebAuthnCreds: reg, CredFanoutCap: fanoutCap,
		Routes: map[string]string{"/transfer": "attested"},
	}
	s, err := NewServer(cfg, sink, vault, verdicts, control.Handler())
	if err != nil {
		t.Fatal(err)
	}
	return s, &hits, tk, em, priv, credID
}

// aReq builds a request with an explicit socket subnet + UA (both feed tokenBind).
func aReq(method, path, remoteAddr, ua string, cookies ...string) *http.Request {
	r := httptest.NewRequest(method, "http://proxy"+path, nil)
	r.RemoteAddr = remoteAddr
	r.Header.Set("User-Agent", ua)
	if len(cookies) > 0 {
		r.Header.Set("Cookie", strings.Join(cookies, "; "))
	}
	return r
}

func serve(srv *Server, r *http.Request) *httptest.ResponseRecorder {
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)
	return w
}

// TestAttestFloorDemotesUnattestedAllow: a scoring-ALLOW (valid verdict token) is
// fast-pathed on a PUBLIC route but demoted to a step-up challenge on the attestFloor
// route — the coherent-spoof laundering path is closed, the browse route untouched.
func TestAttestFloorDemotesUnattestedAllow(t *testing.T) {
	srv, hits, key, em, _, _ := buildAttestStack(t, 0)
	now := srv.nowFn()
	sid := "human-allow"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictAllow, updated: now})

	ua, addr := "Chrome/126-real", "198.51.100.7:5555"
	bind := tokenBind(aReq("GET", "/", addr, ua))
	vt := issueVerdictToken(key, sid, bind, em.Current(), now.Add(30*time.Minute))
	cookies := []string{"hsid=" + sid, verdictCookie + "=" + vt}

	// Public balanced route: the ALLOW token fast-paths (origin contacted).
	before := *hits
	if w := serve(srv, aReq("GET", "/", addr, ua, cookies...)); w.Code != http.StatusOK || *hits != before+1 {
		t.Fatalf("public route: expected fast-path 200 + origin hit, got %d (hits+%d)", w.Code, *hits-before)
	}
	// attestFloor route: same ALLOW token is DEMOTED to a challenge; origin NOT contacted.
	before = *hits
	if w := serve(srv, aReq("GET", "/transfer", addr, ua, cookies...)); w.Code != http.StatusUnauthorized || *hits != before {
		t.Fatalf("attestFloor route: expected step-up challenge (401) + no origin hit, got %d (hits+%d)", w.Code, *hits-before)
	}
}

// TestAttestFloorStepUpProofPasses: the same session, now holding a step-up proof,
// takes the fast-path on the attestFloor route.
func TestAttestFloorStepUpProofPasses(t *testing.T) {
	srv, hits, key, em, _, _ := buildAttestStack(t, 0)
	now := srv.nowFn()
	sid := "human-stepped"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictAllow, updated: now})

	ua, addr := "Chrome/126-real", "198.51.100.9:5555"
	bind := tokenBind(aReq("GET", "/transfer", addr, ua))
	su := issueStepUpToken(key, sid, bind, em.Current(), now.Add(30*time.Minute))
	cookies := []string{"hsid=" + sid, stepUpCookie + "=" + su}

	before := *hits
	if w := serve(srv, aReq("GET", "/transfer", addr, ua, cookies...)); w.Code != http.StatusOK || *hits != before+1 {
		t.Fatalf("step-up proof should pass the floor: got %d (hits+%d)", w.Code, *hits-before)
	}
}

// TestAttestFloorStickyAllowLaunderingClosed: a sticky-ALLOW session with NO verdict
// cookie (the other laundering route) is still demoted on the attestFloor route.
func TestAttestFloorStickyAllowLaunderingClosed(t *testing.T) {
	srv, hits, _, _, _, _ := buildAttestStack(t, 0)
	sid := "sticky-allow"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictAllow, updated: srv.nowFn()})
	before := *hits
	w := serve(srv, aReq("GET", "/transfer", "198.51.100.11:1", "Chrome/126", "hsid="+sid))
	if w.Code != http.StatusUnauthorized || *hits != before {
		t.Fatalf("sticky-ALLOW must be demoted on attestFloor: got %d (hits+%d)", w.Code, *hits-before)
	}
}

// TestStepUpProofNotLiftable: a step-up proof minted for one fingerprint+subnet is
// rejected when replayed from a different machine (binding mismatch) — demoted.
func TestStepUpProofNotLiftable(t *testing.T) {
	srv, hits, key, em, _, _ := buildAttestStack(t, 0)
	now := srv.nowFn()
	sid := "victim"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictAllow, updated: now})
	// Minted for the victim's fp+subnet…
	bindV := tokenBind(aReq("GET", "/transfer", "203.0.113.5:1", "Chrome/126-victim"))
	su := issueStepUpToken(key, sid, bindV, em.Current(), now.Add(30*time.Minute))
	// …replayed from a bot on a different subnet + UA.
	before := *hits
	w := serve(srv, aReq("GET", "/transfer", "45.66.77.88:1", "HeadlessChrome/126", "hsid="+sid, stepUpCookie+"="+su))
	if w.Code != http.StatusUnauthorized || *hits != before {
		t.Fatalf("a lifted step-up proof must not pass: got %d (hits+%d)", w.Code, *hits-before)
	}
}

// TestStepUpDomainSeparation: a VERDICT token presented in the hmn_su slot must not
// satisfy the floor — the two token classes are domain-separated.
func TestStepUpDomainSeparation(t *testing.T) {
	srv, hits, key, em, _, _ := buildAttestStack(t, 0)
	now := srv.nowFn()
	sid := "confuse"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictAllow, updated: now})
	ua, addr := "Chrome/126", "198.51.100.21:1"
	bind := tokenBind(aReq("GET", "/transfer", addr, ua))
	// A perfectly valid VERDICT token, but shoved into the step-up cookie slot.
	vt := issueVerdictToken(key, sid, bind, em.Current(), now.Add(30*time.Minute))
	before := *hits
	w := serve(srv, aReq("GET", "/transfer", addr, ua, "hsid="+sid, stepUpCookie+"="+vt))
	if w.Code != http.StatusUnauthorized || *hits != before {
		t.Fatalf("a verdict token must NOT work as a step-up proof: got %d (hits+%d)", w.Code, *hits-before)
	}
}

// TestStepUpEndpointMintsProof drives the full receipt→proof exchange: a Pass-server
// receipt redeemed at /__hmn/stepup mints a socket-bound hmn_su that then passes the
// floor; an invalid receipt is refused.
func TestStepUpEndpointMintsProof(t *testing.T) {
	srv, hits, key, _, _, _ := buildAttestStack(t, 0)
	now := srv.nowFn()
	sid := "pass-solver"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictAllow, updated: now})
	ua, addr := "Chrome/126", "198.51.100.31:1"

	// Bad receipt → 403, no cookie.
	bad := aReq("POST", "/__hmn/stepup", addr, ua, "hsid="+sid)
	bad.Header.Set(stepUpReceiptHeader, "not-a-receipt")
	if w := serve(srv, bad); w.Code != http.StatusForbidden {
		t.Fatalf("invalid step-up receipt must be 403, got %d", w.Code)
	}

	// Valid receipt (as the Pass server would mint with the shared key) → mints hmn_su.
	receipt := IssueStepUpReceipt(key, sid, now.Add(2*time.Minute))
	good := aReq("POST", "/__hmn/stepup", addr, ua, "hsid="+sid)
	good.Header.Set(stepUpReceiptHeader, receipt)
	w := serve(srv, good)
	if w.Code != http.StatusOK {
		t.Fatalf("valid receipt must mint a proof: got %d %s", w.Code, w.Body.String())
	}
	var su string
	for _, c := range w.Result().Cookies() {
		if c.Name == stepUpCookie {
			su = c.Value
		}
	}
	if su == "" {
		t.Fatal("no hmn_su cookie minted")
	}
	// The minted proof passes the floor from the SAME fingerprint+subnet.
	before := *hits
	if r := serve(srv, aReq("GET", "/transfer", addr, ua, "hsid="+sid, stepUpCookie+"="+su)); r.Code != http.StatusOK || *hits != before+1 {
		t.Fatalf("minted proof should pass the floor: got %d (hits+%d)", r.Code, *hits-before)
	}
}

// TestWebAuthnFanoutCap (ceiling-guard #2): one credential fast-paths from up to the
// cap of distinct /24s; beyond it the trust-upgrade is withheld and the request falls
// to the floor (challenge). Degrades to CHALLENGE, never DENY.
func TestWebAuthnFanoutCap(t *testing.T) {
	srv, _, _, _, priv, credID := buildAttestStack(t, 2) // cap = 2 distinct /24s
	subnets := []string{"203.0.113.1:1", "203.0.114.1:1", "203.0.115.1:1"}
	var counter uint32
	code := func(addr string) int {
		counter++
		r := aReq("GET", "/transfer", addr, "Chrome/126")
		r.Header.Set("Webauthn-Assertion", signAssertion(t, priv, credID, counter))
		return serve(srv, r).Code
	}
	if c := code(subnets[0]); c != http.StatusOK {
		t.Fatalf("subnet #1 within cap should trust-upgrade (200), got %d", c)
	}
	if c := code(subnets[1]); c != http.StatusOK {
		t.Fatalf("subnet #2 within cap should trust-upgrade (200), got %d", c)
	}
	if c := code(subnets[2]); c != http.StatusUnauthorized {
		t.Fatalf("subnet #3 beyond the fan-out cap must revert to the floor (401 challenge), got %d", c)
	}
}

// TestCohortShadowFlagsSharedGenerator: many unlinked sessions sharing one motor
// signature flag the cohort — and it is strictly log-only (returns nothing, touches
// no verdict). A single distinct session does not flag.
func TestCohortShadowFlagsSharedGenerator(t *testing.T) {
	cs := newCohortShadow()
	now := time.Now()
	key := "cohortA"
	synthetic := signals.BehaviorSummary{
		Mouse: signals.MouseFeatures{Samples: 20, MeanCurvature: 0.30, VelocityStdDev: 2.0, CoalescedRatio: 1.0, StraightLineFrac: 0.5},
		Key:   signals.KeyFeatures{Keystrokes: 5, MeanFlightMs: 40, FlightStdDev: 5},
	}
	for i := 0; i < cs.minCohort; i++ {
		cs.observe(key, synthetic, "sid-"+itoaInt(i), now)
	}
	sig := behaviorSignature(synthetic)
	if _, flagged := cs.cohorts[key].flagged[sig]; !flagged {
		t.Fatal("a cohort of identical synthetic signatures should be flagged (log-only)")
	}
	// A lone session with a different signature is not flagged.
	other := synthetic
	other.Mouse.MeanCurvature = 0.9
	cs.observe("cohortB", other, "solo", now)
	if len(cs.cohorts["cohortB"].flagged) != 0 {
		t.Fatal("a single session must not flag a cohort")
	}
}

// TestStepUpTokenVerify exercises the step-up proof + receipt token checks directly.
func TestStepUpTokenVerify(t *testing.T) {
	key := []byte(attestTokenKey)
	now := time.Now()
	tok := issueStepUpToken(key, "sidX", "bindX", "e1", now.Add(time.Minute))
	if r := verifyStepUpToken(key, tok, "bindX", "sidX", now, "e1"); r != tokenOK {
		t.Fatalf("valid step-up token should verify, got %q", r)
	}
	if r := verifyStepUpToken(key, tok, "bindY", "sidX", now, "e1"); r != tokenBindingMismatch {
		t.Fatalf("wrong bind must mismatch, got %q", r)
	}
	if r := verifyStepUpToken(key, tok, "bindX", "sidX", now.Add(2*time.Minute), "e1"); r != tokenExpired {
		t.Fatalf("expired token must be rejected, got %q", r)
	}
	// A verdict token must not verify as a step-up (domain separation).
	vt := issueVerdictToken(key, "sidX", "bindX", "e1", now.Add(time.Minute))
	if r := verifyStepUpToken(key, vt, "bindX", "sidX", now, "e1"); r == tokenOK {
		t.Fatal("a verdict token must never verify as a step-up proof")
	}
	// Receipt round-trip + session binding.
	rc := IssueStepUpReceipt(key, "sidX", now.Add(time.Minute))
	if !verifyStepUpReceipt(key, rc, "sidX", now) {
		t.Fatal("valid receipt should verify")
	}
	if verifyStepUpReceipt(key, rc, "sidOther", now) {
		t.Fatal("receipt must be session-bound")
	}
}
