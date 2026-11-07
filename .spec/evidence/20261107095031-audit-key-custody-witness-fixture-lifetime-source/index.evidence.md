---
id: evidence:root/20261107095031
title: Audit key custody witness fixture lifetime source
status: active
created: 2026-11-07T09:50:31Z
updated: 2026-11-07T09:50:31Z
rel:
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody witness fixture lifetime source

## Historical scope

Original governing path: `crates/audit/tests/durable_witness.rs` in the former
humanymous-audit package. Initial evidence/source-recording commit:
54fe5b8b52c3884fe936811416b99c75517e227c. The follow-up source correction and
NOT_RUN compile/process status remain exactly as recorded below.
After Runtime relocation64f74c7a this retained evidence has no active path
governance; it is not measured lifetime proof for the moved Runtime fixtures.

## What was measured

Source review found synchronous read_frame, write_frame and child.wait in actual
witness service fixtures had no lifetime bound. The bounded helper capture used
elsewhere did not cover these direct service exchanges. Authored ChildLifetime
using existing ManagedChild private duplex spawn/kill/reap and an independent
60-second timer. Pipe I/O never holds its child mutex. Deadline stops only the
owned direct child; observed timeout returns TimedOut rather than an expected
nonzero service status. Drop cancels/joins its timer and stops/reaps its child.
Stderr is suppressed rather than left as an undrained private pipe.

WitnessProcess and the real common typed-client test now retain this guard.
One native regression starts the real witness without configuration, keeps its
stdin open, blocks on frame read and requires EOF plus TimedOut after a short
fixture deadline. No ignored entrypoint was added or renamed. Format and scoped
Specfold checks only; actual compile/process execution is NOT_RUN.

## How to read it

Follow-up source review caught one legacy wrong-password test accessing the raw
child stderr field after replacing its owner type. That test now uses existing
bounded capture_with_stderr for its one-shot configuration and still requires
actual nonzero exit, empty stdout and exact secret-free static stderr. Interactive
fixtures keep stderr suppressed. This correction precedes new compile evidence;
the first source checkpoint alone was not compile-qualified.

This is an absolute fixture lifetime, not a per-exchange protocol deadline or
production transport timeout. It relies on these binaries not spawning pipe-
holding descendants and on normal OS termination/reaping behavior. This does not
establish separate OS account/storage isolation. New source does not inherit
8132117 test results. The regression is authored, not a measured timeout result.
No new Cargo/runtime/Docker command ran during the benchmark quiet window.
