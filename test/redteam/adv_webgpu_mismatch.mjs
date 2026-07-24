// adv_webgpu_mismatch.mjs — Red profile: an anti-detect browser that spoofs the
// WebGL vendor string (claiming an NVIDIA discrete GPU) but forgets to keep the
// WebGPU adapter consistent — it still reports the host's Intel iGPU. The GPU
// families disagree. A few other capability-absence tells corroborate. Blue
// trips l2.adv.webgpu_mismatch -> CHALLENGE.
//
// Local target only. Requires no dependencies.

export const label = 'bot:webgpu-mismatch';
export const needsBrowser = false;

const CHROME_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: CHROME_UA,
    engineVersion: 'wasm-1.0.0',
    advanced: {
      probed: true,
      webgpuPresent: true,
      webglVendor: 'NVIDIA Corporation / NVIDIA GeForce RTX 3080',
      webgpuVendor: 'intel',
      voiceCount: 0,
      widevineSupported: false,
      mediaDeviceCount: 0,
    },
    environment: { probed: true },
    behavior: {
      durationS: 5,
      mouse: {
        samples: 10, meanCurvature: 0.3, velocityStdDev: 0.4, accelEntropy: 1.5,
        straightLineFrac: 0.4, pauseCount: 2, maxTeleportPx: 50, meanJerk: 0.2,
        coalescedRatio: 0.4,
      },
      key: {
        keystrokes: 0, meanDwellMs: 0, dwellStdDevMs: 0, meanFlightMs: 0,
        flightStdDevMs: 0, pasteEvents: 0, zeroDwellFrac: 0,
      },
      events: {
        totalEvents: 12, untrustedFrac: 0, noMoveClicks: 0, syntheticFlags: 0,
        clickCount: 1, longGapCount: 0, burstVarMs: 150, minReactionMs: 200,
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
