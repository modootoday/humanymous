#!/bin/sh
set -eu

: "${HM_EXTERNAL_MODE:?HM_EXTERNAL_MODE is required}"
: "${HM_EXTERNAL_BROWSER:?HM_EXTERNAL_BROWSER is required}"
: "${HM_EXTERNAL_RFB_HOST:?HM_EXTERNAL_RFB_HOST is required}"
: "${HM_EXTERNAL_VISUAL_MANIFEST:?HM_EXTERNAL_VISUAL_MANIFEST is required}"
: "${HM_EXTERNAL_RESET_PROOF:?HM_EXTERNAL_RESET_PROOF is required}"
: "${HM_EXTERNAL_PURITY_PATH:?HM_EXTERNAL_PURITY_PATH is required}"

case "$HM_EXTERNAL_MODE" in
  external_input_virtual|external_input_dom_virtual) ;;
  external_input_usb|external_input_dom_usb|external_input_vusb|external_input_dom_vusb)
    : "${HM_EXTERNAL_USB_COMMAND_PATH:?USB modes require HM_EXTERNAL_USB_COMMAND_PATH}"
    : "${HM_EXTERNAL_USB_ATTESTATION:?USB modes require HM_EXTERNAL_USB_ATTESTATION}"
    ;;
  *)
    echo "invalid HM_EXTERNAL_MODE" >&2
    exit 2
    ;;
esac

case "$HM_EXTERNAL_MODE" in
  external_input_dom_virtual|external_input_dom_usb|external_input_dom_vusb)
    : "${HM_EXTERNAL_DOM_SOCKET:?DOM modes require HM_EXTERNAL_DOM_SOCKET}"
    ;;
esac

exec node /app/test/e2e/external-input-runner.mjs
