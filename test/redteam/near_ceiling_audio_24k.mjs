// near_ceiling_audio_24k.mjs — T4-adjacent residual (promotion target).
// Looks nearly coherent (Chrome UA, JS evidence, human-ish motor) but leaves the
// classic headless AudioContext 24 kHz tell (l2.adv.audio_24k). Detection should
// NOT treat this as the honest T4 ceiling ALLOW — it is a bot residual class.
//
// Local target only. Public residual research: headless audio sample-rate tells.
export const label = 'bot:near-ceiling-audio-24k';
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
      mediaDeviceCount: 2,
      hasAudioInput: true,
      hasVideoInput: true,
      voiceCount: 80,
      widevineSupported: true,
      webgpuPresent: true,
      webgpuVendor: 'nvidia',
      webglVendor: 'NVIDIA Corporation / NVIDIA GeForce RTX 3080',
      // Residual tell — headless/no-hardware audio path
      audioSampleRate: 24000,
      connectionPresent: true,
      connectionRtt: 40,
      batteryPresent: true,
      batteryLevel: 0.7,
      timezoneIana: 'America/New_York',
      language: 'en-US',
      maxTouchPoints: 0,
    },
    environment: { probed: true },
    behavior: {
      durationS: 7,
      mouse: {
        samples: 40,
        velocityStdDev: 0.55,
        straightLineFrac: 0.18,
        accelEntropy: 1.9,
        meanJerk: 0.35,
        meanCurvature: 0.28,
        coalescedRatio: 2.5,
      },
      key: {
        keystrokes: 12,
        meanDwellMs: 90,
        dwellStdDevMs: 25,
        meanFlightMs: 130,
        flightStdDevMs: 32,
      },
      events: { totalEvents: 50, untrustedFrac: 0, clickCount: 1 },
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
      'Sec-Fetch-Dest': 'document',
      'Sec-Fetch-Mode': 'navigate',
      'Sec-Fetch-Site': 'none',
    },
    body: JSON.stringify(report),
  });
  return res.json();
}
