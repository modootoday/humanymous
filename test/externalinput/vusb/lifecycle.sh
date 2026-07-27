#!/bin/sh
set -eu

ACTION="${1:-}"
RUN_ID="${HM_VUSB_RUN_ID:-}"
GADGET="humanymous-${RUN_ID}"
CONFIGFS_ROOT="${HM_VUSB_CONFIGFS_ROOT:-/sys/kernel/config}"
GADGET_ROOT="${CONFIGFS_ROOT}/usb_gadget/${GADGET}"
RECEIPT_ROOT="${HM_VUSB_RECEIPT_ROOT:-/receipts}"
INPUT_ROOT="${HM_VUSB_INPUT_ROOT:-$RECEIPT_ROOT}"
LOCK_FILE="${HM_VUSB_LOCK_FILE:-/run/lock/humanymous-vusb.lock}"
OUTPUT_UID="${HM_VUSB_OUTPUT_UID:-}"

log() {
  level="$1"
  code="$2"
  message="$3"
  printf '{"level":"%s","component":"external-vusb-lifecycle","code":"%s","message":"%s"}\n' \
    "$level" "$code" "$message"
}

fail() {
  log error "${2:-SAFETY_ABORT}" "$1"
  exit "${3:-1}"
}

validate_run_id() {
  case "$RUN_ID" in
    ""|*[!a-z0-9-]*) fail "HM_VUSB_RUN_ID is invalid" "UNAVAILABLE" 3 ;;
  esac
  [ "${#RUN_ID}" -ge 6 ] && [ "${#RUN_ID}" -le 64 ] ||
    fail "HM_VUSB_RUN_ID length is invalid" "UNAVAILABLE" 3
}

require_linux() {
  [ "$(uname -s)" = "Linux" ] || fail "native Linux is required" "UNAVAILABLE" 3
  [ -r /proc/modules ] || fail "module inventory is unavailable" "UNAVAILABLE" 3
  mkdir -p "$RECEIPT_ROOT"
}

require_linux_root() {
  require_linux
  [ "$(id -u)" = "0" ] || fail "lifecycle mutation requires root" "UNAVAILABLE" 3
}

write_receipt() {
  kind="$1"
  body="$2"
  destination="${RECEIPT_ROOT}/${kind}.json"
  temporary="${destination}.tmp.$$"
  printf '{"schemaVersion":"humanymous.virtual-usb-receipt/v1","kind":"%s","runId":"%s","recordedAt":"%s"%s}\n' \
    "$kind" "$RUN_ID" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$body" > "$temporary"
  chmod 0600 "$temporary"
  if [ -n "$OUTPUT_UID" ]; then
    case "$OUTPUT_UID" in *[!0-9]*) fail "HM_VUSB_OUTPUT_UID is invalid" ;; esac
    chown "$OUTPUT_UID" "$temporary"
  fi
  ln "$temporary" "$destination" ||
    fail "${kind} receipt already exists or cannot be published"
  rm "$temporary"
}

publish_file() {
  source="$1"
  destination="$2"
  temporary="${destination}.tmp.$$"
  cp "$source" "$temporary"
  chmod 0600 "$temporary"
  if [ -n "$OUTPUT_UID" ]; then chown "$OUTPUT_UID" "$temporary"; fi
  ln "$temporary" "$destination" ||
    fail "$(basename "$destination") already exists or cannot be published"
  rm "$temporary"
}

module_inventory() {
  awk '{print $1}' /proc/modules | LC_ALL=C sort
}

preflight() {
  require_linux
  [ -d /lib/modules/"$(uname -r)" ] ||
    fail "modules for the running kernel are unavailable" "UNAVAILABLE" 3
  command -v modprobe >/dev/null 2>&1 || fail "modprobe is unavailable" "UNAVAILABLE" 3
  command -v flock >/dev/null 2>&1 || fail "flock is unavailable" "UNAVAILABLE" 3
  [ -e /dev/uinput ] && fail "uinput must be absent from the virtual USB lifecycle service" "PURITY_FAIL" 1
  config_path="/boot/config-$(uname -r)"
  [ -r "$config_path" ] || config_path="/proc/config.gz"
  [ -r "$config_path" ] || fail "running-kernel configuration is unavailable" "UNAVAILABLE" 3
  kernel_config_sha="$(sha256sum "$config_path" | awk '{print $1}')"
  grep -Eq '(^| )dummy_hcd( |$)' /proc/modules && already_dummy=true || already_dummy=false
  before_tmp="/tmp/modules.before.$$"
  module_inventory > "$before_tmp"
  modules_sha="$(sha256sum "$before_tmp" | awk '{print $1}')"
  publish_file "$before_tmp" "${RECEIPT_ROOT}/modules.before"
  rm "$before_tmp"
  write_receipt preflight ",\"kernelRelease\":\"$(uname -r)\",\"kernelConfigSha256\":\"sha256:${kernel_config_sha}\",\"loadedModulesSha256\":\"sha256:${modules_sha}\",\"dummyHcdPreloaded\":${already_dummy},\"uinputPresent\":false"
}

write_keyboard_descriptor() {
  # Fixed boot keyboard descriptor, report length 8.
  printf '\005\001\011\006\241\001\005\007\031\340\051\347\025\000\045\001\165\001\225\010\201\002\225\001\165\010\201\001\225\005\165\001\005\010\031\001\051\005\221\002\225\001\165\003\221\001\225\006\165\010\025\000\045\145\005\007\031\000\051\145\201\000\300'
}

write_pointer_descriptor() {
  # Three buttons plus bounded relative X/Y and vertical wheel, report length 4.
  printf '\005\001\011\002\241\001\011\001\241\000\005\011\031\001\051\003\025\000\045\001\225\003\165\001\201\002\225\001\165\005\201\001\005\001\011\060\011\061\011\070\025\201\045\177\165\010\225\003\201\006\300\300'
}

sysfs_for_node() {
  node="$1"
  device_hex="$(stat -Lc '%t:%T' "$node")"
  major_hex="${device_hex%:*}"
  minor_hex="${device_hex#*:}"
  major_dec="$(printf '%d' "0x${major_hex}")"
  minor_dec="$(printf '%d' "0x${minor_hex}")"
  readlink -f "/sys/dev/char/${major_dec}:${minor_dec}"
}

node_belongs_to_run() {
  node="$1"
  current="$(sysfs_for_node "$node")" || return 1
  depth=0
  while [ "$depth" -lt 12 ]; do
    if [ -r "$current/serial" ] &&
       [ "$(tr -d '\r\n' < "$current/serial")" = "$RUN_ID" ]; then
      return 0
    fi
    parent="$(readlink -f "$current/..")"
    [ "$parent" != "$current" ] || break
    current="$parent"
    depth=$((depth + 1))
  done
  return 1
}

find_run_link() {
  directory="$1"
  suffix="$2"
  FOUND_LINK=""
  for candidate in "$directory"/*; do
    [ -e "$candidate" ] || continue
    case "$(basename "$candidate")" in
      *"$suffix") ;;
      *) continue ;;
    esac
    target="$(readlink -f "$candidate")"
    [ -c "$target" ] || continue
    node_belongs_to_run "$target" || continue
    [ -z "$FOUND_LINK" ] || fail "multiple run-owned device links are ambiguous"
    FOUND_LINK="$target"
  done
  [ -n "$FOUND_LINK" ]
}

find_run_gadget_node() {
  kind="$1"
  FOUND_NODE=""
  case "$kind" in
    command) set -- /dev/ttyGS* ;;
    keyboard|pointer) set -- /dev/hidg* ;;
    *) fail "gadget node kind is invalid" ;;
  esac
  for candidate in "$@"; do
    [ -c "$candidate" ] || continue
    if [ "$kind" = "command" ]; then
      port_num="$(cat "$GADGET_ROOT/functions/acm.usb0/port_num" 2>/dev/null || true)"
      case "$port_num" in ""|*[!0-9]*) continue ;; esac
      [ "$(basename "$candidate")" = "ttyGS${port_num}" ] || continue
    else
      device_hex="$(stat -Lc '%t:%T' "$candidate")"
      candidate_device="$(printf '%d:%d' \
        "0x${device_hex%:*}" "0x${device_hex#*:}")"
      expected_device="$(cat "$GADGET_ROOT/functions/hid.${kind}/dev" 2>/dev/null || true)"
      [ "$candidate_device" = "$expected_device" ] || continue
    fi
    [ -z "$FOUND_NODE" ] || fail "multiple run-owned ${kind} gadget nodes are ambiguous"
    FOUND_NODE="$candidate"
  done
  [ -n "$FOUND_NODE" ]
}

module_was_preloaded() {
  grep -qx "$1" "${INPUT_ROOT}/preflight/modules.before"
}

unload_run_modules() {
  for module in usb_f_hid usb_f_acm u_serial libcomposite dummy_hcd udc_core configfs; do
    if grep -Eq "^${module} " /proc/modules && ! module_was_preloaded "$module"; then
      modprobe -r "$module" || return 1
    fi
  done
  after_tmp="/tmp/modules.after.$$"
  module_inventory > "$after_tmp"
  if ! cmp -s "${INPUT_ROOT}/preflight/modules.before" "$after_tmp"; then
    rm "$after_tmp"
    return 1
  fi
  rm "$after_tmp"
}

unbind_udc() {
  [ -e "$GADGET_ROOT/UDC" ] || return 0
  # configfs requires an actual write. A zero-byte printf leaves the gadget
  # bound; a newline is the shell equivalent of `echo "" > UDC`.
  printf '\n' > "$GADGET_ROOT/UDC" || return 1
  attempts=0
  while [ "$attempts" -lt 40 ]; do
    [ -z "$(tr -d '\r\n' < "$GADGET_ROOT/UDC")" ] && return 0
    attempts=$((attempts + 1))
    sleep 0.05
  done
  return 1
}

rollback_partial_setup() {
  rollback_ok=1
  if [ -d "$GADGET_ROOT" ]; then
    if [ -e "$GADGET_ROOT/UDC" ]; then
      unbind_udc 2>/dev/null || rollback_ok=0
    fi
    rm -f \
      "$GADGET_ROOT/configs/c.1/acm.usb0" \
      "$GADGET_ROOT/configs/c.1/hid.keyboard" \
      "$GADGET_ROOT/configs/c.1/hid.pointer"
    rmdir "$GADGET_ROOT/functions/acm.usb0" 2>/dev/null || true
    rmdir "$GADGET_ROOT/functions/hid.keyboard" 2>/dev/null || true
    rmdir "$GADGET_ROOT/functions/hid.pointer" 2>/dev/null || true
    rmdir "$GADGET_ROOT/configs/c.1/strings/0x409" 2>/dev/null || true
    rmdir "$GADGET_ROOT/configs/c.1" 2>/dev/null || true
    rmdir "$GADGET_ROOT/strings/0x409" 2>/dev/null || true
    rmdir "$GADGET_ROOT" 2>/dev/null || true
    [ ! -e "$GADGET_ROOT" ] || rollback_ok=0
  fi
  unload_run_modules || rollback_ok=0
  if [ "$rollback_ok" != "1" ]; then
    log error SAFETY_ABORT "partial virtual USB setup residue requires operator intervention"
    return 1
  fi
  log warn ROLLED_BACK "partial virtual USB setup was rolled back"
}

setup_exit() {
  status="$?"
  if [ "$status" -ne 0 ] && [ "${SETUP_MUTATED:-0}" = "1" ]; then
    rollback_partial_setup || status=1
  fi
  exit "$status"
}

setup() {
  require_linux_root
  [ -r "${INPUT_ROOT}/admission/admission.json" ] ||
    fail "admission receipt must exist before kernel mutation" "ADMISSION_REJECTED" 2
  [ -r "${INPUT_ROOT}/profile/profile-verification.json" ] ||
    fail "profile verification must exist before kernel mutation" "ADMISSION_REJECTED" 2
  [ -r "${INPUT_ROOT}/static-guard/compose-static-guard.json" ] ||
    fail "static Compose guard must pass before kernel mutation" "PURITY_FAIL" 1
  [ -r "${INPUT_ROOT}/preflight/preflight.json" ] &&
    [ -r "${INPUT_ROOT}/preflight/modules.before" ] ||
    fail "kernel preflight evidence must exist before mutation" "UNAVAILABLE" 3
  [ -r "${INPUT_ROOT}/lock/lock.json" ] ||
    fail "parent supervisor lock receipt is missing" "UNAVAILABLE" 3
  if [ -n "${HM_EXTERNAL_IME_LOCALE:-}" ]; then
    [ -r "${INPUT_ROOT}/policy/ime-policy.json" ] ||
      fail "run-bound IME policy must exist before kernel mutation" "PURITY_FAIL" 1
  fi
  [ ! -e "$GADGET_ROOT" ] || fail "run gadget already exists" "PURITY_FAIL" 1
  for existing_gadget in "$CONFIGFS_ROOT"/usb_gadget/*; do
    [ -e "$existing_gadget" ] || continue
    fail "another configfs USB gadget already exists" "PURITY_FAIL" 1
  done
  for existing_node in /dev/ttyGS* /dev/hidg*; do
    [ -e "$existing_node" ] || continue
    fail "stale gadget-side device node already exists" "PURITY_FAIL" 1
  done

  SETUP_MUTATED=1
  trap setup_exit EXIT
  trap 'exit 1' HUP INT TERM
  modprobe dummy_hcd
  modprobe libcomposite
  modprobe usb_f_acm
  modprobe usb_f_hid
  [ -d "$CONFIGFS_ROOT/usb_gadget" ] ||
    fail "configfs USB gadget tree is unavailable" "UNAVAILABLE" 3

  mkdir "$GADGET_ROOT"
  printf '0x1d6b' > "$GADGET_ROOT/idVendor"
  printf '0x0104' > "$GADGET_ROOT/idProduct"
  printf '0x0100' > "$GADGET_ROOT/bcdDevice"
  printf '0x0200' > "$GADGET_ROOT/bcdUSB"
  mkdir "$GADGET_ROOT/strings/0x409"
  printf '%s' "$RUN_ID" > "$GADGET_ROOT/strings/0x409/serialnumber"
  printf 'humanymous reference lab' > "$GADGET_ROOT/strings/0x409/manufacturer"
  printf 'kernel-emulated USB HID' > "$GADGET_ROOT/strings/0x409/product"

  mkdir "$GADGET_ROOT/configs/c.1" "$GADGET_ROOT/configs/c.1/strings/0x409"
  printf '120' > "$GADGET_ROOT/configs/c.1/MaxPower"
  printf 'CDC plus keyboard and relative pointer' > "$GADGET_ROOT/configs/c.1/strings/0x409/configuration"

  mkdir "$GADGET_ROOT/functions/acm.usb0"
  mkdir "$GADGET_ROOT/functions/hid.keyboard"
  printf '1' > "$GADGET_ROOT/functions/hid.keyboard/protocol"
  printf '1' > "$GADGET_ROOT/functions/hid.keyboard/subclass"
  printf '8' > "$GADGET_ROOT/functions/hid.keyboard/report_length"
  write_keyboard_descriptor > "$GADGET_ROOT/functions/hid.keyboard/report_desc"

  mkdir "$GADGET_ROOT/functions/hid.pointer"
  printf '2' > "$GADGET_ROOT/functions/hid.pointer/protocol"
  printf '1' > "$GADGET_ROOT/functions/hid.pointer/subclass"
  printf '4' > "$GADGET_ROOT/functions/hid.pointer/report_length"
  write_pointer_descriptor > "$GADGET_ROOT/functions/hid.pointer/report_desc"

  ln -s "$GADGET_ROOT/functions/acm.usb0" "$GADGET_ROOT/configs/c.1/acm.usb0"
  ln -s "$GADGET_ROOT/functions/hid.keyboard" "$GADGET_ROOT/configs/c.1/hid.keyboard"
  ln -s "$GADGET_ROOT/functions/hid.pointer" "$GADGET_ROOT/configs/c.1/hid.pointer"

  udc=""
  for candidate in /sys/class/udc/*; do
    [ -e "$candidate" ] || continue
    name="$(basename "$candidate")"
    case "$name" in dummy_udc.*|dummy_udc) [ -z "$udc" ] || fail "multiple dummy UDCs are ambiguous"; udc="$name" ;; esac
  done
  [ -n "$udc" ] || fail "dummy_hcd did not expose a dummy UDC" "UNAVAILABLE" 3
  printf '%s' "$udc" > "$GADGET_ROOT/UDC"

  gadget_command=""
  gadget_keyboard=""
  gadget_pointer=""
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    if find_run_gadget_node command; then gadget_command="$FOUND_NODE"; else gadget_command=""; fi
    if find_run_gadget_node keyboard; then gadget_keyboard="$FOUND_NODE"; else gadget_keyboard=""; fi
    if find_run_gadget_node pointer; then gadget_pointer="$FOUND_NODE"; else gadget_pointer=""; fi
    [ -n "$gadget_command" ] && [ -n "$gadget_keyboard" ] &&
      [ -n "$gadget_pointer" ] && break
    sleep 0.2
  done
  [ -n "$gadget_command" ] && [ -n "$gadget_keyboard" ] &&
    [ -n "$gadget_pointer" ] ||
    fail "gadget-side device nodes did not appear" "FAIL" 1
  host_command=""
  host_keyboard=""
  host_pointer=""
  for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    if find_run_link /dev/serial/by-id ""; then host_command="$FOUND_LINK"; else host_command=""; fi
    if find_run_link /dev/input/by-id "-event-kbd"; then host_keyboard="$FOUND_LINK"; else host_keyboard=""; fi
    if find_run_link /dev/input/by-id "-event-mouse"; then host_pointer="$FOUND_LINK"; else host_pointer=""; fi
    [ -n "$host_command" ] && [ -n "$host_keyboard" ] && [ -n "$host_pointer" ] && break
    sleep 0.2
  done
  [ -n "$host_command" ] && [ -n "$host_keyboard" ] && [ -n "$host_pointer" ] ||
    fail "host-side USB device links did not appear" "FAIL" 1
  chown 0:12001 "$gadget_command" "$gadget_keyboard" "$gadget_pointer" \
    "$host_command" "$host_keyboard" "$host_pointer"
  chmod 0660 "$gadget_command" "$gadget_keyboard" "$gadget_pointer" "$host_command"
  chmod 0640 "$host_keyboard" "$host_pointer"
  descriptor_sha="$(cat \
    "$GADGET_ROOT/functions/hid.keyboard/report_desc" \
    "$GADGET_ROOT/functions/hid.pointer/report_desc" | sha256sum | awk '{print $1}')"
  write_receipt setup ",\"gadgetName\":\"${GADGET}\",\"udc\":\"${udc}\",\"gadgetCommand\":\"${gadget_command}\",\"gadgetKeyboard\":\"${gadget_keyboard}\",\"gadgetPointer\":\"${gadget_pointer}\",\"descriptorSha256\":\"sha256:${descriptor_sha}\",\"kernelEmulated\":true,\"transport\":\"dummy-hcd\""
  SETUP_MUTATED=0
  trap - EXIT HUP INT TERM
  log info PREPARED "virtual USB gadget prepared"
}

hold_lock() {
  require_linux
  mkdir -p "$(dirname "$LOCK_FILE")"
  exec 9>"$LOCK_FILE"
  flock -n 9 || fail "another virtual USB run holds the VM-wide lock" "UNAVAILABLE" 3
  write_receipt lock ",\"bootId\":\"$(cat /proc/sys/kernel/random/boot_id)\",\"kernelRelease\":\"$(uname -r)\""
  trap 'exit 1' INT TERM HUP
  while [ ! -e "${INPUT_ROOT}/terminal/release-lock" ]; do sleep 0.2; done
  [ -r "${INPUT_ROOT}/terminal/terminal.json" ] ||
    fail "lock release requested without terminal assertion receipt"
  log info LOCK_RELEASED "terminal assertion authorized VM-wide lock release"
}

neutral_report() {
  path="$1"
  octets="$2"
  [ -c "$path" ] || fail "required HID node is missing during neutral release"
  case "$octets" in
    8) printf '\000\000\000\000\000\000\000\000' > "$path" ||
      fail "keyboard neutral report write failed" ;;
    4) printf '\000\000\000\000' > "$path" ||
      fail "pointer neutral report write failed" ;;
    *) fail "neutral report length is invalid" ;;
  esac
}

cleanup() {
  require_linux_root
  [ -r "${INPUT_ROOT}/setup/setup.json" ] ||
    fail "setup receipt is missing during cleanup"
  [ -d "$GADGET_ROOT" ] ||
    fail "run gadget is missing during cleanup"
  find_run_gadget_node keyboard || fail "run-owned keyboard node is missing during cleanup"
  gadget_keyboard="$FOUND_NODE"
  find_run_gadget_node pointer || fail "run-owned pointer node is missing during cleanup"
  gadget_pointer="$FOUND_NODE"
  grep -Fq "\"gadgetKeyboard\":\"${gadget_keyboard}\"" "${INPUT_ROOT}/setup/setup.json" ||
    fail "keyboard node differs from setup receipt"
  grep -Fq "\"gadgetPointer\":\"${gadget_pointer}\"" "${INPUT_ROOT}/setup/setup.json" ||
    fail "pointer node differs from setup receipt"
  neutral_report "$gadget_keyboard" 8
  neutral_report "$gadget_pointer" 4
  unbind_udc || fail "UDC remained bound after unbind"
  rm -f "$GADGET_ROOT/configs/c.1/acm.usb0"
  rm -f "$GADGET_ROOT/configs/c.1/hid.keyboard"
  rm -f "$GADGET_ROOT/configs/c.1/hid.pointer"
  rmdir "$GADGET_ROOT/functions/acm.usb0"
  rmdir "$GADGET_ROOT/functions/hid.keyboard"
  rmdir "$GADGET_ROOT/functions/hid.pointer"
  rmdir "$GADGET_ROOT/configs/c.1/strings/0x409"
  rmdir "$GADGET_ROOT/configs/c.1"
  rmdir "$GADGET_ROOT/strings/0x409"
  rmdir "$GADGET_ROOT"
  [ ! -e "$GADGET_ROOT" ] || fail "configfs gadget residue remains"
  unload_run_modules ||
    fail "loaded module set differs from the preflight inventory"
  write_receipt kernel-cleanup ",\"neutralRelease\":true,\"neutralKeyboardBytes\":8,\"neutralPointerBytes\":4,\"udcUnbound\":true,\"configfsResidue\":false,\"moduleSetRestored\":true"
  log info CLEAN "virtual USB kernel/device cleanup completed"
}

validate_run_id
case "$ACTION" in
  lock) hold_lock ;;
  preflight) preflight ;;
  setup) setup ;;
  cleanup) cleanup ;;
  *) fail "usage: lifecycle.sh lock|preflight|setup|cleanup" "UNAVAILABLE" 3 ;;
esac
