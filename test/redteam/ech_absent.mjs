// ech_absent.mjs — Encrypted Client Hello absence: a scraper pinning a PRE-ECH uTLS fingerprint.
// Sends a real Chrome_Auto ClientHello over h2 with the ECH (Encrypted Client Hello, ext 0xfe0d /
// 65037) extension STRIPPED, plus a coherent RIT-signed Chrome report. Chrome 117+ / Firefox 118+
// send ECH (real or GREASE) by default (measured: Chrome/149, HeadlessChrome/149, Firefox/151 all
// carry 65037); a pre-ECH parrot omits it. The sole residual is the missing ECH
// -> l5.tls.ech_absent -> HR-24 net.tls.ech CHALLENGE. Wargame R16 (2026-07-29).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:ech-absent';
export const needsBrowser = false;
export function run(baseURL) { return attack('ech-absent', hostOf(baseURL)); }
