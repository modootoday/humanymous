// h2_flow_control.mjs — HTTP/2 flow-control fingerprint split: a raw h2 client that ships
// Chrome's m,a,s,p pseudo-order AND a coherent Chrome SETTINGS profile (incl HEADER_TABLE_SIZE,
// so the R6 residual stays quiet) but opens a 1 GiB connection flow-control window via a
// connection-level WINDOW_UPDATE — no browser does this (Chrome ~15 MB / 15663105, Firefox
// ~12 MB; Go's http2 default is 1 GiB). Coherent Chrome UA + report + RIT, so the sole residual
// tell is the gigabyte window -> l5.http2.flow_control_atypical -> HR-24 net.h2.spoof CHALLENGE.
// The flow-control (W) dimension of the Akamai h2 fingerprint. Wargame R11 (2026-07-28).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:h2-flow-control';
export const needsBrowser = false;
export function run(baseURL) { return attack('h2-flow-control', hostOf(baseURL)); }
