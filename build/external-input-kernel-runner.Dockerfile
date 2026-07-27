FROM docker:29.1.3-dind@sha256:173f284a4299164772a90f52b373e73e087583c0963f1334c9995f190ef6f3f5 AS docker-runtime

FROM debian:trixie-slim@sha256:020c0d20b9880058cbe785a9db107156c3c75c2ac944a6aa7ab59f2add76a7bd AS guest

ARG DEBIAN_FRONTEND=noninteractive
ARG DEBIAN_KERNEL_VERSION=6.12.96-1
ARG BUSYBOX_VERSION=1:1.37.0-6+b8
ARG KMOD_VERSION=34.2-2
ARG CPIO_VERSION=2.15+dfsg-2

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      "linux-image-amd64=${DEBIAN_KERNEL_VERSION}" \
      "busybox-static=${BUSYBOX_VERSION}" \
      "kmod=${KMOD_VERSION}" \
      "cpio=${CPIO_VERSION}" \
      xz-utils \
 && rm -rf /var/lib/apt/lists/*

COPY deployments/external-input/kernel-runner/init /tmp/init

RUN set -eux; \
    kernel="$(basename "$(find /lib/modules -mindepth 1 -maxdepth 1 -type d | head -n 1)")"; \
    root=/tmp/root; \
    mkdir -p "$root/bin" "$root/sbin" "$root/dev" "$root/etc" "$root/proc" "$root/sys" "$root/lib/modules/$kernel"; \
    cp /bin/busybox "$root/bin/busybox"; \
    for applet in $(/bin/busybox --list); do \
      [ "$applet" = busybox ] || ln -s busybox "$root/bin/$applet"; \
    done; \
    ln -s ../bin/busybox "$root/sbin/mdev"; \
    cp /tmp/init "$root/init"; \
    chmod 0555 "$root/init"; \
    modules="$(for requested in \
      dummy_hcd libcomposite usb_f_acm usb_f_hid cdc_acm usbhid hid_generic evdev; do \
        modprobe -S "$kernel" --show-depends "$requested"; \
      done | awk '$1 == "insmod" { print $2 }' | sort -u)"; \
    for module in $modules; do \
      relative="${module#/lib/modules/$kernel/}"; \
      mkdir -p "$root/lib/modules/$kernel/$(dirname "$relative")"; \
      cp "$module" "$root/lib/modules/$kernel/$relative"; \
    done; \
    find "$root/lib/modules/$kernel" -name '*.ko.xz' -exec unxz '{}' ';'; \
    find "$root/lib/modules/$kernel" -name '*.ko.zst' -exec unzstd --rm '{}' ';' 2>/dev/null || true; \
    depmod -b "$root" "$kernel"; \
    mkdir -p /out; \
    cp "/boot/vmlinuz-$kernel" /out/vmlinuz; \
    cd "$root"; \
    find . -print0 | cpio --null -o --format=newc | gzip -9 > /out/initramfs.cpio.gz; \
    sha256sum /out/vmlinuz /out/initramfs.cpio.gz > /out/SHA256SUMS

FROM debian:trixie-slim@sha256:020c0d20b9880058cbe785a9db107156c3c75c2ac944a6aa7ab59f2add76a7bd AS vmroot

ARG DEBIAN_FRONTEND=noninteractive
ARG DEBIAN_KERNEL_VERSION=6.12.96-1

COPY deployments/external-input/kernel-runner/vm-init.sh /sbin/humanymous-init
COPY --from=docker-runtime \
  /usr/local/bin/ /usr/local/bin/
COPY --from=docker-runtime \
  /usr/local/libexec/docker/cli-plugins/ /usr/local/libexec/docker/cli-plugins/

RUN set -eux; \
    printf '#!/bin/sh\nexit 101\n' > /usr/sbin/policy-rc.d; \
    chmod 0755 /usr/sbin/policy-rc.d; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
      "linux-image-amd64=${DEBIAN_KERNEL_VERSION}" \
      bash busybox-static ca-certificates jq kmod nodejs tar util-linux; \
    groupadd --system docker; \
    useradd --create-home --uid 12000 --shell /bin/bash humanymous; \
    usermod --append --groups docker humanymous; \
    chmod 0555 /sbin/humanymous-init; \
    rm -f /usr/sbin/policy-rc.d; \
    rm -rf \
      /var/lib/apt/lists/* /var/cache/apt/* /var/log/* /tmp/* \
      /usr/share/doc/* /usr/share/info/* /usr/share/lintian/* \
      /usr/share/locale/* /usr/share/man/*

RUN set -eux; \
    rm -f \
      /usr/local/bin/ctr \
      /usr/local/bin/dind \
      /usr/local/bin/docker-compose \
      /usr/local/bin/docker-entrypoint.sh \
      /usr/local/bin/docker-init \
      /usr/local/bin/docker-proxy \
      /usr/local/bin/dockerd-entrypoint.sh \
      /usr/local/bin/modprobe \
      /usr/local/libexec/docker/cli-plugins/docker-buildx; \
    kernel="$(basename "$(find /lib/modules -mindepth 1 -maxdepth 1 -type d | head -n 1)")"; \
    source="/lib/modules/$kernel"; \
    pruned="/tmp/modules/$kernel"; \
    mkdir -p "$pruned"; \
    modules="$(for requested in \
      configfs overlay bridge br_netfilter veth nf_nat \
      dummy_hcd libcomposite usb_f_acm usb_f_hid \
      cdc_acm usbhid hid_generic evdev; do \
        modprobe -S "$kernel" --show-depends "$requested"; \
      done | awk '$1 == "insmod" { print $2 }' | sort -u)"; \
    for module in $modules; do \
      relative="${module#"$source/"}"; \
      mkdir -p "$pruned/$(dirname "$relative")"; \
      cp "$module" "$pruned/$relative"; \
    done; \
    for metadata in modules.builtin modules.builtin.modinfo modules.order; do \
      [ ! -f "$source/$metadata" ] || cp "$source/$metadata" "$pruned/$metadata"; \
    done; \
    rm -rf "$source"; \
    mv "$pruned" "$source"; \
    rmdir /tmp/modules; \
    depmod "$kernel"

FROM alpine:3.22@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce AS vmimage

ARG GUEST_ROOT_HEADROOM_MIB=256
ARG MAXIMUM_GUEST_BASE_BYTES=201326592

RUN apk add --no-cache \
      coreutils=9.7-r1 \
      e2fsprogs=1.47.2-r2 \
      qemu-img=10.0.0-r1

COPY --from=vmroot / /guest-root/

RUN set -eux; \
    kernel="$(basename "$(find /guest-root/lib/modules -mindepth 1 -maxdepth 1 -type d | head -n 1)")"; \
    mkdir -p /out; \
    cp "/guest-root/boot/vmlinuz-$kernel" /out/vmlinuz; \
    cp "/guest-root/boot/initrd.img-$kernel" /out/initramfs.img; \
    find /guest-root/boot -mindepth 1 -maxdepth 1 \
      ! -name "config-$kernel" -exec rm -rf '{}' +; \
    test -r "/guest-root/boot/config-$kernel"; \
    used_mib="$(du -sm /guest-root | awk '{print $1}')"; \
    disk_mib="$((used_mib + GUEST_ROOT_HEADROOM_MIB))"; \
    disk_mib="$(( (disk_mib + 63) / 64 * 64 ))"; \
    truncate -s "${disk_mib}M" /tmp/guest-root.ext4; \
    mke2fs -q -t ext4 -U 6b8e7d20-536f-42f0-8b1a-000000000042 \
      -L humanymous-root -d /guest-root \
      -E root_owner=0:0,lazy_itable_init=0,lazy_journal_init=0 \
      /tmp/guest-root.ext4; \
    qemu-img convert -q -f raw -O qcow2 -c \
      /tmp/guest-root.ext4 /out/guest-root.qcow2; \
    base_bytes="$(stat -c %s /out/guest-root.qcow2)"; \
    [ "$base_bytes" -le "$MAXIMUM_GUEST_BASE_BYTES" ]; \
    printf '%s\n' "$disk_mib" > /out/guest-root.virtual-mib; \
    printf '%s\n' "$base_bytes" > /out/guest-root.allocated-bytes; \
    rm -rf /tmp/guest-root.ext4 /guest-root; \
    sha256sum /out/vmlinuz /out/initramfs.img /out/guest-root.qcow2 \
      > /out/SHA256SUMS

FROM alpine:3.22@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce AS smoke

RUN apk add --no-cache \
      qemu-system-x86_64=10.0.0-r1

COPY --from=guest /out/ /opt/humanymous-kernel/
COPY deployments/external-input/kernel-runner/run-qemu.sh /usr/local/bin/run-qemu
RUN chmod 0555 /usr/local/bin/run-qemu \
 && mkdir /output

ENTRYPOINT ["/usr/local/bin/run-qemu"]

FROM alpine:3.22@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce AS full

RUN apk add --no-cache \
      coreutils=9.7-r1 \
      e2fsprogs=1.47.2-r2 \
      jq=1.8.1-r0 \
      qemu-img=10.0.0-r1 \
      qemu-system-x86_64=10.0.0-r1

COPY --from=vmimage /out/ /opt/humanymous-vm/
COPY deployments/external-input/kernel-runner/run-qemu.sh /usr/local/bin/run-qemu
RUN set -eux; \
    chmod 0555 /usr/local/bin/run-qemu; \
    mkdir /output /seed /opt/humanymous-kernel; \
    qemu_sha="$(sha256sum /usr/bin/qemu-system-x86_64 | awk '{print $1}')"; \
    kernel_sha="$(sha256sum /opt/humanymous-vm/vmlinuz | awk '{print $1}')"; \
    initramfs_sha="$(sha256sum /opt/humanymous-vm/initramfs.img | awk '{print $1}')"; \
    guest_sha="$(sha256sum /opt/humanymous-vm/guest-root.qcow2 | awk '{print $1}')"; \
    guest_virtual_mib="$(cat /opt/humanymous-vm/guest-root.virtual-mib)"; \
    guest_allocated_bytes="$(cat /opt/humanymous-vm/guest-root.allocated-bytes)"; \
    printf '{"schemaVersion":"humanymous.kernel-runner-identity/v1","qemuVersion":"10.0.0","qemuBinarySha256":"sha256:%s","guestBaseSha256":"sha256:%s","guestBaseFormat":"qcow2","guestBaseVirtualMiB":%s,"guestBaseAllocatedBytes":%s,"kernelSha256":"sha256:%s","initramfsSha256":"sha256:%s"}\n' \
      "$qemu_sha" "$guest_sha" "$guest_virtual_mib" "$guest_allocated_bytes" \
      "$kernel_sha" "$initramfs_sha" \
      > /opt/humanymous-kernel/runner-identity.json

ENTRYPOINT ["/usr/local/bin/run-qemu"]
