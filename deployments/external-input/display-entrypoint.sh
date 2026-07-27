#!/bin/sh
set -eu

mode="${HM_EXTERNAL_MODE:?HM_EXTERNAL_MODE is required}"
display="${DISPLAY:-:99}"
screen="${HM_EXTERNAL_SCREEN:-1280x720}"
secret_dir=/run/external-secrets
x11_dir=/tmp/.X11-unix
config=/run/external-input/xorg.conf

case "$mode" in
  external_input_virtual|external_input_dom_virtual)
    xtest="+extension XTEST"
    accept_input=1
    ;;
  external_input_usb|external_input_dom_usb|external_input_vusb|external_input_dom_vusb)
    xtest="-extension XTEST"
    accept_input=0
    : "${HM_EXTERNAL_HID_KEYBOARD:?USB keyboard event path is required}"
    : "${HM_EXTERNAL_HID_POINTER:?USB pointer event path is required}"
    [ -c "$HM_EXTERNAL_HID_KEYBOARD" ] || { echo "USB keyboard event device is absent" >&2; exit 3; }
    [ -c "$HM_EXTERNAL_HID_POINTER" ] || { echo "USB pointer event device is absent" >&2; exit 3; }
    ;;
  *)
    echo "unknown external-input mode" >&2
    exit 2
    ;;
esac

mkdir -p "$secret_dir" "$x11_dir" /run/external-input
chmod 1777 "$x11_dir"
rm -f "$secret_dir/Xauthority" "$secret_dir/Xauthority.local" \
  "$secret_dir/rfb.password" "$secret_dir/rfb.passwd"

cat >"$config" <<EOF
Section "ServerFlags"
  Option "AutoAddDevices" "false"
  Option "AutoEnableDevices" "false"
  Option "DontVTSwitch" "true"
EndSection
Section "Device"
  Identifier "dummy-video"
  Driver "dummy"
  # 1280x720x32bpp needs under 4 MiB per framebuffer. Keep bounded room for
  # multiple buffers without reserving a quarter-gigabyte in the nested VM.
  VideoRam 65536
EndSection
Section "Monitor"
  Identifier "dummy-monitor"
  HorizSync 5.0-1000.0
  VertRefresh 5.0-200.0
  Modeline "1280x720" 74.50 1280 1344 1472 1664 720 723 728 748
EndSection
Section "Screen"
  Identifier "dummy-screen"
  Device "dummy-video"
  Monitor "dummy-monitor"
  DefaultDepth 24
  SubSection "Display"
    Depth 24
    Modes "1280x720"
  EndSubSection
EndSection
EOF

if [ "$accept_input" = 0 ]; then
  cat >>"$config" <<EOF
Section "InputDevice"
  Identifier "external-keyboard"
  Driver "libinput"
  Option "Device" "$HM_EXTERNAL_HID_KEYBOARD"
EndSection
Section "InputDevice"
  Identifier "external-pointer"
  Driver "libinput"
  Option "Device" "$HM_EXTERNAL_HID_POINTER"
EndSection
Section "ServerLayout"
  Identifier "external-seat"
  Screen "dummy-screen"
  InputDevice "external-keyboard"
  InputDevice "external-pointer"
EndSection
EOF
  HM_VUSB_SEAT_EVIDENCE="${HM_VUSB_SEAT_EVIDENCE:-/run/external-evidence/seat-events.json}" \
    node /opt/external-input/seat-observer.mjs &
fi

cookie="$(mcookie)"
local_auth="$secret_dir/Xauthority.local"
xauth -f "$local_auth" add "$display" . "$cookie"
xauth -f "$local_auth" nlist "$display" \
  | sed 's/^..../ffff/' \
  | xauth -f "$secret_dir/Xauthority" nmerge -
rm -f "$local_auth"
chgrp 12001 "$secret_dir" "$secret_dir/Xauthority"
chmod 0750 "$secret_dir"
chmod 0440 "$secret_dir/Xauthority"

password="$(head -c 24 /dev/urandom | base64 | tr -dc A-Za-z0-9 | cut -c 1-8)"
printf '%s\n' "$password" >"$secret_dir/rfb.password"
printf '%s' "$password" | vncpasswd -f >"$secret_dir/rfb.passwd"
chgrp 12001 "$secret_dir/rfb.password" "$secret_dir/rfb.passwd"
chmod 0440 "$secret_dir/rfb.password" "$secret_dir/rfb.passwd"

Xorg "$display" -noreset -nolisten tcp -config "$config" \
  -auth "$secret_dir/Xauthority" -logfile /run/external-input/Xorg.log $xtest &
xorg_pid=$!
trap 'kill "$xorg_pid" 2>/dev/null || true' EXIT INT TERM

for _ in $(seq 1 100); do
  DISPLAY="$display" XAUTHORITY="$secret_dir/Xauthority" xdpyinfo >/dev/null 2>&1 && break
  sleep 0.1
done
DISPLAY="$display" XAUTHORITY="$secret_dir/Xauthority" xdpyinfo >/dev/null

export DISPLAY="$display"
export XAUTHORITY="$secret_dir/Xauthority"
exec x0vncserver \
  -fg \
  -display "$display" \
  -rfbport 5900 \
  -localhost=no \
  -PasswordFile="$secret_dir/rfb.passwd" \
  -SecurityTypes=VncAuth \
  -NeverShared=1 \
  -DisconnectClients=0 \
  -AcceptSetDesktopSize=0 \
  -AcceptKeyEvents="$accept_input" \
  -AcceptPointerEvents="$accept_input"
