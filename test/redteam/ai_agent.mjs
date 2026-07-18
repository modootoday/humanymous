// ai_agent.mjs — LLM browser-agent signature (SoT-16, FP-Agent): a real browser
// driven in a think-pause -> instant-action-burst loop, teleport clicks (no
// approach trajectory) and machine-speed typing. Models Operator / browser-use /
// Claude-computer-use cadence. Caught by l4.agent.* behavioral tells.
import { drive } from './_driver.mjs';
export const label = 'bot:ai-agent';
export const needsBrowser = true;

async function agentLoop(page) {
  await page.evaluate(() => { document.body.tabIndex = 0; document.body.focus(); });
  // Two think->act cycles inside the observation window: long inference pause,
  // then an instant burst (teleport clicks + machine-speed typing).
  for (let cycle = 0; cycle < 3; cycle++) {
    await page.waitForTimeout(1550); // model "thinking" pause (> long-gap threshold)
    await page.mouse.click(200 + cycle * 80, 150 + cycle * 40); // teleport click, no approach
    await page.mouse.click(320, 260);
    for (const ch of 'searchquery') await page.keyboard.press(ch.toUpperCase(), { delay: 0 });
  }
}

export async function run(baseURL) {
  return drive(baseURL, {
    headless: false,
    launchArgs: ['--disable-blink-features=AutomationControlled'],
    interact: agentLoop,
  });
}
