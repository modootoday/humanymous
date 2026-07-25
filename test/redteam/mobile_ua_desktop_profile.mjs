// mobile_ua_desktop_profile.mjs — residual class from UA / Client-Hints / JS
// fingerprint-inconsistency research (2025–2026): bots often forge a *mobile*
// User-Agent (or MobileUA flag) while the rest of the profile is desktop
// (no touch points, fine pointer). Blue derives l2.adv.mobile_inconsistent.
//
// Defensive self-validation only — local collect endpoint.
// Local target only. Requires no browser dependencies.

export const label = 'bot:mobile-ua-desktop-profile';
export const needsBrowser = false;

// iPhone-shaped UA (mobile claim) with desktop advanced probe fields.
const MOBILE_UA =
  'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1';

export async function run(baseURL) {
  const report = {
    userAgent: MOBILE_UA,
    engineVersion: 'wasm-1.0.0',
    // Client-hints surface also claims Android while UA says iPhone — common
    // partial-spoof residual (UA-CH vs UA string drift in bot kits).
    uaClientHints: {
      mobile: true,
      platform: 'Android',
      brands: [
        { brand: 'Chromium', version: '126' },
        { brand: 'Google Chrome', version: '126' },
      ],
    },
    advanced: {
      probed: true,
      mobileUA: true, // client-reported mobile claim
      maxTouchPoints: 0, // desktop residual
      pointerCoarse: false, // fine pointer = desktop
      mediaDeviceCount: 0,
      voiceCount: 0,
      widevineSupported: false,
      webgpuPresent: false,
    },
    environment: { probed: true },
    // Minimal behavior so we are not only an HTTP parrot (HR-18); keep motor thin.
    behavior: {
      durationS: 4,
      mouse: {
        samples: 4,
        meanCurvature: 0.1,
        velocityStdDev: 0.1,
        accelEntropy: 0.2,
        straightLineFrac: 0.8,
        pauseCount: 0,
        maxTeleportPx: 200,
        meanJerk: 0.05,
        coalescedRatio: 0.1,
      },
      key: {
        keystrokes: 0,
        meanDwellMs: 0,
        dwellStdDevMs: 0,
        meanFlightMs: 0,
        flightStdDevMs: 0,
        pasteEvents: 0,
        zeroDwellFrac: 0,
      },
      events: {
        totalEvents: 6,
        untrustedFrac: 0,
        noMoveClicks: 1,
        syntheticFlags: 0,
        clickCount: 1,
        longGapCount: 0,
        burstVarMs: 80,
        minReactionMs: 100,
      },
    },
    signals: [],
  };

  const res = await fetch(baseURL + '/api/collect?label=' + encodeURIComponent(label), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'User-Agent': MOBILE_UA,
      // Header UA-CH disagrees with iPhone UA string (platform Android) — residual.
      'Sec-CH-UA': '"Chromium";v="126", "Google Chrome";v="126", "Not.A/Brand";v="24"',
      'Sec-CH-UA-Mobile': '?1',
      'Sec-CH-UA-Platform': '"Android"',
    },
    body: JSON.stringify(report),
  });
  return res.json();
}
