// runner.mjs — Red vs Blue e2e harness. Runs each available Red profile against
// the LOCAL Blue server, collects verdicts, and writes results.json. Browser
// profiles are skipped gracefully if playwright-core is not installed
// (plan/04 §3). Target is hard-coded to localhost — no external targets.

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
  'human.mjs',                 // baseline (expect ALLOW/not-denied, FPR=0)
  // --- T0 · trivial ($0, a script, no browser) — HTTP/uTLS clients & raw-protocol abuse ---
  'http_client.mjs',           // non-browser scraper (L5/L6)
  'tls_parrot.mjs',            // uTLS Chrome ClientHello parrot (header/JS residual)
  'tls_static.mjs',            // static parrot (no TLS permutation)
  'tls_rotate.mjs',            // mid-session TLS fingerprint rotation -> HR-14
  'ua_rotate.mjs',             // mid-session User-Agent rotation
  'signal_forgery.mjs',        // forged l7.pass.solved/l7.pow.solved -> stripped, no ALLOW (round-3 provenance)
  'xff_spoof.mjs',             // forged private X-Forwarded-For -> l5.header.forwarded_private
  'rit_replay.mjs',            // RIT token replay -> HR-17
  'rit_tamper.mjs',            // RIT body tamper -> HR-16
  'flood.mjs',                 // application-layer request flood -> score CHALLENGE + ban ladder
  'rapid_reset.mjs',           // HTTP/2 Rapid Reset DoS (CVE-2023-44487) -> HR-21 (SoT-17)
  // --- T1 · low ($, off-the-shelf tools on defaults) — naive automation frameworks ---
  'selenium.mjs',              // cdc_ artifacts (HR-1)
  'puppeteer.mjs',             // headless + webdriver (HR-7)
  'playwright_plain.mjs',      // headless + webdriver (HR-7)
  'undetected.mjs',            // headless, webdriver stripped (HR-7)
  'direct_cdp.mjs',            // raw CDP driver
  'video_scrape.mjs',          // media Range-storm on heavy resource
  'watermark_strip.mjs',       // resource leak + metadata strip (forensic trace)
  // --- T2 · moderate ($$, stealth plugins / patched natives) ---
  'puppeteer_stealth.mjs',     // stealth evasions (patched natives L3) (HR-8)
  'playwright_stealth.mjs',    // patched natives (L3) (HR-8)
  'patchright.mjs',            // console disabled (Patchright) (HR-9)
  // --- T3 · high ($$$, real browser engine + proxy/AI infra) — degrades gracefully ---
  'nodriver.mjs',              // headful frontier (behavior) (HR-12)
  'xvfb_headful.mjs',          // headful, no display tells (behavior)
  'antidetect.mjs',            // anti-detect browser (coherent spoof + behavior)
  'camoufox.mjs',              // Firefox fork (Playwright Firefox stand-in)
  'ai_agent.mjs',              // LLM browser-agent cadence -> HR-20
  'distributed.mjs',           // rotating residential-proxy pool -> HR-19
  'privacy_evasion.mjs',       // proxy-rotation + forged adBlock/GPC -> still HR-19 DENY (round-5 regression)
];
assertCoverage(PROFILES); // fail loudly if the catalog and the tier ladder (tiers.mjs) drift

function classify(label, verdict) {
  const isBot = label.startsWith('bot:');
  const denied = verdict === 'DENY' || verdict === 'CHALLENGE';
  if (isBot) return denied ? 'TP' : 'FN';      // bot caught?
  return verdict === 'DENY' ? 'FP' : 'TN';     // human wrongly denied?
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
  let prevBlocked = true;
  for (const t of TIERS) {
    const recs = t.profiles.map((p) => byProfile[p]).filter(Boolean);
    const ran = recs.length;
    const blocked = recs.filter((r) => r.verdict === 'DENY' || r.verdict === 'CHALLENGE').length;
    const allowed = recs.filter((r) => r.verdict === 'ALLOW');
    const skipped = t.profiles.length - ran;
    const rate = ran ? Math.round((blocked / ran) * 100) : (t.id === 'T4' ? 100 : 0);
    const bar = t.id === 'T4'
      ? 'n/a — ceiling (no detection profile; rate/reputation only)'
      : `${blocked}/${ran} blocked${skipped ? ` (+${skipped} skipped)` : ''} = ${rate}%`;
    console.log(`\n  [${t.id}] ${t.cost}`);
    console.log(`       ${t.desc}`);
    console.log(`       expect: ${t.expect}`);
    console.log(`       result: ${bar}`);
    if (allowed.length) {
      console.log(`       !! reached ALLOW: ${allowed.map((r) => r.label).join(', ')}`);
    }
    if (ran && blocked < ran) prevBlocked = false;
    if (prevBlocked && ran && blocked === ran && t.id !== 'T4') {
      // still fully caught at this cost tier
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
