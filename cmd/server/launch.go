package main

import (
	"context"
	"encoding/json"
	"net/http"
	"net/url"
	"os/exec"
	"strings"
	"sync"
	"time"
)

// launch.go implements the SoT-30 Phase-3 local Red launcher: it fires ONE
// bundled catalog profile at the LOCAL Blue engine and lets the normal scorer +
// tap A stream the result. Safety is structural, not advisory:
//   - the target is hard-coded 127.0.0.1:8443 in the launcher; there is no
//     host/url/upstream parameter (any such key is a 400);
//   - a single-use, short-lived launch nonce + a loopback Origin/Host check
//     defeat DNS-rebinding and cross-origin/embedded drivers (§11.2);
//   - runs are serialized (max one in flight) and the stateful fingerprint/subnet
//     detectors are RESET before each run, so a flood can't poison an organic
//     baseline or make sweep rates non-reproducible (§10);
//   - an ABORT-ALL cancels the in-flight run.

// nonceStore issues single-use, short-lived launch nonces.
type nonceStore struct {
	mu sync.Mutex
	m  map[string]time.Time
}

func newNonceStore() *nonceStore { return &nonceStore{m: map[string]time.Time{}} }

func (n *nonceStore) issue() string {
	tok := randHex(16)
	now := time.Now()
	n.mu.Lock()
	n.m[tok] = now.Add(2 * time.Minute)
	for k, exp := range n.m { // opportunistic GC
		if now.After(exp) {
			delete(n.m, k)
		}
	}
	n.mu.Unlock()
	return tok
}

func (n *nonceStore) consume(tok string) bool {
	if tok == "" {
		return false
	}
	n.mu.Lock()
	defer n.mu.Unlock()
	exp, ok := n.m[tok]
	if !ok {
		return false
	}
	delete(n.m, tok) // single-use
	return time.Now().Before(exp)
}

// launchProfiles is the strict allowlist: exactly the test/redteam catalog
// profiles (the _bin/_driver helpers export no run() and are excluded). A
// launch maps a profileId onto ONLY these — never an arbitrary path or host.
// This set MUST stay in lock-step with test/e2e/runner.mjs PROFILES — the parity
// is enforced by TestLaunchProfilesMatchCatalog (PLAN-07 R5), which failed loudly
// when xff_spoof.mjs (and later the deployment-review evasions) were added to the
// catalog but not here.
var launchProfiles = map[string]bool{
	"human.mjs": true, "http_client.mjs": true, "tls_parrot.mjs": true,
	"curl_impersonate_chrome.mjs": true, "curl_impersonate_chrome99_android.mjs": true,
	"selenium.mjs":  true,
	"puppeteer.mjs": true, "puppeteer_stealth.mjs": true, "playwright_plain.mjs": true,
	"playwright_stealth.mjs": true, "undetected.mjs": true, "patchright.mjs": true,
	"rebrowser_cdp_stripped.mjs": true, "mobile_ua_desktop_profile.mjs": true,
	"near_ceiling_audio_24k.mjs": true, "near_ceiling_no_widevine.mjs": true,
	"browser_use_cdp.mjs": true,
	"direct_cdp.mjs":      true, "nodriver.mjs": true, "xvfb_headful.mjs": true, "antidetect.mjs": true,
	"camoufox.mjs": true, "tls_static.mjs": true, "pq_absent.mjs": true, "tls_rotate.mjs": true, "ua_rotate.mjs": true,
	"rit_replay.mjs": true, "rit_tamper.mjs": true, "video_scrape.mjs": true, "watermark_strip.mjs": true,
	"ai_agent.mjs": true, "distributed.mjs": true, "xff_spoof.mjs": true, "flood.mjs": true, "rapid_reset.mjs": true,
	// deployment-review-hardened evasions (rounds 3 & 5) — permanent regression wargame cases.
	"signal_forgery.mjs": true, "privacy_evasion.mjs": true,
	// proxy/VPN/Tor residual layer (Squid, OpenVPN, WireGuard, Tor, anonymous chains…).
	"squid_forward.mjs": true, "openvpn_exit.mjs": true, "wireguard_hop.mjs": true, "tor_exit.mjs": true,
	"anon_proxy_chain.mjs": true, "elite_anon_proxy.mjs": true, "cdn_ip_spoof.mjs": true,
	"proxy_ua_rotate.mjs": true, "fp_churn_proxy.mjs": true, "stacked_proxy_vpn.mjs": true,
	"socks_exit_hop.mjs": true,
	// web-research-designed cost-ladder expansion (T0-T4 gradations) — go-utls-http scenarios:
	"nonbrowser_ua.mjs": true, "sec_chua_absent.mjs": true, "sec_fetch_absent.mjs": true,
	"rit_absent.mjs": true, "ja4_churn.mjs": true, "multi_axis_rotate.mjs": true, "grease_absent_js.mjs": true,
	// simulated-tells fingerprint/behavior/AI scenarios + the T4 coherent-spoof ceiling:
	"behavior_untrusted.mjs": true, "behavior_no_interaction.mjs": true, "behavior_machine_keystroke.mjs": true,
	"behavior_teleport_click.mjs": true, "behavior_bezier_mouse.mjs": true, "behavior_fixed_typing.mjs": true,
	"ai_burst_silence.mjs": true, "ai_full_cadence.mjs": true, "adv_webgpu_mismatch.mjs": true,
	"headless_ua_token.mjs": true, "native_coherent_ceiling.mjs": true,
}

// dockerOnlyLaunchProfiles are catalog entries whose execution contract requires
// the isolated multi-container topology from SoT-41. They are deliberately not
// members of launchProfiles: the local launcher has neither a Docker socket nor
// authority to attach physical USB devices. Recognizing them separately lets the
// API fail honestly instead of reporting "unknown profile" or trying to spawn a
// single-process substitute.
var dockerOnlyLaunchProfiles = map[string]bool{
	"external_input_virtual":     true,
	"external_input_dom_virtual": true,
	"external_input_usb":         true,
	"external_input_dom_usb":     true,
	"external_input_vusb":        true,
	"external_input_dom_vusb":    true,
}

// realRunProfile shells the thin per-profile wrapper against the local Blue. The
// base is passed by the launcher (always loopback); run-one.mjs cannot redirect it.
func realRunProfile(ctx context.Context, profile, base string) ([]byte, error) {
	cmd := exec.CommandContext(ctx, "node", "test/redteam/run-one.mjs", profile, base)
	return cmd.CombinedOutput()
}

// handlePlaygroundNonce issues a single-use launch nonce.
func (a *app) handlePlaygroundNonce(w http.ResponseWriter, r *http.Request) {
	if !a.playgroundGuard(w, r) {
		return
	}
	writeJSON(w, map[string]any{"nonce": a.nonces.issue()})
}

// handlePlaygroundLaunch validates and fires one local profile run.
func (a *app) handlePlaygroundLaunch(w http.ResponseWriter, r *http.Request) {
	if !a.playgroundGuard(w, r) {
		return
	}
	if r.Method != http.MethodPost {
		http.Error(w, "POST only", http.StatusMethodNotAllowed)
		return
	}
	if o := r.Header.Get("Origin"); o != "" && !originIsLoopback(o) {
		http.Error(w, "cross-origin launch refused", http.StatusForbidden)
		return
	}
	var req struct {
		ProfileID string `json:"profileId"`
		Runs      int    `json:"runs"`
		Nonce     string `json:"nonce"`
		// Any target-override key is rejected outright (defense in depth): the
		// launcher hard-codes the loopback target and never reads these.
		Host     string `json:"host"`
		URL      string `json:"url"`
		Upstream string `json:"upstream"`
	}
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<12)).Decode(&req); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}
	if req.Host != "" || req.URL != "" || req.Upstream != "" {
		http.Error(w, "no target override permitted — the observatory targets local Blue only", http.StatusBadRequest)
		return
	}
	if !a.nonces.consume(req.Nonce) {
		http.Error(w, "invalid or expired launch nonce", http.StatusForbidden)
		return
	}
	if dockerOnlyLaunchProfiles[req.ProfileID] {
		http.Error(w, "Docker-only profile — run it through the isolated external-input harness", http.StatusConflict)
		return
	}
	if !launchProfiles[req.ProfileID] {
		http.Error(w, "unknown profile", http.StatusBadRequest)
		return
	}
	runs := req.Runs
	if runs < 1 {
		runs = 1
	}
	if runs > 5 {
		runs = 5
	}
	// Serialize: at most one launch in flight (§10 — concurrent runs would make
	// the shared fingerprint/subnet detectors' verdicts order-dependent).
	select {
	case a.launchSem <- struct{}{}:
	default:
		http.Error(w, "a launch is already in flight", http.StatusTooManyRequests)
		return
	}
	a.resetDetectionState()
	ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
	a.launchMu.Lock()
	a.launchCancel = cancel
	a.launchMu.Unlock()
	go a.runLaunch(ctx, cancel, req.ProfileID, runs)
	writeJSON(w, map[string]any{"accepted": true, "profileId": req.ProfileID, "runs": runs})
}

// handlePlaygroundAbort cancels the in-flight launch (ABORT-ALL).
func (a *app) handlePlaygroundAbort(w http.ResponseWriter, r *http.Request) {
	if !a.playgroundGuard(w, r) {
		return
	}
	a.launchMu.Lock()
	if a.launchCancel != nil {
		a.launchCancel()
	}
	a.launchMu.Unlock()
	writeJSON(w, map[string]any{"aborted": true})
}

// runLaunch executes the profile N times against the local Blue, publishing
// lifecycle events; each run's scored session streams via tap A. The launch
// semaphore is released when done.
func (a *app) runLaunch(ctx context.Context, cancel context.CancelFunc, profile string, runs int) {
	defer func() {
		cancel()
		<-a.launchSem
	}()
	const base = "https://127.0.0.1:8443" // hard-coded loopback target — no override path exists
	a.hub.Publish("attack.launched", map[string]any{"profile": profile, "runs": runs})
	for i := 0; i < runs; i++ {
		if ctx.Err() != nil {
			a.hub.Publish("attack.aborted", map[string]any{"profile": profile, "run": i})
			break
		}
		out, err := a.runProfile(ctx, profile, base)
		if err != nil {
			a.hub.Publish("attack.error", map[string]any{
				"profile": profile, "run": i,
				"reason":  strings.TrimSpace(string(out)) + " " + err.Error(),
				"skipped": looksLikeMissingDep(out, err),
			})
			continue
		}
		a.hub.Publish("attack.ran", map[string]any{"profile": profile, "run": i, "out": strings.TrimSpace(string(out))})
	}
	a.hub.Publish("attack.completed", map[string]any{"profile": profile})
}

// resetDetectionState clears the process-global stateful detectors so a launched
// run starts clean and cannot poison an organic baseline (SoT-30 §10).
func (a *app) resetDetectionState() {
	a.limiter.Reset()
	a.authLim.Reset()
	a.corr.Reset()
	a.tlog.Reset()
}

// originIsLoopback reports whether an Origin header names a loopback host.
func originIsLoopback(origin string) bool {
	u, err := url.Parse(origin)
	if err != nil || u.Host == "" {
		return false
	}
	return hostIsLoopback(u.Host)
}

// looksLikeMissingDep classifies a run failure as an environment gap (a browser
// driver or node absent) vs a real error, so the UI can grey the card honestly.
func looksLikeMissingDep(out []byte, err error) bool {
	s := strings.ToLower(string(out))
	if err != nil {
		s += " " + strings.ToLower(err.Error())
	}
	for _, m := range []string{"cannot find", "not found", "err_module", "no such file", "executable file", "playwright", "browsertype", "econnrefused"} {
		if strings.Contains(s, m) {
			return true
		}
	}
	return false
}
