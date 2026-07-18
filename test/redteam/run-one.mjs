// run-one.mjs — thin launch wrapper for the Detection Observatory (SoT-30 §4.4).
// Runs exactly ONE bundled catalog profile against the LOCAL Blue engine and
// prints its verdict as a single JSON line. Invoked by the Go launcher with the
// base URL hard-coded to loopback; this wrapper never chooses a target itself.
// Local target only — this validates your own detection engine.
import { pathToFileURL, fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';

process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0'; // self-signed dev cert

const [, , profile, base] = process.argv;
if (!profile || !base) {
  console.error('usage: run-one.mjs <profile.mjs> <base-url>');
  process.exit(2);
}
// Refuse anything but a bare profile filename (no path traversal) and a loopback
// base, so even a misinvocation cannot reach off-box.
if (profile.includes('/') || profile.includes('\\') || profile.includes('..') || !profile.endsWith('.mjs')) {
  console.error('invalid profile name');
  process.exit(2);
}
if (!/^https?:\/\/(127\.0\.0\.1|localhost|\[::1\])(:\d+)?/.test(base)) {
  console.error('base must be loopback');
  process.exit(2);
}

const __dir = dirname(fileURLToPath(import.meta.url));
try {
  const mod = await import(pathToFileURL(join(__dir, profile)).href);
  if (typeof mod.run !== 'function') {
    console.error('profile exports no run()');
    process.exit(2);
  }
  const v = await mod.run(base);
  console.log(JSON.stringify({
    label: mod.label || profile,
    verdict: v.verdict,
    riskScore: v.riskScore,
    hardRuleFired: v.hardRuleFired || '',
  }));
} catch (e) {
  console.error('run failed: ' + ((e && e.message) || e));
  process.exit(1);
}
