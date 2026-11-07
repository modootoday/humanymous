---
id: plan:root/20261107052000
title: Rust-only delivery completion and exact-source pilot integration
status: active
created: 2026-11-07T05:20:00Z
updated: 2026-11-07T05:20:00Z
rel:
  depends_on: [sot:root/delivery, sot:root/program-roadmap]
---

# Rust-only delivery completion and exact-source pilot integration

## Scope and order

Workflow naming follow-up after separate pinfixec14b90: normalize display
namespace only; keep all10technical filenames/jobkeys/OIDC/registry paths stable.
Add single workflows.json catalog + actual Rust inventory/name/job/concurrency/
artifact validation;9purepolicytests and staticpolicy/actionlintpass. File rename
ci-* migration deferred with rationale; existing artifacts not attempt-unique
remain named debt. No security gate removed or broad paths classifier introduced.

First actual free-hosted run42347732666 attempt1 bfc9f623 completedfailure:
22deliverytests/policy passed, action-pins authenticated aquasecurity public
commit API HTTP403 due orgIPallowlist. Heavy lanes not_run/ready failed asrequired.
Follow-up removes GH_TOKEN from pin lookup and validates exact public commit
identity with unauthenticated HTTP, retaining all failure/security gates and
deduplication; no retry or paidfixedIP. Root confirmed this least-privilege design.
After no active Cargo and ~32GiB physical available were observed, bounded pure
policy units use buildjobs1; no localruntime/Docker. Corrected hosted runpending.
Bounded policy7unitspassed,16unrelatedtestsfiltered explicitly; rustfmt,
actionlint and diffcheckpassed. Do not call the full23test suite executed here.

Latest authorization supersedes design-only remote restriction: apply free
standard public hosted validation; root alone integrates/pushes/starts first run.
Existing workflows now serialize heavy validation, pin Ubuntu24.04, disable
default branch artifact/cache storage and suppress duplicate codex/rust component
pushes. Only static actionlint/diff checks performed locally, no new Cargo/Docker.
Source closure before first CI: common27member Cargo/lock (root), lab runner+
runtime+Compose/witness/collector producer checkpoint, Node/native/authority and
current collector source commits, delivery checkpoint. Missing members or stale
lock are real failures, not fetched from uncommitted local private files.

Cross-chat policy consultation requested after this draft: full serial chain is
provisional, not the final DAG. Its drawback is independent results becoming
not_run after an early failure. Prefer dependency/impact classification with
bounded heavy concurrency and strict final aggregation, subject to an explicit
scheduler/policy update and actual evidence. Preserve required release gates.
Local fast checks must separate pure unit logic from runtime-spawning tests;
hosted standard VMs do not prove real interactive AT/SCM/CEF capability by label.
No existing source or required check is deleted for this consultation.

Follow-up consultation direction now implemented as two hosted lanes behind
policy, with exact failure-independent scheduling predicate and strict ready
aggregation; see delivery SoT correction. bdff906 is the local serial draft,
not the final source to push alone. Source closure now includes Nodeedf83b7 and
lab9a41235; latest producer execution stillnot_run, root common lock pending.

Resource triage update2026-11-07: heavy new execution is held; project-wide one
slot awaits root allocation. P0 runtime+collector implementation, P1 adapter
source checkpoints, P2 remote CI/Compose design, P3 sequential full reruns.
No process kill, new agent, cloud creation, deployment or remote dispatch.
CI owner now owns GitHub Ubuntu source/evidence design and top-level cost
comparison; Compose owner owns Containers security/control-plane gaps. See the
delivery SoT remote delta. Official docs inspected and repository PUBLIC checked;
no actual remote capacity/equivalence or raw-evidence transport test performed.

The user reauthorized completion across existing project chats. This lane owns
workflows, non-Windows delivery CI tools, migration tooling, build/rust contexts,
delivery canon and its own evidence. Root alone owns common Cargo/lock/toolchain;
Windows/native, Compose, runtime, Node, Pages and other product owners retain
their responsibilities. Do not duplicate their runners, rewrite other lane's
sources or call a component-only checkpoint whole-product completion.

1. Record exact terminal outcome for full CI42339951733/f32042a.
2. Replace authored Python delivery implementations/tests with contextual Rust.
3. Root-register tool; integrate owner native, Docker/browser and Pages APIs.
4. Verify real source-bound components and keep unavailable product cases blocked.
5. Complete package/docs/trust/supply-chain gates, preserve external prerequisites.
6. Validate LF/Git bytes, tests/Clippy/actionlint/spec; commit own paths only.
7. Continue actual remaining dependencies or leave exact blocked handover; no publish.

## Execution checklist

- [x] Read program-roadmap v1.2, current sources, ownership and integration docs; claim e2e-infra.
- [x] Observe same run to completed/failure, collect exact run metadata/log/artifacts; retain terminal log digest.
- [x] Implement Rust command/source/policy/package/registry/pilot/remote/evidence modules; initial14unit tests pass. Own preliminary89dependency lock audit has no known findings; final common-lock scan is separate.
- [x] Communicate native all-OS/actor, Compose immutable source/private bundle, Pages native build, runtime binary names and Node generated-glue/receipt boundaries; persist accepted/proposed state in delivery SoT and scripts/ci/INTEGRATION.md.
- [x] Replace generic delivery Python calls in CI/release with Rust; Pages invokes proposed owner Rust contract. Actual owner qualification remains pending.
- [x] Root registered tool/common589-package lock; removed temporary workspace/child lock after preserving exact private original and SHA256. Locked16tests/Clippy/policy passed on the common lock, not just the preliminary isolated closure.
- [x] Removed the temporary independent workspace declaration and requested root member/common-lock registration. Preliminary16tests/Clippy/pins passed; common-lock qualification remains pending.
- [ ] Finish Rust parity/negative tests; migrate legacy authored delivery.py/check_action_pins.py/test_delivery.py after actual Rust interface validation.
- [x] Retired all three generic authored Python delivery files after Rust policy/pin/negative checks.19tests/Clippy passed before checksum-resume addition; final20-test recheck waits for the latest owner manifest/root-lock synchronization.
- [x] Added same-run Cargo archive proof/checksum-verified immutable resume, Node validated tgz/receipt binding and strict two-platform image/source/SPDX/OIDC checks. Live publication remains unverified and unauthorized.
- [ ] Owner native runner build/process/standard-user lifecycle qualification; wire actual Windows actor mode only after delivery, preserve known ignored helper accounting.
- [ ] Owner lab runner default actual runtime plus collector Docker execution using private Specfold bundle; retain component counts/image/source commitments and full missing IDs. Do not substitute topology-only pass.
- [ ] Replace narrow Docker context with allowlisted workspace member closure; real image build after root23member expansion and follow-up members. Ensure legacy advanced.js oracle is available only to test collector image.
- [x] Updated four Rust build contexts for current27-member roots (crates, docs/tooling, CI tool, skills/agent-maintenance, test/lab runner/runtime); excluded targets/caches/private/credential files and fixed advanced.js test oracle omission. Runtime pilot now uses the shared root target path. Actual refreshed image build remains pending owner readiness.
- [ ] Pages owner real two-baseurl Rust builds, reviewed exact source/doc digests. Freshness is read-only and never auto-refreshed by CI.
- [ ] Native Node generation/inspect and Node/Bun/real authority receipts, including target-qualified artifact digests and package/provenance evidence; no legacy-only npm release readiness.
- [ ] Complete maintained Cargo locks + Ruby/npm/image scans, preserve database time and warnings, maintain CVE residuals and immutable SBOM/provenance. Frozen Go findings are not patched by this lane.
- [ ] Add sanitized public source-bound summary through Specfold attachments; retain private raw all outcomes/failed attempts and per-command environment/counts.
- [ ] Root final runtime/Gate/Pass/human/AT/witness/source inventory and source cleanup gate; external registry bootstrap/trust/OIDC exercise. These remain blockers, not YAML completion.

## Current evidence and changes

Private bundle .spec.local/root/evidence/20261107052000-delivery-followup contains
terminal CI42339951733/f32042af9136560b41c1765f565c2e04fe373a22. Exactly16jobs,
4success/12failure. Windows actor report is implemented-crate scope and false
full_runtime_qualified; no other run's green was imported. Rust replacement
initial compile caught unavailable Result::is_err_or and Clippy collapse/new
time API warnings; changes are being corrected without suppressing checks.
Source is dirty during concurrent work; final manifests/command logs must bind
the actual validated bytes and later canonical Git bytes separately.

The first accepted Compose runner source encountered actual concurrent audit
missing-module compile failure and retained it; its subsequent checkpoint is a
different source. The default runner must execute actual runtime; topology-only
is not substituted. Runtime currently has no complete new full-product pilot.

Actual delivery attempts retained independently:052430 browser failed before
execution on common-lock update requirement;053622 browser compiled lab runner
then failed snapshot metadata lookup (deleted tracked child lock, fixed by owner);
053706 native compiled release artifacts then owning native harness rejected
its summary. New owner diagnostics later identify authority local_service exit101
on a different source; that does not rewrite this older opaque failure. Raw
before/after manifests, exits, logs and reports are attached privately. Uploaded
pilot summaries now omit raw actor identity/trace payloads. Common Cargo updates
and owner process/profile fixes gate the next actual retries.

## Acceptance and handoff

Owner update16c9e89: exact durable_witness/key_backups helper catalog committed,
runner7 tests/Clippy passed. Actual73-harness workspace retry still fails on
durable_witness9 nested child summaries even with process exit0. Await audit
producer repair then fresh execution; no green workspace status inferred.
Separate91f93 native14/14harness67passed/errors0/requiredignored1 remainsblocked2.

Latest local verification: delivery22 tests and strict Clippy pass; docs-review8
tests and strict Clippy pass. Generated `.build-cache` files are excluded from
filesystem inventory, but Git-tracked authored files cannot hide there. Pages
owner confirms the remaining missing reviews concern authored nginx sources.
Actions observation/2 now separates attempts before terminal writes and binds
artifact IDs to source and attempt time window; its negative source/window test
passes. Live observation/2 retrieval of42339951733 attempt1 succeeded into a
separate private directory, retaining its failure conclusion, exact original
log hash and three source/window-matching artifact ZIPs. The subsequent addition
of archive hashes to terminal markers passes its targeted test and strict Clippy.

Windows source checkpoints7c9321a and91f93c5 add the workspace producer and
workspace public summary. Actual all-workspace qualification still fails on
new audit child-helper contract pending its owner; do not mark the migrated CI
entrypoint verified by the separate four-package native component results.

Use the exact commands in scripts/ci/INTEGRATION.md and owner INTEGRATION files.
Native actor/process success, Docker bounded fixture, collector browser success,
full product qualification and real OIDC publication are separate outcomes.
This plan stays active until its owned work and necessary actual verification
are finished; external/product dependencies remain named, never set green by
missing-runner placeholders. No main merge, tag, push or publish is authorized.
