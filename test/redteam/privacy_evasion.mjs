// privacy_evasion.mjs — the distributed residential-proxy-rotation scraper that ALSO claims
// genuine privacy tooling (environment.adBlock/gpc/doNotTrack) to try to disarm the
// cross-session correlation hard rule. This pins the round-5 regression: gating the
// server-authoritative proxy_rotation hard-DENY on a CLIENT-forgeable privacy flag let a
// scraper post adBlock:true and lie its way to ALLOW. The Blue engine must NOT honor a
// self-asserted privacy flag to exempt a server-authoritative rule -> l5.correlation.
// proxy_rotation -> HR-19 DENY. A permanent regression wargame case.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:privacy-evasion';
export const needsBrowser = false;
export function run(baseURL) { return attack('privacy-evasion', hostOf(baseURL)); }
