// puppeteer.mjs — vanilla Puppeteer signature: CDP-driven headless Chromium
// with navigator.webdriver=true and no evasions (SoT-04). Modeled with headless
// Edge (same CDP transport / Chromium engine).

import { drive } from './_driver.mjs';

export const label = 'bot:puppeteer';
export const needsBrowser = true;

export async function run(baseURL) {
  return drive(baseURL, { headless: true });
}
