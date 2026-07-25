// squid_forward.mjs — browser/scraper behind an open Squid forward proxy.
// Squid injects Via + X-Cache on the origin request (direct browsers never do).
// Blue: l5.header.proxy_hop → HR-24 CHALLENGE.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:squid-forward';
export const needsBrowser = false;
export function run(baseURL) { return attack('squid-forward', hostOf(baseURL)); }
