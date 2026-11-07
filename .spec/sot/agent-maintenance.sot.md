---
id: sot:root/agent-maintenance
title: Rust agent canon and qualified skill evaluation
status: active
created: 2026-11-07T05:10:13Z
updated: 2026-11-07T05:10:13Z
verify:
  - cargo test --locked --manifest-path skills/agent-maintenance/Cargo.toml
  - cargo clippy --locked --manifest-path skills/agent-maintenance/Cargo.toml --all-targets -- -D warnings
  - cargo run --locked --manifest-path skills/agent-maintenance/Cargo.toml -- check .
  - cargo run --locked --manifest-path skills/agent-maintenance/Cargo.toml -- routing .
rel:
  governs: [AGENTS.md, .agents/rules/**, .agents/skills/**, .agents/agents/**, .agents/personas/**, skills/**]
  depends_on: [sot:root/rust-migration, sot:root/program-roadmap]
---

# Rust agent canon and qualified skill evaluation

## Workflow convention guide delta

Root checkpoint089a09a updates github-actions-ops, cut-release and its preflight
asset to reference scripts/ci/workflows.json, stable workflow identities, the
accepted local-unit/hosted-runtime policy, and separate npm/crates environment
bindings. The Rust read-only canon checker reported0errors/92mirror drift;
workflow-guides-canon-check.json is retained under evidence:root/20261106194410.
This source change has not received model-based skill evaluation. Existing eval
results do not qualify it. Active vendor mirrors remain untouched under the
AGENTS retirement exception; no recursive legacy sync or manual mirror copy ran.
GitHub environment preparation does not authorize tags, publication or token
exchange, and external registry trust remains unverified. The sole execution
policy stays in program-roadmap/delivery, not duplicated in operational skills.

## Rule

M11 agent-maintenance responsibility only; REA remains preserved reference and
is not extended. Canonical project rules/skills/review roles live in
`.agents/{rules,skills,agents}`. Repository-root `skills/agent-maintenance` is
Rust 1.99.1/edition 2024 maintenance/evaluation tooling, not a duplicate catalog.
User-global skills are observed as external inputs, never overwritten by a
project migration. Selection path and hash, not just a skill name, identify what
a runtime actually used.

AGENTS and implementation skills must require locked Rust tests, fmt/Clippy and
actual boundary pilots. Product e2e stays Docker-only. New foreign implementation,
test scripts, executable foreign strings in Rust or generated foreign templates
are forbidden. Existing oracle/upstream glue and operational session tools remain
explicit separately qualified migration debt. Do not remove working coordination
oracles before a replacement passes. Detection freeze, missing-signal fail-closed,
dual-control, motor/speed-free a11y, owned-lab targeting and T4 ceiling are unchanged.

The Go-specific rule filename is retained as a compatibility pointer. Six old
persona paths now point to their canonical agent contracts. Generated vendor
mirrors stay untouched while active chats depend on them. Root owns final
workflow/CI migration, qualified adapter synchronization and manifest-driven
retirement; do not run recursive legacy sync mid-session. The manifest is a plan,
not proof of deletion or a deletion executable.

## Why

The previous root gate and implement/Pass/e2e skills directed new work toward Go
and legacy JS harnesses despite the accepted Rust-only program convention. The
change redirects current implementation to Cargo/crates and marks legacy recipes
as oracles. It does not infer full Rust product readiness from component success.

The standalone Rust tool provides explicit canonical path/metadata/LF checks,
content hashes, duplicate/name validation, role-pointer checks, read-only mirror
drift, opt-in trusted-checkout CRLF normalization, external-skill hash observation,
and deterministic intent routing. Symlinks and bare CR are refused during
normalization. This is not a hostile-directory race-resistant publisher. It never
rewrites mirrors or global skills. The root workspace is unchanged pending its
owner's integration; the tool has a locked independent workspace.

The separate real Codex CLI pilot uses fresh OS-temp roots, native read-only
sandbox, inherited login without reading credential files, ignored user config,
CLI default model (no override) and no user/project rule bypass. It caps 12 calls,
two runs per arm, 90 seconds per process and 100,000 observed input/output tokens.
The cap stops future calls, not in-flight usage or provider billing. It does not
implement bwrap/plugin installation, judge infrastructure, guaranteed process-tree
containment or a full plugin harness. Never promote its success to those claims.

Eval cases precede measurement: real-peer authority implementation after mock
success; WASM collector implementation after native-only units; conceptual TLS
explanation without implementation as a near miss. Behavioral graders are shared
between arms; version, locked Cargo verification and actual boundary evidence are
requirements, while claimed invocation and observed SKILL.md read stay separate.

## Verification

### Tool contracts and change state

| Contract | State | Version and data/error/lifecycle |
|---|---|---|
| `check ROOT` | implemented; Windows process-verified | schema_version1; files[{path,bytes,sha256}], skills, role_pointers, errors[], mirror_drift[], elapsed_micros. Read-only; exit0 iff no structural/LF/invariant errors, exit1 for measured failure, exit2 for I/O/usage/setup. Drift is recorded separately during transition. |
| `routing ROOT` | implemented; fixture and40case verified | schema_version1; total, failed, rows[{query,skill,split,expected,actual}], elapsed_micros. Missing routing rule/empty corpus is an error, never an implicit negative result. No LLM claim. |
| `normalize ROOT` | implemented; actual fixture-verified | JSON changed-path array, explicit opt-in, trusted checkout only. Canon/compat/tool source allowlist; symlink/non-UTF8/bare-CR failure; no mirror/global writes. Preflight validates prospective text before content writes; disk-write failure can leave partial normalization and must be reported. |
| `observe-skills PATH` | implemented; actual user path observed | Array of {relative path,bytes,sha256} for SKILL.md; no body/config/auth output and no mutation. Symlink tree refused. Observation is not provider qualification. |
| `eval ROOT CLI NEW_OUTPUT SUITE` | implemented; partial runtime pilot, unqualified | schema_version1; runtime/version, sandbox, bounds, rows; rows distinguish process_ok,timed_out,structured_answer,nullable quality,claimed_skill,observed_skill_read,skill_read_blocked,tools_observed,timing,tokens,trace_sha256. Creates output only if absent; ephemeral fixture cleanup on ordinary return; per-CLI-process timeout does not guarantee descendant-tree containment. Current code stops on infrastructure/policy failure or token cap; existing raw older result is immutable. |
| Root Cargo/workflow/CI/mirror change request | proposed to root, not yet integrated/qualified | Exact delta in `.agents/INTEGRATION.md`; no assumed acceptance or completed foreign-tool retirement. |
| Full modoo plugin harness | blocked external dependency | Original script preserved by hash; missing complete shared/plugin package prevents startup. Independent pilot cannot resolve this status. |

Version1 is a local maintenance-output contract, not a public network service or
new product authority. No listener/admin privilege is introduced. New fields are
additive within version1; changed meaning/exit policy requires a version change
and fixture review. Consumers must tolerate mirror drift as an observation only
during this explicit transition, never as permission to drop the final CI gate.

Windows actual executable fixture tests (4) and unit tests (7) pass; locked Clippy
and fmt pass. Canon check observes 14 skills/6 pointers with zero structural/LF
errors. Deterministic routing passes 40/40; it is not an LLM activation result.
The observed 92 skill mirror differences are transition observations, not ignored
hard failures or a completed adapter migration. Twenty user-global skill files
were hashed read-only; their compatibility/performance is not qualified.

Named upstream eval was discovered through modoo.today tools: without a runtime
verification filter, `modootoday/skill-factory/codex-plugin-eval` is returned.
The verified-runtime-only ordering did not expose it. Script SHA-256 is
7430f009e37a556d1a96f512e31e508569297d35dbf1686766e6686fe48cfd3a.
The catalog bundle contains only SKILL.md and that script, not the required shared
plugin files. WSL bubblewrap 0.9.0-1ubuntu0.3 was provisioned privately; a real
read-only-root/user/pid namespace probe succeeds. The upstream help invocation
still fails before inference with ERR_MODULE_NOT_FOUND for shared/owned-process.mjs.
The complete shared/plugin package is an external dependency; no fake replacement
or --isolation none fallback was used.

The separate Windows Codex 0.166.0-alpha.6 pilot produced two baseline answers and
one treatment answer, then stopped at its observed-token cap (123,450 tokens,
including the completed in-flight call). Treatment claimed the skill but its
Get-Content SKILL.md tool was policy-rejected, with zero completed tool events.
Thus no verified body read and no valid completed paired quality delta exists.
The old pilot's automatic quality=false treatment is overridden in interpretation
as infrastructure, not silently rewritten. Current code records blocked reads as
null quality and stops. Model ID was not observed; no named-model comparison is
claimed. No additional inference was run to evade the denied action or token cap.

Required follow-through is still open: obtain the complete upstream plugin,
capture its runtime lock and allowance without exposing credentials, run all
three cases twice per arm under the prescribed bwrap harness, record actual model,
discovery/reads, quality and usage, and improve only from valid paired evidence.
Root must also qualify fresh provider compatibility, migrate CI sync and integrate
the maintenance crate before final retirement. This slice does not complete M11's
whole supply-chain inventory or product gates. See evidence:root/20261107051014
and `.agents/INTEGRATION.md` for source-bound commands and residual dependencies.
