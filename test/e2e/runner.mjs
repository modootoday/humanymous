// runner.mjs — Red vs Blue e2e harness. Runs each available Red profile against
// the LOCAL Blue server, collects verdicts, and writes results.json. Browser
// profiles are skipped gracefully if playwright-core is not installed
// (plan/04 §3). Target is hard-coded to localhost — no external targets.

import { writeFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';

const BASE = process.env.HM_BASE || 'https://127.0.0.1:8443';
const RUNS = Number(process.env.HM_RUNS || 3);
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0'; // self-signed dev cert

const __dir = dirname(fileURLToPath(import.meta.url));
const redteam = join(__dir, '..', 'redteam');

// Full SoT-04 automation catalog (local target only). Tools that need a binary
// we don't have are simulated via their documented client/network tells, clearly
// labeled in each file; camoufox needs Playwright Firefox and skips if absent.
const PROFILES = [
  'human.mjs',                 // baseline (expect ALLOW, FPR=0)
  'http_client.mjs',           // non-browser scraper (L5/L6)
  'tls_parrot.mjs',            // uTLS Chrome ClientHello parrot (header/JS residual)
  'selenium.mjs',              // cdc_ artifacts
  'puppeteer.mjs',             // headless + webdriver
  'puppeteer_stealth.mjs',     // stealth evasions (patched natives L3)
  'playwright_plain.mjs',      // headless + webdriver
  'playwright_stealth.mjs',    // patched natives (L3)
  'undetected.mjs',            // headless, webdriver stripped
  'patchright.mjs',            // console disabled (Patchright)
  'direct_cdp.mjs',            // raw CDP driver
  'nodriver.mjs',              // headful frontier (behavior)
  'xvfb_headful.mjs',          // headful, no display tells (behavior)
  'antidetect.mjs',            // anti-detect browser (coherent spoof + behavior)
  'camoufox.mjs',              // Firefox fork (Playwright Firefox stand-in)
  // --- aggressive anti-bypass evasions (SoT-07/08/10/12) ---
  'tls_static.mjs',            // static parrot (no TLS permutation) -> HR-14
  'tls_rotate.mjs',            // mid-session TLS fingerprint rotation -> HR-14
  'ua_rotate.mjs',             // mid-session User-Agent rotation
  'rit_replay.mjs',            // RIT token replay -> HR-17
  'rit_tamper.mjs',            // RIT body tamper -> HR-16
  'video_scrape.mjs',          // media Range-storm on heavy resource
  'watermark_strip.mjs',       // resource leak + metadata strip (forensic trace)
  // --- frontier threats (SoT-15/16): AI agents + distributed proxy pools ---
  'ai_agent.mjs',              // LLM browser-agent cadence -> HR-20
  'distributed.mjs',           // rotating residential-proxy pool -> HR-19
  'xff_spoof.mjs',             // forged private X-Forwarded-For -> l5.header.forwarded_private
  'flood.mjs',                 // application-layer request flood -> score CHALLENGE + ban ladder
  'rapid_reset.mjs',           // HTTP/2 Rapid Reset DoS (CVE-2023-44487) -> HR-21 (SoT-17)
  // --- deployment-review-hardened evasions (rounds 3 & 5): must stay caught ---
  'signal_forgery.mjs',        // forged l7.pass.solved/l7.pow.solved -> stripped, no ALLOW (round-3 provenance blocker)
  'privacy_evasion.mjs',       // proxy-rotation + forged adBlock/GPC -> still HR-19 DENY (round-5 regression)
];

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
