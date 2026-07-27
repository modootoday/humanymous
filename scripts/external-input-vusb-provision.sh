#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export SOURCE_DATE_EPOCH=0

build() {
  local dockerfile="$1"
  local tag="$2"
  local target="${3:-}"
  local args=(
    buildx build --load
    -f "$dockerfile" -t "$tag"
  )
  [[ -n "$target" ]] && args+=(--target "$target")
  args+=(.)
  docker "${args[@]}"
}

build build/external-input-core.Dockerfile humanymous/external-input-core:vusb-local
build build/external-input-pki.Dockerfile humanymous/external-input-pki:vusb-local
build build/external-input-browser.Dockerfile humanymous/external-input-display:vusb-local display
build build/external-input-browser.Dockerfile humanymous/external-input-browser-chromium:vusb-local browser-chromium
build build/external-input-browser.Dockerfile humanymous/external-input-browser-chromium-dom:vusb-local browser-chromium-dom
build build/external-input-browser.Dockerfile humanymous/external-input-browser-chromium-ime:vusb-local browser-chromium-ime
build build/external-input-browser.Dockerfile humanymous/external-input-browser-firefox:vusb-local browser-firefox
build build/external-input-browser.Dockerfile humanymous/external-input-browser-firefox-dom:vusb-local browser-firefox-dom
build build/external-input-browser.Dockerfile humanymous/external-input-browser-firefox-ime:vusb-local browser-firefox-ime
build build/external-input-controller.Dockerfile humanymous/external-input-controller:vusb-local
build build/external-input-vusb-lifecycle.Dockerfile humanymous/external-input-vusb-lifecycle:vusb-local
build build/external-input-vusb-gateway.Dockerfile humanymous/external-input-vusb-gateway:vusb-local
build build/external-input-vusb-profile.Dockerfile humanymous/external-input-vusb-profile:reference-relative-v1

runtime_json="$ROOT/deployments/artifacts/external-input-vusb-runtime-images.json"
mkdir -p "$(dirname "$runtime_json")"
export HM_VUSB_RUNTIME_IMAGES_JSON="$runtime_json"
node scripts/external-input-vusb-images.mjs
printf 'preloaded runtime image lock: %s\n' "$runtime_json"

if [[ -n "${HM_VUSB_CATALOG_SIGNING_KEY:-}" ]]; then
  : "${HM_VUSB_ATTESTATION_BUNDLE_INPUT:?canonical signing requires a release attestation bundle}"
  if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
    echo "canonical catalog signing requires a clean source tree" >&2
    exit 2
  fi
  head_revision="$(git rev-parse HEAD)"
  if [[ -n "${HM_VUSB_SOURCE_REVISION:-}" &&
        "$HM_VUSB_SOURCE_REVISION" != "$head_revision" ]]; then
    echo "HM_VUSB_SOURCE_REVISION must equal the checked-out clean revision" >&2
    exit 2
  fi
  export HM_VUSB_SOURCE_REVISION="$head_revision"
  profile_json="$(docker image inspect humanymous/external-input-vusb-profile:reference-relative-v1 --format '{{json .}}')"
  export HM_VUSB_PROFILE_MANIFEST_DIGEST="$(
    node -e 'const x=JSON.parse(process.argv[1]);process.stdout.write(x.Descriptor.digest)' "$profile_json"
  )"
  export HM_VUSB_PROFILE_CONFIG_DIGEST="$(
    node -e 'const x=JSON.parse(process.argv[1]);process.stdout.write(x.Descriptor.annotations["config.digest"])' "$profile_json"
  )"
  node scripts/external-input-vusb-catalog.mjs
  printf 'signed profile catalog generated under deployments/external-input/vusb\n'
else
  printf 'catalog signing skipped: HM_VUSB_CATALOG_SIGNING_KEY is not set\n'
fi
