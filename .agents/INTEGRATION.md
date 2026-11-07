# M11 agent-maintenance integration

Owner: agents-meta / rust-agents-m11. Accepted scope is roadmap1.2 M11; REA stays
reference-only. Current implementation is canonical Rust-era AGENTS/rules/skills,
six `.agents/agents` contracts plus persona pointers, retirement manifest and
`humanymous-agent-maintenance` under repository `skills/agent-maintenance`.

## Exact root Cargo delta

Add `skills/agent-maintenance` to root workspace members and remove only its
standalone `[workspace]` table. Keep package publish=false, edition2024 and
rust-version1.99.1. Root resolves Cargo.lock; dependencies are serde1.0,
serde_json1.0, regex1.12 semver (resolved1.13.1 in local lock), sha2 0.10.9.
The standalone lock is intentionally committed until integration; root should
retire it after workspace locked tests pass. No common Cargo files were edited here.

## Run and source boundaries

```text
cargo test --locked --manifest-path skills/agent-maintenance/Cargo.toml
cargo clippy --locked --manifest-path skills/agent-maintenance/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path skills/agent-maintenance/Cargo.toml -- --check
cargo run --locked --manifest-path skills/agent-maintenance/Cargo.toml -- check .
cargo run --locked --manifest-path skills/agent-maintenance/Cargo.toml -- routing .
```

Actual Windows result: 7 unit/4 executable fixture tests, no ignored; canon14
skills/6role pointers/no errors; deterministic routing40/40;92skill mirror drift
entries retained. External user skills20 hashed read-only, no edits/qualification.
Check/routing microsecond measurements are single warm local observations, not
a comparative skill-performance claim. Evidence: evidence:root/20261107051014.
The final source manifest and Rust logs are private attached evidence; public
results contain sanitized counts, identities and explicit limitations.
Use attached source-manifest-final.json/canon-check-final.json/routing-final.json
for final source. Earlier snapshots remain preserved; schema initialization first
failed Clippy and was corrected. Final clippy-final.txt and unit-process-final.txt
pass; the failed log remains separate.

## Remaining upstream eval dependency

modoo.today supplied codex-plugin-eval script SHA7430f009e37a556d1a96f512e31e508569297d35dbf1686766e6686fe48cfd3a
but its skill_bundle omits required shared files (owned-process, isolate,
cred-proxy, plugin-copy, options, runtime-contract, pricing and their dependencies).
WSL bwrap0.9.0-1ubuntu0.3 was acquired in the private evidence directory and a real
namespace probe passes. The exact upstream script --help fails before any login
or inference on missing shared/owned-process.mjs. Supply the complete versioned
marketplace/plugin package; do not mock shared files or use --isolation none.

The separate Windows CLI pilot is partial and unqualified: Codex0.166.0-alpha.6,
read-only sandbox, two baseline and one treatment answers,123450 observed tokens
after the100000 future-call stop bound. The skill body read was policy-rejected,
zero completed tool events. No positive paired delta; do not weaken policy or
claim a complete modoo eval. The specific subject model was not observed.
The treatment's historical automatic false grade is infrastructure on review;
the original result is preserved and current code emits null on blocked reads.

## Final transition (root only)

1. Migrate CI `sync-adapters`/legacy verify to the Rust gate and qualified provider
   adapter generation where still needed. Do not drop the adapter compatibility
   gate merely because the92drift entries are intentional during transition.
2. Migrate `.agents/workflows` role references to `.agents/agents`; keep old persona
   pointers until all old consumers are qualified. Operational scripts/hooks and
   foreign oracle retirement are separate outstanding Rust migration work.
3. Obtain complete upstream eval package and run two positive/one near miss,
   twice per arm under bwrap with bounded authorized allowance and runtime lock.
   Require actual discovery/read plus behavioral results, not claimed invocation.
4. After current chats release their old paths, qualify fresh provider discovery
   and only then remove manifest-listed obsolete mirrors. Never bulk-delete user
   global skills. Keep board/claims untouched and coordinate Git under its mutex.
5. Run whole-program final supply-chain/foreign-source/native/product gates.
   This component is not full M11 or full product completion.
