// behavior_machine_keystroke.mjs — Red profile: an automation that types via
// keyboard.type() with a fixed sub-25ms delay. The flight times are impossibly
// fast and uniform, dwell is ~0, and zeroDwellFrac is high. Blue trips
// l4.key.machine_speed -> CHALLENGE.
//
// Local target only. Requires no dependencies.

export const label = 'bot:machine-keystroke';
export const needsBrowser = false;

const CHROME_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: CHROME_UA,
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
        keystrokes: 12, meanDwellMs: 1, dwellStdDevMs: 0.3, meanFlightMs: 8,
        flightStdDevMs: 1, pasteEvents: 0, zeroDwellFrac: 0.9,
      },
      events: {
        totalEvents: 14, untrustedFrac: 0, noMoveClicks: 0, syntheticFlags: 0,
        clickCount: 0, longGapCount: 0, burstVarMs: 5, minReactionMs: 8,
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
