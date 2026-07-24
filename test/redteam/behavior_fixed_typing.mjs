// behavior_fixed_typing.mjs — Red profile: an automation that types with a
// fixed inter-key delay meant to look "human-slow", but the dwell and flight
// variances are near zero — no real typist is that metronomic. Blue trips the
// l4.key dwell_std/flight_std cluster -> CHALLENGE.
//
// Local target only. Requires no dependencies.

export const label = 'bot:fixed-typing';
export const needsBrowser = false;

const CHROME_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: CHROME_UA,
    engineVersion: 'wasm-1.0.0',
    advanced: { probed: true },
    environment: { probed: true },
    behavior: {
      durationS: 9,
      mouse: {
        samples: 3, meanCurvature: 0, velocityStdDev: 0, accelEntropy: 0,
        straightLineFrac: 0, pauseCount: 0, maxTeleportPx: 0, meanJerk: 0,
        coalescedRatio: 0,
      },
      key: {
        keystrokes: 20, meanDwellMs: 40, dwellStdDevMs: 0.5, meanFlightMs: 120,
        flightStdDevMs: 0.7, pasteEvents: 0, zeroDwellFrac: 0,
      },
      events: {
        totalEvents: 22, untrustedFrac: 0, noMoveClicks: 0, syntheticFlags: 0,
        clickCount: 0, longGapCount: 0, burstVarMs: 10, minReactionMs: 120,
      },
    },
    signals: [],
  };
  const res = await fetch(baseURL + '/api/collect?label=' + encodeURIComponent(label), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'User-Agent': CHROME_UA },
    body: JSON.stringify(report),
  });
  return res.json();
}
