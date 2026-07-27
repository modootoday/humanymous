#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CATALOG_DIR="$ROOT/deployments/external-input/vusb"
ARTIFACT_ROOT="$ROOT/deployments/artifacts/external-input-vusb"
COMPOSE_BASE="$ROOT/deployments/compose.yaml"
COMPOSE_EXTERNAL="$ROOT/deployments/compose/external-input-bots.yaml"
COMPOSE_DOM="$ROOT/deployments/compose/external-input-dom.yaml"
COMPOSE_VUSB="$ROOT/deployments/compose/external-input-vusb.yaml"
COMPOSE_MANIFEST="$ROOT/deployments/compose/external-input-vusb-manifest.yaml"

if [[ $# -ne 2 || "$1" != "--model" || ! "$2" =~ ^[a-z][a-z0-9-]{2,63}$ ]]; then
  echo "usage: bash scripts/external-input-vusb-docker.sh --model <allowlisted-model-id>" >&2
  exit 2
fi
MODEL_ID="$2"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo '{"status":"UNAVAILABLE","capability":"native-linux-vm","reason":"canonical virtual USB requires a native Linux Docker host"}' >&2
  exit 3
fi
if [[ "$(id -u)" == "0" ]]; then
  echo '{"status":"UNAVAILABLE","capability":"non-root-operator","reason":"canonical supervisor must run as a non-root operator"}' >&2
  exit 3
fi
docker_os="$(docker info --format '{{.OperatingSystem}}')"
if [[ "$docker_os" == *"Docker Desktop"* ]]; then
  echo '{"status":"UNAVAILABLE","capability":"native-linux-vm","reason":"Docker Desktop is a comparison backend, not canonical authority"}' >&2
  exit 3
fi

ladder_id="vusb-$(date -u +%Y%m%d%H%M%S)-$$"
ladder_root="$ARTIFACT_ROOT/$ladder_id"
mkdir -p "$ladder_root"
chmod 0700 "$ladder_root"
mkdir -p "$ladder_root/summary"
chmod 0700 "$ladder_root/summary"
export HM_VUSB_LADDER_ID="$ladder_id"
export HM_VUSB_LADDER_MANIFEST_DIR="$ladder_root/manifest"
export HM_VUSB_LADDER_MANIFEST="$HM_VUSB_LADDER_MANIFEST_DIR/ladder.json"
mkdir -p "$HM_VUSB_LADDER_MANIFEST_DIR"
chmod 0700 "$HM_VUSB_LADDER_MANIFEST_DIR"
runtime_env="$ladder_root/runtime.env"
export HM_VUSB_CATALOG_DIR="$CATALOG_DIR"
export HM_VUSB_CATALOG_TRUST_ROOT="$ROOT/deployments/external-input/trust/catalog-ed25519.pub"
export HM_VUSB_LADDER_ROOT="$ladder_root"
export HM_VUSB_HOST_UID="$(id -u)"
lock_base="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export HM_VUSB_LOCK_DIR="$lock_base/humanymous-vusb"
mkdir -p "$HM_VUSB_LOCK_DIR"
chmod 0700 "$HM_VUSB_LOCK_DIR"
export HM_VUSB_MODEL_ID="$MODEL_ID"
export HM_VUSB_RUNTIME_ENV="$runtime_env"
if [[ "${HM_KERNEL_SINGLE_CELL:-0}" == "1" ]]; then
  [[ "${HM_KERNEL_GUEST:-0}" == "1" &&
      "${HM_VUSB_ADMISSION_AUTHORITY:-}" == "seed-bound-development" ]] || {
    echo '{"status":"FAIL","reason":"development seed authority is confined to the kernel guest"}' >&2
    exit 1
  }
  jq -er --arg model "$MODEL_ID" '
    [
      "HM_VUSB_MODEL_ID=" + $model,
      "HM_VUSB_CATALOG_SHA256=" + .seedContentSha256,
      "HM_VUSB_LAB_CORE_IMAGE_ID=" + .runtimeImages.labCore,
      "HM_VUSB_PKI_IMAGE_ID=" + .runtimeImages.pki,
      "HM_VUSB_DISPLAY_IMAGE_ID=" + .runtimeImages.display,
      (if .runtimeImages.browserChromium then
        "HM_VUSB_BROWSER_CHROMIUM_IMAGE_ID=" + .runtimeImages.browserChromium
      else empty end),
      (if .runtimeImages.browserChromiumDom then
        "HM_VUSB_BROWSER_CHROMIUM_DOM_IMAGE_ID=" + .runtimeImages.browserChromiumDom
      else empty end),
      (if .runtimeImages.browserChromiumIme then
        "HM_VUSB_BROWSER_CHROMIUM_IME_IMAGE_ID=" + .runtimeImages.browserChromiumIme
      else empty end),
      (if .runtimeImages.browserFirefox then
        "HM_VUSB_BROWSER_FIREFOX_IMAGE_ID=" + .runtimeImages.browserFirefox
      else empty end),
      (if .runtimeImages.browserFirefoxDom then
        "HM_VUSB_BROWSER_FIREFOX_DOM_IMAGE_ID=" + .runtimeImages.browserFirefoxDom
      else empty end),
      (if .runtimeImages.browserFirefoxIme then
        "HM_VUSB_BROWSER_FIREFOX_IME_IMAGE_ID=" + .runtimeImages.browserFirefoxIme
      else empty end),
      "HM_VUSB_CONTROLLER_IMAGE_ID=" + .runtimeImages.controller,
      "HM_VUSB_LIFECYCLE_IMAGE_ID=" + .runtimeImages.lifecycle,
      "HM_VUSB_GATEWAY_IMAGE_ID=" + .runtimeImages.gateway,
      "HM_VUSB_PROFILE_IMAGE_ID=" + .runtimeImages.profile
    ] | .[]
  ' /seed/seed.json > "$runtime_env"
  chmod 0600 "$runtime_env"
else
  node "$ROOT/test/externalinput/vusb/resolve-runtime.mjs"
fi
set -a
# File contents are strict NAME=sha256 lines emitted only after signature and
# digest validation; no caller-supplied value is evaluated.
source "$runtime_env"
set +a

profile_mount_alias="hmnvusbprofile:${ladder_id}"
profile_mount_id="$(
  docker image inspect --format '{{.Id}}' "$HM_VUSB_PROFILE_IMAGE_ID"
)"
[[ "$profile_mount_id" == "$HM_VUSB_PROFILE_IMAGE_ID" ]] || {
  echo '{"status":"FAIL","reason":"profile image identity differs before mount alias creation"}' >&2
  exit 1
}
docker image tag "$HM_VUSB_PROFILE_IMAGE_ID" "$profile_mount_alias"
profile_mount_alias_id="$(
  docker image inspect --format '{{.Id}}' "$profile_mount_alias"
)"
[[ "$profile_mount_alias_id" == "$HM_VUSB_PROFILE_IMAGE_ID" ]] || {
  echo '{"status":"FAIL","reason":"profile mount alias is not digest-bound"}' >&2
  exit 1
}
export HM_VUSB_PROFILE_MOUNT_IMAGE="$profile_mount_alias"
cleanup_profile_mount_alias() {
  docker image rm "$profile_mount_alias" >/dev/null 2>&1 || true
}
trap cleanup_profile_mount_alias EXIT

export HM_EXTERNAL_CORE_IMAGE="$HM_VUSB_LAB_CORE_IMAGE_ID"
export HM_EXTERNAL_PKI_IMAGE="$HM_VUSB_PKI_IMAGE_ID"
export HM_EXTERNAL_DISPLAY_IMAGE="$HM_VUSB_DISPLAY_IMAGE_ID"
export HM_EXTERNAL_CONTROLLER_IMAGE="$HM_VUSB_CONTROLLER_IMAGE_ID"
export HM_EXTERNAL_VUSB_LADDER=1

manifest_compose() {
  local project="$1"
  shift
  docker compose \
    --project-directory "$ROOT/deployments" \
    -f "$COMPOSE_MANIFEST" \
    -p "$project" "$@"
}

if [[ "${HM_KERNEL_SINGLE_CELL:-0}" != "1" ]]; then
  ladder_manifest_project="hmn-vusb-manifest-${ladder_id}"
  ladder_manifest_project="${ladder_manifest_project:0:63}"
  manifest_compose "$ladder_manifest_project" \
    run --rm --no-deps external-vusb-ladder-manifest
  manifest_compose "$ladder_manifest_project" down --volumes --remove-orphans
fi

child_files() {
  local dom="$1"
  local receipts="$2"
  local args=(
    --project-directory "$ROOT/deployments"
    -f "$COMPOSE_BASE"
    -f "$COMPOSE_EXTERNAL"
  )
  [[ "$dom" == "1" ]] && args+=(-f "$COMPOSE_DOM")
  [[ "${HM_EXTERNAL_BROWSER:-}" == "firefox" ]] &&
    args+=(-f "$ROOT/deployments/compose/external-input-firefox.yaml")
  args+=(-f "$COMPOSE_VUSB")
  [[ -r "$receipts/mapping/device-override.json" ]] &&
    args+=(-f "$receipts/mapping/device-override.json")
  printf '%s\n' "${args[@]}"
}

parent_compose() {
  local project="$1"
  shift
  docker compose \
    --project-directory "$ROOT/deployments" \
    -f "$COMPOSE_VUSB" \
    --profile vusb-parent \
    --profile vusb-parent-assert \
    -p "$project" "$@"
}

child_compose() {
  local project="$1"
  local dom="$2"
  local receipts="$3"
  shift 3
  mapfile -t files < <(child_files "$dom" "$receipts")
  docker compose "${files[@]}" --profile vusb-run -p "$project" "$@"
}

run_virtual_mode() {
  local engine="$1"
  local mode="$2"
  local sequence="$3"
  local dom="$4"
  local locale="${5:-}"
  local episode="m${sequence}"
  [[ -n "$locale" ]] && episode="ime-${locale%%-*}"
  local run_id="${ladder_id}-${engine:0:2}-${episode}"
  run_id="${run_id:0:48}"
  local child_project="hmn-ext-${run_id}-m${sequence}"
  child_project="${child_project:0:63}"
  local parent_project="hmn-vusb-parent-${run_id}"
  parent_project="${parent_project:0:63}"
  local receipts="$ladder_root/${engine}-${episode}"
  local receipt_stages=(
    manifest lock admission profile static-config static-guard preflight setup prepare
    mapping resolved-config resolved-guard attestation policy gateway broker run
    ime-observer provisional cleanup teardown terminal control-empty
  )
  mkdir -p "$receipts"
  chmod 0700 "$receipts"
  local stage
  for stage in "${receipt_stages[@]}"; do
    mkdir -p "$receipts/$stage"
    chmod 0700 "$receipts/$stage"
  done
  chmod 0770 "$receipts/gateway" "$receipts/broker"

  export HM_VUSB_RUN_ID="$run_id"
  export HM_EXTERNAL_RUN_ID="$run_id"
  export HM_VUSB_HOST_UID="$(id -u)"
  export HM_VUSB_CHILD_PROJECT="$child_project"
  export HM_VUSB_PARENT_PROJECT="$parent_project"
  export HM_VUSB_RECEIPT_DIR="$receipts"
  export HM_VUSB_DEVICE_OVERRIDE="$receipts/mapping/device-override.json"
  export HM_EXTERNAL_MODE="$mode"
  if [[ -n "$locale" ]]; then
    export HM_VUSB_AXIS=ime
  else
    export HM_VUSB_AXIS=control
  fi
  export HM_VUSB_SEQUENCE="$sequence"
  export HM_EXTERNAL_BROWSER="$engine"
  export HM_EXTERNAL_IME_LOCALE="$locale"
  export HM_EXTERNAL_COMPOSE_PROFILE="vusb-run"
  export HM_EXTERNAL_SKIP_DOWN=1
  export HM_EXTERNAL_ONLY_MODE="$mode"
  if [[ -n "$locale" ]]; then
    if [[ "$engine" == "chromium" ]]; then
      export HM_VUSB_BROWSER_IMAGE_ID="$HM_VUSB_BROWSER_CHROMIUM_IME_IMAGE_ID"
    else
      export HM_VUSB_BROWSER_IMAGE_ID="$HM_VUSB_BROWSER_FIREFOX_IME_IMAGE_ID"
    fi
  elif [[ "$engine" == "chromium" ]]; then
    if [[ "$dom" == "1" ]]; then
      export HM_VUSB_BROWSER_IMAGE_ID="$HM_VUSB_BROWSER_CHROMIUM_DOM_IMAGE_ID"
    else
      export HM_VUSB_BROWSER_IMAGE_ID="$HM_VUSB_BROWSER_CHROMIUM_IMAGE_ID"
    fi
  else
    if [[ "$dom" == "1" ]]; then
      export HM_VUSB_BROWSER_IMAGE_ID="$HM_VUSB_BROWSER_FIREFOX_DOM_IMAGE_ID"
    else
      export HM_VUSB_BROWSER_IMAGE_ID="$HM_VUSB_BROWSER_FIREFOX_IMAGE_ID"
    fi
  fi
  export HM_EXTERNAL_BROWSER_IMAGE="$HM_VUSB_BROWSER_IMAGE_ID"
  export HM_VUSB_ATTEMPT_MANIFEST_DIR="$receipts/manifest"
  if [[ "${HM_KERNEL_SINGLE_CELL:-0}" != "1" ]]; then
    local manifest_project="hmn-vusb-manifest-${run_id}"
    manifest_project="${manifest_project:0:63}"
    manifest_compose "$manifest_project" \
      run --rm --no-deps external-vusb-attempt-manifest
    manifest_compose "$manifest_project" down --volumes --remove-orphans
  fi
  local prepared=0
  local mutation_attempted=0
  local kernel_cleaned=0
  local child_started=0
  local terminal=0
  cleanup_failed_run() {
    local failure_code=$?
    local stop_code=0
    local cleanup_code=0
    local down_code=0
    trap - ERR INT TERM
    set +e
    if [[ "$child_started" == "1" ]]; then
      child_compose "$child_project" "$dom" "$receipts" \
        stop external-vusb-ime-framebuffer-observer external-vusb-broker \
        external-vusb-gateway external-browser \
        external-display core
      stop_code=$?
    fi
    if [[ "$prepared" == "1" && "$kernel_cleaned" != "1" ]]; then
      child_compose "$child_project" "$dom" "$receipts" \
        --profile vusb-cleanup run --rm --no-deps external-vusb-cleanup
      cleanup_code=$?
      [[ "$cleanup_code" == "0" ]] && kernel_cleaned=1
    fi
    if [[ "$cleanup_code" == "0" &&
          ( "$child_started" == "1" || "$prepared" == "1" ) ]]; then
      child_compose "$child_project" "$dom" "$receipts" \
        down --volumes --remove-orphans
      down_code=$?
    fi
    if [[ "$mutation_attempted" == "0" ]]; then
      parent_compose "$parent_project" down --volumes --remove-orphans
    else
      echo "{\"status\":\"SAFETY_ABORT\",\"reason\":\"attempt failed after kernel-mutation authority began\",\"quarantinedParentProject\":\"$parent_project\"}" >&2
    fi
    set -e
    if [[ "$terminal" != "1" || "$stop_code" != "0" ||
          "$cleanup_code" != "0" || "$down_code" != "0" ]]; then
      echo '{"status":"SAFETY_ABORT","reason":"virtual USB attempt did not reach parent-validated terminal cleanup"}' >&2
    fi
    if [[ "$stop_code" != "0" || "$cleanup_code" != "0" || "$down_code" != "0" ]]; then
      failure_code=1
    fi
    return "$failure_code"
  }
  trap cleanup_failed_run ERR INT TERM

  parent_compose "$parent_project" up -d external-vusb-lock
  if [[ "${HM_KERNEL_SINGLE_CELL:-0}" == "1" ]]; then
    export HM_KERNEL_SEED_PATH=/seed/seed.json
    export HM_KERNEL_SEED_ADMISSION="$receipts/admission/admission.json"
    export HM_KERNEL_PROFILE_MANIFEST="$ROOT/deployments/external-input/vusb/profile/virtual-usb-profile.json"
    node "$ROOT/test/externalinput/vusb/kernel-seed-admission.mjs"
  else
    child_compose "$child_project" "$dom" "$receipts" \
      --profile vusb-admission run --rm --no-deps external-vusb-admission
  fi
  child_compose "$child_project" "$dom" "$receipts" \
    --profile vusb-admission run --rm --no-deps external-vusb-profile-verify
  if [[ -n "$locale" ]]; then
    child_compose "$child_project" "$dom" "$receipts" \
      --profile vusb-ime-policy run --rm --no-deps external-vusb-ime-policy
  fi
  child_compose "$child_project" "$dom" "$receipts" \
    --profile '*' config --format json > "$receipts/static-config/compose-config.json"
  child_compose "$child_project" "$dom" "$receipts" \
    --profile vusb-prepare run --rm --no-deps external-vusb-static-compose-guard
  child_compose "$child_project" "$dom" "$receipts" \
    --profile vusb-prepare run --rm --no-deps external-vusb-preflight
  mutation_attempted=1
  child_compose "$child_project" "$dom" "$receipts" \
    --profile vusb-prepare run --rm --no-deps external-vusb-init
  prepared=1
  child_compose "$child_project" "$dom" "$receipts" \
    --profile vusb-prepare run --rm --no-deps external-vusb-discover
  child_compose "$child_project" "$dom" "$receipts" \
    --profile vusb-prepare run --rm --no-deps external-vusb-render
  child_compose "$child_project" "$dom" "$receipts" \
    --profile '*' config --format json > "$receipts/resolved-config/compose-config.json"
  child_compose "$child_project" "$dom" "$receipts" \
    --profile vusb-prepare run --rm --no-deps external-vusb-compose-guard
  child_compose "$child_project" "$dom" "$receipts" \
    --profile vusb-prepare run --rm --no-deps external-vusb-attestation

  child_started=1
  if [[ -n "$locale" ]]; then
    export HM_EXTERNAL_CONTROL_DIR="$receipts/control-empty"
    child_compose "$child_project" 0 "$receipts" \
      --profile vusb-ime-run up -d lab-pki core external-display external-browser \
      external-vusb-gateway external-vusb-broker external-vusb-ime-framebuffer-observer
    child_compose "$child_project" 0 "$receipts" \
      run --rm --no-deps --entrypoint node external-controller \
      /app/test/externalinput/vusb/ime-runner.mjs
    child_compose "$child_project" 0 "$receipts" \
      --profile vusb-ime-run wait external-vusb-ime-framebuffer-observer
    child_compose "$child_project" 0 "$receipts" \
      stop external-browser external-vusb-broker
    child_compose "$child_project" 0 "$receipts" \
      --profile vusb-ime-run-receipt run --rm --no-deps external-vusb-ime-run-receipt
  else
    node "$ROOT/scripts/external-input-smoke.mjs" \
      --browser "$engine" --no-build --run-id "$run_id" \
      --dom-required "$dom"
    child_compose "$child_project" "$dom" "$receipts" \
      --profile vusb-run-receipt run --rm --no-deps external-vusb-run-receipt
  fi
  if [[ "${HM_KERNEL_SINGLE_CELL:-0}" != "1" ]]; then
    parent_compose "$parent_project" run --rm --no-deps external-vusb-parent-provisional
  fi

  child_compose "$child_project" "$dom" "$receipts" \
    stop external-vusb-broker external-vusb-gateway external-browser external-display core
  child_compose "$child_project" "$dom" "$receipts" \
    --profile vusb-cleanup run --rm --no-deps external-vusb-cleanup
  kernel_cleaned=1
  set +e
  child_compose "$child_project" "$dom" "$receipts" \
    down --volumes --remove-orphans
  down_exit=$?
  set -e
  export HM_VUSB_DOWN_EXIT_CODE="$down_exit"
  export HM_VUSB_TEARDOWN_OBSERVATION="$receipts/teardown/teardown-observation.json"
  node "$ROOT/scripts/external-input-vusb-teardown.mjs"
  if [[ "${HM_KERNEL_SINGLE_CELL:-0}" == "1" ]]; then
    export HM_KERNEL_RESULT="$ROOT/deployments/artifacts/external-input/$run_id/$mode.result.json"
    export HM_KERNEL_SCORE="$ROOT/deployments/artifacts/external-input/$run_id/score-m${sequence}/$mode.score.json"
    export HM_KERNEL_ATTESTATION="$receipts/attestation/virtual-usb-attestation.json"
    export HM_KERNEL_CLEANUP="$receipts/cleanup/kernel-cleanup.json"
    export HM_KERNEL_TEARDOWN="$receipts/teardown/teardown-observation.json"
    export HM_KERNEL_COMPOSE_CONFIG="$receipts/resolved-config/compose-config.json"
    export HM_KERNEL_TERMINAL="$receipts/terminal/terminal.json"
    node "$ROOT/test/externalinput/vusb/kernel-seed-terminal.mjs"
    touch "$receipts/terminal/release-lock"
    parent_compose "$parent_project" wait external-vusb-lock
  else
    parent_compose "$parent_project" run --rm --no-deps external-vusb-parent-assert
    parent_compose "$parent_project" wait external-vusb-lock
  fi
  terminal=1
  parent_compose "$parent_project" down --volumes --remove-orphans
  if [[ -n "$locale" ]]; then
    cp "$ROOT/deployments/artifacts/external-input/$run_id/ime.result.json" \
      "$ladder_root/${engine}-${locale}.ime.result.json"
    cp "$receipts/terminal/terminal.json" "$ladder_root/${engine}-${locale}.ime.terminal.json"
  else
    cp "$ROOT/deployments/artifacts/external-input/$run_id/$mode.result.json" \
      "$ladder_root/${engine}-m${sequence}.result.json"
    cp "$receipts/terminal/terminal.json" "$ladder_root/${engine}-m${sequence}.terminal.json"
  fi
  unset HM_EXTERNAL_IME_LOCALE
  unset HM_VUSB_PARENT_PROJECT
  trap - ERR INT TERM
}

run_non_usb_mode() {
  local engine="$1"
  local mode="$2"
  local sequence="$3"
  local dom="$4"
  local run_id="${ladder_id}-${engine:0:2}-m${sequence}"
  run_id="${run_id:0:48}"
  local child_project="hmn-ext-${run_id}-m${sequence}"
  child_project="${child_project:0:63}"
  local receipts="$ladder_root/${engine}-m${sequence}"
  mkdir -p "$receipts/manifest"
  chmod 0700 "$receipts" "$receipts/manifest"
  export HM_VUSB_RUN_ID="$run_id"
  export HM_VUSB_CHILD_PROJECT="$child_project"
  export HM_VUSB_AXIS=control
  export HM_VUSB_SEQUENCE="$sequence"
  export HM_VUSB_ATTEMPT_MANIFEST_DIR="$receipts/manifest"
  export HM_EXTERNAL_BROWSER="$engine"
  export HM_EXTERNAL_MODE="$mode"
  export HM_EXTERNAL_ONLY_MODE="$mode"
  export HM_EXTERNAL_SKIP_DOWN=0
  unset HM_VUSB_DEVICE_OVERRIDE
  unset HM_VUSB_PARENT_PROJECT
  if [[ "$engine" == "chromium" ]]; then
    [[ "$dom" == "1" ]] &&
      export HM_EXTERNAL_BROWSER_IMAGE="$HM_VUSB_BROWSER_CHROMIUM_DOM_IMAGE_ID" ||
      export HM_EXTERNAL_BROWSER_IMAGE="$HM_VUSB_BROWSER_CHROMIUM_IMAGE_ID"
  else
    [[ "$dom" == "1" ]] &&
      export HM_EXTERNAL_BROWSER_IMAGE="$HM_VUSB_BROWSER_FIREFOX_DOM_IMAGE_ID" ||
      export HM_EXTERNAL_BROWSER_IMAGE="$HM_VUSB_BROWSER_FIREFOX_IMAGE_ID"
  fi
  local manifest_project="hmn-vusb-manifest-${run_id}"
  manifest_project="${manifest_project:0:63}"
  manifest_compose "$manifest_project" \
    run --rm --no-deps external-vusb-attempt-manifest
  manifest_compose "$manifest_project" down --volumes --remove-orphans
  node "$ROOT/scripts/external-input-smoke.mjs" \
    --browser "$engine" --no-build --run-id "$run_id" \
    --dom-required "$dom"
  cp "$ROOT/deployments/artifacts/external-input/$run_id/$mode.result.json" \
    "$ladder_root/${engine}-m${sequence}.result.json"
}

kernel_selected_cells=0
while IFS=$'\t' read -r engine sequence profile_id dom_required; do
  if [[ "${HM_KERNEL_SINGLE_CELL:-0}" == "1" ]]; then
    [[ "$engine" == "${HM_KERNEL_CELL_BROWSER:?set fixed kernel browser}" ]] ||
      continue
    [[ "$sequence" == "${HM_KERNEL_CELL_SEQUENCE:?set fixed kernel sequence}" ]] ||
      continue
    kernel_selected_cells=$((kernel_selected_cells + 1))
  fi
  if [[ "$sequence" -le 2 ]]; then
    run_non_usb_mode "$engine" "$profile_id" "$sequence" "$dom_required"
  else
    run_virtual_mode "$engine" "$profile_id" "$sequence" "$dom_required"
  fi
done < <(node "$ROOT/scripts/external-input-matrix.mjs" control)

if [[ "${HM_KERNEL_SINGLE_CELL:-0}" == "1" ]]; then
  if [[ "$kernel_selected_cells" -ne 1 ]]; then
    echo '{"status":"FAIL","reason":"kernel single-cell selector matched other than one cell"}' >&2
    exit 1
  fi
  exit 0
fi

while IFS=$'\t' read -r engine sequence profile_id locale; do
  run_virtual_mode "$engine" "$profile_id" "$sequence" 0 "$locale"
done < <(node "$ROOT/scripts/external-input-matrix.mjs" ime)

aggregate_project="hmn-vusb-aggregate-${ladder_id}"
aggregate_project="${aggregate_project:0:63}"
set +e
docker compose \
  --project-directory "$ROOT/deployments" \
  -f "$COMPOSE_VUSB" \
  --profile vusb-ladder-assert \
  -p "$aggregate_project" \
  run --rm --no-deps external-vusb-ladder-assert
aggregate_status="$?"
docker compose \
  --project-directory "$ROOT/deployments" \
  -f "$COMPOSE_VUSB" \
  --profile vusb-ladder-assert \
  -p "$aggregate_project" \
  down --volumes --remove-orphans
aggregate_down_status="$?"
set -e
if [[ "$aggregate_status" -ne 0 || "$aggregate_down_status" -ne 0 ]]; then
  exit 1
fi
