// curl_impersonate_chrome99_android.mjs — Android Chrome TLS/UA-CH parrot via
// curl_chrome99_android (curl-impersonate). Mobile ClientHello + mobile headers
// without JS — residual for mobile TLS impersonation (not a real device).
//
// Docker bots image only. Local skip if binary missing.
import { curlImpersonateCollect } from './_curl_impersonate.mjs';

export const label = 'bot:curl-impersonate-chrome99-android';
export const needsBrowser = false;

const UA =
  'Mozilla/5.0 (Linux; Android 12; Pixel 6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.4844.58 Mobile Safari/537.36';

export function run(baseURL) {
  return curlImpersonateCollect('curl_chrome99_android', baseURL, {
    body: {
      label,
      report: {
        userAgent: UA,
        signals: [],
      },
    },
  });
}
