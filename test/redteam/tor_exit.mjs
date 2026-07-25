// tor_exit.mjs — Tor Browser-shaped client behind a Tor circuit residual:
// ≥3-hop XFF (entry/middle/exit) → l5.proxy.tor_circuit → HR-24, then the same
// fingerprint across multiple exit subnets → l5.correlation.proxy_rotation → HR-19.
// Production Tor exit lists wire via SetTorExitCIDRs → l5.ip.tor_exit (also HR-24).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:tor-exit';
export const needsBrowser = false;
export function run(baseURL) { return attack('tor-exit', hostOf(baseURL)); }
