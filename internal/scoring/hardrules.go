package scoring

import "github.com/modootoday/humanymous/internal/signals"

// hardrules.go implements the point-independent promotions/demotions of SoT-05
// §4. Rules are evaluated in order; the first matching rule that sets a verdict
// wins. FP-mitigation rules adjust signal scores before combination.

// ruleContext is the working set a rule inspects.
type ruleContext struct {
	sigs         []signals.Signal
	cross        []signals.CrossCheck
	hasClient    bool // client (WASM/JS) report present at all
	browserClaim bool // UA claims a real browser (not a library default)
	combined     float64
	// ruleModes is SoT-39 RuleMode map (HR id → enforce|monitor). nil/empty = all enforce.
	ruleModes map[string]string
	// netPolicy residual class → enforce|monitor (SoT-39 §3.5). nil = code defaults.
	netPolicy map[string]string
}

// hardVerdict is the outcome of the promotion rules.
type hardVerdict struct {
	verdict string // "" => no override
	rule    string
}

// has reports whether any signal with the given id has a BOT/SUSPICIOUS verdict.
func (c ruleContext) fired(id string) bool {
	for _, s := range c.sigs {
		if s.ID == id && (s.Verdict == signals.VerdictBot || s.Verdict == signals.VerdictSuspicious) {
			return true
		}
	}
	return false
}

// firedBot reports whether any signal with the id is a confirmed BOT.
func (c ruleContext) firedBot(id string) bool {
	for _, s := range c.sigs {
		if s.ID == id && s.Verdict == signals.VerdictBot {
			return true
		}
	}
	return false
}

// crossFail reports whether a named cross-check came back inconsistent.
func (c ruleContext) crossFail(id string) bool {
	for _, x := range c.cross {
		if x.ID == id && !x.Consistent {
			return true
		}
	}
	return false
}

// promoRule is one point-independent promotion rule (SoT-05 §4). Rules are held
// as data so the production evaluator (applyPromotionRules) and the Detection
// Observatory trace (tracePromotionRules, SoT-30 §6) share the EXACT SAME
// ordered predicates — the trace can never diverge from the real verdict.
type promoRule struct {
	id      string
	verdict string
	pred    func(ruleContext) bool
	// why is a one-line rationale for the observatory teaching layer.
	why string
}

// promotionRules is the ordered rule table (SoT-05 §4.1/§4.2; SoT-30 §2.1 —
// engine plane, HR-1..HR-21 only). First matching rule wins.
var promotionRules = []promoRule{
	{"HR-1", VerdictDeny, func(c ruleContext) bool {
		return c.firedBot("l1.artifact.selenium") || c.firedBot("l1.artifact.playwright") || c.firedBot("l1.artifact.phantom")
	}, "hard automation artifact (Selenium/Puppeteer/Playwright)"},
	{"HR-2", VerdictDeny, func(c ruleContext) bool {
		return c.crossFail("x.ua_vs_ja4") && c.crossFail("x.ua_vs_h2")
	}, "UA claims a browser but TLS + HTTP/2 both resolve to a different engine"},
	{"HR-3", VerdictDeny, func(c ruleContext) bool {
		return c.firedBot("l4.event.untrusted") && (c.fired("l1.navigator.webdriver") || c.fired("l1.cdp.runtime_enable"))
	}, "synthetic (untrusted) events plus a webdriver/CDP tell"},
	{"HR-4", VerdictDeny, func(c ruleContext) bool {
		return c.fired("l1.navigator.webdriver") && c.fired("l3.integrity.native_tostring")
	}, "webdriver flag plus a patched native getter hiding it"},
	{"HR-5", VerdictDeny, func(c ruleContext) bool {
		return (c.firedBot("l5.rit.header_tampered") || c.firedBot("l5.rit.invalid_hmac")) && c.crossFail("x.ua_vs_ja4")
	}, "RIT tamper/invalid HMAC together with a TLS engine mismatch"},
	{"HR-6", VerdictDeny, func(c ruleContext) bool {
		return (c.firedBot("l3.guard.native_hooked") || c.firedBot("l3.guard.proto_polluted")) &&
			(c.fired("l1.navigator.webdriver") || c.fired("l1.cdp.runtime_enable"))
	}, "runtime hooking / prototype pollution plus a webdriver/CDP tell"},
	{"HR-7", VerdictDeny, func(c ruleContext) bool {
		return c.firedBot("l1.ua.headless_token") &&
			(c.fired("l1.navigator.webdriver") || c.fired("l1.window.outer_eq_inner") || c.fired("l3.integrity.native_tostring"))
	}, "headless browser plus a second automation indicator"},
	{"HR-8", VerdictDeny, func(c ruleContext) bool {
		return c.fired("l3.integrity.native_tostring") &&
			(c.firedBot("l1.ua.headless_token") || c.fired("l1.window.outer_eq_inner") || c.fired("l3.integrity.iframe_recheck"))
	}, "patched native getter in a chromeless/headless window (stealth tool)"},
	{"HR-9", VerdictDeny, func(c ruleContext) bool {
		return (c.firedBot("l1.cdp.runtime_enable") || c.firedBot("l1.cdp.proxy_leak")) &&
			(c.fired("l1.navigator.webdriver") || c.fired("l1.window.outer_eq_inner") ||
				c.firedBot("l1.ua.headless_token") || c.fired("l3.integrity.native_tostring") ||
				c.fired("l4.event.no_interaction"))
	}, "a CDP leak together with any automation hint"},
	{"HR-13", VerdictDeny, func(c ruleContext) bool {
		return c.firedBot("l3.guard.console_disabled") &&
			(c.fired("l1.window.outer_eq_inner") || c.firedBot("l1.ua.headless_token") ||
				c.fired("l4.event.no_interaction") || c.fired("l3.integrity.native_tostring"))
	}, "a disabled Console API (Patchright) plus a chromeless/automation tell"},
	{"HR-10", VerdictChallenge, func(c ruleContext) bool {
		return !c.hasClient
	}, "no client-side (WASM/JS) signals at all"},
	{"HR-16", VerdictDeny, func(c ruleContext) bool {
		return c.firedBot("l5.rit.header_tampered") || c.firedBot("l5.rit.invalid_hmac") || c.firedBot("l5.rit.body_mismatch")
	}, "a RIT token that fails the body HMAC (request-body tamper)"},
	{"HR-14", VerdictDeny, func(c ruleContext) bool {
		return c.firedBot("l5.traffic.engine_rotation") || c.firedBot("l5.traffic.ja4_rotation")
	}, "the TLS fingerprint rotated within one session"},
	{"HR-18", VerdictDeny, func(c ruleContext) bool {
		return c.crossFail("x.browser_no_js")
	}, "a browser UA that delivered zero client-side JS evidence (HTTP parrot)"},
	{"HR-20", VerdictDeny, func(c ruleContext) bool {
		return c.firedBot("l4.mouse.click_no_trajectory") &&
			(c.firedBot("l4.agent.burst_silence") || c.firedBot("l4.key.machine_speed") ||
				c.fired("l4.mouse.coalesced_synthetic") || c.firedBot("l1.cdp.proxy_leak") ||
				c.firedBot("l1.cdp.runtime_enable"))
	}, "AI browser-agent signature (teleport click + a second agent/CDP tell)"},
	{"HR-19", VerdictDeny, func(c ruleContext) bool {
		// pow.too_fast is integrity — always eligible. Correlation residuals honor net.correlation mode.
		if c.firedBot("l7.pow.too_fast") {
			return true
		}
		if netClassMode(c.netPolicy, "net.correlation") == "monitor" {
			return false
		}
		return c.firedBot("l5.correlation.proxy_rotation") ||
			c.firedBot("l5.correlation.fp_churn_proxy")
	}, "NET-POLICY: proxy rotation / fp-churn (audit+admin; score-exempt residual)"},
	{"HR-21", VerdictDeny, func(c ruleContext) bool {
		// Only the UNAMBIGUOUS HTTP/2 protocol DoS attacks hard-DENY here. Two deliberate
		// exclusions (deep-review): (1) l5.abuse.flood is a SHARED ja4|subnet-bucket signal, so
		// a busy CGNAT would be DENIED by strangers' traffic — it instead drives a score-based
		// CHALLENGE (a real flood is still challenged + rate-limited/banned by the escalating
		// ladder). (2) volumetric credential-stuffing velocity is covered by that same
		// fingerprint-keyed rate-limit BAN LADDER (SoT-27), not a scoring hard rule — there is
		// no l5.abuse.auth_stuffing producer, so the branch was dead and is removed.
		return c.firedBot("l5.h2dos.rapid_reset") || c.firedBot("l5.h2dos.continuation_flood")
	}, "HTTP-2 DoS (Rapid Reset / CONTINUATION flood)"},
	{"HR-15", VerdictDeny, func(c ruleContext) bool {
		return c.fired("l5.traffic.ua_rotation") &&
			(c.firedBot("l5.traffic.ja4_rotation") || c.fired("l5.traffic.ip_hop"))
	}, "multi-axis rotation (UA changed together with TLS/IP rotation)"},
	{"HR-17", VerdictChallenge, func(c ruleContext) bool {
		return c.fired("l5.rit.stale_replay") || c.firedBot("l5.rit.absent")
	}, "a replayed or absent RIT token on an API call"},
	{"HR-12", VerdictChallenge, func(c ruleContext) bool {
		return c.fired("l4.event.no_interaction")
	}, "zero interaction over the observation window (heuristic — can catch some humans)"},
	{"HR-11", VerdictChallenge, func(c ruleContext) bool {
		return c.browserClaim && c.fired("l5.ip.datacenter_asn") &&
			!c.crossFail("x.ua_vs_ja4") && !c.crossFail("x.ua_vs_h2")
	}, "a consistent browser from a datacenter ASN"},
	// HR-22 — T4 promotion path (near-ceiling residuals). A fully coherent
	// BotBrowser-class spoof may still ALLOW (honest ceiling). These strong L2
	// advanced BOT contradictions mean the spoof is *not* coherent — promote to
	// CHALLENGE so residual cracks raise cost without claiming T4 is "solved".
	{"HR-22", VerdictChallenge, func(c ruleContext) bool {
		return c.firedBot("l2.adv.audio_24k") ||
			c.firedBot("l2.adv.webgpu_mismatch") ||
			c.firedBot("l2.adv.mobile_inconsistent")
	}, "near-ceiling advanced residual (audio 24k / GPU mismatch / mobile-desktop profile)"},
	// HR-23 — Chromium container residual under a Chrome UA: Widevine absent plus
	// empty media + empty voices (Playwright/plain Chromium headless profile).
	{"HR-23", VerdictChallenge, func(c ruleContext) bool {
		return c.browserClaim &&
			c.fired("l2.adv.no_widevine") &&
			c.fired("l2.adv.no_media_devices") &&
			c.fired("l2.adv.no_voices")
	}, "Chrome UA with Chromium-container residual (no Widevine/media/voices)"},
	// HR-24 — NETWORK ADMIN POLICY (not risk-score combine). Residual signals are
	// weight-0 / score-exempt; this rule is the operator-facing auto-challenge when
	// a network residual is actionable. Primary ownership remains Audit (detect) +
	// Ban (block). CHALLENGE not DENY: corporate proxies / honest VPN/Tor share surfaces.
	{"HR-24", VerdictChallenge, func(c ruleContext) bool {
		// Residual-class gating (SoT-39): only classes in enforce participate.
		if netClassMode(c.netPolicy, "net.proxy.hop") == "enforce" && c.firedBot("l5.header.proxy_hop") {
			return true
		}
		if netClassMode(c.netPolicy, "net.vpn") == "enforce" && c.firedBot("l5.proxy.vpn_webrtc_leak") {
			return true
		}
		if netClassMode(c.netPolicy, "net.proxy.spoof") == "enforce" &&
			(c.firedBot("l5.header.client_ip_spoof") || c.firedBot("l5.header.forwarded_private")) {
			return true
		}
		if netClassMode(c.netPolicy, "net.proxy.anon") == "enforce" && c.firedBot("l5.proxy.anon_chain") {
			return true
		}
		if netClassMode(c.netPolicy, "net.tor") == "enforce" &&
			(c.fired("l5.proxy.tor_circuit") || c.fired("l5.ip.tor_exit")) {
			return true
		}
		if netClassMode(c.netPolicy, "net.proxy.hop") == "enforce" &&
			c.fired("l5.header.xff_multi_hop") && c.fired("l5.traffic.ip_hop") {
			return true
		}
		// TCP never auto-enforces. proxy ASN class.
		if netClassMode(c.netPolicy, "net.proxy.hop") == "enforce" {
			return c.browserClaim && c.fired("l5.ip.proxy_vpn_tor")
		}
		return false
	}, "NET-POLICY: network residual (proxy/VPN/Tor) — audit+admin; score-exempt"},
}

// netClassMode returns enforce when unset (code default for empty overlay HR-24 path).
// Exception: when netPolicy map is non-nil but class missing, default monitor for hop
// classes and enforce for correlation — handled by callers for correlation separately.
func netClassMode(netPolicy map[string]string, class string) string {
	if netPolicy == nil {
		// Empty overlay: preserve pre-Settings HR-24/19 residual participation = enforce.
		return "enforce"
	}
	if m, ok := netPolicy[class]; ok && m != "" {
		return m
	}
	// Partial overlay: classes not listed stay enforce for correlation, monitor for others? 
	// Spec: empty netPolicy map means code defaults. Non-empty map with missing key:
	// treat as monitor (operator opted into explicit NET-POLICY surface).
	if class == "net.correlation" {
		return "enforce"
	}
	return "monitor"
}

// applyPromotionRules returns a verdict override, if any (SoT-05 §4.1/§4.2 + SoT-39).
// Walk order is fixed. Modes:
//   - enforce (default): first matching predicate wins and stops.
//   - monitor: if predicate matches, do NOT set verdict; continue (surgical demotion).
//   - off / unknown: treated as continue (v1 refuses off at Settings validate).
func applyPromotionRules(c ruleContext) hardVerdict {
	for _, r := range promotionRules {
		mode := ruleModeOf(c.ruleModes, r.id)
		if mode == "monitor" || mode == "off" {
			continue // never short-circuit; monitor shadows are trace-only
		}
		if r.pred(c) {
			return hardVerdict{r.verdict, r.id}
		}
	}
	return hardVerdict{}
}

func ruleModeOf(modes map[string]string, id string) string {
	if modes == nil {
		return "enforce"
	}
	if m, ok := modes[id]; ok && m != "" {
		return m
	}
	return "enforce"
}

// applyWeightMultipliers scales contribution scores by overlay multipliers (SoT-39).
// Missing id ⇒ 1.0. m is clamped to [0, 2] defensively.
func applyWeightMultipliers(in []scored, mult map[string]float64) []scored {
	if len(mult) == 0 {
		return in
	}
	for i := range in {
		m, ok := mult[in[i].id]
		if !ok {
			continue
		}
		if m < 0 {
			m = 0
		}
		if m > 2 {
			m = 2
		}
		in[i].score *= m
	}
	return in
}

// applyFPMitigation adjusts scores in place before combination (SoT-05 §4.3).
// It returns the possibly-modified slice.
func applyFPMitigation(sigs []scored) []scored {
	// SoT-05 §4.3 / SoT-09 FP-mitigation (NOT hard rule HR-20): privacy-browser noise
	// (canvas/webgl tampered) with a consistent UA/TLS is a protective user, not a bot
	// -> damp canvas/webgl by 50%.
	privacy := false
	for _, s := range sigs {
		if s.id == "l2.canvas.tampered" || s.id == "l2.webgl.param_consistency" {
			privacy = true
		}
	}
	if privacy {
		for i := range sigs {
			if sigs[i].group == "canvas" || sigs[i].group == "webgl" {
				sigs[i].score *= 0.5
			}
		}
	}
	return sigs
}
