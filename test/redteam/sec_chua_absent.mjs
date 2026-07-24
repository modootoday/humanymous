// sec_chua_absent.mjs — a Chromium UA (with JS evidence) that omits the Sec-CH-UA client
// hint a real Chromium always sends -> x.uach_present cross-check fails (w40) -> CHALLENGE.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:sec-chua-absent';
export const needsBrowser = false;
export function run(baseURL) { return attack('sec-chua-absent', hostOf(baseURL)); }
