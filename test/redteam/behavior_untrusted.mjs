// behavior_untrusted.mjs — Red profile: a bot that dispatches synthetic
// (isTrusted=false) DOM events. The report carries a plausible event volume,
// but a high untrustedFrac betrays that the events were injected, not produced
// by a real input device. Blue trips l4.event.untrusted -> CHALLENGE.
//
// Local target only. Requires no dependencies.

export const label = 'bot:untrusted-events';
export const needsBrowser = false;

const CHROME_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: CHROME_UA,
    engineVersion: 'wasm-1.0.0',
    advanced: { probed: true },
    environment: { probed: true },
    behavior: {
      durationS: 8,
      mouse: {
        samples: 20, meanCurvature: 0.4, velocityStdDev: 0.5, accelEntropy: 1.8,
        straightLineFrac: 0.3, pauseCount: 3, maxTeleportPx: 40, meanJerk: 0.3,
        coalescedRatio: 0.5,
      },
      key: {
        keystrokes: 0, meanDwellMs: 0, dwellStdDevMs: 0, meanFlightMs: 0,
        flightStdDevMs: 0, pasteEvents: 0, zeroDwellFrac: 0,
      },
      events: {
        totalEvents: 30, untrustedFrac: 0.8, noMoveClicks: 2, syntheticFlags: 5,
        clickCount: 3, longGapCount: 0, burstVarMs: 300, minReactionMs: 220,
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
