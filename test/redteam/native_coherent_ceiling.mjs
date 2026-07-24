// native_coherent_ceiling.mjs — the T4 DETECTION CEILING, exercised over REAL Chrome uTLS (a
// pure-Node fetch cannot be network-coherent: its TLS is not Chrome and it sends no client
// hints, so the network plane alone flags it). A fully coherent session — Chrome JA4 +
// Sec-CH-UA/Sec-Fetch + every advanced capability self-consistent (WebGL/WebGPU agree) + rich
// human-shaped behavior — reconciles on every plane and SCORES ALLOW. This is the honest limit:
// an engine-level spoof (BotBrowser-class) is not separable from a real human by detection
// alone; only rate/reputation raise its cost. Implemented in cmd/redteam (coherent-ceiling).
import { attack, hostOf } from './_bin.mjs';
export const label = 'ceiling:native-coherent';
export const needsBrowser = false;
export function run(baseURL) { return attack('coherent-ceiling', hostOf(baseURL)); }
