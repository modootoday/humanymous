// nodriver.mjs — nodriver signature (SoT-04): the current evasion frontier. It
// minimizes/eliminates CDP-as-driver, strips the webdriver flag, leaves no
// injected globals, and can run headful (xvfb). Static L1 tells are gone, so the
// Blue engine must fall back to behavior (no human input) and network/reputation.
// Modeled as headful Edge, AutomationControlled disabled, NO interaction. This
// is honestly the hard case: expect CHALLENGE (blocked pending interaction
// proof), not necessarily DENY, on a single page view (plan/04, SoT-06 §6).

import { drive } from './_driver.mjs';

export const label = 'bot:nodriver';
export const needsBrowser = true;

export async function run(baseURL) {
  return drive(baseURL, {
    headless: false,
    launchArgs: ['--disable-blink-features=AutomationControlled'],
    // no interact: nodriver-class scrapers take content without human input
  });
}
