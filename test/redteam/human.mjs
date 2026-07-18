// human.mjs — baseline "human" profile: headful Edge with human-like mouse
// curves and variable-rhythm typing. Used to measure FALSE POSITIVES: the Blue
// engine must NOT deny this session (plan/04 §2.1).

import { drive } from './_driver.mjs';

export const label = 'human';
export const needsBrowser = true;

// curved, jittery mouse motion with pauses + overshoot (SoT-03 §1).
async function humanMouse(page) {
  const rnd = (a, b) => a + Math.random() * (b - a);
  let [x, y] = [rnd(100, 300), rnd(100, 300)];
  for (let stroke = 0; stroke < 6; stroke++) {
    const [tx, ty] = [rnd(80, 900), rnd(80, 500)];
    const cx = (x + tx) / 2 + rnd(-120, 120); // control point -> curvature
    const cy = (y + ty) / 2 + rnd(-120, 120);
    const steps = Math.floor(rnd(18, 34));
    for (let i = 1; i <= steps; i++) {
      const t = i / steps;
      const mt = 1 - t;
      // quadratic bezier + small jitter
      const px = mt * mt * x + 2 * mt * t * cx + t * t * tx + rnd(-1.5, 1.5);
      const py = mt * mt * y + 2 * mt * t * cy + t * t * ty + rnd(-1.5, 1.5);
      await page.mouse.move(px, py);
      await page.waitForTimeout(rnd(6, 22)); // variable inter-sample timing
    }
    x = tx; y = ty;
    if (Math.random() < 0.5) await page.waitForTimeout(rnd(120, 400)); // pauses
  }
}

async function humanType(page) {
  await page.evaluate(() => document.body.setAttribute('tabindex', '0'));
  await page.evaluate(() => document.body.focus());
  const text = 'the quick brown fox';
  for (const ch of text) {
    await page.keyboard.press(keyFor(ch), { delay: 40 + Math.random() * 90 });
    await page.waitForTimeout(20 + Math.random() * 60);
  }
}

function keyFor(ch) {
  if (ch === ' ') return 'Space';
  return ch.toUpperCase();
}

export async function run(baseURL) {
  return drive(baseURL, {
    headless: false,
    // Simulate a real, non-automated browser: this flag makes Chromium report
    // navigator.webdriver=false natively (no JS patch), as a human's browser
    // would. Bot profiles deliberately omit it.
    launchArgs: ['--disable-blink-features=AutomationControlled'],
    interact: async (page) => {
      await humanMouse(page);
      await humanType(page);
    },
  });
}
