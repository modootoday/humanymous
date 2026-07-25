// proxy_ua_rotate.mjs — residential exit hop + mid-session User-Agent rotate
// (multi-axis evasion). Blue: ua_rotation + ip_hop → HR-15 (and/or HR-19).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:proxy-ua-rotate';
export const needsBrowser = false;
export function run(baseURL) { return attack('proxy-ua-rotate', hostOf(baseURL)); }
