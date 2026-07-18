// xvfb_headful.mjs — headful Chromium under a virtual framebuffer (SoT-04): runs
// "headful" to shed headless cosmetic tells, but is still CDP-driven with no human
// input. Modeled as headful Edge, webdriver stripped, NO interaction => HR-12.
import { drive } from './_driver.mjs';
export const label = 'bot:xvfb-headful';
export const needsBrowser = true;
export async function run(baseURL) {
  return drive(baseURL, {
    headless: false,
    launchArgs: ['--disable-blink-features=AutomationControlled'],
  });
}
