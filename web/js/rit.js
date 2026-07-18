// rit.js — client side of the Rotating Integrity Token (SoT-07). It fetches the
// current seed from /api/session, then signs each /api/* request body with
// HMAC-SHA256 over the canonical: sid \n n \n tb \n sha256hex(body). The seed
// rotates every response, so a captured token can't be replayed and a tampered
// body fails the HMAC. Uses the pristine fetch captured by injector.js.

import { nativeRefs } from './injector.js';

const state = { sid: null, seed: null, n: 0, w: 10, on: false };

const enc = new TextEncoder();

async function hmacSha256(keyBytes, msg) {
  const key = await crypto.subtle.importKey('raw', keyBytes, { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
  const sig = await crypto.subtle.sign('HMAC', key, enc.encode(msg));
  return new Uint8Array(sig);
}

async function sha256hex(bytes) {
  const d = await crypto.subtle.digest('SHA-256', bytes);
  return [...new Uint8Array(d)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

function b64urlToBytes(s) {
  const pad = s.length % 4 ? '='.repeat(4 - (s.length % 4)) : '';
  const b = atob(s.replace(/-/g, '+').replace(/_/g, '/') + pad);
  return Uint8Array.from(b, (c) => c.charCodeAt(0));
}
function bytesToB64url(bytes) {
  let s = btoa(String.fromCharCode(...bytes));
  return s.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

// init fetches the session + first seed. Returns whether RIT is active.
// Never throws: RIT is an enhancement, not a hard dependency of detection.
export async function initRIT() {
  try {
    const res = await nativeRefs.fetch('/api/session', { credentials: 'same-origin' });
    const j = await res.json();
    state.sid = j.sessionId;
    state.on = !!j.ritOn && !!(globalThis.crypto && crypto.subtle);
    if (state.on) {
      state.seed = b64urlToBytes(j.ritSeed);
      state.n = j.ritN;
      state.w = j.ritW || 10;
    }
  } catch (e) {
    state.on = false; // proceed unsigned; server applies first-request grace
  }
  return state.on;
}

// sign computes the RIT headers for a request body (Uint8Array). Returns {} if
// RIT is inactive.
export async function sign(bodyBytes) {
  if (!state.on || !state.seed) return {};
  const nextN = state.n + 1;
  const tb = Math.floor(Date.now() / 1000 / state.w);
  const bodyHash = await sha256hex(bodyBytes);
  const canonical = `${state.sid}\n${nextN}\n${tb}\n${bodyHash}`;
  const mac = await hmacSha256(state.seed, canonical);
  return {
    'X-HM-Token': bytesToB64url(mac),
    'X-HM-N': String(nextN),
    'X-HM-TB': String(tb),
  };
}

// rotate updates the seed from a response's headers (server issues seed_{n+1}).
export function rotate(headers) {
  const seed = headers.get('X-HM-Seed');
  if (seed) {
    state.seed = b64urlToBytes(seed);
    state.n = Number(headers.get('X-HM-N') || state.n + 1);
  }
}
