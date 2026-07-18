// distributed.mjs — scraper behind a rotating residential-proxy pool: same device
// fingerprint, rotating exit IPs (X-Forwarded-For). Each session looks legit, but
// the shared fingerprint across many subnets is unmasked by cross-session
// correlation -> l5.correlation.proxy_rotation -> HR-19 DENY (SoT-15).
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:distributed-proxy';
export const needsBrowser = false;
export function run(baseURL) { return attack('distributed', hostOf(baseURL)); }
