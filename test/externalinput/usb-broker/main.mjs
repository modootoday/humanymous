import { lstat, mkdir, open, readFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { Duplex } from 'node:stream';
import { UsbBroker } from './broker.mjs';
import { FirmwareChannel } from './firmware-channel.mjs';
import { validateHostAttestation } from './protocol.mjs';

const devicePath = process.env.HM_EXTERNAL_USB_COMMAND_DEVICE;
const socketPath = process.env.HM_EXTERNAL_USB_SOCKET;
const attestationPath = process.env.HM_EXTERNAL_USB_ATTESTATION;

function required(value, name) {
  if (!value) throw new Error(`${name} is required`);
  return value;
}

async function loadAttestation(path) {
  const stat = await lstat(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 8 * 1024) {
    throw new Error('USB attestation must be a small regular file');
  }
  return validateHostAttestation(JSON.parse(await readFile(path, 'utf8')));
}

async function openSerialTransport(path) {
  const stat = await lstat(path);
  if (!stat.isCharacterDevice()) {
    throw new Error('USB command interface is not a character device');
  }
  const handle = await open(path, 'r+');
  const readable = handle.createReadStream({ autoClose: false });
  const writable = handle.createWriteStream({ autoClose: false });
  const transport = Duplex.from({ readable, writable });
  transport.once('close', () => void handle.close().catch(() => {}));
  return transport;
}

async function run() {
  required(devicePath, 'HM_EXTERNAL_USB_COMMAND_DEVICE');
  required(socketPath, 'HM_EXTERNAL_USB_SOCKET');
  required(attestationPath, 'HM_EXTERNAL_USB_ATTESTATION');
  const attestation = await loadAttestation(attestationPath);
  const transport = await openSerialTransport(devicePath);
  const firmware = new FirmwareChannel(transport, attestation);
  await firmware.handshake();
  await mkdir(dirname(socketPath), { recursive: true });
  const broker = new UsbBroker({ socketPath, firmware });
  await broker.start();

  let stopping = false;
  const stop = async (signal) => {
    if (stopping) return;
    stopping = true;
    try {
      await broker.stop();
      process.exitCode = 0;
    } catch (error) {
      process.stderr.write(`USB broker ${signal} release failed: ${error.message}\n`);
      process.exitCode = 1;
    }
  };
  process.once('SIGINT', () => void stop('SIGINT'));
  process.once('SIGTERM', () => void stop('SIGTERM'));
}

run().catch((error) => {
  process.stderr.write(`USB broker startup failed: ${error.message}\n`);
  process.exitCode = 1;
});
