// tls_static.mjs — static uTLS Chrome parrot: sends the SAME TLS extension order
// on every connection (real Chrome 110+ permutes), across several requests in
// one session -> l5.traffic.tls_no_permutation -> HR-14 DENY (SoT-14 §B1).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:tls-static';
export const needsBrowser = false;
export function run(baseURL) { return attack('tls-static', hostOf(baseURL)); }
