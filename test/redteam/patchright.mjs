// patchright.mjs — Patchright signature (SoT-04): undetected Playwright that
// avoids Runtime.enable, DISABLES the Console API, strips the Headless UA token
// and the automation flag. The disabled console is itself the anomaly the Blue
// engine keys on => HR-13.
import { drive } from './_driver.mjs';
export const label = 'bot:patchright';
export const needsBrowser = true;
const killConsole = () => {
  // Patchright disables the Console API to kill the console-serialization leak.
  const noop = () => {};
  try {
    for (const k of ['log', 'debug', 'info', 'warn', 'error', 'trace', 'dir', 'table']) {
      Object.defineProperty(window.console, k, { value: noop, configurable: true });
    }
  } catch (_) {}
};
export async function run(baseURL) {
  return drive(baseURL, {
    headless: true,
    launchArgs: ['--disable-blink-features=AutomationControlled'],
    userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36',
    initScripts: [killConsole],
  });
}
