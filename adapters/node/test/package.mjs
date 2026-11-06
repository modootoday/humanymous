import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import * as esm from '@humanymous/node';
const require=createRequire(import.meta.url);
assert.deepEqual(Object.keys(esm).sort(),Object.keys(require('@humanymous/node')).sort());
assert.equal(esm.VERSION,'humanymous.adapter/0.1');
console.log('ESM/CJS public surface passed');
