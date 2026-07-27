#!/bin/sh
set -eu

mode="${HM_EXTERNAL_MODE:?HM_EXTERNAL_MODE is required}"
engine="${HM_EXTERNAL_BROWSER:?HM_EXTERNAL_BROWSER is required}"
target="${HM_EXTERNAL_TARGET:-https://core:8443/static/external-input.html}"
profile=/run/browser-profile
nss=/home/extbrowser/.pki/nssdb

case "$mode" in
  external_input_virtual|external_input_usb|external_input_vusb)
    dom=0
    ;;
  external_input_dom_virtual|external_input_dom_usb|external_input_dom_vusb)
    dom=1
    ;;
  *)
    echo "unknown external-input mode" >&2
    exit 2
    ;;
esac

for _ in $(seq 1 200); do
  [ -S /tmp/.X11-unix/X99 ] && [ -s /pki/ca.pem ] && break
  sleep 0.1
done
[ -S /tmp/.X11-unix/X99 ] || { echo "Xorg socket did not become ready" >&2; exit 1; }
[ -s /pki/ca.pem ] || { echo "lab CA did not become ready" >&2; exit 1; }

find "$profile" -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +
rm -rf "$nss"
mkdir -p "$profile" "$nss"
certutil -N --empty-password -d "sql:$nss"
certutil -A -d "sql:$nss" -n "humanymous external-input lab CA" -t "C,," -i /pki/ca.pem
certutil -N --empty-password -d "sql:$profile"
certutil -A -d "sql:$profile" -n "humanymous external-input lab CA" -t "C,," -i /pki/ca.pem

export DISPLAY="${DISPLAY:-:99}"
export XAUTHORITY=/run/external-secrets/Xauthority
dbus_pid=""
ibus_pid=""
ime_engine=""
ime_engine_package=""
ime_locale=""
ime_managed_pids=""
ime_exit_requested=false

write_ime_stop_receipt() {
  readiness="${HM_EXTERNAL_IME_STOP:-/run/external-evidence/ime-stop.json}"
  temporary="${readiness}.tmp.$$"
  active_before="${1:-}"
  bus_residue=false
  process_residue=false
  state_roots_removed=true
  [ ! -S /tmp/ime-runtime/session-bus ] || bus_residue=true
  for state_root in /tmp/ime-runtime /tmp/ime-config /tmp/ime-cache; do
    [ ! -e "$state_root" ] || state_roots_removed=false
  done
  for managed_pid in $ime_managed_pids; do
    if kill -0 "$managed_pid" 2>/dev/null; then process_residue=true; fi
  done
  config_entry_count="$(find /tmp/ime-config -mindepth 1 -print 2>/dev/null | wc -l | tr -d ' ')"
  cache_entry_count="$(find /tmp/ime-cache -mindepth 1 -print 2>/dev/null | wc -l | tr -d ' ')"
  printf '{"schemaVersion":"humanymous.ime-stop/v1","runId":"%s","locale":"%s","engineId":"%s","activeEngineBeforeStop":"%s","imeExitRequested":%s,"busSocketResidue":%s,"managedProcessResidue":%s,"stateRootsRemoved":%s,"configEntryCount":%s,"cacheEntryCount":%s}\n' \
    "${HM_EXTERNAL_RUN_ID:?HM_EXTERNAL_RUN_ID is required for IME}" \
    "$ime_locale" "$ime_engine" "$active_before" "$ime_exit_requested" \
    "$bus_residue" "$process_residue" "$state_roots_removed" \
    "$config_entry_count" "$cache_entry_count" > "$temporary"
  chgrp 12001 "$temporary"
  chmod 0640 "$temporary"
  mv "$temporary" "$readiness"
}

stop_ime_session() {
  active_before="$(ibus engine 2>/dev/null || true)"
  ime_managed_pids="$dbus_pid $ibus_pid"
  if [ -n "$ibus_pid$dbus_pid" ]; then
    pending="$ibus_pid $dbus_pid"
    while [ -n "$pending" ]; do
      next=""
      for parent_pid in $pending; do
        children="$(pgrep -P "$parent_pid" 2>/dev/null || true)"
        ime_managed_pids="$ime_managed_pids $children"
        next="$next $children"
      done
      pending="$next"
    done
  fi
  if ibus exit >/dev/null 2>&1; then ime_exit_requested=true; fi
  if [ -n "$ibus_pid" ]; then
    kill "$ibus_pid" 2>/dev/null || true
    wait "$ibus_pid" 2>/dev/null || true
  fi
  if [ -n "$dbus_pid" ]; then
    kill "$dbus_pid" 2>/dev/null || true
    wait "$dbus_pid" 2>/dev/null || true
  fi
  for _ in $(seq 1 20); do
    residue=false
    for managed_pid in $ime_managed_pids; do
      if kill -0 "$managed_pid" 2>/dev/null; then residue=true; fi
    done
    [ "$residue" = false ] && break
    sleep 0.05
  done
  rm -rf -- /tmp/ime-runtime /tmp/ime-config /tmp/ime-cache
  write_ime_stop_receipt "$active_before"
}

start_ime_session() {
  locale="$1"
  engine_id="$2"
  runtime=/tmp/ime-runtime
  config_home=/tmp/ime-config
  cache_home=/tmp/ime-cache
  install -d -m 0700 "$runtime" "$config_home" "$cache_home"
  runtime_root_fresh=true
  config_root_fresh=true
  cache_root_fresh=true
  [ -z "$(find "$runtime" -mindepth 1 -print -quit)" ] || runtime_root_fresh=false
  [ -z "$(find "$config_home" -mindepth 1 -print -quit)" ] || config_root_fresh=false
  [ -z "$(find "$cache_home" -mindepth 1 -print -quit)" ] || cache_root_fresh=false
  if [ "$runtime_root_fresh" != true ] ||
     [ "$config_root_fresh" != true ] ||
     [ "$cache_root_fresh" != true ]; then
    echo "private IME state roots were not fresh" >&2
    exit 3
  fi
  export XDG_RUNTIME_DIR="$runtime"
  export XDG_CONFIG_HOME="$config_home"
  export XDG_CACHE_HOME="$cache_home"
  export LANG=C.UTF-8
  export LC_ALL=C.UTF-8
  export GDK_BACKEND=x11
  export GTK_IM_MODULE=ibus
  export XMODIFIERS=@im=ibus
  export IBUS_ENABLE_SYNC_MODE=1
  export DBUS_SESSION_BUS_ADDRESS="unix:path=$runtime/session-bus"
  dbus-daemon --session --nofork --address="$DBUS_SESSION_BUS_ADDRESS" --nopidfile &
  dbus_pid="$!"
  [ -S "$runtime/session-bus" ] ||
    { echo "private browser session bus did not start" >&2; exit 3; }
  ibus-daemon --replace --xim --panel=disable &
  ibus_pid="$!"
  ready=0
  for _ in $(seq 1 100); do
    if ibus engine "$engine_id" >/dev/null 2>&1 &&
       [ "$(ibus engine 2>/dev/null)" = "$engine_id" ]; then
      ready=1
      break
    fi
    sleep 0.1
  done
  [ "$ready" = 1 ] || { echo "pinned IBus engine did not become ready" >&2; exit 3; }
  readiness="${HM_EXTERNAL_IME_READINESS:-/run/external-evidence/ime-readiness.json}"
  temporary="${readiness}.tmp.$$"
  framework_version="$(ibus version | tr -d '\r\n')"
  printf '%s' "$framework_version" | grep -Eq '^IBus [0-9]+([.][0-9]+)*$' ||
    { echo "IBus version output is unsafe" >&2; exit 3; }
  engine_package_version="$(dpkg-query -W -f='${Version}' "$ime_engine_package")"
  font_package_version="$(dpkg-query -W -f='${Version}' fonts-noto-cjk)"
  xkb_package_version="$(dpkg-query -W -f='${Version}' xkb-data)"
  package_content_inventory_sha256() {
    dpkg-query -L "$1" |
      LC_ALL=C sort |
      while IFS= read -r package_path; do
        if [ -f "$package_path" ] && [ ! -L "$package_path" ]; then
          sha256sum "$package_path"
        fi
      done |
      sha256sum |
      awk '{print $1}'
  }
  engine_package_content_inventory_sha256="$(package_content_inventory_sha256 "$ime_engine_package")"
  font_package_content_inventory_sha256="$(package_content_inventory_sha256 fonts-noto-cjk)"
  xkb_package_content_inventory_sha256="$(package_content_inventory_sha256 xkb-data)"
  for inventory_sha256 in "$engine_package_content_inventory_sha256" \
      "$font_package_content_inventory_sha256" "$xkb_package_content_inventory_sha256"; do
    printf '%s' "$inventory_sha256" | grep -Eq '^[a-f0-9]{64}$' ||
      { echo "IME package inventory hash is invalid" >&2; exit 3; }
  done
  printf '{"schemaVersion":"humanymous.ime-readiness/v1","runId":"%s","locale":"%s","framework":"ibus","frameworkVersion":"%s","engineId":"%s","activeEngine":"%s","enginePackage":"%s","enginePackageVersion":"%s","enginePackageContentInventorySha256":"%s","fontPackage":"fonts-noto-cjk","fontPackageVersion":"%s","fontPackageContentInventorySha256":"%s","xkbPackage":"xkb-data","xkbPackageVersion":"%s","xkbPackageContentInventorySha256":"%s","privateSessionBus":true,"networkScope":"compose-internal-target-only","imeStatePersistence":"tmpfs-only","runtimeRootFresh":%s,"configRootFresh":%s,"cacheRootFresh":%s}\n' \
    "${HM_EXTERNAL_RUN_ID:?HM_EXTERNAL_RUN_ID is required for IME}" \
    "$locale" "$framework_version" "$engine_id" "$(ibus engine)" \
    "$ime_engine_package" "$engine_package_version" "$engine_package_content_inventory_sha256" \
    "$font_package_version" "$font_package_content_inventory_sha256" \
    "$xkb_package_version" "$xkb_package_content_inventory_sha256" \
    "$runtime_root_fresh" "$config_root_fresh" "$cache_root_fresh" > "$temporary"
  chgrp 12001 "$temporary"
  chmod 0640 "$temporary"
  mv "$temporary" "$readiness"
}

ime_locale="${HM_EXTERNAL_IME_LOCALE:-}"
if [ -n "$ime_locale" ]; then
  case "$ime_locale" in
    ko-KR) ime_engine=hangul; ime_engine_package=ibus-hangul ;;
    zh-CN) ime_engine=libpinyin; ime_engine_package=ibus-libpinyin ;;
    ja-JP) ime_engine=anthy; ime_engine_package=ibus-anthy ;;
    *)
      echo "IME locale must be ko-KR, zh-CN, or ja-JP" >&2
      exit 2
      ;;
  esac
  start_ime_session "$ime_locale" "$ime_engine"
  case "$target" in
    *\?*) target="${target}&ime=${ime_locale}" ;;
    *) target="${target}?ime=${ime_locale}" ;;
  esac
fi

case "$engine" in
  chromium)
    # Call the packaged browser binary directly. Debian's /usr/bin/chromium
    # wrapper injects an empty --load-extension flag even when no observer is
    # configured, which would make framebuffer-only purity ambiguous.
    set -- /usr/lib/chromium/chromium \
      --user-data-dir="$profile" \
      --no-first-run \
      --no-default-browser-check \
      --disable-background-networking \
      --disable-component-update \
      --disable-sync \
      --window-position=0,0 \
      --window-size=1280,720
    if [ "$dom" = 1 ]; then
      set -- "$@" \
        --disable-extensions-except=/opt/external-input/dom-observer/extension \
        --load-extension=/opt/external-input/dom-observer/extension
    fi
    if [ -z "$ime_locale" ]; then exec "$@" "$target"; fi
    "$@" "$target" &
    browser_pid="$!"
    ;;
  firefox)
    if [ -z "$ime_locale" ]; then
      exec firefox-esr -no-remote -profile "$profile" -width 1280 -height 720 "$target"
    fi
    firefox-esr -no-remote -profile "$profile" -width 1280 -height 720 "$target" &
    browser_pid="$!"
    ;;
  *)
    echo "browser engine must be chromium or firefox" >&2
    exit 2
    ;;
esac

terminate_browser() {
  kill "$browser_pid" 2>/dev/null || true
  wait "$browser_pid" 2>/dev/null || true
  stop_ime_session
  exit 0
}
trap terminate_browser INT TERM HUP
set +e
wait "$browser_pid"
browser_status="$?"
set -e
stop_ime_session
exit "$browser_status"
