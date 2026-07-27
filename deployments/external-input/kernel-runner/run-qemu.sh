#!/bin/sh
set -eu

output="${HM_KERNEL_RUNNER_OUTPUT:-/output}"
mkdir -p "$output"

if [ -c /dev/kvm ] && [ "${HM_KERNEL_ACCEL:-auto}" != tcg ]; then
  accel=kvm
  cpu=host
else
  accel=tcg
  cpu=max
fi

console="$output/console.log"

run_measurement() {
  work=/run-kernel
  mkdir -p "$work"
  [ -z "$(find "$work" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
    echo "kernel work volume is not empty" >&2
    return 1
  }
  identity=/opt/humanymous-kernel/runner-identity.json
  [ -r "$identity" ] || {
    echo "runner identity missing" >&2
    return 1
  }

  seed_files="$(
    find /seed -mindepth 1 -maxdepth 1 -type f |
      while IFS= read -r path; do basename "$path"; done |
      sort
  )"
  [ "$seed_files" = "$(printf 'bundle.tar\nimages.oci.tar\nseed.json')" ] || {
    echo "seed directory must contain exactly bundle.tar, images.oci.tar, seed.json" >&2
    return 1
  }
  [ "$(find /seed -mindepth 1 -maxdepth 1 ! -type f | wc -l)" -eq 0 ] || {
    echo "seed directory contains non-regular entries" >&2
    return 1
  }

  actual_seed_sha="sha256:$(sha256sum /seed/seed.json | awk '{print $1}')"
  [ -n "${HM_KERNEL_RUNNER_SEED_SHA256:-}" ] &&
    [ "$actual_seed_sha" = "$HM_KERNEL_RUNNER_SEED_SHA256" ] || {
      echo "seed JSON does not match outer-attested digest" >&2
      return 1
    }
  jq -e '
    .platform == "linux/amd64" and
    (.budgets.cpus >= 1 and .budgets.cpus <= 8) and
    (.budgets.memoryMiB >= 512 and .budgets.memoryMiB <= 8192) and
    (.budgets.deadlineSeconds >= 60 and .budgets.deadlineSeconds <= 3600) and
    (.budgets.outputBytes >= 1048576 and .budgets.outputBytes <= 1073741824) and
    (.imageArchive.path == "images.oci.tar") and
    (.imageArchive.bytes >= 1 and .imageArchive.bytes <= 805306368) and
    (.innerCompose.pullPolicy == "never")
  ' /seed/seed.json >/dev/null
  seed_content="$(jq -cS '.seedContentSha256 = ""' /seed/seed.json)"
  actual_seed_content_sha="sha256:$(printf '%s' "$seed_content" | sha256sum | awk '{print $1}')"
  [ "$actual_seed_content_sha" = "$(jq -er '.seedContentSha256' /seed/seed.json)" ] || {
    echo "seed canonical content digest mismatch" >&2
    return 1
  }

  image_sha="sha256:$(sha256sum /seed/images.oci.tar | awk '{print $1}')"
  bundle_sha="sha256:$(sha256sum /seed/bundle.tar | awk '{print $1}')"
  [ "$image_sha" = "$(jq -er '.imageArchive.sha256' /seed/seed.json)" ]
  [ "$bundle_sha" = "$(jq -er '.sourceBundle.sha256' /seed/seed.json)" ]
  [ "$(stat -c %s /seed/images.oci.tar)" = "$(jq -er '.imageArchive.bytes' /seed/seed.json)" ]
  [ "$(stat -c %s /seed/bundle.tar)" = "$(jq -er '.sourceBundle.bytes' /seed/seed.json)" ]

  jq -e --slurpfile actual "$identity" '
    .runner.qemuVersion == $actual[0].qemuVersion and
    .runner.qemuBinarySha256 == $actual[0].qemuBinarySha256 and
    .runner.guestBaseSha256 == $actual[0].guestBaseSha256 and
    .runner.guestBaseFormat == "qcow2" and
    .runner.guestBaseFormat == $actual[0].guestBaseFormat and
    .runner.guestBaseVirtualMiB == $actual[0].guestBaseVirtualMiB and
    .runner.guestBaseAllocatedBytes == $actual[0].guestBaseAllocatedBytes and
    .runner.kernelSha256 == $actual[0].kernelSha256 and
    .runner.initramfsSha256 == $actual[0].initramfsSha256
  ' /seed/seed.json >/dev/null

  seed_bytes="$(du -sb /seed | awk '{print $1}')"
  seed_mib="$(( (seed_bytes + 67108863) / 1048576 ))"
  [ "$seed_mib" -ge 128 ] || seed_mib=128
  seed_image="$work/kernel-seed.ext4"
  truncate -s "${seed_mib}M" "$seed_image"
  seed_uuid="$(jq -er '.runNonce' /seed/seed.json | sed 's/^\(........\)\(....\)\(....\)\(....\)\(............\).*$/\1-\2-\3-\4-\5/')"
  mke2fs -q -t ext4 -U "$seed_uuid" -L humanymous-seed \
    -E lazy_itable_init=0,lazy_journal_init=0 -d /seed "$seed_image"

  result_bytes="$(jq -er '.budgets.outputBytes' /seed/seed.json)"
  result_mib="$(( (result_bytes + 67108863) / 1048576 ))"
  [ "$result_mib" -ge 64 ] || result_mib=64
  result_image="$work/kernel-result.ext4"
  truncate -s "${result_mib}M" "$result_image"
  result_uuid="$(jq -er '.runNonce' /seed/seed.json | cut -c33-64 | sed 's/^\(........\)\(....\)\(....\)\(....\)\(............\)$/\1-\2-\3-\4-\5/')"
  mke2fs -q -t ext4 -U "$result_uuid" -L humanymous-result \
    -E lazy_itable_init=0,lazy_journal_init=0 "$result_image"

  docker_data_image="$work/kernel-docker-data.ext4"
  truncate -s 2048M "$docker_data_image"
  # This is a disposable, root-owned Docker data disk. The ext4 default keeps
  # five percent for a privileged recovery user, which strands about 102 MiB in
  # every 2 GiB cell and previously forced needless virtual-capacity growth.
  mke2fs -q -t ext4 -m 0 -L humanymous-docker \
    -E lazy_itable_init=0,lazy_journal_init=0 "$docker_data_image"

  overlay="$work/kernel-guest.qcow2"
  qemu-img create -q -f qcow2 -F qcow2 \
    -b /opt/humanymous-vm/guest-root.qcow2 "$overlay"

  cpus="$(jq -er '.budgets.cpus' /seed/seed.json)"
  memory="$(jq -er '.budgets.memoryMiB' /seed/seed.json)"
  deadline="$(jq -er '.budgets.deadlineSeconds' /seed/seed.json)"

  set +e
  timeout --signal=TERM --kill-after=10 "${deadline}s" \
    qemu-system-x86_64 \
      -machine "q35,accel=${accel},usb=off,i8042=off" \
      -cpu "$cpu" \
      -smp "$cpus" \
      -m "${memory}M" \
      -nodefaults \
      -no-reboot \
      -display none \
      -monitor none \
      -serial stdio \
      -device virtio-rng-pci \
      -kernel /opt/humanymous-vm/vmlinuz \
      -initrd /opt/humanymous-vm/initramfs.img \
      -append "console=ttyS0 root=/dev/vda rw init=/sbin/humanymous-init panic=-1 oops=panic random.trust_cpu=on" \
      -drive "file=${overlay},format=qcow2,if=virtio,cache=unsafe" \
      -drive "file=${seed_image},format=raw,if=virtio,readonly=on" \
      -drive "file=${result_image},format=raw,if=virtio,cache=unsafe" \
      -drive "file=${docker_data_image},format=raw,if=virtio,cache=unsafe" \
      > "$console" 2>&1
  qemu_status=$?
  set -e
  cat "$console"
  [ "$qemu_status" -eq 0 ] || {
    printf '{"schemaVersion":"humanymous.kernel-runner-receipt/v1","kind":"kernel-runner","status":"FAIL","reason":"guest-exit-%s"}\n' \
      "$qemu_status" > "$output/runner.json"
    return 1
  }

  extract="$work/kernel-result"
  mkdir "$extract"
  debugfs -R "rdump / $extract" "$result_image" >/dev/null 2>&1
  expected_guest_files="$(printf '%s\n' \
    docker-load.log dockerd.log guest-console.log guest-terminal.json \
    measurement-result.json measurement-score.json measurement-terminal.json)"
  actual_guest_files="$(
    find "$extract" -mindepth 1 -maxdepth 1 -type f |
      while IFS= read -r path; do basename "$path"; done |
      sort
  )"
  [ "$actual_guest_files" = "$expected_guest_files" ] || {
    echo "guest result disk contains missing or unexpected regular files" >&2
    return 1
  }
  [ "$(find "$extract" -mindepth 1 -maxdepth 1 \
      ! -type f ! -name lost+found -print -quit)" = "" ] || {
    echo "guest result disk contains an unexpected entry type" >&2
    return 1
  }
  extracted_bytes="$(find "$extract" -mindepth 1 -maxdepth 1 -type f \
      -exec stat -c %s '{}' ';' |
    awk '{ total += $1 } END { print total + 0 }')"
  [ "$extracted_bytes" -le "$result_bytes" ] || {
    echo "guest output exceeds the seed budget" >&2
    return 1
  }
  for name in \
    guest-console.log dockerd.log docker-load.log guest-terminal.json \
    measurement-result.json measurement-score.json measurement-terminal.json; do
    [ -f "$extract/$name" ] || {
      echo "guest result file missing: $name" >&2
      return 1
    }
    [ "$(stat -c %h "$extract/$name")" -eq 1 ] || {
      echo "guest result hard links are forbidden: $name" >&2
      return 1
    }
    cp "$extract/$name" "$output/$name"
  done

  jq -e '.schemaVersion == "humanymous.kernel-guest-terminal/v1" and
         .status == "PASS" and .physicalUsb == false' \
    "$output/guest-terminal.json" >/dev/null
  guest_sha="$(sha256sum "$output/guest-terminal.json" | awk '{print $1}')"
  identity_sha="$(sha256sum "$identity" | awk '{print $1}')"
  qemu_version="$(qemu-system-x86_64 --version | sed -n '1s/.*version \([^ ]*\).*/\1/p')"
  printf '{"schemaVersion":"humanymous.kernel-runner-receipt/v1","kind":"kernel-runner","status":"PASS","accelerator":"%s","qemuVersion":"%s","runnerIdentitySha256":"sha256:%s","guestTerminalSha256":"sha256:%s","guestTerminal":"guest-terminal.json","consoleLog":"console.log"}\n' \
    "$accel" "$qemu_version" "$identity_sha" "$guest_sha" > "$output/runner.json"
}

if [ -r /seed/seed.json ]; then
  run_measurement
  exit $?
fi

[ -r /opt/humanymous-kernel/vmlinuz ] &&
  [ -r /opt/humanymous-kernel/initramfs.cpio.gz ] || {
  printf '{"schemaVersion":1,"kind":"kernel-runner","status":"FAIL","reason":"seed-required"}\n' \
    > "$output/runner.json"
  exit 2
}

qemu-system-x86_64 \
  -machine "q35,accel=${accel},usb=off,i8042=off" \
  -cpu "$cpu" \
  -smp "${HM_KERNEL_CPUS:-2}" \
  -m "${HM_KERNEL_MEMORY:-768M}" \
  -nodefaults \
  -no-reboot \
  -display none \
  -monitor none \
  -serial stdio \
  -kernel /opt/humanymous-kernel/vmlinuz \
  -initrd /opt/humanymous-kernel/initramfs.cpio.gz \
  -append "console=ttyS0 rdinit=/init panic=-1 oops=panic random.trust_cpu=on" \
  > "$console" 2>&1

cat "$console"
marker="$(grep '^HM_KERNEL_SMOKE:' "$console" | tail -n 1 | cut -d: -f2-)"
[ -n "$marker" ] || {
  printf '{"schemaVersion":1,"kind":"kernel-runner","status":"FAIL","reason":"guest-receipt-missing"}\n' > "$output/runner.json"
  exit 1
}
printf '%s\n' "$marker" > "$output/guest-smoke.json"
grep -q '"status":"PASS"' "$output/guest-smoke.json" || exit 1

kernel_sha="$(sha256sum /opt/humanymous-kernel/vmlinuz | awk '{print $1}')"
initramfs_sha="$(sha256sum /opt/humanymous-kernel/initramfs.cpio.gz | awk '{print $1}')"
qemu_version="$(qemu-system-x86_64 --version | sed -n '1s/.*version \([^ ]*\).*/\1/p')"
printf '{"schemaVersion":1,"kind":"kernel-runner","status":"PASS","accelerator":"%s","qemuVersion":"%s","kernelSha256":"sha256:%s","initramfsSha256":"sha256:%s","guestReceipt":"guest-smoke.json","consoleLog":"console.log"}\n' \
  "$accel" "$qemu_version" "$kernel_sha" "$initramfs_sha" > "$output/runner.json"
