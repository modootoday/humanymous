import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { TOKENS } from './dom-observer/extension/protocol.mjs';

const htmlUrl = new URL('./fixture.html', import.meta.url);
const scriptUrl = new URL('./fixture.mjs', import.meta.url);
const imeScriptUrl = new URL('./ime-fixture.mjs', import.meta.url);

test('fixture exposes each canonical task token exactly once and no observer token for verdict data', async () => {
  const html = await readFile(htmlUrl, 'utf8');
  const found = [...html.matchAll(/data-hmn-token="([^"]+)"/g)].map((match) => match[1]);
  assert.deepEqual(found.sort(), [...TOKENS].sort());
  assert.equal(new Set(found).size, TOKENS.length);
  assert.doesNotMatch(html, /data-hmn-token="(?:verdict|risk|score|signal)/);
  assert.match(html, /id="verdict-icon"/);
  assert.match(html, /id="verdict-text"/);
  assert.match(html, /id="verdict-region"/);
});

test('IME fixture accepts only trusted native composition and never mutates the input value', async () => {
  const script = await readFile(imeScriptUrl, 'utf8');
  for (const event of [
    'compositionstart', 'compositionupdate', 'compositionend', 'beforeinput', 'input',
  ]) assert.match(script, new RegExp(`addEventListener\\('${event}'`));
  assert.match(script, /event\.isTrusted === true/);
  assert.match(script, /\.normalize\('NFC'\)/);
  assert.doesNotMatch(script, /dispatchEvent|execCommand|clipboard|\.value\s*=|preventDefault/);
});

test('fixture is CSP-clean and loads the real detector, collectors, and transport', async () => {
  const html = await readFile(htmlUrl, 'utf8');
  const script = await readFile(scriptUrl, 'utf8');
  assert.doesNotMatch(html, /<style\b/i);
  assert.doesNotMatch(html, /<script(?![^>]*\bsrc=)[^>]*>/i);
  for (const module of [
    '/static/js/injector.js',
    '/static/js/collector.js',
    '/static/js/env.js',
    '/static/js/advanced.js',
    '/static/js/transport.js',
    '/static/js/rit.js',
  ]) {
    assert.match(script, new RegExp(module.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }
  assert.match(script, /fetch\('\/static\/detector\.wasm'/);
  assert.match(script, /postReport\(report\)/);
  assert.doesNotMatch(script, /riskScore|hardRule|topContributors|signalIds/);
});

test('fixture renders only the coarse verdict enum with text and shape cues', async () => {
  const script = await readFile(scriptUrl, 'utf8');
  assert.match(script, /new Set\(\['ALLOW', 'CHALLENGE', 'DENY'\]\)/);
  assert.match(script, /byId\('verdict-icon'\)\.textContent = icon/);
  assert.match(script, /byId\('verdict-text'\)\.textContent = normalized/);
  assert.match(script, /verdict-\$\{normalized\.toLowerCase\(\)/);
});
