// tiers.mjs — the Red cost-escalation ladder. Maps every catalog profile to an attacker COST
// tier so the final Red/Blue wargame runs and reports as a STAGED escalation, from the
// cheapest trivial script (T0) up to a funded real-engine adversary and the honest detection
// ceiling (T4). This is the canonical, code-retained tiering (every wargame is part of the
// development record, not a one-off): a future profile added to runner.mjs must be placed in a
// tier here, and the runner fails loudly if the two lists drift.
//
// Tiers follow the documented T0-T4 attacker model (docs/explanation/where-gate-fits.md),
// annotated with the escalating cost/effort a real adversary spends. The ladder was designed
// from a 10-surface web-research survey of 2025-2026 anti-bot evasion (TLS/JA4 mimicry, HTTP/2
// & smuggling, stealth frameworks, anti-detect browsers, fingerprint spoofing, behavioral
// evasion, AI agents, proxy/token/credential abuse). Blue's honest expectation: reliable
// through T2, degrading-but-scoring at T3, and NOT solved at T4 (a coherent engine-level spoof
// or a real human is not separable by detection alone — only rate/reputation raise its cost).
//
// A profile's OUTCOME class is derived by the runner from its label prefix: 'bot:' = must-block
// (TP if DENY/CHALLENGE, FN if ALLOW), 'ceiling:' = detection ceiling (ALLOW is the honest
// expected result, not a miss), 'human'/baseline = must-not-block (FP if DENY).

export const BASELINE = 'human.mjs';

export const TIERS = [
  {
    id: 'T0',
    cost: 'trivial — $0, a script, no browser',
    desc: 'non-browser HTTP/uTLS clients, header/token tricks, and the cheapest behavior tells',
    expect: 'reliable',
    profiles: [
      'nonbrowser_ua.mjs', 'http_client.mjs', 'tls_parrot.mjs',
      'curl_impersonate_chrome.mjs', 'curl_impersonate_chrome99_android.mjs',
      'tls_static.mjs', 'pq_absent.mjs', 'alps_absent.mjs', 'cert_compression_absent.mjs',
      'sec_chua_absent.mjs', 'sec_fetch_absent.mjs', 'rit_absent.mjs', 'rit_replay.mjs',
      'rit_tamper.mjs', 'ua_rotate.mjs', 'xff_spoof.mjs',
      'behavior_no_interaction.mjs', 'behavior_untrusted.mjs',
    ],
  },
  {
    id: 'T1',
    cost: 'low — $, off-the-shelf tools on defaults',
    desc: 'naive automation frameworks, single-axis TLS/engine churn, and cheap resource/DoS abuse',
    expect: 'reliable',
    profiles: [
      'selenium.mjs', 'puppeteer.mjs', 'playwright_plain.mjs', 'undetected.mjs', 'direct_cdp.mjs',
      'tls_rotate.mjs', 'ja4_churn.mjs', 'grease_absent_js.mjs',
      'video_scrape.mjs', 'watermark_strip.mjs', 'flood.mjs', 'rapid_reset.mjs',
      'h2_flow_control.mjs',
    ],
  },
  {
    id: 'T2',
    cost: 'moderate — $$, stealth plugins / patched natives / behavioral humanizers',
    desc: 'stealth-patched automation, multi-axis rotation, fingerprint-spoof contradictions, and mouse/keystroke humanizers',
    expect: 'reliable',
    profiles: [
      'puppeteer_stealth.mjs', 'playwright_stealth.mjs', 'patchright.mjs',
      'rebrowser_cdp_stripped.mjs', 'mobile_ua_desktop_profile.mjs',
      'near_ceiling_audio_24k.mjs', 'near_ceiling_no_widevine.mjs', 'browser_use_cdp.mjs',
      'multi_axis_rotate.mjs', 'adv_webgpu_mismatch.mjs', 'headless_ua_token.mjs', 'signal_forgery.mjs',
      'behavior_machine_keystroke.mjs', 'behavior_teleport_click.mjs', 'behavior_bezier_mouse.mjs',
      'behavior_fixed_typing.mjs', 'ai_burst_silence.mjs',
    ],
  },
  {
    id: 'T3',
    cost: 'high — $$$, real browser engine + proxy/AI infrastructure',
    desc: 'real-engine anti-detect, AI cadence, residential/anonymous proxy chains, Squid/OpenVPN/WireGuard/Tor/SOCKS, CDN IP spoof, fp-churn evasion',
    expect: 'degrades gracefully — scores + challenges/denies at lower confidence',
    profiles: [
      'nodriver.mjs', 'xvfb_headful.mjs', 'antidetect.mjs', 'camoufox.mjs',
      'ai_agent.mjs', 'ai_full_cadence.mjs', 'distributed.mjs', 'privacy_evasion.mjs',
      'squid_forward.mjs', 'openvpn_exit.mjs', 'wireguard_hop.mjs', 'tor_exit.mjs',
      'anon_proxy_chain.mjs', 'elite_anon_proxy.mjs', 'cdn_ip_spoof.mjs',
      'proxy_ua_rotate.mjs', 'fp_churn_proxy.mjs', 'stacked_proxy_vpn.mjs', 'socks_exit_hop.mjs',
    ],
  },
  {
    id: 'T4',
    cost: 'very high — $$$$, an engine-level spoof or a real human behind residential proxies',
    desc: 'a fully coherent BotBrowser-class spoof, or genuine human behavior on a real engine — not separable by DETECTION alone',
    expect: 'NOT solved (stated ceiling) — the coherent case SCORES ALLOW; mitigated only by rate limiting + cross-session reputation',
    // native_coherent is a ceiling: profile (ALLOW is the honest expected result). The
    // remaining T4 vectors (JA4T TCP-SYN OS, QUIC/H3 JA4Q, population-scale JA4 analytics) are
    // NOT locally exercisable against this HTTP/2 engine and are documented, not shipped.
    profiles: ['native_coherent_ceiling.mjs'],
  },
];

// tierOf returns the tier id for a profile file (or 'baseline' / '?').
export function tierOf(profile) {
  if (profile === BASELINE) return 'baseline';
  for (const t of TIERS) if (t.profiles.includes(profile)) return t.id;
  return '?';
}

// assertCoverage fails loudly if the flat catalog and the tier map have drifted — the same
// no-silent-drift discipline as the launchProfiles parity guard.
export function assertCoverage(catalog) {
  const nonBaseline = catalog.filter((p) => p !== BASELINE);
  const tiered = new Set(TIERS.flatMap((t) => t.profiles));
  const missing = nonBaseline.filter((p) => !tiered.has(p));
  const extra = [...tiered].filter((p) => !catalog.includes(p));
  if (missing.length || extra.length) {
    throw new Error(
      `tier/catalog drift — untiered profiles: [${missing.join(', ')}]; tier-only profiles not in catalog: [${extra.join(', ')}]`,
    );
  }
}
