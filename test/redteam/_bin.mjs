// _bin.mjs — spawn the Go redteam attack binary (cmd/redteam) and parse the
// verdict JSON. The binary controls TLS (uTLS) + RIT tokens precisely.
import { execFile } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
// Path is env-overridable for containers (HM_REDTEAM_BIN); locally it defaults to
// the built binary, platform-suffixed (.exe on Windows, bare on Linux/macOS).
const bin = process.env.HM_REDTEAM_BIN
  || join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'bin', 'redteam' + (process.platform === 'win32' ? '.exe' : ''));
export function attack(name, host) {
  return new Promise((resolve, reject) => {
    execFile(bin, ['-attack', name, '-host', host], { timeout: 15000 }, (err, stdout) => {
      if (err && !stdout) return reject(err);
      try { resolve(JSON.parse(stdout.trim().split('\n').pop())); }
      catch (e) { reject(new Error('redteam parse: ' + (stdout || e.message))); }
    });
  });
}
export function hostOf(baseURL) { return baseURL.replace(/^https?:\/\//, '').replace(/\/$/, ''); }
