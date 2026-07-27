// Docker-only one-mode entry point for SoT-41. The host-side supervisor owns
// strict 1→2→3→4 sequencing and resets containers/state between invocations.
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createEnvironmentContext } from '../externalinput/environment.mjs';
import { runExternalProfile } from '../externalinput/runner.mjs';

async function main() {
  const context = await createEnvironmentContext(process.env);
  const result = await runExternalProfile(context.selectedMode.profileId, context);
  const artifactRoot = resolve(process.env.HM_EXTERNAL_ARTIFACT_ROOT || '/artifacts/external-input');
  await mkdir(artifactRoot, { recursive: true });
  const resultPath = resolve(artifactRoot, `${result.profileId}.result.json`);
  if (!resultPath.startsWith(`${artifactRoot}\\`) && !resultPath.startsWith(`${artifactRoot}/`)) {
    throw new Error('result path escaped artifact root');
  }
  await writeFile(resultPath, `${JSON.stringify(result, null, 2)}\n`, { encoding: 'utf8' });
  process.stdout.write(`${JSON.stringify(result)}\n`);
  if (result.status === 'UNAVAILABLE') process.exitCode = 3;
  else if (result.status === 'FAIL') process.exitCode = 1;
}

main().catch((error) => {
  process.stderr.write(`external-input runner failed: ${error.message || error}\n`);
  process.exitCode = 1;
});
