// pq_absent.mjs — post-quantum TLS mismatch: a scraper that pins a PRE-post-quantum uTLS
// fingerprint (HelloChrome_100, whose ClientHello supported_groups lack X25519MLKEM768 /
// 0x11EC) while its UA + sec-ch-ua claim Chrome 131 — the first stable Chrome that ships the
// hybrid PQ key share by DEFAULT (M131, Nov 2024; Firefox 132 similarly). Real PQ-era
// browsers always send it (measured vs headless Chromium 149), so a coherent, RIT-signed
// report whose ONLY residual is the missing PQ group -> l5.tls.pq_keyshare -> HR-24 CHALLENGE
// (net.tls.pq enforce). Wargame R9 (2026-07-28).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:pq-absent';
export const needsBrowser = false;
export function run(baseURL) { return attack('pq-absent', hostOf(baseURL)); }
