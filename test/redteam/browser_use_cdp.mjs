// browser_use_cdp.mjs — residual class inspired by public “pure CDP / no Playwright”
// agent stacks (e.g. Browser Use CDP era posts). Suppresses classic webdriver and
// Runtime.enable-shaped leaks, but keeps CDP proxy residual + zero real motor.
// Local self-validation only.
import { drive } from './_driver.mjs';

export const label = 'bot:browser-use-cdp';
export const needsBrowser = true;

export async function run(baseURL) {
  return drive(baseURL, {
    headless: true,
    launchArgs: ['--disable-blink-features=AutomationControlled'],
    initScripts: [
      () => {
        try {
          Object.defineProperty(navigator, 'webdriver', { get: () => false, configurable: true });
        } catch { /* sealed */ }
        // Residual headless geometry (common when agents run headless Chrome over CDP).
        try {
          Object.defineProperty(window, 'outerWidth', { get: () => window.innerWidth });
          Object.defineProperty(window, 'outerHeight', { get: () => window.innerHeight });
        } catch { /* ignore */ }
      },
    ],
    // No interact: pure CDP agents often navigate without human motor microstructure.
  });
}
