import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { chmod, chown, lstat, mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';

const EVENT_BYTES = 24;
const EV_SYN = 0;
const EV_KEY = 1;
const EV_REL = 2;
const EVDEV_KEY_CODES = Object.freeze({
  A: 30, B: 48, C: 46, D: 32, E: 18, F: 33, G: 34, H: 35,
  I: 23, J: 36, K: 37, L: 38, M: 50, N: 49, O: 24, P: 25,
  Q: 16, R: 19, S: 31, T: 20, U: 22, V: 47, W: 17, X: 45,
  Y: 21, Z: 44, Enter: 28, Space: 57,
});

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

async function main() {
  if (process.platform !== 'linux' || process.arch !== 'x64') {
    throw new Error('seat observer requires the canonical Linux amd64 input_event layout');
  }
  const runId = required('HM_EXTERNAL_RUN_ID');
  const destination = required('HM_VUSB_SEAT_EVIDENCE');
  const keyboardPath = required('HM_EXTERNAL_HID_KEYBOARD');
  const pointerPath = required('HM_EXTERNAL_HID_POINTER');
  const policyPath = process.env.HM_VUSB_IME_POLICY || '';
  let imePolicyFileSha256 = '';
  let expectedKeyboardTransitions = [];
  if (policyPath) {
    const policyRaw = await readFile(policyPath);
    const policy = JSON.parse(policyRaw);
    if (!Array.isArray(policy.actions)) throw new Error('IME policy has no action vector');
    imePolicyFileSha256 = `sha256:${createHash('sha256').update(policyRaw).digest('hex')}`;
    expectedKeyboardTransitions = policy.actions
      .filter(({ kind }) => kind === 'keyStroke')
      .flatMap(({ key }) => {
        const code = EVDEV_KEY_CODES[key];
        if (!code) throw new Error('IME policy contains an unmapped evdev key');
        return [{ code, value: 1 }, { code, value: 0 }];
      });
    if (expectedKeyboardTransitions.length < 2 ||
        expectedKeyboardTransitions.length > 128) {
      throw new Error('IME policy keyboard sequence is unbounded');
    }
  }
  const keyboardStat = await lstat(keyboardPath, { bigint: true });
  const pointerStat = await lstat(pointerPath, { bigint: true });
  if (!keyboardStat.isCharacterDevice() || !pointerStat.isCharacterDevice() ||
      keyboardStat.rdev === pointerStat.rdev) {
    throw new Error('seat observer requires two distinct character devices');
  }
  const devices = Object.freeze({
    keyboard: Object.freeze({
      target: 'vusb-keyboard',
      rdev: keyboardStat.rdev.toString(),
    }),
    pointer: Object.freeze({
      target: 'vusb-pointer',
      rdev: pointerStat.rdev.toString(),
    }),
  });
  const hash = createHash('sha256');
  const state = {
    keyboardEvents: 0,
    pointerEvents: 0,
    syncEvents: 0,
    records: 0,
    keyboardTransitions: [],
    sequenceFailed: false,
  };
  await mkdir(dirname(destination), { recursive: true });

  let writing = Promise.resolve();
  const publish = () => {
    const receipt = {
      schemaVersion: 'humanymous.virtual-usb-seat-evidence/v1',
      runId,
      devices,
      imePolicyFileSha256,
      keyboardTransitions: state.keyboardTransitions.map(({ code, value }) => ({ code, value })),
      sequenceComplete: expectedKeyboardTransitions.length > 0 &&
        !state.sequenceFailed &&
        state.keyboardTransitions.length === expectedKeyboardTransitions.length,
      keyboardEvents: state.keyboardEvents,
      pointerEvents: state.pointerEvents,
      syncEvents: state.syncEvents,
      records: state.records,
      eventStreamSha256: hash.copy().digest('hex'),
    };
    writing = writing.then(async () => {
      const temporary = `${destination}.tmp-${process.pid}`;
      await writeFile(temporary, `${JSON.stringify(receipt)}\n`, {
        encoding: 'utf8',
        mode: 0o600,
      });
      await chown(temporary, 0, 12001);
      await chmod(temporary, 0o640);
      await rename(temporary, destination);
    });
  };

  function observe(path, device) {
    const stream = createReadStream(path, { highWaterMark: EVENT_BYTES * 32 });
    let pending = Buffer.alloc(0);
    stream.on('data', (chunk) => {
      pending = Buffer.concat([pending, chunk]);
      while (pending.length >= EVENT_BYTES) {
        const record = pending.subarray(0, EVENT_BYTES);
        pending = pending.subarray(EVENT_BYTES);
        const type = record.readUInt16LE(16);
        const code = record.readUInt16LE(18);
        const value = record.readInt32LE(20);
        if (![EV_SYN, EV_KEY, EV_REL].includes(type)) continue;
        hash.update(`${device}:${type}:${code}:${value}\n`);
        state.records += 1;
        if (type === EV_SYN) state.syncEvents += 1;
        if (device === 'keyboard' && type === EV_KEY) {
          state.keyboardEvents += 1;
          if (state.keyboardTransitions.length < 128) {
            state.keyboardTransitions.push({ code, value });
          }
          const index = state.keyboardTransitions.length - 1;
          const expected = expectedKeyboardTransitions[index];
          if (!expected || expected.code !== code || expected.value !== value ||
              value === 2) {
            state.sequenceFailed = true;
          }
        }
        if (device === 'pointer' && (type === EV_KEY || type === EV_REL)) {
          state.pointerEvents += 1;
        }
        publish();
      }
    });
    stream.on('error', (error) => {
      process.stderr.write(`seat observer ${device} failed: ${error.message}\n`);
      process.exitCode = 1;
    });
  }

  observe(keyboardPath, 'keyboard');
  observe(pointerPath, 'pointer');
  await new Promise(() => {});
}

main().catch((error) => {
  process.stderr.write(`seat observer failed: ${error.message}\n`);
  process.exitCode = 1;
});
