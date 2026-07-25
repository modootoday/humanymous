package gate

import (
	"net/http"
	"testing"
	"time"
)

// T4 promotion at the Gate: detection-plane ALLOW (coherent spoof / human-like)
// must still be PRICED on high-value routes — step-up / attestation floor.
// This is the Blue-team ceiling-guard promotion path (not "detection solved T4").

func TestWargameR346_T4AllowTokenPricedOnAttestedRoute(t *testing.T) {
	// Re-state ceiling-guard #1 under wargame numbering: coherent ALLOW token
	// does not launder high-value routes.
	srv, hits, key, em, _, _ := buildAttestStack(t, 0)
	now := srv.nowFn()
	sid := "t4-coherent-allow"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictAllow, updated: now})

	ua, addr := "Chrome/126-coherent", "198.51.100.46:1"
	bind := tokenBind(aReq("GET", "/", addr, ua))
	vt := issueVerdictToken(key, sid, bind, em.Current(), now.Add(30*time.Minute))
	cookies := []string{"hsid=" + sid, verdictCookie + "=" + vt}

	before := *hits
	if w := serve(srv, aReq("GET", "/", addr, ua, cookies...)); w.Code != http.StatusOK || *hits != before+1 {
		t.Fatalf("browse route may fast-path ALLOW: got %d hits+%d", w.Code, *hits-before)
	}
	before = *hits
	if w := serve(srv, aReq("GET", "/transfer", addr, ua, cookies...)); w.Code != http.StatusUnauthorized || *hits != before {
		t.Fatalf("T4 promotion: attested route must price ALLOW to step-up (401), got %d hits+%d",
			w.Code, *hits-before)
	}
}

func TestWargameR347_T4StepUpUnlocksAttestedAfterProof(t *testing.T) {
	srv, hits, key, em, _, _ := buildAttestStack(t, 0)
	now := srv.nowFn()
	sid := "t4-stepped"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictAllow, updated: now})
	ua, addr := "Chrome/126-coherent", "198.51.100.47:1"
	bind := tokenBind(aReq("GET", "/transfer", addr, ua))
	su := issueStepUpToken(key, sid, bind, em.Current(), now.Add(30*time.Minute))
	before := *hits
	w := serve(srv, aReq("GET", "/transfer", addr, ua, "hsid="+sid, stepUpCookie+"="+su))
	if w.Code != http.StatusOK || *hits != before+1 {
		t.Fatalf("after Pass/step-up, attested route may forward: got %d hits+%d", w.Code, *hits-before)
	}
}

func TestWargameR348_T4StickyAllowCannotSkipFloor(t *testing.T) {
	srv, hits, _, _, _, _ := buildAttestStack(t, 0)
	sid := "t4-sticky"
	srv.verdicts.Set(sid, stickyVerdict{verdict: VerdictAllow, updated: srv.nowFn()})
	before := *hits
	w := serve(srv, aReq("GET", "/transfer", "198.51.100.48:1", "Chrome/126", "hsid="+sid))
	if w.Code != http.StatusUnauthorized || *hits != before {
		t.Fatalf("sticky ALLOW without step-up must not reach origin: %d hits+%d", w.Code, *hits-before)
	}
}
