// behavior_bezier_mouse.mjs — Red profile: a "humanizer" that synthesizes mouse
// paths with smooth Bezier / ghost-cursor curves. The trajectory is rich in
// samples but pathologically smooth: near-straight, near-zero velocity variance
// and jerk. Blue trips the l4.mouse straightness/velocity_std cluster ->
// CHALLENGE.
//
// Local target only. Requires no dependencies.

export const label = 'bot:bezier-mouse';
export const needsBrowser = false;

const CHROME_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: CHROME_UA,
    engineVersion: 'wasm-1.0.0',
    advanced: { probed: true },
    environment: { probed: true },
    behavior: {
      durationS: 7,
      mouse: {
        samples: 40, meanCurvature: 0.01, velocityStdDev: 0.02, accelEntropy: 0.2,
        straightLineFrac: 0.92, pauseCount: 0, maxTeleportPx: 30, meanJerk: 0.01,
        coalescedRatio: 0.1,
      },
      key: {
        keystrokes: 0, meanDwellMs: 0, dwellStdDevMs: 0, meanFlightMs: 0,
        flightStdDevMs: 0, pasteEvents: 0, zeroDwellFrac: 0,
      },
      events: {
        totalEvents: 42, untrustedFrac: 0, noMoveClicks: 0, syntheticFlags: 0,
        clickCount: 1, longGapCount: 0, burstVarMs: 200, minReactionMs: 180,
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
