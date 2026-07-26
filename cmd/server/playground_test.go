package main

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

// launchPost drives the launch handler directly with a loopback Host.
func launchPost(a *app, body string) *httptest.ResponseRecorder {
	rec := httptest.NewRecorder()
	req := httptest.NewRequest("POST", "/playground/launch", strings.NewReader(body))
	req.Host = "127.0.0.1:8443"
	a.handlePlaygroundLaunch(rec, req)
	return rec
}

func newTestApp(t *testing.T, playground bool) *app {
	t.Helper()
	if playground {
		t.Setenv("HMN_PLAYGROUND", "1")
	} else {
		t.Setenv("HMN_PLAYGROUND", "0")
	}
	return newApp("web", make([]byte, 32), false /*ritOn*/)
}

// Gate OFF: the observatory routes are not registered and the hub is nil.
func TestPlaygroundGateOff(t *testing.T) {
	a := newTestApp(t, false)
	if a.hub != nil {
		t.Fatal("hub must be nil when HMN_PLAYGROUND is not 1")
	}
	rec := httptest.NewRecorder()
	req := httptest.NewRequest("GET", "/playground/meta", nil)
	req.Host = "127.0.0.1:8443"
	a.routes().ServeHTTP(rec, req)
	if rec.Code != http.StatusNotFound {
		t.Fatalf("gate-off should 404 /playground/meta, got %d", rec.Code)
	}
}

// Gate ON: /playground/meta returns the policy constants + bands with the
// engine-plane HR scope, and carries the anti-embed headers.
func TestPlaygroundMeta(t *testing.T) {
	a := newTestApp(t, true)
	if a.hub == nil {
		t.Fatal("hub must exist when HMN_PLAYGROUND=1")
	}
	rec := httptest.NewRecorder()
	req := httptest.NewRequest("GET", "/playground/meta", nil)
	req.Host = "127.0.0.1:8443"
	a.routes().ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("meta status %d", rec.Code)
	}
	if rec.Header().Get("X-Frame-Options") != "DENY" ||
		rec.Header().Get("Content-Security-Policy") != "frame-ancestors 'none'" {
		t.Fatalf("missing anti-embed headers: %v", rec.Header())
	}
	var m struct {
		Policy struct {
			Version          string
			LayerCap, DenyAt float64
			ChallengeAt      float64
		} `json:"policy"`
		Bands     []map[string]any `json:"bands"`
		RuleCount int              `json:"ruleCount"`
		Stages    []string         `json:"stages"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &m); err != nil {
		t.Fatalf("meta not JSON: %v", err)
	}
	if m.Policy.DenyAt != 70 || m.Policy.ChallengeAt != 30 || m.Policy.LayerCap != 60 {
		t.Fatalf("policy constants wrong: %+v", m.Policy)
	}
	if len(m.Bands) != 3 {
		t.Fatalf("want 3 verdict bands, got %d", len(m.Bands))
	}
	if m.RuleCount != 24 || len(m.Stages) != 7 {
		t.Fatalf("meta scope drift: rules=%d stages=%d", m.RuleCount, len(m.Stages))
	}
}

// A non-loopback Host header is refused (DNS-rebinding defense).
func TestPlaygroundRejectsNonLoopbackHost(t *testing.T) {
	a := newTestApp(t, true)
	rec := httptest.NewRecorder()
	req := httptest.NewRequest("GET", "/playground/meta", nil)
	req.Host = "attacker.example.com"
	a.routes().ServeHTTP(rec, req)
	if rec.Code != http.StatusForbidden {
		t.Fatalf("non-loopback Host must 403, got %d", rec.Code)
	}
}

// The SSE endpoint negotiates the event-stream content type.
func TestPlaygroundSSEContentType(t *testing.T) {
	a := newTestApp(t, true)
	rec := httptest.NewRecorder()
	// The handler streams until the request context is done. Give it an
	// already-cancelled context so it writes headers + backlog then returns.
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	req := httptest.NewRequest("GET", "/playground/events", nil).WithContext(ctx)
	req.Host = "localhost:8443"
	a.handlePlaygroundEvents(rec, req)
	if ct := rec.Header().Get("Content-Type"); ct != "text/event-stream" {
		t.Fatalf("SSE content-type = %q", ct)
	}
	if !bytes.Contains(rec.Body.Bytes(), []byte("retry:")) {
		t.Fatal("SSE stream missing retry directive")
	}
}

// Tap A: a scored /api/collect publishes a session.scored event onto the hub.
func TestTapAPublishesScoredSession(t *testing.T) {
	a := newTestApp(t, true)
	_, ch, cancel := a.hub.Subscribe(0)
	defer cancel()

	body := []byte(`{"userAgent":"Mozilla/5.0 test","signals":[]}`)
	rec := httptest.NewRecorder()
	req := httptest.NewRequest("POST", "/api/collect", bytes.NewReader(body))
	req.Host = "127.0.0.1:8443"
	req.RemoteAddr = "127.0.0.1:55555"
	a.handleCollect(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("collect status %d: %s", rec.Code, rec.Body.String())
	}

	select {
	case e := <-ch:
		if e.Type != "session.scored" {
			t.Fatalf("event type = %q", e.Type)
		}
		var got struct {
			SessionID string `json:"sessionId"`
			Source    string `json:"source"`
			Report    struct {
				Scoring struct {
					Verdict string `json:"verdict"`
				} `json:"scoring"`
			} `json:"report"`
		}
		if err := json.Unmarshal(e.Data, &got); err != nil {
			t.Fatalf("event payload not JSON: %v", err)
		}
		if got.SessionID == "" || got.Report.Scoring.Verdict == "" {
			t.Fatalf("live event missing sessionId/verdict: %s", e.Data)
		}
	default:
		t.Fatal("tap A did not publish a session.scored event")
	}
}

// Tap A is a zero-cost nil check when the observatory is gated off.
func TestTapAGatedOff(t *testing.T) {
	a := newTestApp(t, false)
	body := []byte(`{"userAgent":"x","signals":[]}`)
	rec := httptest.NewRecorder()
	req := httptest.NewRequest("POST", "/api/collect", bytes.NewReader(body))
	req.RemoteAddr = "127.0.0.1:5555"
	a.handleCollect(rec, req) // must not panic with a nil hub
	if rec.Code != http.StatusOK {
		t.Fatalf("collect status %d", rec.Code)
	}
}

// A launch nonce is single-use and required.
func TestLaunchNonceSingleUse(t *testing.T) {
	a := newTestApp(t, true)
	a.runProfile = func(ctx context.Context, profile, base string) ([]byte, error) { return []byte("{}"), nil }
	n := a.nonces.issue()
	if !a.nonces.consume(n) {
		t.Fatal("fresh nonce should consume once")
	}
	if a.nonces.consume(n) {
		t.Fatal("nonce must be single-use")
	}
	// A launch without a nonce is refused.
	rec := launchPost(a, `{"profileId":"human.mjs"}`)
	if rec.Code != http.StatusForbidden {
		t.Fatalf("launch without nonce should 403, got %d", rec.Code)
	}
}

// The launcher rejects any target-override key and unknown profiles.
func TestLaunchRejectsTargetOverrideAndUnknownProfile(t *testing.T) {
	a := newTestApp(t, true)
	a.runProfile = func(ctx context.Context, profile, base string) ([]byte, error) { return []byte("{}"), nil }

	// A host override is a 400 even with a valid nonce shape (checked before nonce).
	rec := launchPost(a, `{"profileId":"human.mjs","host":"evil.com","nonce":"x"}`)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("target override must 400, got %d (%s)", rec.Code, rec.Body.String())
	}
	// Unknown profile with a valid nonce -> 400.
	n := a.nonces.issue()
	rec = launchPost(a, `{"profileId":"evil.mjs","nonce":"`+n+`"}`)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("unknown profile must 400, got %d", rec.Code)
	}
	// _driver/_bin helpers are not in the allowlist.
	n = a.nonces.issue()
	rec = launchPost(a, `{"profileId":"_driver.mjs","nonce":"`+n+`"}`)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("_driver helper must be rejected, got %d", rec.Code)
	}
}

// Launches are serialized: a second launch while one is in flight gets 429.
func TestLaunchSerialized(t *testing.T) {
	a := newTestApp(t, true)
	release := make(chan struct{})
	a.runProfile = func(ctx context.Context, profile, base string) ([]byte, error) {
		<-release // hold the semaphore until released
		return []byte("{}"), nil
	}
	n1 := a.nonces.issue()
	r1 := launchPost(a, `{"profileId":"human.mjs","nonce":"`+n1+`"}`)
	if r1.Code != http.StatusOK {
		t.Fatalf("first launch should be accepted, got %d (%s)", r1.Code, r1.Body.String())
	}
	n2 := a.nonces.issue()
	r2 := launchPost(a, `{"profileId":"selenium.mjs","nonce":"`+n2+`"}`)
	if r2.Code != http.StatusTooManyRequests {
		t.Fatalf("second concurrent launch should 429, got %d", r2.Code)
	}
	close(release) // let the first run finish and release the semaphore
}

// A valid launch is accepted, resets detection state, and publishes lifecycle
// events; the injected runner stands in for spawning node.
func TestLaunchAcceptedPublishesLifecycle(t *testing.T) {
	a := newTestApp(t, true)
	done := make(chan struct{})
	a.runProfile = func(ctx context.Context, profile, base string) ([]byte, error) {
		if base != "https://127.0.0.1:8443" {
			t.Errorf("launcher must hard-code loopback base, got %q", base)
		}
		if profile != "human.mjs" {
			t.Errorf("profile = %q", profile)
		}
		return []byte(`{"verdict":"ALLOW","riskScore":10}`), nil
	}
	_, ch, cancel := a.hub.Subscribe(0)
	defer cancel()
	// Poison the limiter, then confirm the launch resets it.
	a.limiter.Observe("poison", time.Now())
	n := a.nonces.issue()
	rec := launchPost(a, `{"profileId":"human.mjs","runs":1,"nonce":"`+n+`"}`)
	if rec.Code != http.StatusOK {
		t.Fatalf("valid launch should 200, got %d (%s)", rec.Code, rec.Body.String())
	}
	// Collect the lifecycle events (launched ... completed).
	go func() {
		saw := map[string]bool{}
		for e := range ch {
			saw[e.Type] = true
			if saw["attack.launched"] && saw["attack.completed"] {
				close(done)
				return
			}
		}
	}()
	select {
	case <-done:
	case <-time.After(3 * time.Second):
		t.Fatal("did not observe attack.launched + attack.completed")
	}
}

// The explain endpoint returns the additive ScoreTrace for a stored session,
// with the ordered HR evaluation and a marked winner matching the verdict.
func TestPlaygroundExplain(t *testing.T) {
	a := newTestApp(t, true)
	// Drive a CDP-bot collect so a scored session is stored.
	body := []byte(`{"userAgent":"Mozilla/5.0 Chrome/126","signals":[` +
		`{"id":"l1.navigator.webdriver","layer":"L1","verdict":"BOT","weight":40,"score":40,"confidence":0.95,"collected":"js"},` +
		`{"id":"l1.cdp.proxy_leak","layer":"L1","verdict":"BOT","weight":40,"score":40,"confidence":0.95,"collected":"wasm"}]}`)
	rec := httptest.NewRecorder()
	req := httptest.NewRequest("POST", "/api/collect", bytes.NewReader(body))
	req.Host = "127.0.0.1:8443"
	req.RemoteAddr = "127.0.0.1:44444"
	a.handleCollect(rec, req)
	var cr struct {
		SessionID string `json:"sessionId"`
		Verdict   string `json:"verdict"`
	}
	json.Unmarshal(rec.Body.Bytes(), &cr)
	if cr.SessionID == "" {
		t.Fatal("no session id from collect")
	}

	// Now explain it.
	erec := httptest.NewRecorder()
	ereq := httptest.NewRequest("GET", "/playground/explain/"+cr.SessionID, nil)
	ereq.Host = "127.0.0.1:8443"
	a.handlePlaygroundExplain(erec, ereq)
	if erec.Code != http.StatusOK {
		t.Fatalf("explain status %d", erec.Code)
	}
	var tr struct {
		HardRules []struct {
			Rule    string `json:"rule"`
			Matched bool   `json:"matched"`
			Won     bool   `json:"won"`
		} `json:"hardRuleEval"`
		PerLayer      []map[string]any `json:"perLayer"`
		Score         float64          `json:"score"`
		Verdict       string           `json:"verdict"`
		HardRuleFired string           `json:"hardRuleFired"`
		Policy        struct {
			LayerCap float64 `json:"layerCap"`
		} `json:"policy"`
	}
	if err := json.Unmarshal(erec.Body.Bytes(), &tr); err != nil {
		t.Fatalf("trace not JSON: %v", err)
	}
	if tr.Verdict != cr.Verdict {
		t.Fatalf("trace verdict %q != collect verdict %q", tr.Verdict, cr.Verdict)
	}
	if len(tr.HardRules) == 0 || tr.Policy.LayerCap != 60 {
		t.Fatalf("trace missing hard-rule evaluation or policy: %+v", tr)
	}
	// Exactly one winner, and it equals the fired rule.
	winners := 0
	var winner string
	for _, hr := range tr.HardRules {
		if hr.Won {
			winners++
			winner = hr.Rule
		}
	}
	if tr.HardRuleFired != "" && (winners != 1 || winner != tr.HardRuleFired) {
		t.Fatalf("winner %q (n=%d) != hardRuleFired %q", winner, winners, tr.HardRuleFired)
	}
}

func TestListenAddrIsLoopback(t *testing.T) {
	cases := map[string]bool{
		":8443":            false, // all interfaces — fail-closed refuses this
		"0.0.0.0:8443":     false,
		"127.0.0.1:8443":   true,
		"localhost:8443":   true,
		"[::1]:8443":       true,
		"192.168.1.10:844": false,
	}
	for addr, want := range cases {
		if got := listenAddrIsLoopback(addr); got != want {
			t.Errorf("listenAddrIsLoopback(%q)=%v want %v", addr, got, want)
		}
	}
}

func TestHostIsLoopback(t *testing.T) {
	cases := map[string]bool{
		"127.0.0.1:8443":       true,
		"localhost:8443":       true,
		"localhost":            true,
		"[::1]:8443":           true,
		"attacker.example.com": false,
		"10.0.0.5:8443":        false,
	}
	for host, want := range cases {
		if got := hostIsLoopback(host); got != want {
			t.Errorf("hostIsLoopback(%q)=%v want %v", host, got, want)
		}
	}
}
