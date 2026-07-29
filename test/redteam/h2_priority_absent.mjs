// h2_priority_absent.mjs — HTTP/2 priority fingerprint absence (4th Akamai component): a raw h2
// client that ships Chrome's m,a,s,p pseudo-order, a coherent Chrome SETTINGS profile (passes
// R6), AND a real Chrome connection window 15663105 (passes R11) — but whose HEADERS frame
// carries NO priority field and sends no separate PRIORITY frame. Every real browser signals
// priority (measured: Chrome HEADERS excl=1/weight=255, Firefox excl=0/weight=41); a raw framer
// omits it. Coherent Chrome UA + report + RIT, so the sole residual is the missing priority
// -> l5.http2.priority_atypical -> HR-24 net.h2.spoof CHALLENGE. Wargame R14 (2026-07-29).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:h2-priority-absent';
export const needsBrowser = false;
export function run(baseURL) { return attack('h2-priority-absent', hostOf(baseURL)); }
