// ai_full_cadence.mjs — Red profile: a fully autonomous AI agent that both
// clicks by teleport (no cursor path), types at machine speed, AND paces its
// actions on inference bursts. Three coherent AI tells stack into the HR-20
// AI-agent rule -> DENY.
//
// Local target only. Requires no dependencies.

export const label = 'bot:ai-full-cadence';
export const needsBrowser = false;

const CHROME_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: CHROME_UA,
    engineVersion: 'wasm-1.0.0',
    advanced: { probed: true },
    environment: { probed: true },
    behavior: {
      durationS: 18,
      mouse: {
        // teleport clicks: many clicks, near-zero trajectory
        samples: 3, meanCurvature: 0, velocityStdDev: 0, accelEntropy: 0,
        straightLineFrac: 0, pauseCount: 0, maxTeleportPx: 1400, meanJerk: 0,
        coalescedRatio: 0,
      },
      key: {
        // machine-speed keystrokes
        keystrokes: 8, meanDwellMs: 1, dwellStdDevMs: 0.3, meanFlightMs: 10,
        flightStdDevMs: 1, pasteEvents: 0, zeroDwellFrac: 0.9,
      },
      events: {
        // burst-silence AI cadence
        totalEvents: 12, untrustedFrac: 0, noMoveClicks: 4, syntheticFlags: 0,
        clickCount: 4, longGapCount: 3, burstVarMs: 2, minReactionMs: 40,
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
