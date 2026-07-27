import { EXECUTION_KIND } from '../externalinput/contracts.mjs';
import { runExternalProfile } from '../externalinput/runner.mjs';

export const label = 'bot:external-input-virtual';
export const needsBrowser = true;
export const executionKind = EXECUTION_KIND;
export const profileId = 'external_input_virtual';
export const sequence = 1;

export async function run(context) {
  return runExternalProfile(profileId, context);
}
