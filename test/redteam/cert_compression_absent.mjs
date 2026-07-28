// cert_compression_absent.mjs — certificate-compression absence: a non-browser TLS stack
// wearing a Chrome UA. Sends a real Chrome_Auto ClientHello over h2 (ALPN offers h2) with the
// compress_certificate extension (RFC 8879, codepoint 27) STRIPPED, plus a coherent RIT-signed
// Chrome report. Every modern browser (Chrome/Firefox/Safari) advertises cert compression;
// Go/curl-plain stacks do not — so the missing extension is the sole residual tell
// -> l5.tls.cert_compression_absent -> HR-24 net.tls.certcomp CHALLENGE. Wargame R12 (2026-07-29).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:cert-compression-absent';
export const needsBrowser = false;
export function run(baseURL) { return attack('cert-compression-absent', hostOf(baseURL)); }
