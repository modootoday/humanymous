import { createHash } from 'node:crypto';
import { lstat, mkdir, open, readFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { Duplex } from 'node:stream';
import { UsbBroker } from '../usb-broker/broker.mjs';
import { assertVirtualUsbAttestation } from '../input.mjs';
import { GatewayChannel } from './gateway-channel.mjs';
import { createImeActionTracker, loadImePolicy } from './ime-policy.mjs';
import { atomicJson, receiptBase } from './common.mjs';

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

async function run() {
  const devicePath = required('HM_VUSB_HOST_COMMAND_DEVICE');
  const socketPath = required('HM_EXTERNAL_USB_SOCKET');
  const attestationPath = required('HM_EXTERNAL_USB_ATTESTATION');
  const stat = await lstat(devicePath);
  if (!stat.isCharacterDevice()) throw new Error('host CDC path is not a character device');
  const attestation = assertVirtualUsbAttestation(
    JSON.parse(await readFile(attestationPath, 'utf8')),
  );
  const handle = await open(devicePath, 'r+');
  const transport = Duplex.from({
    readable: handle.createReadStream({ autoClose: false }),
    writable: handle.createWriteStream({ autoClose: false }),
  });
  const gateway = new GatewayChannel(transport, attestation);
  await gateway.handshake();
  await mkdir(dirname(socketPath), { recursive: true });
  const imePolicyPath = process.env.HM_VUSB_IME_POLICY || '';
  const imePolicy = imePolicyPath
    ? await loadImePolicy(imePolicyPath, required('HM_VUSB_RUN_ID'))
    : null;
  const actionPolicy = imePolicy ? createImeActionTracker(imePolicy) : null;
  const acceptedHash = createHash('sha256');
  let firmwareAcceptedCount = 0;
  let lastSequenceId = '';
  const broker = new UsbBroker({
    socketPath,
    firmware: gateway,
    actionPolicy,
    onFirmwareAccepted(envelope) {
      firmwareAcceptedCount += 1;
      lastSequenceId = envelope.sequenceId;
      acceptedHash.update(JSON.stringify({
        sequenceId: envelope.sequenceId,
        action: envelope.action,
      }));
      acceptedHash.update('\n');
    },
  });
  await broker.start();

  let stopping = false;
  const stop = async (signal) => {
    if (stopping) return;
    stopping = true;
    try {
      await broker.stop();
      if (imePolicy) {
        const policy = actionPolicy.snapshot();
        if (!policy.complete ||
            policy.acceptedActionCount + policy.releaseAllCount !== firmwareAcceptedCount) {
          throw new Error('broker IME policy did not complete through firmware acknowledgement');
        }
        await atomicJson(required('HM_VUSB_BROKER_EVIDENCE'), {
          ...receiptBase('broker-policy-evidence', required('HM_VUSB_RUN_ID')),
          locale: imePolicy.locale,
          vectorId: imePolicy.vectorId,
          policySha256: imePolicy.policySha256,
          firmwareAcceptedCount,
          lastSequenceId,
          acceptedActionSha256: acceptedHash.digest('hex'),
          policy,
        });
      }
      await handle.close().catch(() => {});
    } catch (error) {
      process.stderr.write(`${JSON.stringify({
        level: 'error',
        component: 'external-vusb-broker',
        code: 'SAFETY_ABORT',
        signal,
        message: error.message,
      })}\n`);
      process.exitCode = 1;
    }
  };
  process.once('SIGINT', () => void stop('SIGINT'));
  process.once('SIGTERM', () => void stop('SIGTERM'));
}

run().catch((error) => {
  process.stderr.write(`${JSON.stringify({
    level: 'error',
    component: 'external-vusb-broker',
    code: 'SAFETY_ABORT',
    message: error.message,
  })}\n`);
  process.exitCode = 1;
});
