---
id: evidence:root/20261107134100
title: Final report deadline and hard-expiry fixes with provisional scoped checks
status: active
created: 2026-11-07T13:41:00Z
updated: 2026-11-07T13:41:00Z
rel:
  depends_on: [sot:root/cli-installation]
---

The390cc552 ordering checkpoint keeps final stdout serialize/write/flush and final
SCM status publication under the original stop watchdog, outside the state lock.
This follow-up validates the original monotonic deadline at both validation and
completion; equality or late delivery cannot commit supervisor Stopped even if
the watchdog thread has not run. The hard-expiry path no longer writes stderr
before exit74. No new deadline, detached logger or unsafe code is introduced.

Root allocated warm D, exclusive target/validation-cli, jobs1/inc0 and locked
offline Cargo. Observed admission was31.60GiB free and81.13percent committed.
After the stderr change, all-target check, nine reviewed pure runtime_host tests
and all-target warnings-denied Clippy returned zero. Exact test accounting:
9passed/0failed/0ignored/15filtered. Tests inject before/during-delivery expiry
without sleeps or threads and preserve Draining plus the original deadline.
Initial source capture and successful check preceded the stderr follow-up and
remain separate raw attempts, not newest-source qualification.

Full dependency closure CHANGED during these successful commands. Before02 was
7329ab4ee358928ed975733a467b90b732b69e00bf9d7ee1e7057fcc3cd0e951;
after was d6dd418ffb0128ceefd1ecef9122cd1f346c57ce8915e65e1213d7459061983b.
The after snapshot inventories409files/142dependency nodes. Changed paths were
runtime/qualification-contract.json, runtime/src/service.rs,
runtime/src/service/observed.rs, runtime/src/service/observed/retained.rs and
runtime/src/service/observed/retained/tests.rs, all below crates/. CLI and common
Cargo inputs stayed unchanged. HEAD moved66c59c59692f21ac0ef19eb0f1d3a917292b8952
to6a3678eaa45c1f49f362ce7f92ea79c98f0b65a8. Canonical comparison also retains
working mismatches and untracked runtime modules; no clean immutable-source or
stable full-closure pass is claimed. Root/runtime owners were notified; D was
returned without an automatic rerun loop.

Private bundle20261107140000 retains all raw captures, warnings and outputs with
attachment hashes. Its numeric label is opaque; execution began13:37:41UTC.
Scoped fmt/diff/Specfold passed. These are provisional component observations,
not actual blocked stdout/stderr process-boundary proof. That native pilot and
actual hosted service/SCM lifecycle remain NOT_RUN; no local service/account
change, foreign process termination or product qualification occurred.
