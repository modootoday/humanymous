// multi_axis_rotate.mjs — rotate the User-Agent AND the TLS engine together in one session
// -> HR-15 (ua_rotation + ja4/engine axis), a coordinated multi-axis rotation a single
// browser never performs.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:multi-axis-rotate';
export const needsBrowser = false;
export function run(baseURL) { return attack('multi-axis-rotate', hostOf(baseURL)); }
