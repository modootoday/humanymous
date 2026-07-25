// wireguard_hop.mjs — WireGuard mid-session exit rotation + multi-hop XFF chain.
// Same cookie/fingerprint, two public exits; second request carries multi-hop XFF.
// Blue: l5.traffic.ip_hop + l5.header.xff_multi_hop → HR-24 CHALLENGE.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:wireguard-hop';
export const needsBrowser = false;
export function run(baseURL) { return attack('wireguard-hop', hostOf(baseURL)); }
