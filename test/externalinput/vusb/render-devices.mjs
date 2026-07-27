import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { atomicJson, exactObject, receiptBase, sha256 } from './common.mjs';
import { parseStrictJson } from './strict-json.mjs';

const DEVICE_FIELDS = [
  'hostPath', 'deviceHex', 'sysfsDevice', 'sysfsPath', 'driverPath',
];

function device(value, label) {
  exactObject(value, DEVICE_FIELDS, label);
  if (!value.hostPath.startsWith('/dev/') || !/^[0-9a-f]+:[0-9a-f]+$/.test(value.deviceHex) ||
      !value.sysfsPath.startsWith('/sys/')) {
    throw new TypeError(`${label} has an invalid path or device identity`);
  }
  return value;
}

export function renderDeviceOverride(prepare) {
  if (prepare?.kind !== 'prepare' || prepare?.stableObservations !== 2) {
    throw new TypeError('stable prepare receipt is required');
  }
  if (prepare.deviceIdentityCount !== 6 || prepare.driverContractVerified !== true) {
    throw new TypeError('verified six-device driver contract is required');
  }
  exactObject(prepare.gadget, ['command', 'keyboard', 'pointer'], 'gadget devices');
  exactObject(prepare.host, ['command', 'keyboard', 'pointer'], 'host devices');
  for (const [side, values] of [['gadget', prepare.gadget], ['host', prepare.host]]) {
    for (const name of ['command', 'keyboard', 'pointer']) device(values[name], `${side}.${name}`);
  }
  const unique = new Set([
    ...Object.values(prepare.gadget),
    ...Object.values(prepare.host),
  ].map(({ hostPath }) => hostPath));
  if (unique.size !== 6) throw new TypeError('the exact six device mappings must be distinct');
  return {
    services: {
      'external-vusb-gateway': {
        devices: [
          `${prepare.gadget.command.hostPath}:/dev/vusb-command:rw`,
          `${prepare.gadget.keyboard.hostPath}:/dev/vusb-keyboard:rw`,
          `${prepare.gadget.pointer.hostPath}:/dev/vusb-pointer:rw`,
        ],
      },
      'external-vusb-broker': {
        devices: [`${prepare.host.command.hostPath}:/dev/vusb-host-command:rw`],
      },
      'external-display': {
        devices: [
          `${prepare.host.keyboard.hostPath}:/dev/input/vusb-keyboard:r`,
          `${prepare.host.pointer.hostPath}:/dev/input/vusb-pointer:r`,
        ],
      },
    },
  };
}

export async function render({
  preparePath,
  overridePath,
  receiptPath,
  runId,
  now = new Date(),
}) {
  const prepare = parseStrictJson(await readFile(preparePath, 'utf8'), 'prepare receipt');
  if (prepare.runId !== runId) throw new TypeError('prepare receipt run ID mismatch');
  const override = renderDeviceOverride(prepare);
  await atomicJson(overridePath, override);
  const raw = `${JSON.stringify(override, null, 2)}\n`;
  const receipt = {
    ...receiptBase('device-mapping', runId, now),
    mappingCount: 6,
    exclusiveAssignment: true,
    mode: 'compose-exact-path',
    cdi: false,
    overrideSha256: `sha256:${sha256(raw)}`,
  };
  await atomicJson(receiptPath, receipt);
  return receipt;
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  render({
    preparePath: required('HM_VUSB_PREPARE_RECEIPT'),
    overridePath: required('HM_VUSB_DEVICE_OVERRIDE'),
    receiptPath: required('HM_VUSB_MAPPING_RECEIPT'),
    runId: required('HM_VUSB_RUN_ID'),
  }).catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-device-render',
      code: 'PURITY_FAIL',
      message: error.message,
    })}\n`);
    process.exitCode = 1;
  });
}
