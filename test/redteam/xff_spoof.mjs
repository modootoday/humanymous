// xff_spoof.mjs — forges a PRIVATE X-Forwarded-For to impersonate a trusted-LAN
// client and shed IP-intel signals (datacenter/correlation). Otherwise-clean
// Chrome uTLS + fabricated report, so the forged IP is the only variable. Blue
// must reject a forwarded client IP that is private/reserved (a real proxy
// forwards the client's PUBLIC ip) -> l5.header.forwarded_private.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:xff-spoof';
export const needsBrowser = false;
export function run(baseURL) { return attack('xff-spoof', hostOf(baseURL)); }
