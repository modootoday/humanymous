package signals

// registry.go is the single source of truth for signal weights and the
// human-expected baseline (SoT-00 §4, SoT-01..08 tables). Detection code looks
// up Definition by ID to construct Signals with a consistent weight, so weights
// live in exactly one place.

// Definition is the static metadata for a signal id.
type Definition struct {
	ID            string
	Layer         Layer
	Weight        float64
	ExpectedHuman any
	Group         string // correlated-signal group for dedup (SoT-05 §2)
	Desc          string
}

// registry maps signal id -> Definition. Populated from the SoT tables.
var registry = map[string]Definition{}

func def(d Definition) Definition {
	if _, dup := registry[d.ID]; dup {
		panic("duplicate signal id in registry: " + d.ID)
	}
	registry[d.ID] = d
	return d
}

// Lookup returns the Definition for id and whether it exists.
func Lookup(id string) (Definition, bool) {
	d, ok := registry[id]
	return d, ok
}

// All returns a copy of every registered Definition (unordered).
func All() []Definition {
	out := make([]Definition, 0, len(registry))
	for _, d := range registry {
		out = append(out, d)
	}
	return out
}

// New builds a Signal from a registered definition, applying the definition's
// weight. verdict/value/confidence are supplied by the collector; score is
// computed via ComputeScore. Unknown ids fall back to weight 0 (UNKNOWN-safe).
func New(id string, value any, verdict Verdict, confidence float64, src Source, notes string) Signal {
	d, ok := registry[id]
	weight := 0.0
	layer := Layer("")
	var expected any
	if ok {
		weight, layer, expected = d.Weight, d.Layer, d.ExpectedHuman
	}
	return Signal{
		ID:            id,
		Layer:         layer,
		Value:         value,
		ExpectedHuman: expected,
		Verdict:       verdict,
		Weight:        weight,
		Score:         ComputeScore(weight, confidence, verdict),
		Confidence:    confidence,
		Collected:     src,
		Notes:         notes,
	}
}

// GroupOf returns the correlation group of a signal id ("" if none / unknown).
func GroupOf(id string) string {
	if d, ok := registry[id]; ok {
		return d.Group
	}
	return ""
}

// ---- L1 static (SoT-01) ----
var (
	_ = def(Definition{"l1.navigator.webdriver", LayerStatic, 15, false, "webdriver", "navigator.webdriver true"})
	_ = def(Definition{"l1.ua.headless_token", LayerStatic, 25, false, "headless", "HeadlessChrome UA token"})
	_ = def(Definition{"l1.navigator.plugins_empty", LayerStatic, 8, false, "plugins", "empty plugins array"})
	_ = def(Definition{"l1.navigator.pdfviewer", LayerStatic, 6, true, "plugins", "pdfViewerEnabled false"})
	_ = def(Definition{"l1.navigator.languages_empty", LayerStatic, 8, false, "languages", "empty languages"})
	_ = def(Definition{"l1.window.chrome_missing", LayerStatic, 8, true, "chrome", "window.chrome absent"})
	_ = def(Definition{"l1.permissions.mismatch", LayerStatic, 18, false, "permissions", "Notification vs permissions.query mismatch"})
	_ = def(Definition{"l1.window.outer_eq_inner", LayerStatic, 10, false, "windowsize", "outer==inner (no chrome UI)"})
	_ = def(Definition{"l1.artifact.selenium", LayerStatic, 40, false, "artifact", "selenium/cdc_ artifacts"})
	_ = def(Definition{"l1.artifact.playwright", LayerStatic, 40, false, "artifact", "playwright globals"})
	_ = def(Definition{"l1.artifact.phantom", LayerStatic, 30, false, "artifact", "phantomjs globals"})
	_ = def(Definition{"l1.cdp.runtime_enable", LayerStatic, 12, false, "cdp", "CDP Runtime.enable leak (weak, mostly patched)"})
	_ = def(Definition{"l1.cdp.proxy_leak", LayerStatic, 8, false, "cdp", "CDP prototype-Proxy preview leak (current Chrome over CDP)"})
)

// ---- L2 fingerprint (SoT-01) ----
var (
	_ = def(Definition{"l2.webgl.swiftshader", LayerFingerprint, 30, false, "webgl", "SwiftShader software renderer"})
	_ = def(Definition{"l2.webgl.param_consistency", LayerFingerprint, 25, true, "webgl", "renderer vs params mismatch"})
	_ = def(Definition{"l2.canvas.hash", LayerFingerprint, 10, nil, "canvas", "low-entropy/farm canvas hash"})
	_ = def(Definition{"l2.canvas.tampered", LayerFingerprint, 15, false, "canvas", "canvas API not native"})
	_ = def(Definition{"l2.audio.hash", LayerFingerprint, 8, nil, "audio", "implausible audio hash"})
	_ = def(Definition{"l2.audio.samplerate", LayerFingerprint, 6, nil, "audio", "anomalous sampleRate"})
	_ = def(Definition{"l2.screen.metrics", LayerFingerprint, 12, nil, "screen", "avail==full / default viewport"})
	_ = def(Definition{"l2.screen.dpr", LayerFingerprint, 10, nil, "screen", "devicePixelRatio inconsistency"})
	_ = def(Definition{"l2.font.count", LayerFingerprint, 12, nil, "font", "sparse fonts vs claimed OS"})
	_ = def(Definition{"l2.hardware.concurrency", LayerFingerprint, 8, nil, "hardware", "hardwareConcurrency worker mismatch"})
	_ = def(Definition{"l2.hardware.devicememory", LayerFingerprint, 15, nil, "hardware", "deviceMemory quantization violation"})
)

// ---- L2 environment: adblock / tracking protection (SoT-09) ----
// These are neutral-to-human; weights are low and mainly enable consistency
// cross-checks / FP mitigation. A privacy-conscious human is NOT a bot.
var (
	_ = def(Definition{"l2.env.adblock", LayerFingerprint, 0, nil, "env", "ad-block present (privacy human)"})
	_ = def(Definition{"l2.env.tracking_protection", LayerFingerprint, 0, nil, "env", "tracking protection present"})
	_ = def(Definition{"l2.env.privacy_inconsistent", LayerFingerprint, 12, nil, "env", "claimed privacy posture vs observed blocking mismatch"})
	_ = def(Definition{"l2.env.probe_dead", LayerFingerprint, 10, nil, "env", "environment probe did not execute (no liveness)"})
)

// ---- L2 advanced fingerprint surfaces (SoT-14) ----
var (
	_ = def(Definition{"l2.adv.audio_24k", LayerFingerprint, 20, nil, "adv-audio", "AudioContext sampleRate 24000 (headless)"})
	_ = def(Definition{"l2.adv.no_voices", LayerFingerprint, 12, nil, "adv-voice", "no speech-synthesis voices (headless)"})
	_ = def(Definition{"l2.adv.no_media_devices", LayerFingerprint, 12, nil, "adv-media", "no media devices under desktop UA"})
	_ = def(Definition{"l2.adv.no_widevine", LayerFingerprint, 12, nil, "adv-drm", "Widevine unsupported under Chrome UA (plain Chromium)"})
	_ = def(Definition{"l2.adv.rtt_zero", LayerFingerprint, 12, nil, "adv-net", "connection.rtt 0 (headless/datacenter)"})
	_ = def(Definition{"l2.adv.webgpu_mismatch", LayerFingerprint, 25, nil, "adv-gpu", "WebGL vendor vs WebGPU vendor mismatch"})
	_ = def(Definition{"l2.adv.measure_mem_throws", LayerFingerprint, 10, nil, "adv-mem", "measureUserAgentSpecificMemory throws (new-headless)"})
	_ = def(Definition{"l2.adv.mobile_inconsistent", LayerFingerprint, 20, nil, "adv-mobile", "mobile UA but desktop sensor/touch profile"})
)

// ---- L3 integrity (SoT-01) ----
var (
	_ = def(Definition{"l3.integrity.native_tostring", LayerIntegrity, 30, true, "integrity", "patched getter non-native"})
	_ = def(Definition{"l3.integrity.proxy", LayerIntegrity, 25, false, "integrity", "proxy-wrapped native"})
	_ = def(Definition{"l3.integrity.illegal_invocation", LayerIntegrity, 20, true, "integrity", "missing TypeError on illegal invocation"})
	_ = def(Definition{"l3.integrity.iframe_recheck", LayerIntegrity, 30, true, "integrity", "iframe realm exposes top-level patch"})
	_ = def(Definition{"l3.integrity.worker_recheck", LayerIntegrity, 25, true, "integrity", "worker context value mismatch"})
	_ = def(Definition{"l3.integrity.error_stack", LayerIntegrity, 15, false, "integrity", "injected script url in stack"})
)

// ---- L3 injection / eval guard (SoT-11) ----
var (
	_ = def(Definition{"l3.guard.script_injected", LayerIntegrity, 20, false, "guard", "no-nonce script node injected"})
	_ = def(Definition{"l3.guard.eval_used", LayerIntegrity, 15, false, "guard", "eval/indirect-eval/new Function/string setTimeout/dynamic import"})
	_ = def(Definition{"l3.guard.func_ctor", LayerIntegrity, 15, false, "guard", "Function constructor code generation"})
	_ = def(Definition{"l3.guard.proto_polluted", LayerIntegrity, 25, false, "guard", "core prototype mutated"})
	_ = def(Definition{"l3.guard.native_hooked", LayerIntegrity, 25, false, "guard", "fetch/XHR/JSON/toString monkeypatched"})
	_ = def(Definition{"l3.guard.csp_violation", LayerIntegrity, 18, false, "guard", "server CSP violation report"})
	_ = def(Definition{"l3.guard.trusted_types_off", LayerIntegrity, 8, false, "guard", "Trusted Types unsupported/bypassed"})
	_ = def(Definition{"l3.guard.console_disabled", LayerIntegrity, 22, false, "guard", "Console API disabled/non-native (Patchright)"})
)

// ---- L4 behavioral (SoT-03) ----
var (
	_ = def(Definition{"l4.mouse.straightness", LayerBehavioral, 12, nil, "mouse", "straightness ~1.0 / fixed"})
	_ = def(Definition{"l4.mouse.velocity_std", LayerBehavioral, 10, nil, "mouse", "near-zero velocity variance"})
	_ = def(Definition{"l4.mouse.accel_entropy", LayerBehavioral, 8, nil, "mouse", "low acceleration entropy"})
	_ = def(Definition{"l4.mouse.max_teleport", LayerBehavioral, 10, nil, "mouse", "large instantaneous jump"})
	_ = def(Definition{"l4.mouse.samples", LayerBehavioral, 8, nil, "mouse", "click with no preceding move"})
	_ = def(Definition{"l4.key.dwell_std", LayerBehavioral, 10, nil, "key", "near-zero dwell variance"})
	_ = def(Definition{"l4.key.flight_std", LayerBehavioral, 8, nil, "key", "near-zero flight variance"})
	_ = def(Definition{"l4.key.zero_dwell_frac", LayerBehavioral, 8, nil, "key", "synthetic zero-dwell keys"})
	_ = def(Definition{"l4.key.paste", LayerBehavioral, 8, nil, "key", "field filled by paste, no keystrokes"})
	_ = def(Definition{"l4.event.untrusted", LayerBehavioral, 40, 0.0, "event", "isTrusted=false events"})
	_ = def(Definition{"l4.event.no_move_click", LayerBehavioral, 20, nil, "event", "clicks without movement"})
	_ = def(Definition{"l4.event.perfect_timing", LayerBehavioral, 18, nil, "event", "zero-jitter integer-ms timing"})
	_ = def(Definition{"l4.pointer.scroll_no_wheel", LayerBehavioral, 12, nil, "pointer", "scroll change without wheel"})
	_ = def(Definition{"l4.pointer.touch_mismatch", LayerBehavioral, 12, nil, "pointer", "mobile UA but maxTouchPoints 0"})
	_ = def(Definition{"l4.event.no_interaction", LayerBehavioral, 15, nil, "event", "no human interaction over observation window"})
	_ = def(Definition{"l4.mouse.coalesced_synthetic", LayerBehavioral, 15, nil, "mouse", "movement without coalesced sub-events (CDP/OS-injected input)"})
	_ = def(Definition{"l4.mouse.jerk_anomaly", LayerBehavioral, 8, nil, "mouse", "abnormal jerk profile (scripted trajectory)"})
	// WIRED (SoT-42 Pillar A): score-exempt residual (weight 0). A small policy-specific model
	// (internal/mlserve) scores the shared behavioral feature vector (internal/behavior) and emits
	// a calibrated p(bot). Surfaced for Audit/Console/NET-POLICY only; moves no Combine risk until
	// an operator opts it into a weight (freeze-spend). With no model bundle loaded the seam
	// abstains and this signal is never emitted (freeze-safe by construction).
	_ = def(Definition{"l4.ml.behavioral", LayerBehavioral, 0, true, "ml", "behavioral model p(bot) (audit/admin)"})
	// AI-agent behavioral tells (SoT-16, FP-Agent arXiv 2605.01247).
	_ = def(Definition{"l4.agent.burst_silence", LayerBehavioral, 25, nil, "agent", "LLM inference-loop cadence (think-pause -> instant action burst)"})
	_ = def(Definition{"l4.mouse.click_no_trajectory", LayerBehavioral, 20, nil, "agent", "mouse events only at click time (no approach trajectory)"})
	_ = def(Definition{"l4.key.machine_speed", LayerBehavioral, 18, nil, "agent", "machine-speed inter-key latency (<15ms)"})
)

// ---- L5 network (SoT-02) ----
var (
	_ = def(Definition{"l5.tls.ja3", LayerNetwork, 2, nil, "tls", "JA3 (compat/logging)"})
	// l5.tls.not_observed is a weight-0 observability marker: no ClientHello captured, so the
	// TLS fingerprint plane is inactive (gate, or a TLS-terminating CDN/L7-LB in front). Known
	// signal, never scored (deep-review topology visibility).
	_ = def(Definition{"l5.tls.not_observed", LayerNetwork, 0, true, "tls", "no ClientHello captured (network plane inactive)"})
	_ = def(Definition{"l5.tls.ja4_engine", LayerNetwork, 30, nil, "tls", "JA4 engine vs UA"})
	_ = def(Definition{"l5.tls.grease_absent", LayerNetwork, 15, nil, "tls", "no GREASE (non-browser)"})
	// WIRED (wargame R9, freeze-spend 2026-07-28): score-exempt (weight 10 -> 0). A UA claiming
	// a post-quantum-era browser (Chrome >= 131 / Firefox >= 132) whose TLS ClientHello omits
	// the X25519MLKEM768 group (0x11EC) that those browsers send by default (measured vs real
	// headless Chromium 149). Acted on by HR-24 NET-POLICY (net.tls.pq, operator-overridable —
	// a PQ-disabling middlebox/policy is the deployment-delta), not risk combine.
	_ = def(Definition{"l5.tls.pq_keyshare", LayerNetwork, 0, true, "tls", "missing PQ keyshare for claimed Chrome"})
	// WIRED (wargame R10, freeze-spend 2026-07-28): score-exempt residual (weight 0). A Chromium-
	// claiming UA (chrome/ token) offering h2 whose ClientHello omits the ALPS application_settings
	// extension (codepoint 17513 or 17613) that every Chromium build sends. Firefox/Safari/Go/curl
	// send neither. Gated on h2-in-ALPN so an http/1.1-only client is never flagged. Acted on by
	// HR-24 NET-POLICY (net.tls.alps, operator-overridable), not risk combine.
	_ = def(Definition{"l5.tls.alps_absent", LayerNetwork, 0, true, "tls", "Chromium UA over h2 without the ALPS extension"})
	// WIRED (wargame R12, freeze-spend 2026-07-29): score-exempt residual (weight 0). A browser-
	// claiming UA offering h2 whose ClientHello omits the compress_certificate extension (RFC 8879,
	// codepoint 27) that every modern browser (Chrome/Firefox/Safari) sends. Broader than ALPS
	// (all engines, not just Chromium). Gated on h2-in-ALPN. HR-24 NET-POLICY (net.tls.certcomp,
	// operator-overridable — a cert-compression-stripping middlebox is the deployment-delta).
	_ = def(Definition{"l5.tls.cert_compression_absent", LayerNetwork, 0, true, "tls", "browser UA over h2 without certificate compression"})
	// WIRED (wargame R16, freeze-spend 2026-07-29): score-exempt residual (weight 0). A UA claiming
	// an ECH-era browser (Chrome >= 117 / Firefox >= 118) offering h2 whose ClientHello omits the
	// Encrypted Client Hello extension (0xfe0d) that those browsers send by default (measured vs
	// real Chrome/149 + Firefox/151). Version-gated. HR-24 NET-POLICY (net.tls.ech, operator monitor
	// for an ECH-disabling enterprise policy), not risk combine.
	_ = def(Definition{"l5.tls.ech_absent", LayerNetwork, 0, true, "tls", "ECH-era browser UA over h2 without Encrypted Client Hello"})
	_ = def(Definition{"l5.http2.engine_mismatch", LayerNetwork, 30, nil, "http2", "h2 profile engine vs UA"})
	// Score-exempt residual (weight 0): a browser-claiming UA over an HTTP/2 profile the
	// engine cannot classify as any known browser (EngineFromH2 == unknown). A real browser
	// always presents a KNOWN h2 fingerprint, so this is the 2026 "protocol-split" tell
	// (Chrome TLS/JA4 + library h2 frame layout). It participates in Audit/Console/NET-POLICY
	// only — it does NOT move Combine risk, so the frozen verdict is unchanged (a separate
	// detection-release event owns any verdict-altering enforcement).
	_ = def(Definition{"l5.http2.unknown_under_browser", LayerNetwork, 0, true, "http2", "browser UA over an unclassifiable HTTP/2 profile (audit/admin)"})
	// Score-exempt residual (weight 0): a client whose HTTP/2 pseudo-header order was
	// classified as a browser (Chrome/Firefox/Safari) but whose SETTINGS profile is not a
	// real browser's — specifically it omits SETTINGS_HEADER_TABLE_SIZE (id 1), which every
	// real browser sends and libraries (Go/many h2 clients) do not. EngineFromH2 keys the
	// browser engines on pseudo-order ALONE, so a library that mimics the order but not the
	// full Akamai SETTINGS/WINDOW_UPDATE profile is accepted as that browser. Audit/Console/
	// NET-POLICY only — it moves no Combine risk, so the frozen verdict is unchanged.
	_ = def(Definition{"l5.http2.browser_settings_atypical", LayerNetwork, 0, true, "http2", "browser HTTP/2 pseudo-order with a non-browser SETTINGS profile (audit/admin)"})
	// WIRED (wargame R11, freeze-spend 2026-07-28): score-exempt residual (weight 0). A client
	// whose HTTP/2 pseudo-header order classified as a browser but whose connection-level
	// WINDOW_UPDATE opens a gigabyte-scale flow-control window. Real browsers use a bounded
	// window (Chrome ~15 MB / 15663105; Firefox ~12 MB); Go's http2 default is 1 GiB. This is the
	// flow-control (W) dimension of the Akamai h2 fingerprint — R6 covered SETTINGS, R7 the
	// pseudo-order. HR-24 net.h2.spoof (same class as the other h2-spoof residuals), not combine.
	_ = def(Definition{"l5.http2.flow_control_atypical", LayerNetwork, 0, true, "http2", "browser HTTP/2 pseudo-order with a gigabyte flow-control window (audit/admin)"})
	// WIRED (wargame R14, freeze-spend 2026-07-29): score-exempt residual (weight 0). A Chrome/
	// Firefox-classified h2 profile (by pseudo-order) that sends NO priority signal — neither a
	// HEADERS-frame priority field nor a separate PRIORITY frame. Measured: real Chrome sets
	// HEADERS priority excl=1/weight=255, Firefox excl=0/weight=41 (both always present); a
	// raw-framer library omits it. The 4th (priority) Akamai h2 component. HR-24 net.h2.spoof.
	_ = def(Definition{"l5.http2.priority_atypical", LayerNetwork, 0, true, "http2", "browser HTTP/2 pseudo-order with no priority signal (audit/admin)"})
	// WIRED (wargame R15, freeze-spend 2026-07-29): score-exempt residual (weight 0). A browser
	// TLS stack (JA4 engine Chrome/Firefox/Safari) whose request was delivered over HTTP/1.1, not
	// h2. Real modern browsers ALWAYS negotiate and speak h2 to an h2-capable server; a browser-TLS
	// parrot driven by an HTTP/1.1 client library speaks h1 — the JA4↔JA4H cross-layer tell ("JA4
	// says Chrome, JA4H says HTTP/1.1"). HR-24 net.http.h1 (operator monitor for a TLS-inspecting
	// h2->h1 downgrade middlebox), not risk combine.
	_ = def(Definition{"l5.http.browser_tls_over_h1", LayerNetwork, 0, true, "http", "browser TLS fingerprint delivered over HTTP/1.1 (audit/admin)"})
	// WIRED (wargame R8, freeze-spend 2026-07-28): a raw on-wire header-order capture now
	// feeds this (h1 peek / h2 HEADERS frame → HeaderInfo.OrderReliable). Score-exempt
	// (weight 0): a Chrome-UA request whose client-hints (sec-ch-ua) come AFTER user-agent is
	// a non-browser order — real Chrome always sends the client-hints cluster before user-agent
	// (measured against real headless Chromium). Acted on by HR-24 NET-POLICY (net.header.order,
	// operator-overridable), not risk combine, so the frozen score is unchanged.
	_ = def(Definition{"l5.header.order", LayerNetwork, 0, true, "header", "header order vs claimed browser"})
	_ = def(Definition{"l5.header.h2_uppercase", LayerNetwork, 25, nil, "header", "uppercase header in h2 (malformed)"})
	_ = def(Definition{"l5.header.sec_fetch_missing", LayerNetwork, 25, nil, "header", "Chrome UA but sec-fetch missing"})
	_ = def(Definition{"l5.header.accept_encoding", LayerNetwork, 8, nil, "header", "accept-encoding mismatch"})
	// Network-plane residuals below are WEIGHT 0 (score-exempt). They still fire
	// BOT/SUSPICIOUS verdicts for console/audit/admin policy (HR-24 NET-POLICY /
	// Ban), but never move Combine risk — operators own detect+block via Audit.
	_ = def(Definition{"l5.header.forwarded_private", LayerNetwork, 0, true, "header", "forwarded client IP is private/reserved (audit/admin)"})
	_ = def(Definition{"l5.header.proxy_hop", LayerNetwork, 0, true, "proxy", "forward-proxy hop residual (audit/admin)"})
	_ = def(Definition{"l5.header.client_ip_spoof", LayerNetwork, 0, true, "proxy", "forged CDN/edge client-IP (audit/admin)"})
	_ = def(Definition{"l5.header.xff_multi_hop", LayerNetwork, 0, true, "proxy", "multi-hop XFF chain (audit/admin)"})
	_ = def(Definition{"l5.proxy.vpn_webrtc_leak", LayerNetwork, 0, true, "proxy", "WebRTC≠TCP VPN leak (audit/admin)"})
	_ = def(Definition{"l5.proxy.tor_circuit", LayerNetwork, 0, true, "proxy", "≥3-hop XFF Tor-class path (audit/admin)"})
	_ = def(Definition{"l5.proxy.anon_chain", LayerNetwork, 0, true, "proxy", "≥4-hop anon open-proxy chain (audit/admin)"})
	_ = def(Definition{"l5.tcp.not_observed", LayerNetwork, 0, true, "tcp", "TCP L4 plane inactive (no SYN/PROXY TLV)"})
	_ = def(Definition{"l5.tcp.ttl", LayerNetwork, 0, true, "tcp", "observed IP TTL (audit)"})
	_ = def(Definition{"l5.tcp.mss", LayerNetwork, 0, true, "tcp", "TCP MSS option (audit)"})
	_ = def(Definition{"l5.tcp.window", LayerNetwork, 0, true, "tcp", "TCP initial window (audit)"})
	_ = def(Definition{"l5.tcp.ttl_os", LayerNetwork, 0, true, "tcp", "TTL/OS vs UA OS mismatch (audit/admin)"})
	_ = def(Definition{"l5.ip.datacenter_asn", LayerNetwork, 0, true, "ip", "datacenter/hosting ASN (audit/admin)"})
	_ = def(Definition{"l5.ip.proxy_vpn_tor", LayerNetwork, 0, true, "ip", "proxy/vpn exit (audit/admin)"})
	_ = def(Definition{"l5.ip.tor_exit", LayerNetwork, 0, true, "ip", "Tor exit relay (audit/admin)"})
)

// ---- L5 request integrity (SoT-07) ----
var (
	// l5.rit.ok is an intentional OK signal (weight 0 — a valid RIT is not evidence of a
	// bot). Declared here so it is a KNOWN signal, not an UNKNOWN-safe fallback that a
	// resolution test (PLAN-07 R9) could not tell apart from a typo.
	_ = def(Definition{"l5.rit.ok", LayerNetwork, 0, true, "rit", "valid RIT token (no tamper)"})
	// l5.rit.bootstrap is the one-shot tokenless first-request grace (weight 0). Distinct
	// from l5.rit.ok so the HR-18 cross-check does not mistake the grace for verified JS
	// execution (deep-review).
	_ = def(Definition{"l5.rit.bootstrap", LayerNetwork, 0, true, "rit", "first-request token grace (one-shot)"})
	_ = def(Definition{"l5.rit.absent", LayerNetwork, 30, nil, "rit", "no RIT token on API call"})
	_ = def(Definition{"l5.rit.invalid_hmac", LayerNetwork, 35, nil, "rit", "RIT HMAC invalid"})
	_ = def(Definition{"l5.rit.header_tampered", LayerNetwork, 40, nil, "rit", "signed headers != observed"})
	_ = def(Definition{"l5.rit.stale_replay", LayerNetwork, 30, nil, "rit", "RIT counter/timebucket out of window"})
	_ = def(Definition{"l5.rit.body_mismatch", LayerNetwork, 35, nil, "rit", "body hash mismatch (fetch hook)"})
)

// ---- L5 resource / watermark (SoT-08) ----
var (
	_ = def(Definition{"l5.resource.mass_fetch", LayerNetwork, 15, nil, "resource", "many resources in short window"})
	_ = def(Definition{"l5.resource.no_referer", LayerNetwork, 10, nil, "resource", "direct resource fetch, no navigation"})
	_ = def(Definition{"l5.resource.rit_absent", LayerNetwork, 20, nil, "resource", "resource fetch without RIT"})
	_ = def(Definition{"l5.resource.hotlink", LayerNetwork, 8, nil, "resource", "external referer resource fetch"})
)

// ---- L5 media / embed gating (SoT-10) ----
var (
	_ = def(Definition{"l5.media.range_storm", LayerNetwork, 25, nil, "media", "parallel Range request storm"})
	_ = def(Definition{"l5.media.no_playback_fetch", LayerNetwork, 18, nil, "media", "full media fetched without playback"})
	_ = def(Definition{"l5.media.hotlink", LayerNetwork, 10, nil, "media", "external referer media fetch"})
	_ = def(Definition{"l5.embed.origin_disallowed", LayerNetwork, 12, nil, "embed", "embed from disallowed origin"})
	_ = def(Definition{"l5.embed.token_stripped", LayerNetwork, 22, nil, "embed", "RIT token absent in embed context"})
)

// ---- L7 proof-of-work active defense (SoT-13) ----
var (
	_ = def(Definition{"l7.pow.solved", LayerScoring, 0, nil, "pow", "valid PoW solution (trust upgrade)"})
	_ = def(Definition{"l7.pow.failed", LayerScoring, 15, nil, "pow", "invalid/expired PoW solution"})
	_ = def(Definition{"l7.pow.refused", LayerScoring, 15, nil, "pow", "PoW challenge ignored"})
)

// ---- L7 humanymous Pass active defense (SoT-36) ----
// Solving Pass is a trust upgrade (weight 0, via the OK verdict). Pass velocity is
// the axis-③ fusion: a session driving the challenge at machine cadence is flagged
// so it raises risk (a flooding bot that solves the puzzle is STILL caught) and taxes
// its PoW cost — never the puzzle (accessibility: cost shifts to crypto, SoT-36 §2).
var (
	_ = def(Definition{"l7.pass.solved", LayerScoring, 0, nil, "pass", "humanymous Pass cleared (trust upgrade)"})
	_ = def(Definition{"l7.pass.velocity", LayerScoring, 25, nil, "pass", "elevated Pass challenge/solve velocity (automation cadence)"})
	_ = def(Definition{"l7.pass.flood", LayerScoring, 45, nil, "pass", "Pass challenge/solve flood (mass automation)"})
)

// ---- L5 traffic consistency guard (SoT-12) ----
var (
	_ = def(Definition{"l5.traffic.ja4_rotation", LayerNetwork, 35, nil, "traffic", "JA4 fingerprint changed mid-session"})
	_ = def(Definition{"l5.traffic.engine_rotation", LayerNetwork, 40, nil, "traffic", "TLS engine family changed mid-session"})
	_ = def(Definition{"l5.traffic.ua_rotation", LayerNetwork, 30, nil, "traffic", "User-Agent changed mid-session"})
	// ip_hop is network residual (score-exempt); still participates in NET-POLICY (HR-24).
	_ = def(Definition{"l5.traffic.ip_hop", LayerNetwork, 0, true, "traffic", "remote IP /24 changed mid-session (audit/admin)"})
	_ = def(Definition{"l5.traffic.proto_downgrade", LayerNetwork, 15, nil, "traffic", "repeated h2->h1 downgrade"})
	_ = def(Definition{"l5.traffic.header_order_shift", LayerNetwork, 18, nil, "traffic", "header order changed under same UA"})
	_ = def(Definition{"l5.traffic.alpn_flap", LayerNetwork, 10, nil, "traffic", "unstable ALPN across requests"})
	_ = def(Definition{"l5.traffic.tls_no_permutation", LayerNetwork, 30, nil, "traffic", "Chrome TLS extension order never permutes (static parrot)"})
)

// ---- L5 abuse / DoS (SoT-17) ----
var (
	_ = def(Definition{"l5.abuse.rate_exceeded", LayerNetwork, 25, nil, "abuse", "request rate over soft threshold (per fingerprint)"})
	_ = def(Definition{"l5.abuse.flood", LayerNetwork, 45, nil, "abuse", "request flood over hard threshold (DoS/credential-stuffing)"})
	_ = def(Definition{"l5.h2dos.rapid_reset", LayerNetwork, 45, nil, "abuse", "HTTP/2 Rapid Reset (HEADERS then immediate RST_STREAM flood)"})
	_ = def(Definition{"l5.h2dos.continuation_flood", LayerNetwork, 45, nil, "abuse", "HTTP/2 CONTINUATION flood (no END_HEADERS)"})
)

// ---- L5 cross-session correlation (SoT-15) ----
var (
	// Correlation residuals are score-exempt; HR-19 is admin/network POLICY (not risk combine).
	_ = def(Definition{"l5.correlation.proxy_rotation", LayerNetwork, 0, true, "correlation", "one fingerprint across many subnets (audit + NET-POLICY)"})
	_ = def(Definition{"l5.correlation.shared_fingerprint", LayerNetwork, 0, true, "correlation", "one fingerprint shared by many sessions (audit)"})
	_ = def(Definition{"l5.correlation.fp_churn_proxy", LayerNetwork, 0, true, "correlation", "mid-session fingerprintId churn (audit + NET-POLICY)"})
	_ = def(Definition{"l7.pow.too_fast", LayerScoring, 30, nil, "pow", "PoW solved faster than JS-on-claimed-hardware (native/GPU solver)"})
)
