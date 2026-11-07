---
id: plan:root/20261107072600
title: Apache native module implementation and hosted qualification
status: active
created: 2026-11-07T07:26:00Z
updated: 2026-11-07T07:26:00Z
rel:
  governs: [crates/apache-adapter/**, adapters/apache/**]
  depends_on: [sot:root/apache-adapter]
---

# Apache native module implementation

Goal active; lane adapter-apache session rust-apache-goal claimed after root
071ccef authorization. Previous turn researched native hook ordering and shared
contracts; no implementation existed then. This checkpoint authors safe Rust
child/request lifecycle, meaningful pure tests, integration and pilot contract.

- [x] Read coordinate-sessions/design-sot/implement-sot-slice and current canon.
- [x] Read current authority/framework shared APIs without editing their owners.
- [x] Adversarial bypass/fork/pool/body findings reflected in owning SoT.
- [x] Safe child generation/MPM admission, complete bounded body and cancellation.
- [ ] Root workspace registration and local Cargo slot; pure units/fmt/Clippy.
- [ ] Apache integrity/authority/framework profile owned by corresponding owners.
- [ ] Single-consumption authority response and exact final request handoff.
- [ ] Approved minimal unsafe native ABI, generated binding provenance.
- [ ] Actual native module hooks/input replay/pool cleanup/child initialization.
- [ ] Source-bound free hosted Docker MPM and outage/cancellation/reload pilots.
- [ ] Spec check, owned attributed commit and integration evidence.

Current tests are NOT_RUN. No actual httpd, Docker, IPC or origin pilot ran locally.
Followup adds a detached atomic cleanup signal and !Send/!Sync host request owner;
six pure tests are now authored. Cleanup signal revocation is unit-contract only,
not actual APR cleanup qualification. Existing workspace/profile/slot remains
absent on direct current-source reinspection; no Cargo task was started.
No dependency lock, workspace, workflow or public docs changes are owned here.
Root now approved narrowly isolated native ABI, preserving safe-crate forbid.
ABI-SAFETY.md and upstream-lock.json record scope, host preconditions and source
pins before root graph integration. Generated configured closure/tool hashes
remain hosted not_run; no foreign executable source/string/template workaround.
User explicitly resumed this task to finish Clippy, coordinate ABI/slots and
continue through native implementation and actual hosted pilot. Goal API still
reports blocked; it offers no agent-side resume operation. Work proceeds under
the explicit resume instruction without fabricating a goal status change.
Initial6unit pass is root-reported, exact receipt pending; replay3units unrun.

## Native source follow-through after explicit resume

Native source now contains child-only private configuration acquisition, generated
module/command export, initial body guard, quick/fixup drift checks and final
handler consumption of shared BoundResponse. Replay callback bridges bounded
READBYTES/GETLINE/SPECULATIVE input into APR brigades; request-pool callbacks own
cleanup Boxes. Exact callback-thread identity is checked before context access.
The initial tuple is a dedicated HTTP/1 listener with explicit Content-Length,
no mod_cache, no subrequests/internal redirects and no virtualhost merge support.
Socket read timeout is capped by the original pending budget and restored on
scope exit; restoration failure marks the connection aborted.

This source has only been rustfmt/diff/spec checked, not compiled or executed.
There is deliberately no native Cargo manifest until root's graph window.
Generated ABI spelling/layout, hook phase behavior, proxy body replay, socket
timeouts, cancellation/reload and actual origin effects remain unproven. Source
review identified and fixed early response consumption: the authoritative ticket
now survives until final handler checks, preserving its shorter expiry.

Root's live integration chat remains active (observed via wait_threads snapshot),
but it has not assigned a Cargo slot or approved native dependency registration.
User-authorized coordination requests are queued there; no local process/IPC/
Docker/native build was substituted for the required hosted execution.

An owned Dockerfile.native build candidate now uses checksum-pinned upstream
release archives and existing repository-pinned Rust/Debian base images. It
builds all three MPMs and the opt-in native crate, with locked unit/Clippy gates.
It has not been built and is not yet a pilot entrypoint. Runtime USER is65532;
lab must provide isolated network-none execution and private provisioning.
