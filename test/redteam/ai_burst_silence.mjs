// ai_burst_silence.mjs — Red profile: an LLM-driven agent whose interaction
// cadence betrays inference latency — tight bursts of activity separated by long
// silent gaps while the model "thinks", with uniform burst timing and fast
// reaction. Blue trips l4.agent.burst_silence -> CHALLENGE.
//
// Local target only. Requires no dependencies.

export const label = 'bot:ai-burst-silence';
export const needsBrowser = false;

const CHROME_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: CHROME_UA,
    engineVersion: 'wasm-1.0.0',
    advanced: { probed: true },
    environment: { probed: true },
    behavior: {
      durationS: 20,
      mouse: {
        samples: 8, meanCurvature: 0.1, velocityStdDev: 0.05, accelEntropy: 0.3,
        straightLineFrac: 0.7, pauseCount: 3, maxTeleportPx: 200, meanJerk: 0.05,
        coalescedRatio: 0.1,
      },
      key: {
        keystrokes: 0, meanDwellMs: 0, dwellStdDevMs: 0, meanFlightMs: 0,
        flightStdDevMs: 0, pasteEvents: 0, zeroDwellFrac: 0,
      },
      events: {
        totalEvents: 12, untrustedFrac: 0, noMoveClicks: 0, syntheticFlags: 0,
        clickCount: 2, longGapCount: 3, burstVarMs: 2, minReactionMs: 40,
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
