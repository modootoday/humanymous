package gate

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/modootoday/humanymous/internal/audit"
)

// WS4: incident lookups are capped per operator so an authenticated insider
// cannot trawl the whole audit surface; the cap is audited as recon.
func TestIncidentEnumerationCap(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	toks := seedAdmins(t, srv)

	blockedAt := -1
	for i := 0; i < 200; i++ {
		w := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/incidents/INC-NOPE", "")
		if w.Code == http.StatusTooManyRequests && blockedAt < 0 {
			blockedAt = i
		}
	}
	if blockedAt < 0 {
		t.Fatal("incident enumeration was never capped")
	}
	if blockedAt > 130 { // default hard = 120/min
		t.Fatalf("cap should trip near 120, first-429 at %d", blockedAt)
	}
	// The trawl is recorded as recon.
	found := false
	for _, r := range srv.sink.Log().Records() {
		if r.EventType == audit.EventReconProbing {
			found = true
		}
	}
	if !found {
		t.Fatal("enumeration cap breach must emit a recon.decision_probing record")
	}
}

// WS4: the opaque handle is deterministic per seq and does not reveal the seq.
func TestIncidentHandleOpaque(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)
	h1 := srv.incidentHandle(42)
	if h1 != srv.incidentHandle(42) {
		t.Fatal("handle must be deterministic for a seq")
	}
	if h1 == srv.incidentHandle(43) {
		t.Fatal("distinct seqs must give distinct handles")
	}
	if len(h1) < 8 || h1[:4] != "INC-" {
		t.Fatalf("handle should be an opaque INC- token: %s", h1)
	}
}
