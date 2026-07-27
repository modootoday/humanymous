// Canonical supervisor helper: records raw Docker Engine inventory only. It does
// not decide whether residue is acceptable; the parent assertion service does.
import { execFileSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

function inventory(kind, project) {
  const docker = process.env.DOCKER || 'docker';
  const noun = kind === 'containers' ? ['ps', '-a'] : [kind.slice(0, -1), 'ls'];
  const raw = execFileSync(docker, [
    ...noun,
    '--filter', `label=com.docker.compose.project=${project}`,
    '--format', '{{.ID}}',
  ], { encoding: 'utf8', windowsHide: true });
  return raw.split(/\r?\n/).filter(Boolean);
}

export async function captureTeardown({
  runId,
  projectName,
  downExitCode,
  destination,
  now = new Date(),
}) {
  const observation = {
    schemaVersion: 'humanymous.virtual-usb-teardown/v1',
    runId,
    projectName,
    downExitCode,
    containers: inventory('containers', projectName),
    networks: inventory('networks', projectName),
    volumes: inventory('volumes', projectName),
    observedAt: now.toISOString(),
  };
  await writeFile(destination, `${JSON.stringify(observation, null, 2)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o600,
  });
  return observation;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  captureTeardown({
    runId: required('HM_VUSB_RUN_ID'),
    projectName: required('HM_VUSB_CHILD_PROJECT'),
    downExitCode: Number(required('HM_VUSB_DOWN_EXIT_CODE')),
    destination: required('HM_VUSB_TEARDOWN_OBSERVATION'),
  }).catch((error) => {
    process.stderr.write(`virtual USB teardown capture failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
