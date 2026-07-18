// _driver.mjs — shared Playwright driver for browser-based Red profiles. Uses
// the installed Edge (channel 'msedge') so no browser download is needed. If
// playwright-core is not installed, importing this throws and the runner skips
// the profile (plan/04 §3, graceful skip).

let chromium, firefox;
try {
  ({ chromium, firefox } = await import('playwright-core'));
} catch {
  chromium = null;
  firefox = null;
}

export const available = !!chromium;

// Browser channel is env-driven so the same profiles run on a dev workstation and
// in a Linux container:
//   - unset               -> 'msedge' (installed Edge, no download; local default)
//   - 'chromium'/'bundled'/'' -> Playwright's bundled Chromium (the container has it)
//   - any other value     -> that channel (e.g. 'chrome')
// HM_LAUNCH_ARGS (comma-separated) adds flags every launch needs in a container,
// e.g. --no-sandbox,--disable-dev-shm-usage.
const CHANNEL = process.env.HM_BROWSER_CHANNEL ?? 'msedge';
const ENV_ARGS = (process.env.HM_LAUNCH_ARGS || '').split(',').map((s) => s.trim()).filter(Boolean);

// drive launches the browser, applies initScripts, navigates, runs the interact
// hook during the collection window, then captures the /api/collect verdict.
export async function drive(baseURL, { headless = true, initScripts = [], interact, userAgent, launchArgs = [], engine = 'chromium' } = {}) {
  if (!chromium) throw new Error('playwright-core not installed');
  const args = [...launchArgs, ...ENV_ARGS];
  let browser;
  if (engine === 'firefox') {
    if (!firefox) throw new Error('playwright firefox not available');
    browser = await firefox.launch({ headless, args });
  } else {
    const opts = { headless, args };
    if (CHANNEL && CHANNEL !== 'chromium' && CHANNEL !== 'bundled') opts.channel = CHANNEL;
    browser = await chromium.launch(opts);
  }
  try {
    const ctxOpts = { ignoreHTTPSErrors: true };
    if (userAgent) ctxOpts.userAgent = userAgent;
    const context = await browser.newContext(ctxOpts);
    for (const s of initScripts) await context.addInitScript(s);
    const page = await context.newPage();

    const verdictPromise = page
      .waitForResponse((r) => r.url().includes('/api/collect'), { timeout: 20000 })
      .then((r) => r.json())
      .catch(() => null);

    await page.goto(baseURL + '/', { waitUntil: 'domcontentloaded' });
    if (interact) await interact(page);
    const verdict = await verdictPromise;
    return verdict || { verdict: 'NO_RESPONSE' };
  } finally {
    await browser.close();
  }
}
