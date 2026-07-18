package gate

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// WS8: config_version is a deterministic signed hash of the effective policy —
// stable across calls, and it CHANGES when the effective policy changes (e.g. the
// kill switch), so an unapproved policy is detectable against the records.
func TestConfigVersionSignedAndStable(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write([]byte("ok")) }))
	defer up.Close()
	srv, _ := banStack(t, up.URL, 1000)

	v1 := srv.configVersion()
	if !strings.HasPrefix(v1, "cfg-") || v1 != srv.configVersion() {
		t.Fatalf("config version must be a stable cfg- hash, got %q", v1)
	}
	// Flipping the effective policy changes the version.
	srv.SetKillSwitch(true)
	if srv.configVersion() == v1 {
		t.Fatal("config version must change when the effective policy changes")
	}
}
