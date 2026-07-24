// tiers.mjs — the Red cost-escalation ladder. Maps every catalog profile to an attacker
// COST tier so the final Red/Blue wargame runs and reports as a STAGED escalation, from a
// trivial cheap script (T0) up to a funded real-browser-plus-human adversary (T4). This is
// the canonical, code-retained tiering (every wargame is part of the development record, not
// a one-off): a future profile added to runner.mjs must be placed in a tier here, and the
// runner fails loudly if the two lists drift.
//
// The tiers follow the documented T0–T4 attacker model (docs/explanation/where-gate-fits.md),
// annotated with the escalating cost/effort a real adversary spends to reach that tier. Blue's
// honest expectation: reliable through T2, degrading-but-scoring at T3, and NOT solved at T4
// (the detection ceiling — a real human on a real engine cannot be separated by detection
// alone; only rate/reputation raise its cost).

export const BASELINE = 'human.mjs';

export const TIERS = [
  {
    id: 'T0',
    cost: 'trivial — $0, a script, no browser',
    desc: 'non-browser HTTP/uTLS clients & raw-protocol abuse (curl_cffi / bare libs)',
    expect: 'reliable',
    profiles: [
      'http_client.mjs', 'tls_parrot.mjs', 'tls_static.mjs', 'tls_rotate.mjs', 'ua_rotate.mjs',
      'signal_forgery.mjs', 'xff_spoof.mjs', 'rit_replay.mjs', 'rit_tamper.mjs',
      'flood.mjs', 'rapid_reset.mjs',
    ],
  },
  {
    id: 'T1',
    cost: 'low — $, off-the-shelf tools on defaults',
    desc: 'naive automation frameworks (Selenium/Puppeteer/Playwright/undetected, raw CDP) + cheap resource abuse',
    expect: 'reliable',
    profiles: [
      'selenium.mjs', 'puppeteer.mjs', 'playwright_plain.mjs', 'undetected.mjs', 'direct_cdp.mjs',
      'video_scrape.mjs', 'watermark_strip.mjs',
    ],
  },
  {
    id: 'T2',
    cost: 'moderate — $$, stealth plugins / patched natives',
    desc: 'stealth-patched automation with residual leaks (puppeteer-extra-stealth, patchright)',
    expect: 'reliable',
    profiles: ['puppeteer_stealth.mjs', 'playwright_stealth.mjs', 'patchright.mjs'],
  },
  {
    id: 'T3',
    cost: 'high — $$$, real browser engine + proxy/AI infrastructure',
    desc: 'real-engine anti-detect (nodriver/camoufox/xvfb-headful/anti-detect) + residential-proxy rotation + LLM browser agents',
    expect: 'degrades gracefully — scores + challenges/denies at lower confidence',
    profiles: [
      'nodriver.mjs', 'xvfb_headful.mjs', 'antidetect.mjs', 'camoufox.mjs',
      'ai_agent.mjs', 'distributed.mjs', 'privacy_evasion.mjs',
    ],
  },
  {
    id: 'T4',
    cost: 'very high — $$$$, a real browser on real hardware driven by a real human (click-farm) behind residential proxies',
    desc: 'anti-detect stack + genuine human behavior — indistinguishable from a legitimate user by DETECTION alone',
    expect: 'NOT solved (stated ceiling) — mitigated only by rate limiting + reputation, never a detection verdict',
    // No profile: by construction there is no clean detection tell. This tier is represented
    // as a documented boundary so the escalation ladder is honest about where Blue stops.
    profiles: [],
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
  const bots = catalog.filter((p) => p !== BASELINE);
  const tiered = new Set(TIERS.flatMap((t) => t.profiles));
  const missing = bots.filter((p) => !tiered.has(p));
  const extra = [...tiered].filter((p) => !catalog.includes(p));
  if (missing.length || extra.length) {
    throw new Error(
      `tier/catalog drift — untiered profiles: [${missing.join(', ')}]; tier-only profiles not in catalog: [${extra.join(', ')}]`,
    );
  }
}
