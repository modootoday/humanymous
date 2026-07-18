// http_client.mjs — Red profile: a pure HTTP client (no browser, no WASM),
// simulating a scraper that fakes a Chrome UA. It cannot run the WASM detector,
// so the Blue engine must catch it via the TLS/HTTP2/header layers (L5/L6).
//
// Local target only. Requires no dependencies.

export const label = 'bot:http-client';
export const needsBrowser = false;

// run posts a spoofed report to /api/collect and returns the verdict object.
export async function run(baseURL) {
  // Node's fetch does not verify the self-signed cert when this env is set by
  // the runner. The TLS ClientHello is OpenSSL, not Chrome -> engine mismatch.
  const res = await fetch(baseURL + '/api/collect?label=' + encodeURIComponent(label), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      // Spoof a Chrome UA — but the TLS stack and missing sec-* headers betray it.
      'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36',
    },
    body: JSON.stringify({
      userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36',
      signals: [],
    }),
  });
  return res.json();
}
