import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

const sha256 = /^sha256:[0-9a-f]{64}$/;

function exactObject(value, fields, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  const keys = Object.keys(value).sort();
  const expected = [...fields].sort();
  if (keys.length !== expected.length || keys.some((key, index) => key !== expected[index])) {
    throw new TypeError(`${label} has unexpected fields`);
  }
}

export function validateGuestReceipt(receipt) {
  exactObject(receipt, [
    'schemaVersion', 'kind', 'status', 'kernel', 'transport', 'deviceNodeCount',
    'deviceNodes', 'drivers', 'neutralRelease', 'configfsCleanup', 'physicalUsb',
  ], 'guest receipt');
  if (receipt.schemaVersion !== 1 || receipt.kind !== 'kernel-vusb-smoke' ||
      receipt.status !== 'PASS' || receipt.transport !== 'dummy_hcd') {
    throw new TypeError('guest receipt authority is invalid');
  }
  if (!/^[0-9][0-9A-Za-z.+_-]+$/.test(receipt.kernel)) {
    throw new TypeError('guest kernel release is invalid');
  }
  if (receipt.deviceNodeCount !== 6 || receipt.deviceNodes.length !== 6 ||
      new Set(receipt.deviceNodes).size !== 6 ||
      receipt.deviceNodes.some((path) => !path.startsWith('/dev/'))) {
    throw new TypeError('guest must attest exactly six distinct device nodes');
  }
  exactObject(receipt.drivers, ['command', 'keyboard', 'pointer'], 'driver receipt');
  if (receipt.drivers.command !== 'cdc_acm' ||
      !['hid-generic', 'usbhid'].includes(receipt.drivers.keyboard) ||
      !['hid-generic', 'usbhid'].includes(receipt.drivers.pointer)) {
    throw new TypeError('guest USB host drivers are invalid');
  }
  if (receipt.neutralRelease !== true || receipt.configfsCleanup !== true ||
      receipt.physicalUsb !== false) {
    throw new TypeError('guest cleanup or evidence-class contract is invalid');
  }
  return true;
}

export function validateRunnerReceipt(receipt) {
  exactObject(receipt, [
    'schemaVersion', 'kind', 'status', 'accelerator', 'qemuVersion',
    'kernelSha256', 'initramfsSha256', 'guestReceipt', 'consoleLog',
  ], 'runner receipt');
  if (receipt.schemaVersion !== 1 || receipt.kind !== 'kernel-runner' ||
      receipt.status !== 'PASS' || !['kvm', 'tcg'].includes(receipt.accelerator)) {
    throw new TypeError('runner receipt authority is invalid');
  }
  if (!/^[0-9]+\.[0-9]+\.[0-9]+$/.test(receipt.qemuVersion) ||
      !sha256.test(receipt.kernelSha256) || !sha256.test(receipt.initramfsSha256) ||
      receipt.guestReceipt !== 'guest-smoke.json' || receipt.consoleLog !== 'console.log') {
    throw new TypeError('runner artifact identity is invalid');
  }
  return true;
}

export async function validateReceiptDirectory(directory) {
  const guest = JSON.parse(await readFile(`${directory}/guest-smoke.json`, 'utf8'));
  const runner = JSON.parse(await readFile(`${directory}/runner.json`, 'utf8'));
  validateGuestReceipt(guest);
  validateRunnerReceipt(runner);
  return { guest, runner };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  validateReceiptDirectory(process.argv[2] || '/output').then(({ runner }) => {
    process.stdout.write(`${JSON.stringify({
      status: 'PASS',
      accelerator: runner.accelerator,
      kernelSha256: runner.kernelSha256,
    })}\n`);
  }).catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
