import { open, readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { createInterface } from 'node:readline';
import { verifyProfileRoot } from './profile.mjs';
import { HidReportGateway } from './gateway-core.mjs';
import { atomicJson, receiptBase } from './common.mjs';
import { createImeActionTracker, loadImePolicy } from './ime-policy.mjs';

const MAX_FRAME_BYTES = 4 * 1024;

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

async function writer(path, bytes) {
  const handle = await open(path, 'w');
  try {
    await handle.write(bytes, 0, bytes.length, null);
  } finally {
    await handle.close();
  }
}

export async function runGateway({
  commandPath,
  keyboardPath,
  pointerPath,
  profileRoot,
  admissionReceiptPath,
  runId = '',
  imePolicyPath = '',
  evidencePath = '',
}) {
  const [{ profile, profileManifestSha256 }, verification] = await Promise.all([
    verifyProfileRoot(profileRoot),
    readFile(admissionReceiptPath, 'utf8').then(JSON.parse),
  ]);
  if (verification.kind !== 'profile-verification' ||
      verification.modelId !== profile.modelId ||
      verification.profileManifestSha256 !== profileManifestSha256 ||
      verification.entireRootfsValidated !== true) {
    throw new TypeError('gateway profile does not match whole-rootfs verification');
  }
  const imePolicy = imePolicyPath ? await loadImePolicy(imePolicyPath, runId) : null;
  if (imePolicy && !evidencePath) {
    throw new TypeError('IME gateway evidence path is required');
  }
  const actionPolicy = imePolicy ? createImeActionTracker(imePolicy) : null;
  const gateway = new HidReportGateway({
    limits: profile.limits,
    writeKeyboard: (report) => writer(keyboardPath, report),
    writePointer: (report) => writer(pointerPath, report),
    actionPolicy,
  });
  const command = await open(commandPath, 'r+');
  const input = command.createReadStream({ autoClose: false });
  const output = command.createWriteStream({ autoClose: false });
  const lines = createInterface({ input, crlfDelay: Infinity });
  let sessionId = null;
  let lastSequence = 0;
  const send = (value) => output.write(`${JSON.stringify(value)}\n`);
  try {
    for await (const line of lines) {
      if (Buffer.byteLength(line) > MAX_FRAME_BYTES) throw new TypeError('gateway frame exceeds bound');
      const frame = JSON.parse(line);
      if (frame?.protocolVersion !== profile.protocolVersion) throw new TypeError('gateway protocol mismatch');
      if (frame.kind === 'hello') {
        if (sessionId !== null || typeof frame.sessionId !== 'string' || typeof frame.nonce !== 'string') {
          throw new TypeError('gateway hello is invalid or replayed');
        }
        sessionId = frame.sessionId;
        send({
          protocolVersion: profile.protocolVersion,
          kind: 'helloAck',
          sessionId,
          nonce: frame.nonce,
          identity: {
            modelId: profile.modelId,
            profileManifestSha256,
            descriptorSetId: profile.descriptorSetId,
            usbOrigin: 'kernel-emulated',
            physicalCapable: false,
          },
          safety: {
            deadManArmed: true,
            deadManReleaseMs: profile.limits.deadManReleaseMs,
          },
        });
        continue;
      }
      if (frame.kind !== 'command' || frame.sessionId !== sessionId ||
          !Number.isInteger(frame.sequence) || frame.sequence !== lastSequence + 1 ||
          typeof frame.commandId !== 'string' || frame.deadlineUnixMs <= Date.now()) {
        throw new TypeError('gateway command correlation, sequence, or deadline is invalid');
      }
      lastSequence = frame.sequence;
      await gateway.perform(frame.action);
      if (actionPolicy) {
        const evidence = gateway.evidence();
        await atomicJson(evidencePath, {
          ...receiptBase('hid-report-evidence', runId),
          locale: imePolicy.locale,
          vectorId: imePolicy.vectorId,
          policySha256: imePolicy.policySha256,
          acceptedCommandSequence: lastSequence,
          ...evidence,
        }, { replace: true });
      }
      send({
        protocolVersion: profile.protocolVersion,
        kind: 'ack',
        sessionId,
        sequence: frame.sequence,
        commandId: frame.commandId,
        accepted: true,
        releasedAll: frame.action.kind === 'releaseAll',
        safety: {
          deadManArmed: true,
          deadManReleaseMs: profile.limits.deadManReleaseMs,
        },
      });
    }
  } finally {
    await gateway.close();
    await command.close();
  }
}

export async function main() {
  await runGateway({
    commandPath: required('HM_VUSB_COMMAND_DEVICE'),
    keyboardPath: required('HM_VUSB_KEYBOARD_DEVICE'),
    pointerPath: required('HM_VUSB_POINTER_DEVICE'),
    profileRoot: required('HM_VUSB_PROFILE_ROOT'),
    admissionReceiptPath: required('HM_VUSB_ADMISSION_RECEIPT'),
    runId: process.env.HM_VUSB_RUN_ID || '',
    imePolicyPath: process.env.HM_VUSB_IME_POLICY || '',
    evidencePath: process.env.HM_VUSB_HID_EVIDENCE || '',
  });
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-gateway',
      code: 'SAFETY_ABORT',
      message: error.message,
    })}\n`);
    process.exitCode = 1;
  });
}
