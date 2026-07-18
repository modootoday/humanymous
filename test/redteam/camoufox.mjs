// camoufox.mjs — Camoufox signature (SoT-04): a Firefox fork with C++-level
// fingerprint spoofing, driven via a patched Juggler (Playwright Firefox is the
// closest available stand-in). Camoufox's real weakness (2026) is Firefox-build /
// version inconsistency and behavior; here the automation runs Firefox with no
// human input, so the Blue engine blocks it on the no-interaction frontier (HR-12)
// plus Firefox webdriver. Requires Playwright's Firefox build; skips if absent.

import { drive } from './_driver.mjs';

export const label = 'bot:camoufox';
export const needsBrowser = true;

export async function run(baseURL) {
  return drive(baseURL, { engine: 'firefox', headless: true });
}
