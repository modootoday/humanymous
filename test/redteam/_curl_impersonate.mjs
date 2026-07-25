// _curl_impersonate.mjs — shared launcher for curl-impersonate (Chrome TLS/JA4
// parrot from lwthiker/curl-impersonate). Docker bots image ships the binaries
// under HM_CURL_IMPERSONATE_DIR; libcurl-impersonate + musl loader under
// HM_CURL_IMPERSONATE_LIB / /lib/ld-musl-x86_64.so.1.
//
// Local target only (lab network / loopback).

import { execFile } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.env.HM_CURL_IMPERSONATE_DIR || '/app/curl-impersonate/bin';
const lib = process.env.HM_CURL_IMPERSONATE_LIB || '/app/curl-impersonate/lib';

/**
 * POST /api/collect via a curl-impersonate wrapper (e.g. curl_chrome116).
 * Wrappers are shell scripts that exec curl-impersonate-chrome (musl ELF with
 * INTERP /lib/ld-musl-x86_64.so.1). LD_LIBRARY_PATH must include libcurl-impersonate.
 */
export function curlImpersonateCollect(wrapperName, baseURL, { body, extraHeaders = [] } = {}) {
  const wrapper = join(dir, wrapperName);
  if (!existsSync(wrapper)) {
    return Promise.reject(new Error(`curl-impersonate wrapper missing: ${wrapper} (Docker bots image only)`));
  }
  const label = body?.label || 'bot:curl-impersonate';
  const url = baseURL.replace(/\/$/, '') + '/api/collect?label=' + encodeURIComponent(label);
  const payload = JSON.stringify(body?.report || {
    userAgent: 'curl-impersonate',
    signals: [],
  });

  const args = [
    '-k', '-sS', '-X', 'POST', url,
    '-H', 'Content-Type: application/json',
    ...extraHeaders.flatMap((h) => ['-H', h]),
    '--data-binary', payload,
  ];

  const env = {
    ...process.env,
    LD_LIBRARY_PATH: lib + (process.env.LD_LIBRARY_PATH ? `:${process.env.LD_LIBRARY_PATH}` : ''),
    PATH: dir + ':' + (process.env.PATH || ''),
  };

  return new Promise((resolve, reject) => {
    // bash runs the wrapper; wrapper execs musl-linked curl-impersonate-chrome.
    execFile('/bin/bash', [wrapper, ...args], { timeout: 20000, env, maxBuffer: 2 << 20 }, (err, stdout, stderr) => {
      if (err && !stdout) {
        return reject(new Error(`curl-impersonate failed: ${err.message}; stderr=${stderr || ''}`));
      }
      const line = (stdout || '').trim().split('\n').filter(Boolean).pop() || '';
      try {
        resolve(JSON.parse(line));
      } catch (e) {
        reject(new Error(`curl-impersonate parse: ${line.slice(0, 400) || e.message}; stderr=${(stderr || '').slice(0, 300)}`));
      }
    });
  });
}
