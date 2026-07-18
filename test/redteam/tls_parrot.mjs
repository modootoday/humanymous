// tls_parrot.mjs — runner adapter for the uTLS Chrome-ClientHello parrot
// (cmd/tlsparrot). It faithfully spoofs Chrome's TLS fingerprint (JA4 -> chrome)
// but has no JS/DOM layer and omits Chrome's client-hint / sec-fetch headers, so
// the Blue engine blocks it on the header + missing-client residuals (SoT-02 §8).

import { execFile } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

export const label = 'bot:tls-parrot';
export const needsBrowser = false;

// Env-overridable for containers (HM_TLSPARROT_BIN); platform-suffixed default.
const bin = process.env.HM_TLSPARROT_BIN
  || join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'bin', 'tlsparrot' + (process.platform === 'win32' ? '.exe' : ''));

export function run(baseURL) {
  const url = baseURL.replace(/\/$/, '') + '/api/collect';
  return new Promise((resolve, reject) => {
    execFile(bin, ['-url', url], { timeout: 15000 }, (err, stdout) => {
      if (err && !stdout) return reject(err);
      try {
        resolve(JSON.parse(stdout.trim().split('\n').pop()));
      } catch (e) {
        reject(new Error('tlsparrot output parse: ' + (stdout || e.message)));
      }
    });
  });
}
