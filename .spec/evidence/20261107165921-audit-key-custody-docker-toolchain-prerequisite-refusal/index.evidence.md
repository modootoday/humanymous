---
id: evidence:root/20261107165921
title: Audit key custody Docker toolchain prerequisite refusal
status: active
created: 2026-11-07T16:59:21Z
updated: 2026-11-07T17:01:33Z
files:
  - {path: facts.json, sha256: 94b669e577b1993381b04bdd253de06403aa88051dd4ceb2a7bed59412686718, bytes: 646, store: repo}
---

# Audit key custody Docker toolchain prerequisite refusal

## What was measured

Root allocated a15-minute Docker Desktop desktop-linux prerequisite/execution
window for frozenf9b0cda3680c3b6d612cac8ace22e237812ed3cb. Actual existing image
inventory had no Rust image identity. The pinned available Ubuntu candidate
sha256:62d572b92f9f32d3427b6d220ad1f9dca9c7b6ffad37d295425037dbff78abaf,
linux/amd64, had no rustc/cargo in PATH or checked canonical locations under
UID1000. Toolchain prerequisite therefore FAILED before filesystem admission,
compilation or tests. The shell probe's exit0 is not a compiler/test pass.

One new owned container used2CPUs/4GiB, network-none, read-only, cap-drop-all,
no-new-privileges and ordinary UID1000. Terminal inspect matched the image and
limits; owned removal returned0 and the exact-name inventory was empty. No
foreign containers, image pulls, installs, volumes, target lease or WSL commands.
The allocated local window was returned immediately after cleanup.

## How to read it

Selected tests0/13 EXECUTED; actual compiler probes0/12 EXECUTED. Filesystem
capability NOT RUN because the earlier toolchain prerequisite was unavailable;
no filesystem-unsupported claim follows. Allf9b native/type/capacity/product
gates remain open. Exact raw records and concrete build-only standard GitHub
Linux requirements are in private evidence:root/20261107164818. Delivery received
the source/selector/probe/FS/permission/accounting contract and acknowledged it;
Root owns eventual source push and dispatch. This is not a hosted run/result.

A private evidence timestamp conversion initially yielded an invalid+9h elapsed
value. It was preserved separately and corrected from the literal UTC timestamp;
it is not process-runtime evidence. This did not start or restart workload.
