// socks_exit_hop.mjs — SOCKS5 / commercial-VPN L4 tunnel residual: no HTTP hop
// headers, mid-session exit hop + multi-hop XFF. Blue: ip_hop + xff_multi_hop → HR-24.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:socks-exit-hop';
export const needsBrowser = false;
export function run(baseURL) { return attack('socks-exit-hop', hostOf(baseURL)); }
