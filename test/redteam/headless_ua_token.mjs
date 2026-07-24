// headless_ua_token.mjs — Red profile: a naive headless Chrome run that never
// scrubbed its User-Agent — it still carries the "HeadlessChrome" token. Paired
// with synthetic (untrusted) events and near-zero mouse trajectory as a second
// tell. Blue trips l1.ua.headless_token + a corroborating tell -> HR-7 DENY.
//
// Local target only. Requires no dependencies.

export const label = 'bot:headless-ua-token';
export const needsBrowser = false;

const HEADLESS_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: HEADLESS_UA,
    engineVersion: 'wasm-1.0.0',
    advanced: { probed: true },
    environment: { probed: true },
    behavior: {
      durationS: 5,
      mouse: {
        samples: 2, meanCurvature: 0, velocityStdDev: 0, accelEntropy: 0,
        straightLineFrac: 0, pauseCount: 0, maxTeleportPx: 0, meanJerk: 0,
        coalescedRatio: 0,
      },
      key: {
        keystrokes: 0, meanDwellMs: 0, dwellStdDevMs: 0, meanFlightMs: 0,
        flightStdDevMs: 0, pasteEvents: 0, zeroDwellFrac: 0,
      },
      events: {
        totalEvents: 10, untrustedFrac: 0.5, noMoveClicks: 1, syntheticFlags: 3,
        clickCount: 1, longGapCount: 0, burstVarMs: 40, minReactionMs: 50,
      },
    },
    signals: [],
  };
  const res = await fetch(baseURL + '/api/collect?label=' + encodeURIComponent(label), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'User-Agent': HEADLESS_UA },
    body: JSON.stringify(report),
  });
  return res.json();
}
