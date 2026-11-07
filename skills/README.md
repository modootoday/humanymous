# Product skill and repository maintenance tools

`SKILL.md` plus `references/` is the product-user skill for installation,
operation and supported adapters. Internal developer skills remain
`.agents/skills/*/SKILL.md`; do not copy them into the product skill or a user's
global directory. `agent-maintenance/` is a Rust 1.99.1/edition 2024 Cargo tool
integrated into the root workspace. It checks canonical paths, LF bytes, descriptions, duplicate
names, hard-constraint markers and observes compatibility mirror drift read-only.

User-scoped skills (for example the operator's `.agents/skills` directory) are
external inputs. Do not overwrite/uninstall them or assume a repo skill with the
same name is the effective provider selection. Record selected path and content
hash during a real runtime eval. No user auth/configuration changes are required.

Run `cargo run --locked --manifest-path skills/agent-maintenance/Cargo.toml -- check .`
and `... -- routing .`. Routing is an explicitly deterministic regex proxy over
the existing dataset, not model skill activation. `eval ROOT CLI NEW_OUTPUT SUITE`
is a separate bounded real Codex CLI pilot (12 maximum calls, two runs per arm,
90 seconds per process, stop at 100,000 observed input/output tokens). It uses
fresh OS-temp fixture roots, ignores user config, retains authentication through
the CLI, never reads auth files, forces read-only sandbox and refuses an existing
output directory. Preserve outputs in private Specfold evidence only.

The CLI pilot does not implement the upstream plugin harness's bwrap boundary,
marketplace/plugin installation or judge protocol. Do not label it a successful
modoo codex-plugin-eval suite. Token bounds stop future calls, not billing or
in-flight usage. Default provider model is retained. Claimed skill names,
observable SKILL.md read commands, process success and answer quality are separate.

`normalize ROOT` explicitly normalizes CRLF to LF only in canonical skills/rules/
agents text, refusing symlinks and bare CR; it never writes mirrors or user skills.
`observe-skills PATH` reports user skill path/byte/hash identity read-only. These
maintenance operations assume a trusted checkout, not hostile concurrent directory
replacement. Product erasure/dual-control policy is unrelated to text normalization.
