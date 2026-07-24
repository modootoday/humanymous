// behavior_teleport_click.mjs — Red profile: a bot that clicks elements directly
// (element.click() / page.click()) with no cursor path in between. Several
// clicks land with almost no mouse trajectory samples. Blue trips
// l4.mouse.click_no_trajectory -> CHALLENGE.
//
// Local target only. Requires no dependencies.

export const label = 'bot:teleport-click';
export const needsBrowser = false;

const CHROME_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: CHROME_UA,
    engineVersion: 'wasm-1.0.0',
    advanced: { probed: true },
    environment: { probed: true },
    behavior: {
      durationS: 4,
      mouse: {
        samples: 3, meanCurvature: 0, velocityStdDev: 0, accelEntropy: 0,
        straightLineFrac: 0, pauseCount: 0, maxTeleportPx: 1200, meanJerk: 0,
        coalescedRatio: 0,
      },
      key: {
        keystrokes: 0, meanDwellMs: 0, dwellStdDevMs: 0, meanFlightMs: 0,
        flightStdDevMs: 0, pasteEvents: 0, zeroDwellFrac: 0,
      },
      events: {
        totalEvents: 6, untrustedFrac: 0, noMoveClicks: 4, syntheticFlags: 0,
        clickCount: 4, longGapCount: 0, burstVarMs: 50, minReactionMs: 60,
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
