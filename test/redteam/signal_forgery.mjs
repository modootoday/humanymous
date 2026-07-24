// signal_forgery.mjs — a borderline-suspicious browser-ish client that FORGES the
// server-only trust-upgrade signals l7.pass.solved / l7.pow.solved in its own /api/collect
// report, trying to launder a score-based CHALLENGE into an ALLOW without ever solving Pass
// or PoW. This pins the round-3 provenance blocker: trust-upgrades must be honored ONLY from
// server-minted signals, and any client-supplied signal in the reserved L5/L6/L7 namespace is
// stripped at ingest — so the forged upgrades are inert and the verdict stays CHALLENGE/DENY,
// never ALLOW. A permanent regression wargame case.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:signal-forgery';
export const needsBrowser = false;
export function run(baseURL) { return attack('signal-forgery', hostOf(baseURL)); }
