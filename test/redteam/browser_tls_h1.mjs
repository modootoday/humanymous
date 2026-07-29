// browser_tls_h1.mjs — JA4↔JA4H cross-layer inconsistency: a fully coherent Chrome client (real
// Chrome uTLS ClientHello = Chrome JA4, coherent report, browser headers) delivered over
// HTTP/1.1. Real modern browsers negotiate and speak h2 to an h2-capable server; a browser-TLS
// parrot driven by an HTTP/1.1 client library speaks h1. Everything else is coherent, so the sole
// residual is the version mismatch ("JA4 says Chrome, JA4H says HTTP/1.1")
// -> l5.http.browser_tls_over_h1 -> HR-24 net.http.h1 CHALLENGE. Wargame R15 (2026-07-29).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:browser-tls-h1';
export const needsBrowser = false;
export function run(baseURL) { return attack('browser-tls-h1', hostOf(baseURL)); }
