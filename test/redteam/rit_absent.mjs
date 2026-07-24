// rit_absent.mjs — an API client that never presents a RIT anti-tamper token. The first
// tokenless call gets the one-shot bootstrap grace; the SECOND emits l5.rit.absent -> HR-17.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:rit-absent';
export const needsBrowser = false;
export function run(baseURL) { return attack('rit-absent', hostOf(baseURL)); }
