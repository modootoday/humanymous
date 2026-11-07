---
id: evidence:root/20261107100600
title: Current CLI scoped checks with stable working dependency closure
status: active
created: 2026-11-07T10:06:00Z
updated: 2026-11-07T10:06:00Z
rel:
  depends_on: [sot:root/cli-installation]
---

Root allocated B with exclusive target/validation-cli, one Cargo job and
incremental disabled. Locked offline all-target check and all-target Clippy with
warnings denied both returned zero. The reviewed runtime_host:: library subset
passed seven tests, failed zero, ignored zero and filtered fifteen. No actual
service, account, network or GUI operation was executed by these commands.

Rust was actually 1.99.1, host x86_64-pc-windows-msvc. The recursive provenance
helper captured 366 files and 119 resolved dependency nodes before and after.
The working manifest stayed
98ff05fd62908b076b5ec3af2aa13690bef2bdffb0e90c40c4aa950b1a2cb58d.
Recorded HEAD advanced from f05a7f9683028cbed17b6490e469ec745233ba6d to
e691b971c941725c1616cd9f668d020896e05cc1 without changing that closure.
CLI source remains the implementation checkpoint2679089e. This proves a stable
observed working dependency closure, not an entirely clean single Git revision:
canonical comparison retained mismatches in native-security and test-support
INTEGRATION.md and six scoring fixture JSON files. Their bytes did not change
during validation. The metadata graph explicitly includes a feature superset;
registry source bytes are not independently inventoried by this helper.

Private bundle20261107100500 retains complete source captures and raw stdout and
stderr for every command. Its label is opaque; observed execution started near
10:02:38UTC. Admission was31.51GiB free and83.20percent committed; subsequent pure
and Clippy admission stayed below85percent and above20GiB free. B was returned
after the after-capture completed. No owned validation process remains.

Actual hosted runtime install/upgrade/rollback/replay/stop/removal, protected
service input and all13 receipt facts remain unexecuted for this source. The
production observer bootstrap channel and authenticated external witness producer
remain separate open dependencies. These scoped results do not qualify them or
the full product, and do not supersede historical failed/interrupted raw evidence.
