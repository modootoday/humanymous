// ja4_churn.mjs — three+ distinct TLS fingerprints (spanning engine families) inside one
// cookied session -> l5.traffic.engine_rotation / ja4_rotation -> HR-14. A real browser keeps
// one TLS stack per session; churning presets is a parrot rotating fingerprints.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:ja4-churn';
export const needsBrowser = false;
export function run(baseURL) { return attack('ja4-churn', hostOf(baseURL)); }
