#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

die() {
  printf 'kernel guest: %s\n' "$1" >&2
  exit 1
}

if [[ $# -ne 2 || "$1" != "--seed" || "$2" != "/seed/seed.json" ]]; then
  die "fixed invocation is: --seed /seed/seed.json"
fi

seed="$2"
[[ -r "$seed" ]] || die "seed is not readable"
[[ "$(id -u)" -ne 0 ]] || die "measurement supervisor must not run as root"
[[ "${HM_KERNEL_GUEST:-}" == "1" ]] || die "kernel guest authority marker missing"

jq -e '
  (keys == [
    "admission", "budgets", "guestCommand", "imageArchive", "innerCompose",
    "modelId", "platform", "runId", "runNonce", "runner",
    "runtimeImages", "schemaVersion", "seedContentSha256", "sourceBundle",
    "strategy"
  ]) and
  (.schemaVersion | type == "string") and
  (.runId | test("^[a-z][a-z0-9-]{2,63}$")) and
  (.runNonce | test("^[a-f0-9]{64}$")) and
  (.modelId | test("^[a-z][a-z0-9-]{2,63}$")) and
  (.platform == "linux/amd64") and
  (.innerCompose.browser == "chromium" or .innerCompose.browser == "firefox") and
  (.innerCompose.sequence == 3 or .innerCompose.sequence == 4) and
  (.innerCompose.profileId | type == "string") and
  (.innerCompose.domRequired == (.innerCompose.sequence == 4)) and
  (.innerCompose.pullPolicy == "never") and
  (.strategy.variant == "mixed-input") and
  (.strategy.seed | type == "string") and
  (.strategy.oracleFeedback == false) and
  (.imageArchive.path == "images.oci.tar") and
  (.imageArchive.imageKeys == (
    ["labCore", "pki", "display"] +
    [(
      "browser" +
      (if .innerCompose.browser == "chromium" then "Chromium" else "Firefox" end) +
      (if .innerCompose.sequence == 4 then "Dom" else "" end)
    )] +
    ["controller", "lifecycle", "gateway", "profile"]
  )) and
  (.sourceBundle.path == "bundle.tar")
' "$seed" >/dev/null || die "seed policy or identity is invalid"

seed_sha="sha256:$(sha256sum "$seed" | awk '{print $1}')"
images_sha="sha256:$(sha256sum /seed/images.oci.tar | awk '{print $1}')"
bundle_sha="sha256:$(sha256sum /seed/bundle.tar | awk '{print $1}')"
[[ "$images_sha" == "$(jq -er '.imageArchive.sha256' "$seed")" ]] ||
  die "image archive digest mismatch"
[[ "$bundle_sha" == "$(jq -er '.sourceBundle.sha256' "$seed")" ]] ||
  die "source bundle digest mismatch"
[[ "$(stat -c %s /seed/images.oci.tar)" == "$(jq -er '.imageArchive.bytes' "$seed")" ]] ||
  die "image archive size mismatch"
[[ "$(stat -c %s /seed/bundle.tar)" == "$(jq -er '.sourceBundle.bytes' "$seed")" ]] ||
  die "source bundle size mismatch"

docker load --input /seed/images.oci.tar >/output/docker-load.log

mapfile -t expected_images < <(
  jq -er '. as $seed |
    .imageArchive.imageKeys[] |
    $seed.runtimeImages[.]' "$seed" |
    sort -u
)
expected_ids="$(mktemp)"
actual_ids="$(mktemp)"
trap 'rm -f "$expected_ids" "$actual_ids"' EXIT
for expected in "${expected_images[@]}"; do
  docker image inspect "$expected" >/dev/null ||
    die "expected image is absent after offline load: $expected"
  docker image inspect --format '{{.Id}}' "$expected" >> "$expected_ids"
done
sort -u -o "$expected_ids" "$expected_ids"
docker image ls --all --no-trunc --quiet | sort -u > "$actual_ids"
cmp -s "$expected_ids" "$actual_ids" ||
  die "offline load produced a missing or unallowlisted image identity"

run_id="$(jq -er '.runId' "$seed")"
model_id="$(jq -er '.modelId' "$seed")"
sequence="$(jq -er '.innerCompose.sequence' "$seed")"
browser="$(jq -er '.innerCompose.browser' "$seed")"
profile_id="$(jq -er '.innerCompose.profileId' "$seed")"
strategy_variant="$(jq -er '.strategy.variant' "$seed")"
strategy_seed="$(jq -er '.strategy.seed' "$seed")"

export HM_KERNEL_SINGLE_CELL=1
export HM_KERNEL_CELL_BROWSER="$browser"
export HM_KERNEL_CELL_SEQUENCE="$sequence"
export HM_VUSB_ADMISSION_AUTHORITY=seed-bound-development
export HM_EXTERNAL_STRATEGY_VARIANT="$strategy_variant"
export HM_EXTERNAL_STRATEGY_SEED="$strategy_seed"
export HM_EXTERNAL_STRATEGY_ORACLE_FEEDBACK=0
export HM_KERNEL_SEED_RUN_ID="$run_id"
export HM_KERNEL_SEED_FILE_SHA256="${seed_sha#sha256:}"

bash "$ROOT/scripts/external-input-vusb-docker.sh" --model "$model_id"

ladder_root="$(
  find "$ROOT/deployments/artifacts/external-input-vusb" -mindepth 1 -maxdepth 1 \
    -type d -name 'vusb-*' -printf '%T@ %p\n' |
    sort -nr | head -n 1 | cut -d' ' -f2-
)"
[[ -n "$ladder_root" ]] || die "measurement ladder output missing"
result="$ladder_root/${browser}-m${sequence}.result.json"
terminal="$ladder_root/${browser}-m${sequence}.terminal.json"
[[ -s "$result" && -s "$terminal" ]] || die "measurement receipts missing"
measurement_run_id="$(jq -er '.runId' "$result")"
score="$ROOT/deployments/artifacts/external-input/$measurement_run_id/score-m${sequence}/$profile_id.score.json"
[[ -s "$score" ]] || die "Core score receipt missing"

compose_config="$(
  find "$ladder_root" -path '*/resolved-config/compose-config.json' -type f |
    head -n 1
)"
[[ -s "$compose_config" ]] || die "resolved compose evidence missing"

guest_boot_id="$(cat /proc/sys/kernel/random/boot_id)"
kernel_release="$(uname -r)"
result_sha="sha256:$(sha256sum "$result" | awk '{print $1}')"
terminal_sha="sha256:$(sha256sum "$terminal" | awk '{print $1}')"
score_sha="sha256:$(sha256sum "$score" | awk '{print $1}')"
browser_wasm_sha="sha256:$(jq -er '.detectorWasmSha256' "$score")"
framebuffer_verdict="$(jq -er '.measurement.verdict' "$result")"
usb_topology_sha="$(jq -er '.evidence.virtualUsbAttestationSha256' "$terminal")"
teardown_residue_sha="$(jq -er '.evidence.teardownObservationSha256' "$terminal")"
compose_sha="sha256:$(sha256sum "$compose_config" | awk '{print $1}')"

jq -n \
  --arg seedSha256 "$seed_sha" \
  --arg runId "$run_id" \
  --arg runNonce "$(jq -er '.runNonce' "$seed")" \
  --arg modelId "$model_id" \
  --arg strategyVariant "$strategy_variant" \
  --arg strategySeed "$strategy_seed" \
  --argjson sequence "$sequence" \
  --arg browser "$browser" \
  --arg profileId "$profile_id" \
  --arg guestBootId "$guest_boot_id" \
  --arg kernelRelease "$kernel_release" \
  --arg imageArchiveSha256 "$images_sha" \
  --arg sourceBundleSha256 "$bundle_sha" \
  --arg composeConfigSha256 "$compose_sha" \
  --arg resultSha256 "$result_sha" \
  --arg terminalSha256 "$terminal_sha" \
  --arg scoreReceiptSha256 "$score_sha" \
  --arg browserWasmSha256 "$browser_wasm_sha" \
  --arg framebufferVerdict "$framebuffer_verdict" \
  --arg usbTopologyReceiptSha256 "$usb_topology_sha" \
  --arg teardownResidueSha256 "$teardown_residue_sha" \
  '{
    schemaVersion: "humanymous.kernel-guest-terminal/v1",
    kind: "kernel-guest-measurement",
    status: "PASS",
    runId: $runId,
    runNonce: $runNonce,
    modelId: $modelId,
    authority: "seed-bound-development",
    canonical: false,
    baselineEligible: false,
    releaseAttested: false,
    evidenceClass: "kernel-emulated-usb",
    physicalUsb: false,
    seedSha256: $seedSha256,
    imageArchiveSha256: $imageArchiveSha256,
    sourceBundleSha256: $sourceBundleSha256,
    cell: {
      browser: $browser,
      sequence: $sequence,
      profileId: $profileId
    },
    strategy: {
      variant: $strategyVariant,
      seed: $strategySeed,
      oracleFeedback: false
    },
    guestBootId: $guestBootId,
    kernelRelease: $kernelRelease,
    composeConfigSha256: $composeConfigSha256,
    resultSha256: $resultSha256,
    terminalSha256: $terminalSha256,
    scoreReceiptSha256: $scoreReceiptSha256,
    browserWasmSha256: $browserWasmSha256,
    framebufferVerdict: $framebufferVerdict,
    usbTopologyReceiptSha256: $usbTopologyReceiptSha256,
    teardownResidueSha256: $teardownResidueSha256
  }' > /output/guest-terminal.json

cp "$result" /output/measurement-result.json
cp "$terminal" /output/measurement-terminal.json
cp "$score" /output/measurement-score.json
