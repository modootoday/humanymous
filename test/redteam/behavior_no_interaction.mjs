// behavior_no_interaction.mjs — Red profile: a headless-driven page that loads,
// waits, and scrapes without ever moving the mouse or typing. The long dwell
// with near-zero input is the HR-12 tell. Blue trips l4.event.no_interaction
// -> CHALLENGE.
//
// Local target only. Requires no dependencies.

export const label = 'bot:no-interaction';
export const needsBrowser = false;

const CHROME_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: CHROME_UA,
    engineVersion: 'wasm-1.0.0',
    advanced: { probed: true },
    environment: { probed: true },
    behavior: {
      durationS: 6,
      mouse: {
        samples: 1, meanCurvature: 0, velocityStdDev: 0, accelEntropy: 0,
        straightLineFrac: 0, pauseCount: 0, maxTeleportPx: 0, meanJerk: 0,
        coalescedRatio: 0,
      },
      key: {
        keystrokes: 0, meanDwellMs: 0, dwellStdDevMs: 0, meanFlightMs: 0,
        flightStdDevMs: 0, pasteEvents: 0, zeroDwellFrac: 0,
      },
      events: {
        totalEvents: 2, untrustedFrac: 0, noMoveClicks: 0, syntheticFlags: 0,
        clickCount: 0, longGapCount: 0, burstVarMs: 0, minReactionMs: 0,
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
