// elite_anon_proxy.mjs — "elite" anonymous proxy: Via stripped, RFC 7239 Forwarded
// multi for=/by= residual remains. Blue: l5.header.proxy_hop → HR-24.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:elite-anon-proxy';
export const needsBrowser = false;
export function run(baseURL) { return attack('elite-anon-proxy', hostOf(baseURL)); }
