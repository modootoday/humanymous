// pow.js — client-side proof-of-work solver (SoT-13). On a CHALLENGE verdict the
// server sends X-HM-PoW: "seedHex:difficulty:bucket". The client finds a nonce
// so SHA-256(seed || nonce) has `difficulty` leading zero bits, then POSTs the
// solution to /api/pow to upgrade the session's trust. Uses Web Crypto SHA-256.

import { nativeRefs } from './injector.js';

const enc = new TextEncoder();

function hexToBytes(hex) {
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.substr(i * 2, 2), 16);
  return out;
}

function leadingZeroBits(bytes) {
  let n = 0;
  for (const b of bytes) {
    if (b === 0) { n += 8; continue; }
    for (let bit = 7; bit >= 0; bit--) {
      if ((b & (1 << bit)) === 0) n++; else return n;
    }
    return n;
  }
  return n;
}

// solveAndSubmit parses the challenge header, solves it, and submits. Returns
// the upgraded verdict, or null if unsolved / no challenge.
export async function solveAndSubmit(headerValue) {
  if (!headerValue) return null;
  const [seedHex, diffStr, bucketStr] = headerValue.split(':');
  const difficulty = Number(diffStr);
  const seed = hexToBytes(seedHex);

  const maxIters = 1 << 24;
  let nonce = -1;
  for (let i = 0; i < maxIters; i++) {
    const msg = new Uint8Array(seed.length + 0);
    // build seed||nonceStr
    const nb = enc.encode(String(i));
    const buf = new Uint8Array(seed.length + nb.length);
    buf.set(seed, 0); buf.set(nb, seed.length);
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', buf));
    if (leadingZeroBits(digest) >= difficulty) { nonce = i; break; }
  }
  if (nonce < 0) return null;

  const res = await nativeRefs.fetch('/api/pow', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'same-origin',
    body: JSON.stringify({ bucket: Number(bucketStr), nonce: String(nonce) }),
  });
  return res.json();
}
