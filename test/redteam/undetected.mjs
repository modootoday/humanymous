// undetected.mjs — undetected-chromedriver signature (SoT-04): the cdc_ property
// is renamed/removed and the automation flag is stripped (webdriver=false), but
// it still runs a CDP-driven, typically headless Chromium. Modeled as headless
// Edge with AutomationControlled disabled and no artifacts — the Blue engine
// must catch it via headless + chromeless-window tells, not webdriver.

import { drive } from './_driver.mjs';

export const label = 'bot:undetected-chromedriver';
export const needsBrowser = true;

export async function run(baseURL) {
  return drive(baseURL, {
    headless: true,
    launchArgs: ['--disable-blink-features=AutomationControlled'],
  });
}
