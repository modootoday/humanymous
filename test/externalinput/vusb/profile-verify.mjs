import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { atomicJson, receiptBase } from './common.mjs';
import { verifyProfileRoot } from './profile.mjs';
import { parseStrictJson } from './strict-json.mjs';

export async function verifyAdmittedProfile({
  runId,
  profileRoot,
  admissionReceiptPath,
  destination,
  now = new Date(),
}) {
  const admission = parseStrictJson(
    await readFile(admissionReceiptPath, 'utf8'),
    'admission receipt',
  );
  if (admission.kind !== 'admission' || admission.runId !== runId) {
    throw new TypeError('admission receipt is not bound to this run');
  }
  const verified = await verifyProfileRoot(profileRoot);
  if (verified.profile.modelId !== admission.modelId ||
      verified.profileManifestSha256 !== admission.profileManifestSha256) {
    throw new TypeError('mounted profile changed after admission');
  }
  const receipt = {
    ...receiptBase('profile-verification', runId, now),
    modelId: verified.profile.modelId,
    profileManifestSha256: verified.profileManifestSha256,
    entireRootfsValidated: true,
    gatewaySubpath: '/profile',
  };
  await atomicJson(destination, receipt);
  return receipt;
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  verifyAdmittedProfile({
    runId: required('HM_VUSB_RUN_ID'),
    profileRoot: required('HM_VUSB_PROFILE_ROOT'),
    admissionReceiptPath: required('HM_VUSB_ADMISSION_RECEIPT'),
    destination: required('HM_VUSB_PROFILE_VERIFICATION_RECEIPT'),
  }).catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-profile-verify',
      code: 'PURITY_FAIL',
      message: error.message,
    })}\n`);
    process.exitCode = 1;
  });
}
