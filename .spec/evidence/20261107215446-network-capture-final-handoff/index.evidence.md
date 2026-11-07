---
id: evidence:root/20261107215446
title: Final Network Capture fixture and target handoff before Runtime freeze
status: active
created: 2026-11-07T21:54:46Z
updated: 2026-11-07T21:54:46Z
files:
  - {path: handoff.json, sha256: 4fbe3fcb16b00a2d5697adce134ec13ce301d3343a92efa31ade14c44d40d913, bytes: 11760, store: repo}
rel:
  depends_on: [sot:root/structural-development, plan:root/20261107142752, evidence:root/20261107203158]
---

# Final owner readiness for Root's six-context slice

Source remains at current paths. The attached52file inventory at HEADb5f06109
includes Root's latest Platform testing dependency, own fcdf6912 test imports and
3d87e17a formatting. No owned source is dirty, no process/target lease is active,
and this owner has no remaining authorized source edit before Root freeze.
Root's Runtime-selected test and403diagnostic settlement remain external freeze
prerequisites. No source move, target rename or common Cargo edit occurred here.

## Fixtures and unique targets

| Current source | Proposed unique Runtime target | Owned fixture destination |
|---|---|---|
| network/tests/tls_parity.rs | network_tls_parity | tests/fixtures/network/tls.json |
| network/tests/report_parity.rs | network_report_parity | tests/fixtures/network/reports.json |
| network/tests/monitor_parity.rs | network_monitor_parity | tests/fixtures/network/monitor.json |
| network/tests/h2_parity.rs | network_h2_parity | tests/fixtures/network/h2.json |
| capture/tests/hosted_transport.rs | capture_hosted_transport | None; transient rcgen credentials only |

If Root places test sources in tests/network/*.rs, each include becomes
../fixtures/network/<file>.json. These target/path names are an exact proposed
mapping for Root manifest confirmation, not a rename already performed.
All four fixture origin commits, original paths, Git blobs, SHA256 and sizes
are in the attachment; they remain project Apache-2.0 data. Move owned bytes,
do not regenerate or normalize. No outside-package fixture include or data
replica is required by Network/Capture. The separately approved Core/Collector
replicas belong to Authority/Gateway and must follow Root's provenance/equality
contract; this owner neither creates nor authorizes extra replicas.

There is no CARGO_BIN_EXE dependency or executable helper role in these targets.
Delivery's capture command currently names package humanymous-capture and target
hosted_transport in scripts/ci/delivery/src/components.rs. Root/Delivery must
update it to the agreed Runtime package/qualified target and preserve the exact
five required case names, count guard,180s budget, hosted guard and fullfalse.
The hosted-pilot feature/optional rcgen mapping needs Root's explicit manifest
mapping. One package ID must not grant these fixtures Gateway/Authority/native
helper privileges or make ignored cases an executed result.

## Remaining move edits and narrow visibility

Root's actual slice still needs external humanymous_network/humanymous_capture
imports mapped to runtime::network/capture, owning-context crate:: references,
rustdoc links, relative fixture includes, context-qualified targets and explicit
feature/dependency categories. Core and Platform consumer edits are already
committed; no compatibility shim or root alias is needed.

The five Network pub(crate) declarations remain Network-only: is_grease and
OrderedTime/type/new/check/commit (type plus three methods). Use
pub(in crate::network) or narrower; Capture has zero former pub(crate). Preserve
relative pub(super), private validated config fields and private socket-owner
fields. Capture->Network remains the only observation dependency direction;
Gateway consumes methods and cannot construct a handshaken owner or fleet proof.
There is no additional sibling visibility exception from this review.

## Required gates after coherent integration

Root allocates scoped fmt/check/strict Clippy, Network72units and all four parity
targets plus Capture pure units and hosted-feature fixture compilation. Validate
the NEW Runtime package's extracted archive inventory and compile selected tests
from that clean extracted package: every included fixture must be internal and
hash-equal to its producer/original. Normal workspace success is insufficient.
Actual five socket cases need their explicit boundary allocation; compiled or
ignored cases are not execution. Preserve external test access without exposing
private owner/config constructors. Native helper-role negatives, real fleet,
verified epoch/Runtime serving and Docker product remain separate gates.

Ownership remains with the current claim until Root's exact freeze/transfer.
The owner will narrow/release the transferred paths only then; no live resource
requires takeover. Historical72pure/5hosted source receipts stay immutable and
do not qualify a consolidated package by inheritance.
