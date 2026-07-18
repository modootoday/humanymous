// transport.js — posts the ClientReport to /api/collect using the pristine
// fetch captured by injector.js (so a hooked window.fetch can't tamper the
// report), signed with the RIT token (SoT-07) so body/replay tampering is
// detected server-side. Returns the server verdict.

import { nativeRefs } from './injector.js';
import { sign, rotate } from './rit.js';

const enc = new TextEncoder();

export async function postReport(report) {
  const url = '/api/collect' + (location.search.includes('label=')
    ? '?' + location.search.slice(1)
    : '');
  const bodyStr = JSON.stringify(report);
  const bodyBytes = enc.encode(bodyStr);

  const headers = { 'Content-Type': 'application/json' };
  try {
    Object.assign(headers, await sign(bodyBytes));
  } catch (_) { /* RIT optional; proceed unsigned */ }

  const res = await nativeRefs.fetch(url, {
    method: 'POST',
    headers,
    credentials: 'same-origin',
    body: bodyStr,
  });
  rotate(res.headers);
  const verdict = await res.json();
  // Surface any proof-of-work challenge the server issued (SoT-13).
  verdict._powChallenge = res.headers.get('X-HM-PoW') || null;
  return verdict;
}
