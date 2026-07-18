// assert-attack.mjs — CI gate for the detector-vs-bots attack run. Reads the
// results the runner wrote and fails the build on any escaped bot or a human
// false-positive.
//   node scripts/assert-attack.mjs [deployments/artifacts/core-results.json]
import { readFileSync } from 'node:fs';

const path = process.argv[2] || 'deployments/artifacts/core-results.json';
const r = JSON.parse(readFileSync(path, 'utf8'));

const bots = r.filter((x) => x.label && x.label.startsWith('bot:') && !x.skipped && !x.error);
const escaped = bots.filter((x) => x.verdict === 'ALLOW');
const humanFP = r.filter((x) => x.label === 'human' && x.verdict === 'DENY');
const errored = r.filter((x) => x.error);
const blocked = bots.length - escaped.length;

console.log(`bots ${blocked}/${bots.length} blocked | escaped ${escaped.length} | human FP ${humanFP.length} | errored ${errored.length}`);

let ok = true;
if (!bots.length) { console.error('FAIL: no bot records in results'); ok = false; }
if (escaped.length) { console.error('FAIL: bots reached ALLOW → ' + escaped.map((x) => x.label).join(', ')); ok = false; }
if (humanFP.length) { console.error('FAIL: human baseline was DENYed (false positive)'); ok = false; }
if (errored.length) { console.error('WARN: ' + errored.length + ' profile(s) errored: ' + errored.map((x) => x.profile).join(', ')); }

if (!ok) process.exit(1);
console.log('PASS: engine war gate (all bots blocked, no false positive)');
