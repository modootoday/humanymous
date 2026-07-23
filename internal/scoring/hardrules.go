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
		// proxy_rotation is a SERVER-authoritative network-correlation signal (one fingerprint
		// across many /24 subnets). It is NOT gated on a client-reported privacy flag: doing so
		// let a residential-proxy scraper post adBlock:true to disarm the rule and reach ALLOW
		// (deep-review round-5 regression). The genuine-privacy false positive it causes
		// (Tor/Brave/RFP fingerprint-collision + exit rotation) is an inherent, DOCUMENTED
		// limitation — see docs will-this-break-my-app.md — not something a forgeable client
		// bool may resolve. An operator who must spare a known privacy population should exempt
		// it by server-observed identity (e.g. a Tor exit-node CIDR set), never by client claim.
		return c.firedBot("l5.correlation.proxy_rotation") || c.firedBot("l7.pow.too_fast")
	}, "one fingerprint across many subnets (residential-proxy rotation)"},
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
}

// applyPromotionRules returns a verdict override, if any (SoT-05 §4.1/§4.2). The
// first matching rule wins.
func applyPromotionRules(c ruleContext) hardVerdict {
	for _, r := range promotionRules {
		if r.pred(c) {
			return hardVerdict{r.verdict, r.id}
		}
	}
	return hardVerdict{}
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
