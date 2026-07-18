// selenium.mjs — reproduces Selenium/ChromeDriver's documented tells via init
// scripts (SoT-04): the injected cdc_ document property plus webdriver. (Running
// real ChromeDriver needs a matching driver binary; we reproduce the exact
// artifacts so the Blue engine is tested against Selenium's signature.)

import { drive } from './_driver.mjs';

export const label = 'bot:selenium';
export const needsBrowser = true;

const seleniumArtifacts = () => {
  // ChromeDriver injects a cdc_-prefixed key onto document.
  document.$cdc_asdjflasutopfhvcZLmcfl_ = {};
  window.__webdriver_evaluate = true;
  window.__selenium_evaluate = true;
  window.domAutomation = true;
};

export async function run(baseURL) {
  return drive(baseURL, { headless: true, initScripts: [seleniumArtifacts] });
}
