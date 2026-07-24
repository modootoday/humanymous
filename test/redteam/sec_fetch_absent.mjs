// sec_fetch_absent.mjs — a Chrome UA (with JS evidence) that omits the Sec-Fetch-* request
// metadata a real Chrome always sends -> l5.header.sec_fetch_missing + x.ua_vs_header.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:sec-fetch-absent';
export const needsBrowser = false;
export function run(baseURL) { return attack('sec-fetch-absent', hostOf(baseURL)); }
