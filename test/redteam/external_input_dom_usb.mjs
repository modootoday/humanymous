import { EXECUTION_KIND } from '../externalinput/contracts.mjs';
import { runExternalProfile } from '../externalinput/runner.mjs';

export const label = 'bot:external-input-dom-usb';
export const needsBrowser = true;
export const executionKind = EXECUTION_KIND;
export const profileId = 'external_input_dom_usb';
export const sequence = 4;

export async function run(context) {
  return runExternalProfile(profileId, context);
}
