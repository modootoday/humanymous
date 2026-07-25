// assert-attack.mjs — CI gate for the detector-vs-bots attack run. Reads the
// results the runner wrote and fails the build on any escaped bot or a human
// false-positive.
//   node scripts/assert-attack.mjs [deployments/artifacts/core-results.json]
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export function evaluateAttackResults(records, expectedProfiles = 65) {
  if (!Array.isArray(records)) return { ok: false, failures: ['results root must be an array'] };

  const bots = records.filter((x) => x.label?.startsWith('bot:') && !x.skipped && !x.error);
  const escaped = bots.filter((x) => x.verdict === 'ALLOW');
  const human = records.filter((x) => x.label === 'human' && !x.skipped && !x.error);
  const humanFP = human.filter((x) => x.verdict === 'DENY');
  const ceiling = records.filter((x) => x.label?.startsWith('ceiling:') && !x.skipped && !x.error);
  const errored = records.filter((x) => x.error);
  const skipped = records.filter((x) => x.skipped);
  const profiles = new Set(records.map((x) => x.profile).filter(Boolean));
  const failures = [];

  if (!bots.length) failures.push('no bot records in results');
  if (profiles.size !== expectedProfiles) failures.push(`catalog coverage ${profiles.size}/${expectedProfiles} profiles`);
  if (!human.length) failures.push('human baseline is missing');
  if (!ceiling.length) failures.push('detection-ceiling record is missing');
  if (escaped.length) failures.push('bots reached ALLOW: ' + escaped.map((x) => x.label).join(', '));
  if (humanFP.length) failures.push('human baseline was DENYed (false positive)');
  if (errored.length) failures.push('profiles errored: ' + errored.map((x) => x.profile).join(', '));
  if (skipped.length) failures.push('profiles skipped: ' + skipped.map((x) => x.profile).join(', '));

  return {
    ok: failures.length === 0,
    failures,
    summary: {
      blocked: bots.length - escaped.length,
      bots: bots.length,
      escaped: escaped.length,
      humanFP: humanFP.length,
      errored: errored.length,
      skipped: skipped.length,
      profiles: profiles.size,
    },
  };
}

export function main(path = process.argv[2] || '/artifacts/core-results.json') {
  const records = JSON.parse(readFileSync(path, 'utf8'));
  const result = evaluateAttackResults(records, Number(process.env.HM_EXPECTED_PROFILES || 65));
  const s = result.summary || {};
  console.log(`bots ${s.blocked || 0}/${s.bots || 0} blocked | escaped ${s.escaped || 0} | human FP ${s.humanFP || 0} | errored ${s.errored || 0} | skipped ${s.skipped || 0} | profiles ${s.profiles || 0}`);
  for (const failure of result.failures) console.error('FAIL: ' + failure);
  if (!result.ok) return 1;
  console.log('PASS: engine war gate (complete catalog, all bots blocked, no false positive)');
  return 0;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) process.exitCode = main();
