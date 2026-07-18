package sentinel

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

// WS3: an approved erasure is SCHEDULED (not shredded immediately); it resolves
// the console-visible handle to the subject; the shred + signed certificate only
// happen after the hold window; and it can be cancelled during the window.
func TestErasureHoldWindowAndReverseResolve(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	// A subject exists with a linkage key + a console-visible handle (SoT-28 §6).
	p0 := srv.vault.Pseudonymize("subjX", "1.2.3.4")
	srv.vault.Link("psn-handle-X", "subjX")

	// Operator requests erasure BY THE HANDLE; a distinct DPO approves.
	req := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/erasure", `{"Subject":"psn-handle-X","LegalBasis":"GDPR Art.17"}`)
	id := extractField(req.Body.String(), "approvalId")
	commit := adminDo(srv, toks[RoleDPO], "POST", "/__hmn/admin/approvals/"+id, "")
	if commit.Code != http.StatusOK || !strings.Contains(commit.Body.String(), "\"scheduled\":true") {
		t.Fatalf("erasure should be scheduled, not immediate: %d %s", commit.Code, commit.Body.String())
	}

	// During the hold window the subject is NOT yet shredded (pseudonym stable).
	if srv.vault.Pseudonymize("subjX", "1.2.3.4") != p0 {
		t.Fatal("subject must not be shredded during the hold window")
	}
	if n := srv.RunDueShreds(srv.nowFn()); n != 0 {
		t.Fatalf("nothing should execute before the window: %d", n)
	}

	// After the hold window elapses, the shred executes and a certificate is sealed.
	if n := srv.RunDueShreds(srv.nowFn().Add(6 * time.Minute)); n != 1 {
		t.Fatalf("one shred should execute after the window: %d", n)
	}
	if srv.vault.Pseudonymize("subjX", "1.2.3.4") == p0 {
		t.Fatal("subject linkage must be severed after the shred")
	}
	// The handle is no longer resolvable (index purged).
	if _, ok := srv.vault.Resolve("psn-handle-X"); ok {
		t.Fatal("shredded subject's handle must be unresolvable")
	}
	// A signed erasure certificate (with STH root + legal basis) was recorded.
	found := false
	for _, r := range srv.sink.Log().Records() {
		if r.EventType == "erasure.completed" && strings.Contains(r.FailReason, "erasure-cert") && strings.Contains(r.FailReason, "GDPR Art.17") {
			found = true
		}
	}
	if !found {
		t.Fatal("a signed erasure certificate must be sealed on shred")
	}
}

// WS3: a scheduled erasure can be cancelled during the hold window.
func TestErasureCancel(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)
	p0 := srv.vault.Pseudonymize("subjY", "9.9.9.9")

	req := adminDo(srv, toks[RoleOperator], "POST", "/__hmn/admin/erasure", `{"Subject":"subjY","LegalBasis":"PIPA"}`)
	id := extractField(req.Body.String(), "approvalId")
	commit := adminDo(srv, toks[RoleDPO], "POST", "/__hmn/admin/approvals/"+id, "")
	shredID := extractField(commit.Body.String(), "shredId")

	// Cancel it during the hold window.
	c := adminDo(srv, toks[RoleDPO], "POST", "/__hmn/admin/erasures/"+shredID+"/cancel", "")
	if c.Code != http.StatusOK {
		t.Fatalf("cancel failed: %d %s", c.Code, c.Body.String())
	}
	// Now the window elapses but nothing executes (it was cancelled).
	if n := srv.RunDueShreds(srv.nowFn().Add(10 * time.Minute)); n != 0 {
		t.Fatalf("cancelled erasure must not execute: %d", n)
	}
	if srv.vault.Pseudonymize("subjY", "9.9.9.9") != p0 {
		t.Fatal("cancelled subject must keep its linkage")
	}
}
