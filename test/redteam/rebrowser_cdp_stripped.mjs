// rebrowser_cdp_stripped.mjs — residual class from 2025–2026 rebrowser-patches /
// rebrowser-bot-detector research (defensive self-validation only).
//
// rebrowser-style patches suppress the classic CDP Runtime.enable leak and often
// force navigator.webdriver=false. Benchmarks still show residual automation tells
// (headless geometry, no real interaction, secondary integrity signals). This profile
// plants that residual surface — NOT a recipe for third-party evasion.
//
// Expected: CHALLENGE or DENY (bot: label). Typical path: no_interaction → HR-12
// and/or outer_eq_inner + stripped webdriver residual scoring.
import { drive } from './_driver.mjs';

export const label = 'bot:rebrowser-cdp-stripped';
export const needsBrowser = true;

export async function run(baseURL) {
  return drive(baseURL, {
    headless: true,
    initScripts: [
      () => {
        // Suppress classic webdriver flag the way CDP-stealth patches do.
        try {
          Object.defineProperty(navigator, 'webdriver', {
            get: () => false,
            configurable: true,
          });
        } catch { /* already sealed */ }
        // Residual headless geometry tell (outer ≡ inner) — still common when
        // Runtime.enable is patched but the session is still headless automation.
        try {
          Object.defineProperty(window, 'outerWidth', { get: () => window.innerWidth });
          Object.defineProperty(window, 'outerHeight', { get: () => window.innerHeight });
        } catch { /* ignore */ }
        // Residual: no human motor during the collection window (no interact hook).
      },
    ],
    // Intentionally no interact() — zero-interaction residual (HR-12 / behavior).
  });
}
