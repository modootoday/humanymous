import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { EXECUTION_KIND, modeFor } from './contracts.mjs';
import { queryDomSocket } from './dom-client.mjs';
import { CapabilityUnavailableError, ContractViolationError } from './errors.mjs';
import { createActionFirewall } from './firewall.mjs';
import {
  createEmulatedUsbInputAdapter,
  createUsbInputAdapter,
  createVirtualInputAdapter,
} from './input.mjs';
import {
  createDomFramebufferObservation,
  createFramebufferObservation,
} from './observation.mjs';
import { connectRfb } from './rfb-client.mjs';
import { sendUsbBrokerCommand } from './usb-client.mjs';
import {
  classifyFramebufferVerdict,
  locateTokenMarkers,
  validatePixelManifest,
} from './pixel-locator.mjs';
import {
  validateResetProof,
  validateRuntimeEvidence,
} from './runtime-purity.mjs';

async function readJson(path, label) {
  if (!path) throw new ContractViolationError(`${label} path is missing`);
  try {
    const raw = await readFile(resolve(path));
    if (raw.length > 512 * 1024) {
      throw new TypeError(`${label} exceeds its byte bound`);
    }
    return JSON.parse(raw.toString('utf8'));
  } catch (error) {
    throw new ContractViolationError(`${label} is unreadable: ${error.message}`);
  }
}

async function readText(path, label) {
  if (!path) throw new ContractViolationError(`${label} path is missing`);
  try {
    const raw = await readFile(resolve(path));
    if (raw.length > 512 * 1024) {
      throw new TypeError(`${label} exceeds its byte bound`);
    }
    return raw.toString('utf8');
  } catch (error) {
    throw new ContractViolationError(`${label} is unreadable: ${error.message}`);
  }
}

function usbCommandSender(path) {
  if (!path) throw new CapabilityUnavailableError('usb-command', 'USB command path is missing');
  return (action) => sendUsbBrokerCommand(path, action);
}

function delay(delayMs) {
  return new Promise((resolve) => setTimeout(resolve, delayMs));
}

export async function createEnvironmentContext(env = process.env) {
  const mode = modeFor(env.HM_EXTERNAL_MODE);
  const engine = env.HM_EXTERNAL_BROWSER || 'chromium';
  const visualManifestRaw = await readText(
    env.HM_EXTERNAL_VISUAL_MANIFEST,
    'visual manifest',
  );
  let manifest;
  try {
    manifest = validatePixelManifest(JSON.parse(visualManifestRaw));
  } catch (error) {
    throw new ContractViolationError(`visual manifest is invalid: ${error.message}`);
  }
  const resetProof = validateResetProof(
    await readJson(env.HM_EXTERNAL_RESET_PROOF, 'reset proof'),
    {
      runId: env.HM_EXTERNAL_RUN_ID,
      profileId: mode.profileId,
      composeProject: env.HM_EXTERNAL_COMPOSE_PROJECT,
    },
  );
  const purity = await readJson(env.HM_EXTERNAL_PURITY_PATH, 'purity proof');
  const runtimeEvidence = validateRuntimeEvidence(
    await readJson(env.HM_EXTERNAL_RUNTIME_EVIDENCE, 'runtime evidence'),
    {
      runId: env.HM_EXTERNAL_RUN_ID,
      profileId: mode.profileId,
      engine,
    },
  );
  const visualManifestSha256 =
    `sha256:${createHash('sha256').update(visualManifestRaw).digest('hex')}`;
  if (runtimeEvidence.composeConfigSha256 !== resetProof.composeConfigSha256 ||
      visualManifestSha256 !== resetProof.visualManifestSha256) {
    throw new ContractViolationError(
      'runtime, reset, and visual evidence hashes are not bound',
    );
  }
  const firewall = createActionFirewall({
    width: Number(env.HM_EXTERNAL_WIDTH || 1280),
    height: Number(env.HM_EXTERNAL_HEIGHT || 720),
  });

  return {
    executionKind: EXECUTION_KIND,
    engine,
    runId: env.HM_EXTERNAL_RUN_ID,
    seed: env.HM_EXTERNAL_SEED,
    strategyVariant: env.HM_EXTERNAL_STRATEGY_VARIANT || 'mixed-input',
    selectedMode: mode,
    async reset({ mode: requestedMode }) {
      if (requestedMode.profileId !== mode.profileId) {
        throw new ContractViolationError('one-mode invocation cannot execute another profile');
      }
      return resetProof;
    },
    async createAdapters({ mode: requestedMode }) {
      let usbAttestation;
      if (requestedMode.usbRequired) {
        usbAttestation = await readJson(
          env.HM_EXTERNAL_USB_ATTESTATION,
          'USB attestation',
        ).catch((error) => {
          throw new CapabilityUnavailableError('usb-attestation', error.message);
        });
      }
      let rfb;
      try {
        rfb = await connectRfb({
          host: env.HM_EXTERNAL_RFB_HOST,
          port: Number(env.HM_EXTERNAL_RFB_PORT || 5900),
          passwordFile: env.HM_EXTERNAL_RFB_PASSWORD_FILE,
          allowInput: requestedMode.inputBackend === 'rfb-xtest',
        });
      } catch (error) {
        if (error instanceof CapabilityUnavailableError) throw error;
        throw new CapabilityUnavailableError('rfb', `RFB connection failed: ${error.message}`);
      }
      try {
        let latestFrameHash = '';
        const readFrame = async () => {
          const frame = await rfb.capture();
          latestFrameHash = createHash('sha256').update(frame.pixels).digest('hex');
          return { ...frame, sha256: latestFrameHash, cues: [] };
        };
        const locateVisualTargets = ({ frame, targetTokens }) =>
          locateTokenMarkers(frame, manifest, targetTokens);
        const observationOptions = { readFrame, locateVisualTargets };
        const observation = requestedMode.domRequired
          ? createDomFramebufferObservation({
              ...observationOptions,
              queryDom: (request) => queryDomSocket(env.HM_EXTERNAL_DOM_SOCKET, request),
              domManifest: {
                extensionSha256: runtimeEvidence.dom.extensionSha256,
                manifestSha256: runtimeEvidence.dom.manifestSha256,
              },
            })
          : createFramebufferObservation(observationOptions);

        let input;
        if (requestedMode.inputBackend === 'usb-hid') {
          const send = usbCommandSender(env.HM_EXTERNAL_USB_COMMAND_PATH);
          input = createUsbInputAdapter({
            firewall,
            usbAttestation,
            rfbInputEnabled: false,
            xtestEnabled: false,
            send,
            release: send,
          });
        } else if (requestedMode.inputBackend === 'usb-hid-emulated') {
          const send = usbCommandSender(env.HM_EXTERNAL_USB_COMMAND_PATH);
          input = createEmulatedUsbInputAdapter({
            firewall,
            usbAttestation,
            rfbInputEnabled: false,
            xtestEnabled: false,
            send,
            release: send,
          });
        } else {
          input = createVirtualInputAdapter({
            firewall,
            send: (action) => rfb.sendAction(action),
            release: () => rfb.releaseAll(),
          });
        }
        return {
          observation,
          input,
          rfb,
          latestFrameHash: () => latestFrameHash,
          browser: {
            version: runtimeEvidence.browser.version,
            binarySha256: runtimeEvidence.browser.binarySha256,
            sandbox: runtimeEvidence.browser.sandbox,
          },
          cleanup: () => rfb.close(),
        };
      } catch (error) {
        rfb.close();
        throw error;
      }
    },
    async inspectPurity() {
      return purity;
    },
    async classifyFramebuffer({ adapters }) {
      const timeoutMs = Math.min(
        60_000,
        Math.max(1_000, Number(env.HM_EXTERNAL_VERDICT_TIMEOUT_MS || 30_000)),
      );
      const deadline = Date.now() + timeoutMs;
      let frameCount = 0;
      let measured;
      do {
        const frame = await adapters.rfb.capture();
        frameCount += 1;
        const classified = classifyFramebufferVerdict(frame, manifest);
        measured = {
          verdict: classified.verdict,
          source: 'framebuffer',
          confidence: classified.confidence,
          cueCount: classified.cueCount,
          finalFrameSha256: createHash('sha256').update(frame.pixels).digest('hex'),
          frameCount,
        };
        if (classified.verdict !== 'NO_RESPONSE' || Date.now() >= deadline) return measured;
        await delay(250);
      } while (true);
    },
  };
}
