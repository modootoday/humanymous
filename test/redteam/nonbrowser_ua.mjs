// nonbrowser_ua.mjs — the cheapest bot: a bare HTTP library (library User-Agent, no browser
// headers, no JS) -> x.non_browser_ua (w45) + HR-10 (no client evidence).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:nonbrowser-ua';
export const needsBrowser = false;
export function run(baseURL) { return attack('nonbrowser-ua', hostOf(baseURL)); }
