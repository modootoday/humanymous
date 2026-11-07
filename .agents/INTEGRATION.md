# M11 agent-maintenance integration

Owner: agents-meta / rust-agents-m11. REA remains reference-only. Current code
checkpoint is `59c80426`; earlier `1714b27` and `9c3b735` records remain historical.
Shared Cargo/lock/toolchain and final candidate integration belong to root.

## Current source and evidence

- Developer canon: `AGENTS.md`, `.agents/rules`, `.agents/skills`, `.agents/agents`.
- Product-user skill: `skills/SKILL.md` and `skills/references`, separate from internal guides.
- Rust tooling: `skills/agent-maintenance`, already a shared workspace member.
- JSON lane catalog: `.agents/sessions/lanes.json`; default writer mode remains legacy-coexistence.
- Product suite: three cases/twelve subject runs. Developer suite: five cases/twenty runs.
- Latest allocated component batch:36pure passed, zero failed/ignored/filtered, all-target check/Clippy and fmt passed. Private developer-before/after hashes match.
- Public evidence: `evidence:root/20261107073348`; exact logs, source hashes and original artifacts: `evidence:root/20261107073349`.

Do not re-add a standalone workspace/lock or edit common Cargo files. The original
seven-unit/four-process Windows pilot and forty-case routing proxy belong to
historical evidence51014/51015; they do not qualify current source or model behavior.

## Remaining execution gates

Local Cargo work requires one of at most four root-assigned distinct-target
slots A/B/C/D, jobs1/incremental0 and the shared resource guards. Admit only at
commit<85%/freeRAM>20GiB; defer and reduce subsequent allocations at commit>=88%
or freeRAM<16GiB. GPU inference remains separately allocated. The pure command is:

```text
cargo test --locked -p humanymous-agent-maintenance --lib
```

Actual filesystem/process tests require the approved hosted source candidate:

```text
cargo test --locked -p humanymous-agent-maintenance --test process --test session_storage --test product_package
```

The current source has seven process, four storage and two package tests. They
remain unqualified. Run42355168729 at exact8132117 failed its Linux actor after
15/88harnesses; none of these targets appears in the executed list. Preserve the
failed total and missing coverage. A proposed independent scoped hosted step
must not turn a failed whole-workspace/product job into a passing one.

`skills/agent-maintenance/qualification-contract.json` is the single reviewed
source-only inventory for these three targets: exact names, counts and zero
failed/ignored/measured/filtered requirements. Consumers must compare per-target
discovery on the exact candidate before execution and bind each actual summary
to that target. Reject duplicate/missing/extra names or targets and internal
count disagreements. Read this file rather than maintaining another count list.
Its false qualification fields are intentional; the inventory is not a run receipt.

The two new process cases cover competing claim children and retained legacy
BOM claim ownership. They are authored but NOT_RUN during the root benchmark
quiet window. Delivery's exact component accounting must move from the earlier
11-case inventory to 13 on the same candidate; the delivery owner has been
notified. Do not relax exact accounting or inherit a prior-source result.

Read-only canonical checks and artifact staging are actual tool executions, not
pure tests. Respect their assigned execution scope and preserve receipts. The
historical `eval` CLI pilot is not the isolated upstream harness and must not be
rerun around its earlier policy denial.

## Paired measurement and local runtime

The full modootoday/marketplace skill-factory0.3.2 package is acquired at commit
`1c4f32595d9cf3f07a393b8c561b9ab3b7f94981`. Its Codex script SHA256 is
`7430f009e37a556d1a96f512e31e508569297d35dbf1686766e6686fe48cfd3a`.
The old missing-import failure is historical, not a current acquisition blocker.
Actual runtime lock, isolation, provider integration, traces and complete paired
behavior/discovery results remain open. Do not weaken bwrap or substitute a proxy score.

`stage-eval ROOT NEW_OUTPUT product|developer` creates separate fresh plugins.
`paired-check RESULT_JSON [product|developer]` validates distinct12/20run profiles.
Observed nonactivation/misrouting remains in discovery_failures and the behavior
denominator. Missing observations, baseline/cross-plugin contamination and invalid
judge/arm accounting refuse a delta. Accounting never proves source authenticity
or skill-caused improvement; actual plugin/model traces remain mandatory.

The user authorized autonomous no-API-cost setup and requested Rust instead of
Ollama. Pinned mistral.rs0.9.5 CUDA and official Qwen artifacts are installed in
a dedicated cache without global Codex configuration changes. The8B attempt
loaded CUDA weights but host commit reached89% above88%; only the verified owned
server was terminated. The separately allocated4B profile subsequently loaded
and bound localhost, but observed commit94% triggered the same owned-process
termination. Both profiles sent zero API/tool/paired requests. No server remains;
root forbids another load until static KV/workspace review. Initial smoke
is limited to loopback, one process, context4096/output256 and GPU budget0.60,
with preflight commit<85/freeRAM>20GiB and stop commit>=88/freeRAM<16GiB.
These setup results do not replace hosted product or original paired qualification.
The pinned upstream harness has fixed auth/config routing; local endpoint support
needs an explicit supported integration, never a hidden auth/sandbox workaround.

## Canonical transition

Root removed137 tracked generated vendor copies under the later explicit user
instruction. Do not regenerate mirrors or vendor stubs. Canonical roles are
required; surviving persona pointers are optional and validated when present.
Fresh provider discovery after cleanup remains open. Settings/hooks, user-global
skills and untracked state remain separately owned; no broad deletion is authorized.

Live claim inspection found annotations in legacy scope strings:
`scripts/ci/** (except windows-*)` and `.git/** (index/HEAD/ref writes only)`.
The Rust grammar deliberately refuses them. Do not strip annotations and broaden
ownership or edit another writer's claim. Owners must normally release/reclaim
explicit nonoverlapping scopes before coordinated exclusive-writer rollout.
Hosted ownership/atomicity/interop receipts and all-writer migration are required
before changing writer_mode. No force takeover or stale-lock deletion is implied.

Root owns final foreign-source/supply-chain/product gates and main integration.
No scoring changes, publication, paid calls, global user configuration mutation
or another lane's edits are authorized by this component contract.
