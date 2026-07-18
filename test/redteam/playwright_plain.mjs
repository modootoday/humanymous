// playwright_plain.mjs — vanilla Playwright (headless), no evasions. Exhibits
// navigator.webdriver=true and (headless) a Headless UA token, and performs no
// human input. The Blue engine should flag it (>= CHALLENGE) (plan/04 §2).

import { drive } from './_driver.mjs';

export const label = 'bot:playwright';
export const needsBrowser = true;

export async function run(baseURL) {
  // No interaction: a plain scraper just loads the page.
  return drive(baseURL, { headless: true });
}
