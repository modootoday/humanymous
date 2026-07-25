// curl_impersonate_chrome.mjs — Chrome TLS/JA4 parrot via curl-impersonate
// (lwthiker/curl-impersonate:0.6-chrome → curl_chrome116). Real Chrome ClientHello
// + browser-like headers, but NO JS/WASM — Blue should block on missing-client /
// HTTP-parrot residuals (HR-18 / HR-10 family), not ALLOW as T4 coherent.
//
// Docker bots image only (musl binaries + libcurl-impersonate). Local skip if absent.
import { curlImpersonateCollect } from './_curl_impersonate.mjs';

export const label = 'bot:curl-impersonate-chrome';
export const needsBrowser = false;

const UA =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36';

export function run(baseURL) {
  return curlImpersonateCollect('curl_chrome116', baseURL, {
    body: {
      label,
      report: {
        // No advanced probe / no behavior — pure network parrot residual
        userAgent: UA,
        signals: [],
      },
    },
  });
}
