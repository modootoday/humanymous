// playwright_stealth.mjs — Playwright with hand-rolled stealth evasions
// (mirrors puppeteer-extra-stealth): patch navigator.webdriver, window.chrome,
// navigator.plugins/languages, and a clean (non-Headless) UA. This is the hard
// case: static L1 tells are patched, so the Blue engine must rely on L3
// integrity / behavior / network residuals (SoT-04). Verdict is recorded as-is
// to honestly measure evasion (plan/04 §4).

import { drive } from './_driver.mjs';

export const label = 'bot:playwright-stealth';
export const needsBrowser = true;

const stealthScript = () => {
  // navigator.webdriver -> false (via defineProperty; leaves a non-native getter)
  Object.defineProperty(Navigator.prototype, 'webdriver', { get: () => false, configurable: true });
  // window.chrome
  if (!window.chrome) window.chrome = { runtime: {}, app: {}, csi: () => {}, loadTimes: () => {} };
  // plugins (fake a non-empty PluginArray-ish length)
  Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3], configurable: true });
  Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'], configurable: true });
};

export async function run(baseURL) {
  return drive(baseURL, {
    headless: true,
    userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36',
    initScripts: [stealthScript],
  });
}
