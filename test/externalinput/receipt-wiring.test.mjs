import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const composeUrl = new URL(
  '../../deployments/compose/external-input-bots.yaml',
  import.meta.url,
);
const smokeUrl = new URL('../../scripts/external-input-smoke.mjs', import.meta.url);

test('Core score evidence uses the isolated receipt channel', async () => {
  const compose = await readFile(composeUrl, 'utf8');

  assert.match(
    compose,
    /- "-external-input-receipt-dir"\r?\n\s+- "\/score"/,
  );
  assert.match(
    compose,
    /HM_EXTERNAL_CORE_SCORE_RECEIPT: "\/score\/core\.score\.json"/,
  );
  assert.doesNotMatch(compose, /core\.jsonl|HM_EXTERNAL_CORE_SCORE_LOG/);
});

test('score directory admits non-root Core and evaluator writers', async () => {
  const smoke = await readFile(smokeUrl, 'utf8');

  assert.match(smoke, /await chmod\(scoreDir, 0o733\);/);
  assert.match(smoke, /mkdir\(scoreDir, \{ recursive: true \}\)/);
});
