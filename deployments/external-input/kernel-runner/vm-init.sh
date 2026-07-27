#!/bin/bash
set -Eeuo pipefail

export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

log() {
  printf 'HM_KERNEL_GUEST: %s\n' "$*"
}

fail() {
  local reason="${1:-guest-init-failed}"
  log "FAIL ${reason}"
  if mountpoint -q /output; then
    printf '{"schemaVersion":"humanymous.kernel-guest-bootstrap/v1","status":"FAIL","reason":"%s"}\n' \
      "$reason" > /output/guest-bootstrap.json
  fi
  poweroff_guest 1
}

poweroff_guest() {
  local status="${1:-0}"
  set +e
  if [[ -r /run/dockerd.pid ]]; then
    kill "$(cat /run/dockerd.pid)" 2>/dev/null
    for _ in {1..50}; do
      kill -0 "$(cat /run/dockerd.pid)" 2>/dev/null || break
      sleep 0.1
    done
  fi
  sync
  log "poweroff status=${status}"
  /bin/busybox poweroff -f
  sleep 5
  echo o > /proc/sysrq-trigger
  exit "$status"
}

trap 'fail unexpected-init-error' ERR

mountpoint -q /proc || mount -t proc proc /proc
mountpoint -q /sys || mount -t sysfs sysfs /sys
mountpoint -q /dev || mount -t devtmpfs devtmpfs /dev
mkdir -p /dev/pts /run /sys/fs/cgroup /seed /output /workspace /var/lib/docker
mountpoint -q /dev/pts || mount -t devpts devpts /dev/pts
mountpoint -q /run || mount -t tmpfs -o mode=0755,nosuid,nodev tmpfs /run
install -d -m 0700 -o humanymous -g humanymous /run/user/12000
mountpoint -q /sys/fs/cgroup ||
  mount -t cgroup2 -o nosuid,nodev,noexec cgroup2 /sys/fs/cgroup
modprobe configfs
[[ -d /sys/kernel/config ]] || fail configfs-mountpoint-missing
mountpoint -q /sys/kernel/config || mount -t configfs configfs /sys/kernel/config

for _ in {1..100}; do
  [[ -b /dev/vdb && -b /dev/vdc && -b /dev/vdd ]] && break
  sleep 0.05
done
[[ -b /dev/vdb && -b /dev/vdc && -b /dev/vdd ]] ||
  fail evidence-disks-missing
mount -t ext4 -o ro,nodev,nosuid,noexec /dev/vdb /seed
mount -t ext4 -o rw,nodev,nosuid,noexec /dev/vdc /output
mount -t ext4 -o rw,nodev,nosuid /dev/vdd /var/lib/docker

exec > >(tee -a /output/guest-console.log) 2>&1
trap 'fail unexpected-init-error' ERR

[[ -f /seed/seed.json && -f /seed/images.oci.tar && -f /seed/bundle.tar ]] ||
  fail seed-files-missing

seed_bundle_sha="$(
  jq -er '.sourceBundle.sha256 // .bundle.sha256' /seed/seed.json
)"
actual_bundle_sha="sha256:$(sha256sum /seed/bundle.tar | awk '{print $1}')"
[[ "$actual_bundle_sha" == "$seed_bundle_sha" ]] || fail source-bundle-digest-mismatch

bundle_list=/run/bundle.entries
tar --list --file /seed/bundle.tar > "$bundle_list"
expected_entries="$(jq -er '.sourceBundle.entries' /seed/seed.json)"
actual_entries="$(wc -l < "$bundle_list")"
[[ "$actual_entries" == "$expected_entries" ]] || fail source-bundle-entry-count-mismatch
awk '
  /^\/|(^|\/)\.\.(\/|$)|(^|\/)\.(\/|$)/ { exit 1 }
  { if (seen[$0]++) exit 1 }
' "$bundle_list" || fail source-bundle-path-policy
tar --list --verbose --file /seed/bundle.tar |
  awk 'substr($0, 1, 1) != "-" && substr($0, 1, 1) != "d" { exit 1 }' ||
  fail source-bundle-entry-type

tar --extract --file /seed/bundle.tar --directory /workspace \
  --no-same-owner --no-same-permissions --keep-directory-symlink
[[ -f /workspace/scripts/external-input-kernel-guest.sh &&
   -r /workspace/scripts/external-input-kernel-guest.sh ]] ||
  fail guest-command-missing

modprobe overlay
modprobe bridge
modprobe br_netfilter
modprobe veth
modprobe nf_nat

/lib/systemd/systemd-udevd --daemon
udevadm trigger --action=add
udevadm settle --timeout=10

mkdir -p /run/docker
dockerd \
  --host=unix:///run/docker.sock \
  --data-root=/var/lib/docker \
  --exec-root=/run/docker \
  --pidfile=/run/dockerd.pid \
  --iptables=false \
  --ip-masq=false \
  --userland-proxy=false \
  --log-level=warn \
  > /output/dockerd.log 2>&1 &

for _ in {1..300}; do
  docker --host unix:///run/docker.sock info >/dev/null 2>&1 && break
  sleep 0.1
done
docker --host unix:///run/docker.sock info >/dev/null 2>&1 ||
  fail dockerd-not-ready

chgrp docker /run/docker.sock
chmod 0660 /run/docker.sock
chown -R humanymous:humanymous /workspace /output
chmod 0770 /output

set +e
runuser -u humanymous -- env \
  HOME=/home/humanymous \
  XDG_RUNTIME_DIR=/run/user/12000 \
  DOCKER_HOST=unix:///run/docker.sock \
  HM_KERNEL_GUEST=1 \
  bash /workspace/scripts/external-input-kernel-guest.sh \
    --seed /seed/seed.json
guest_status=$?
set -e

if [[ "$guest_status" -ne 0 ]]; then
  fail "guest-command-exit-${guest_status}"
fi
[[ -s /output/guest-terminal.json ]] || fail guest-terminal-missing

log "PASS terminal=/output/guest-terminal.json"
poweroff_guest 0
