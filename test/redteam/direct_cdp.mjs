// direct_cdp.mjs — hand-rolled CDP control (SoT-04): drives Chromium over the
// DevTools Protocol without a stealth layer, leaving navigator.webdriver=true and
// (headless) a Headless UA token. Modeled with headless Edge => HR-7.
import { drive } from './_driver.mjs';
export const label = 'bot:direct-cdp';
export const needsBrowser = true;
export async function run(baseURL) {
  return drive(baseURL, { headless: true });
}
