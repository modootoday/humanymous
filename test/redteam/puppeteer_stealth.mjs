// puppeteer_stealth.mjs — puppeteer-extra-plugin-stealth signature (SoT-04):
// the full evasion bundle patches webdriver, window.chrome, plugins, permissions,
// languages via defineProperty. The Blue engine catches the non-native getters
// (L3) => HR-8.
import { drive } from './_driver.mjs';
export const label = 'bot:puppeteer-stealth';
export const needsBrowser = true;
const stealth = () => {
  Object.defineProperty(Navigator.prototype, 'webdriver', { get: () => false, configurable: true });
  window.chrome = { runtime: {}, app: {}, csi: () => {}, loadTimes: () => {} };
  Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3, 4, 5], configurable: true });
  Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'], configurable: true });
  const q = window.Notification && Notification.requestPermission;
  if (navigator.permissions) {
    const orig = navigator.permissions.query.bind(navigator.permissions);
    navigator.permissions.query = (p) => p && p.name === 'notifications'
      ? Promise.resolve({ state: Notification.permission }) : orig(p);
  }
  void q;
};
export async function run(baseURL) {
  return drive(baseURL, {
    headless: true,
    userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36',
    initScripts: [stealth],
  });
}
