// rit_tamper.mjs — aggressive network/anti-tamper evasion via the Go redteam client.
// Spoofs browser headers (curl_cffi-class) yet is caught by the specific
// anti-bypass signal (SoT-07/12).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:rit-tamper';
export const needsBrowser = false;
export function run(baseURL) { return attack('rit-tamper', hostOf(baseURL)); }
