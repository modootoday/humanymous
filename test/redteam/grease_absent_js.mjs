// grease_absent_js.mjs — a no-GREASE (Go-default) TLS stack under a Chrome UA that DOES run
// JS (so HR-18 stays quiet), isolating the network tells: l5.tls.grease_absent (a real
// browser always sends a GREASE value) + x.ua_vs_ja4 (UA says Chrome, JA4 says Go).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:grease-absent-js';
export const needsBrowser = false;
export function run(baseURL) { return attack('grease-absent-js', hostOf(baseURL)); }
