// runner.mjs — Red vs Blue e2e harness. Runs each available Red profile against
// the detection engine, collects verdicts, and writes results.json.
//
// AUTHORITATIVE PATH: Docker bots container (deployments/bots/run-attack.sh) with
// HM_BASE=https://core:8443 on the internal lab network. See scripts/e2e-docker.sh
// and make e2e. Host/loopback runs (default HM_BASE=https://127.0.0.1:8443) are for
// profile authoring only — not completion authority for L5 / multi-subnet work.
// Browser profiles skip gracefully if playwright-core is missing (plan/04 §3).
// No external targets.

import { writeFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';
import { TIERS, BASELINE, tierOf, assertCoverage } from './tiers.mjs';

const BASE = process.env.HM_BASE || 'https://127.0.0.1:8443';
const RUNS = Number(process.env.HM_RUNS || 3);
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0'; // self-signed dev cert

const __dir = dirname(fileURLToPath(import.meta.url));
const redteam = join(__dir, '..', 'redteam');

// Full SoT-04 automation catalog (local target only), ordered as a STAGED COST ESCALATION:
// the baseline first, then the Red ladder from the cheapest trivial script (T0) up to the
// funded real-engine adversary (T3); T4 (a real human on a real engine) is the documented
// detection ceiling and has no profile. The tier of each entry is the single source of truth
// in tiers.mjs; assertCoverage() below fails loudly if this list and the tiers drift. Tools
// that need a binary we don't have are simulated via their documented tells; camoufox needs
// Playwright Firefox and skips if absent.
const PROFILES = [
  'human.mjs',                     // baseline (expect ALLOW/not-denied, FPR=0)
  // --- T0 · trivial ($0, a script, no browser) — HTTP/uTLS clients, header/token tricks ---
  'nonbrowser_ua.mjs',             // bare HTTP library (library UA) -> x.non_browser_ua + HR-10
  'http_client.mjs',               // non-browser scraper (L5/L6)
  'tls_parrot.mjs',                // uTLS Chrome ClientHello parrot (header/JS residual)
  'curl_impersonate_chrome.mjs',   // curl-impersonate Chrome116 TLS/JA4 parrot (Docker)
  'curl_impersonate_chrome99_android.mjs', // curl-impersonate Android Chrome99 (Docker)
  'tls_static.mjs',                // static parrot (no TLS permutation)
  'pq_absent.mjs',                 // Chrome/131 UA over a pre-PQ uTLS parrot (no X25519MLKEM768) -> HR-24
  'alps_absent.mjs',               // Chrome UA over h2 without the ALPS extension (non-Chromium TLS stack) -> HR-24
  'cert_compression_absent.mjs',   // browser UA over h2 without compress_certificate (RFC 8879 ext 27) -> HR-24
  'sec_chua_absent.mjs',           // Chrome UA without Sec-CH-UA -> x.uach_present
  'sec_fetch_absent.mjs',          // Chrome UA without Sec-Fetch-* -> sec_fetch_missing
  'rit_absent.mjs',                // API call with no RIT token -> HR-17
  'rit_replay.mjs',                // RIT token replay -> HR-17
  'rit_tamper.mjs',                // RIT body tamper -> HR-16
  'ua_rotate.mjs',                 // mid-session User-Agent rotation
  'xff_spoof.mjs',                 // forged private X-Forwarded-For -> l5.header.forwarded_private
  'behavior_no_interaction.mjs',   // zero interaction over the window -> HR-12
  'behavior_untrusted.mjs',        // isTrusted=false injected events -> l4.event.untrusted
  // --- T1 · low ($, off-the-shelf tools) — naive frameworks, TLS churn, resource/DoS ---
  'selenium.mjs',                  // cdc_ artifacts (HR-1)
  'puppeteer.mjs',                 // headless + webdriver (HR-7)
  'playwright_plain.mjs',          // headless + webdriver (HR-7)
  'undetected.mjs',                // headless, webdriver stripped (HR-7)
  'direct_cdp.mjs',                // raw CDP driver
  'tls_rotate.mjs',                // mid-session engine rotation -> HR-14
  'ja4_churn.mjs',                 // 3+ distinct JA4 in one session -> HR-14
  'grease_absent_js.mjs',          // no-GREASE Go TLS + JS -> grease_absent + x.ua_vs_ja4
  'video_scrape.mjs',              // media Range-storm on heavy resource
  'watermark_strip.mjs',           // resource leak + metadata strip (forensic trace)
  'flood.mjs',                     // application-layer request flood -> score CHALLENGE + ban ladder
  'rapid_reset.mjs',               // HTTP/2 Rapid Reset DoS (CVE-2023-44487) -> HR-21 (SoT-17)
  'h2_flow_control.mjs',           // Chrome h2 pseudo-order+SETTINGS but 1 GiB flow-control window -> HR-24
  'h2_priority_absent.mjs',        // Chrome h2 pseudo-order+SETTINGS+window but no priority signal -> HR-24
  // --- T2 · moderate ($$, stealth / rotation / fingerprint-spoof / humanizers) ---
  'puppeteer_stealth.mjs',         // stealth-patched natives (L3) (HR-8)
  'playwright_stealth.mjs',        // patched natives (L3) (HR-8)
  'patchright.mjs',                // console disabled (Patchright) (HR-9)
  'rebrowser_cdp_stripped.mjs',    // rebrowser-class CDP Runtime.enable stripped; residual headless+no-interact
  'mobile_ua_desktop_profile.mjs', // mobile UA/UA-CH claim + desktop touch/pointer residual
  'near_ceiling_audio_24k.mjs',    // T4-adjacent: coherent-ish + AudioContext 24k residual
  'near_ceiling_no_widevine.mjs',  // T4-adjacent: Chrome UA without Widevine/media residual
  'browser_use_cdp.mjs',           // pure-CDP agent residual (public Browser Use CDP era)
  'multi_axis_rotate.mjs',         // UA + TLS rotate together -> HR-14/HR-15
  'adv_webgpu_mismatch.mjs',       // WebGL vs WebGPU vendor contradiction -> l2.adv.webgpu_mismatch
  'headless_ua_token.mjs',         // HeadlessChrome UA token + a second tell
  'signal_forgery.mjs',            // forged l7.pass.solved/l7.pow.solved -> stripped, no ALLOW (round-3)
  'behavior_machine_keystroke.mjs',// sub-25ms machine typing -> l4.key.machine_speed
  'behavior_teleport_click.mjs',   // clicks with no approach trajectory -> l4.mouse.click_no_trajectory
  'behavior_bezier_mouse.mjs',     // pathologically smooth ghost-cursor path -> l4.mouse.*
  'behavior_fixed_typing.mjs',     // fixed-interval typing (near-zero variance) -> l4.key.*
  'ai_burst_silence.mjs',          // LLM inference-loop cadence -> l4.agent.burst_silence
  // --- T3 · high ($$$, real engine + AI + proxy infra) — degrades gracefully ---
  'nodriver.mjs',                  // headful frontier (behavior) (HR-12)
  'xvfb_headful.mjs',              // headful, no display tells (behavior)
  'antidetect.mjs',                // anti-detect browser (coherent spoof + behavior)
  'camoufox.mjs',                  // Firefox fork (Playwright Firefox stand-in)
  'ai_agent.mjs',                  // LLM browser-agent cadence -> HR-20
  'ai_full_cadence.mjs',           // teleport + burst-silence + machine keystroke -> HR-20
  'distributed.mjs',               // rotating residential-proxy pool -> HR-19
  'privacy_evasion.mjs',           // proxy-rotation + forged adBlock/GPC -> still HR-19 (round-5)
  'squid_forward.mjs',             // open Squid forward-proxy hop headers -> HR-24
  'openvpn_exit.mjs',              // OpenVPN exit + WebRTC public IP leak -> HR-24
  'wireguard_hop.mjs',             // WireGuard mid-session exit hop + multi-hop XFF -> HR-24
  'tor_exit.mjs',                  // Tor circuit multi-hop + exit rotation -> HR-24/HR-19
  'anon_proxy_chain.mjs',          // free open-proxy 4+ hop chain -> HR-24
  'elite_anon_proxy.mjs',          // elite Forwarded multi (Via stripped) -> HR-24
  'cdn_ip_spoof.mjs',              // forged CF/True-Client-IP -> HR-24
  'proxy_ua_rotate.mjs',           // exit hop + UA rotate -> HR-15/HR-19
  'fp_churn_proxy.mjs',            // rotate fingerprint per exit (dodge HR-19) -> still HR-19
  'stacked_proxy_vpn.mjs',         // Squid over VPN + WebRTC leak -> HR-24
  'socks_exit_hop.mjs',            // SOCKS5/VPN L4 exit hop -> HR-24
  // --- T4 · very high — the detection ceiling (ALLOW is the honest expected result) ---
  'native_coherent_ceiling.mjs',   // fully-coherent Chrome-TLS engine spoof -> ALLOW (ceiling)
];
assertCoverage(PROFILES); // fail loudly if the catalog and the tier ladder (tiers.mjs) drift

function classify(label, verdict) {
  const denied = verdict === 'DENY' || verdict === 'CHALLENGE';
  // A 'ceiling:' profile is the honest DETECTION CEILING (a coherent engine-level spoof): ALLOW
  // is the EXPECTED result, not a miss. If Blue happens to catch it, that is a bonus, not a fail.
  if (label.startsWith('ceiling:')) return denied ? 'CEIL-CAUGHT' : 'CEILING';
  if (label.startsWith('bot:')) return denied ? 'TP' : 'FN'; // must-block bot caught?
  return verdict === 'DENY' ? 'FP' : 'TN';                   // baseline human wrongly denied?
}

async function main() {
  const results = [];
  for (const file of PROFILES) {
    let mod;
    try {
      mod = await import(pathToFileURL(join(redteam, file)).href);
    } catch (e) {
      results.push({ profile: file, skipped: true, reason: String(e.message || e) });
      console.log(`SKIP ${file}: ${e.message || e}`);
      continue;
    }
    for (let i = 0; i < RUNS; i++) {
      try {
        const v = await mod.run(BASE);
        const rec = {
          profile: file,
          label: mod.label,
          run: i,
          verdict: v.verdict,
          riskScore: v.riskScore,
          hardRuleFired: v.hardRuleFired,
          outcome: classify(mod.label, v.verdict),
          top: (v.topContributors || []).map((c) => c.id),
        };
        results.push(rec);
        console.log(`${mod.label} #${i}: ${v.verdict} (risk=${v.riskScore}, rule=${v.hardRuleFired || '-'}) [${rec.outcome}]`);
      } catch (e) {
        results.push({ profile: file, label: mod.label, run: i, error: String(e.message || e) });
        console.log(`ERROR ${mod.label} #${i}: ${e.message || e}`);
      }
    }
  }

  const outPath = join(__dir, 'results.json');
  writeFileSync(outPath, JSON.stringify(results, null, 2));
  console.log(`\nwrote ${outPath} (${results.length} records)`);
  summarize(results);
  tieredReport(results);
  const incomplete = results.filter((r) => r.error || r.skipped);
  if (incomplete.length) {
    console.error(`\nFAIL: ${incomplete.length} catalog record(s) errored or skipped`);
    process.exitCode = 1;
  }
}

// tieredReport prints the Red cost-escalation ladder (T0 cheapest -> T4 most expensive) with
// Blue's block rate at each tier, so the wargame reads as a staged escalation and shows
// exactly where — and at what attacker cost — Blue's confidence degrades. Retained as code.
function tieredReport(results) {
  // Reduce to one verdict per profile (the last non-error run), keyed by profile file.
  const byProfile = {};
  for (const r of results) {
    if (r.skipped || r.error || !r.label) continue;
    byProfile[r.profile] = r;
  }
  console.log('\n=== Red/Blue cost-escalation ladder (cheap -> expensive) ===');
  const base = results.find((r) => r.profile === BASELINE && !r.error && !r.skipped);
  if (base) {
    const denied = base.verdict === 'DENY';
    console.log(`  baseline (${base.label}) : ${base.verdict}  -> ${denied ? 'FALSE-POSITIVE (a human was denied!)' : 'ok (not denied)'}`);
  }
  for (const t of TIERS) {
    const recs = t.profiles.map((p) => byProfile[p]).filter(Boolean);
    const skipped = t.profiles.length - recs.length;
    // Must-block bots vs honest-ceiling profiles are scored differently.
    const bots = recs.filter((r) => (r.label || '').startsWith('bot:'));
    const ceilings = recs.filter((r) => (r.label || '').startsWith('ceiling:'));
    const blocked = bots.filter((r) => r.verdict === 'DENY' || r.verdict === 'CHALLENGE').length;
    const leaked = bots.filter((r) => r.verdict === 'ALLOW'); // a must-block bot that reached ALLOW = regression
    const rate = bots.length ? Math.round((blocked / bots.length) * 100) : 100;
    console.log(`\n  [${t.id}] ${t.cost}`);
    console.log(`       ${t.desc}`);
    console.log(`       expect: ${t.expect}`);
    if (bots.length) {
      // The rate can only speak for profiles that actually ran, so a tier with
      // skipped profiles must never render as a bare 100%: run 30239746911 read
      // "7/7 blocked (+5 skipped) = 100%" while five profiles could not launch
      // at all. State the block rate and the coverage loss as separate facts.
      const coverage = skipped
        ? ` — INCOMPLETE: ${skipped}/${t.profiles.length} profile(s) did not run`
        : '';
      console.log(`       must-block: ${blocked}/${bots.length} blocked = ${rate}% of attempted${coverage}`);
    }
    for (const c of ceilings) {
      const honest = c.verdict === 'ALLOW';
      console.log(`       ceiling: ${c.label} -> ${c.verdict}  (${honest ? 'ALLOW = the honest ceiling, as expected' : 'caught — a bonus over the ceiling'})`);
    }
    if (leaked.length) {
      console.log(`       !! REGRESSION — must-block bot reached ALLOW: ${leaked.map((r) => r.label).join(', ')}`);
    }
  }
  console.log('\n  Reading it: Blue should hold 100% through T2 and score/challenge (not necessarily');
  console.log('  DENY) at T3; any bot reaching ALLOW below T4 is a regression. T4 is the documented');
  console.log('  detection ceiling — a real human on a real engine, mitigated by rate/reputation only.');
}

function summarize(results) {
  const byLabel = {};
  for (const r of results) {
    if (r.skipped || r.error || !r.label) continue;
    (byLabel[r.label] ||= []).push(r);
  }
  console.log('\n=== summary ===');
  for (const [label, recs] of Object.entries(byLabel)) {
    const verdicts = recs.reduce((m, r) => ((m[r.verdict] = (m[r.verdict] || 0) + 1), m), {});
    console.log(`${label}: ${JSON.stringify(verdicts)}`);
  }
}

main();
