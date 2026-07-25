// anon_proxy_chain.mjs — free/anonymous open-proxy chain (4+ XFF hops) that still
// leaks Proxy-Connection. Blue: l5.proxy.anon_chain / proxy_hop → HR-24.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:anon-proxy-chain';
export const needsBrowser = false;
export function run(baseURL) { return attack('anon-proxy-chain', hostOf(baseURL)); }
