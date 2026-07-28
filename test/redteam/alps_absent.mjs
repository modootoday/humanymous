// alps_absent.mjs — ALPS extension absence: a non-Chromium TLS stack wearing a Chrome UA.
// Sends a real Chrome_Auto ClientHello over h2 (ALPN offers h2) with the ALPS
// (application_settings, codepoint 17513/17613) extension STRIPPED, plus a coherent RIT-signed
// Chrome report. Every genuine Chromium build sends ALPS on h2; Firefox/Safari/Go/curl send
// none — so the missing ALPS is the sole residual tell -> l5.tls.alps_absent -> HR-24 CHALLENGE
// (net.tls.alps enforce). Wargame R10 (2026-07-28).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:alps-absent';
export const needsBrowser = false;
export function run(baseURL) { return attack('alps-absent', hostOf(baseURL)); }
