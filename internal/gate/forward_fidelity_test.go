package gate

import (
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/scoring"
)

// buildStackMonitor wires the proxy in GLOBAL MONITOR mode (score+log, enforce
// nothing) in front of a real upstream, with a known origin-cloaking key, so every
// request forwards and the forwarding fidelity can be asserted in isolation from the
// verdict logic.
func buildStackMonitor(t *testing.T, upstream string, originKey []byte) *Server {
	t.Helper()
	alog := audit.NewLog(audit.Config{NodeID: "test", HMACKey: []byte("k"), CheckpointEvery: 4})
	sink := audit.NewSink(alog)
	vault := audit.NewVault()
	store := collector.NewStore(time.Minute)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(time.Minute)
	control := NewControlPlane(store, engine, verdicts, sink, vault)
	srv, err := NewServer(Config{
		Upstream: upstream, NodeID: "test", ControlPath: "/__hmn/",
		GlobalMonitor: true, OriginKey: originKey,
	}, sink, vault, verdicts, control.Handler())
	if err != nil {
		t.Fatal(err)
	}
	return srv
}

// TestProxyForwardsClientRequestFaithfully asserts the reverse proxy preserves the
// method, path, query, body, and every legitimate client header/cookie to the
// upstream, and returns the upstream's status, headers, Set-Cookie, and body to the
// client unchanged (no injection on non-HTML).
func TestProxyForwardsClientRequestFaithfully(t *testing.T) {
	originKey := []byte("origin-cloak-key")
	var (
		gotMethod, gotPath, gotQuery string
		gotHeaders                   http.Header
		gotBody                      string
		hits                         int
	)
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		hits++
		gotMethod, gotPath, gotQuery = r.Method, r.URL.Path, r.URL.RawQuery
		gotHeaders = r.Header.Clone()
		b, _ := io.ReadAll(r.Body)
		gotBody = string(b)
		// Upstream response: a custom header, a Set-Cookie, a non-200 status, JSON body.
		http.SetCookie(w, &http.Cookie{Name: "up_session", Value: "xyz", Path: "/"})
		w.Header().Set("X-Upstream", "hello")
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		_, _ = w.Write([]byte(`{"ok":true}`))
	}))
	defer up.Close()

	srv := buildStackMonitor(t, up.URL, originKey)

	body := `{"payload":"data","n":42}`
	r := httptest.NewRequest("POST", "http://proxy/api/thing?q=1&x=two", strings.NewReader(body))
	r.RemoteAddr = "203.0.113.5:44321"
	r.Header.Set("Cookie", "csrf=tok123; theme=dark")
	r.Header.Set("Authorization", "Bearer user-token")
	r.Header.Set("X-Custom-Client", "keep-me")
	r.Header.Set("Content-Type", "application/json")
	r.Header.Set("Accept-Language", "ko,en;q=0.9")
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)

	// --- request reached upstream faithfully ---
	if hits != 1 {
		t.Fatalf("upstream should be contacted exactly once, got %d", hits)
	}
	if gotMethod != "POST" {
		t.Errorf("method: got %q want POST", gotMethod)
	}
	if gotPath != "/api/thing" || gotQuery != "q=1&x=two" {
		t.Errorf("path/query: got %q?%q want /api/thing?q=1&x=two", gotPath, gotQuery)
	}
	if gotBody != body {
		t.Errorf("body altered: got %q want %q", gotBody, body)
	}
	if gotHeaders.Get("Cookie") != "csrf=tok123; theme=dark" {
		t.Errorf("Cookie not forwarded intact: %q", gotHeaders.Get("Cookie"))
	}
	if gotHeaders.Get("Authorization") != "Bearer user-token" {
		t.Errorf("Authorization not forwarded: %q", gotHeaders.Get("Authorization"))
	}
	if gotHeaders.Get("X-Custom-Client") != "keep-me" {
		t.Errorf("custom client header not forwarded: %q", gotHeaders.Get("X-Custom-Client"))
	}
	if gotHeaders.Get("Content-Type") != "application/json" {
		t.Errorf("Content-Type not forwarded: %q", gotHeaders.Get("Content-Type"))
	}
	if gotHeaders.Get("Accept-Language") != "ko,en;q=0.9" {
		t.Errorf("Accept-Language not forwarded: %q", gotHeaders.Get("Accept-Language"))
	}

	// --- proxy adds its authoritative trust headers, re-derived from the socket ---
	if xff := gotHeaders.Get("X-Forwarded-For"); xff != "203.0.113.5" {
		t.Errorf("X-Forwarded-For must be the socket IP, got %q", xff)
	}
	if gotHeaders.Get("X-Forwarded-Host") == "" {
		t.Error("X-Forwarded-Host not set by proxy")
	}
	if oa := gotHeaders.Get("X-Hmny-Origin-Auth"); !ValidateOriginAuth(originKey, oa, "e1") {
		t.Errorf("origin-cloak auth missing/invalid: %q", oa)
	}

	// --- upstream response returned to the client faithfully ---
	res := w.Result()
	if res.StatusCode != http.StatusCreated {
		t.Errorf("status: got %d want 201", res.StatusCode)
	}
	if w.Body.String() != `{"ok":true}` {
		t.Errorf("response body altered (should not inject non-HTML): %q", w.Body.String())
	}
	if res.Header.Get("X-Upstream") != "hello" {
		t.Errorf("upstream response header dropped: %q", res.Header.Get("X-Upstream"))
	}
	var upCookie bool
	for _, c := range res.Cookies() {
		if c.Name == "up_session" && c.Value == "xyz" {
			upCookie = true
		}
	}
	if !upCookie {
		t.Errorf("upstream Set-Cookie not returned to client: %v", res.Header.Values("Set-Cookie"))
	}
}

// TestProxyBlocksSpoofedTrustHeaders asserts a client cannot smuggle a forged source
// IP or origin-cloak token to the upstream: the spoof gate blocks the request (DENY)
// and the upstream is never contacted (HR-27b / HR-24).
func TestProxyBlocksSpoofedTrustHeaders(t *testing.T) {
	for _, h := range []string{"X-Forwarded-For", "X-Real-Ip", "X-Hmny-Origin-Auth", "Forwarded", "Cf-Connecting-Ip"} {
		t.Run(h, func(t *testing.T) {
			hits := 0
			up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				hits++
				w.WriteHeader(200)
			}))
			defer up.Close()
			srv := buildStackMonitor(t, up.URL, []byte("k"))

			r := httptest.NewRequest("GET", "http://proxy/api/thing", nil)
			r.RemoteAddr = "203.0.113.9:5555"
			r.Header.Set(h, "9.9.9.9")
			w := httptest.NewRecorder()
			srv.ServeHTTP(w, r)

			if hits != 0 {
				t.Fatalf("upstream MUST NOT be contacted for a spoofed %s (a forged value could reach it)", h)
			}
			if w.Result().StatusCode == http.StatusOK {
				t.Fatalf("spoofed %s must be blocked, got 200", h)
			}
		})
	}
}
