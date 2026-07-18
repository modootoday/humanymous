package gate

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/scoring"
)

// buildStack wires a full proxy in front of an httptest upstream and returns the
// server plus the audit log for verification.
func buildStack(t *testing.T, upstream string) (*Server, *audit.Log) {
	t.Helper()
	alog := audit.NewLog(audit.Config{NodeID: "test", HMACKey: []byte("k"), CheckpointEvery: 4})
	sink := audit.NewSink(alog)
	vault := audit.NewVault()
	store := collector.NewStore(time.Minute)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(time.Minute)
	control := NewControlPlane(store, engine, verdicts, sink, vault)
	srv, err := NewServer(Config{Upstream: upstream, NodeID: "test", ControlPath: "/__hmn/"}, sink, vault, verdicts, control.Handler())
	if err != nil {
		t.Fatal(err)
	}
	return srv, alog
}

func do(srv *Server, method, path, cookie string, body string, hdr map[string]string) *httptest.ResponseRecorder {
	r := httptest.NewRequest(method, "http://proxy"+path, strings.NewReader(body))
	if cookie != "" {
		r.Header.Set("Cookie", cookie)
	}
	for k, v := range hdr {
		r.Header.Set(k, v)
	}
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)
	return w
}

func setCookieOf(w *httptest.ResponseRecorder) string {
	if sc := w.Result().Header.Get("Set-Cookie"); sc != "" {
		return strings.Split(sc, ";")[0]
	}
	return ""
}

// full flow: inject into HTML, score a bot beacon, block the bot at the edge
// without contacting the origin, and verify the audit chain is intact.
func TestProxyEnforcementAndAudit(t *testing.T) {
	originHits := 0
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		originHits++
		w.Header().Set("Content-Type", "text/html")
		_, _ = w.Write([]byte("<html><head><title>x</title></head><body>ORIGIN</body></html>"))
	}))
	defer up.Close()

	srv, alog := buildStack(t, up.URL)

	// 1. GET / injects the bundle, preserves origin bytes.
	w1 := do(srv, "GET", "/", "", "", nil)
	if !strings.Contains(w1.Body.String(), "/__hmn/loader.js") || !strings.Contains(w1.Body.String(), "ORIGIN") {
		t.Fatalf("injection failed: %s", w1.Body.String())
	}

	// 2. Bot beacon -> DENY, sets sticky verdict + cookie.
	botBody := `{"userAgent":"HeadlessChrome/126","engineVersion":"x","signals":[` +
		`{"id":"l1.navigator.webdriver","layer":"L1","value":true,"verdict":"BOT","weight":40,"score":40,"confidence":0.95,"collected":"js"},` +
		`{"id":"l1.ua.headless_token","layer":"L1","value":true,"verdict":"BOT","weight":35,"score":35,"confidence":0.95,"collected":"js"}],` +
		`"behavior":{"mouse":{"samples":0},"events":{"totalEvents":0}}}`
	wb := do(srv, "POST", "/__hmn/collect", "", botBody, map[string]string{"Content-Type": "application/json"})
	cookie := setCookieOf(wb)
	if !strings.Contains(wb.Body.String(), "\"verdict\":\"DENY\"") {
		t.Fatalf("bot not DENYed: %s", wb.Body.String())
	}

	// 3. Bot's next GET / is blocked at the edge; origin not contacted.
	hitsBefore := originHits
	w3 := do(srv, "GET", "/", cookie, "", nil)
	if w3.Code != http.StatusForbidden {
		t.Fatalf("bot not blocked at edge: %d", w3.Code)
	}
	if originHits != hitsBefore {
		t.Fatalf("origin was contacted for a DENYed session")
	}

	// 4. The audit chain must verify (the headline invariant).
	res := audit.Verify(alog.Records(), alog.Checkpoints(), []byte("k"), alog.PublicKey())
	if !res.OK {
		t.Fatalf("audit chain broken: %s at seq %d: %s", res.Class, res.AtSeq, res.Detail)
	}
	// And it must actually contain the deny enforcement event.
	found := false
	for _, r := range alog.Records() {
		if r.EventType == audit.EventEnfDeny {
			found = true
		}
	}
	if !found {
		t.Fatal("no enforcement.deny event recorded for the blocked bot")
	}
}
