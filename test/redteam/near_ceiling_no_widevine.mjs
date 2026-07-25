// near_ceiling_no_widevine.mjs — T4-adjacent residual (promotion target).
// Chrome-branded UA with rich motor, but Widevine absent + empty media devices —
// Playwright/plain Chromium container residual (l2.adv.no_widevine / no_media).
// Must not score as honest T4 ceiling ALLOW.
//
// Local target only.
export const label = 'bot:near-ceiling-no-widevine';
export const needsBrowser = false;

const UA =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

export async function run(baseURL) {
  const report = {
    userAgent: UA,
    engineVersion: 'wasm-1.0.0',
    uaClientHints: {
      platform: 'Windows',
      mobile: false,
      brands: [
        { brand: 'Chromium', version: '126' },
        { brand: 'Google Chrome', version: '126' },
      ],
    },
    advanced: {
      probed: true,
      mediaDeviceCount: 0,
      voiceCount: 0,
      widevineSupported: false, // residual under Chrome UA
      webgpuPresent: true,
      webgpuVendor: 'google',
      webglVendor: 'Google Inc. (Google)',
      audioSampleRate: 48000,
      connectionPresent: true,
      connectionRtt: 30,
      maxTouchPoints: 0,
    },
    environment: { probed: true },
    behavior: {
      durationS: 6,
      mouse: {
        samples: 35,
        velocityStdDev: 0.5,
        straightLineFrac: 0.2,
        accelEntropy: 1.8,
        meanJerk: 0.3,
        meanCurvature: 0.25,
        coalescedRatio: 2.0,
      },
      key: {
        keystrokes: 10,
        meanDwellMs: 100,
        dwellStdDevMs: 30,
        meanFlightMs: 125,
        flightStdDevMs: 28,
      },
      events: { totalEvents: 40, untrustedFrac: 0, clickCount: 1 },
    },
    signals: [],
  };
  const res = await fetch(baseURL + '/api/collect?label=' + encodeURIComponent(label), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'User-Agent': UA,
      'sec-ch-ua': '"Chromium";v="126", "Google Chrome";v="126"',
      'sec-ch-ua-mobile': '?0',
      'sec-ch-ua-platform': '"Windows"',
    },
    body: JSON.stringify(report),
  });
  return res.json();
}
