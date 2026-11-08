---
layout: doc
title: Developer guide
permalink: /start-here/developer.html
reviewed: 2026-11-07
---
# Developer guide

**Diátaxis quadrant:** How-to. **Audience:** project contributors.

This branch replaces the implementation with Rust while preserving observable
contracts. Building an individual crate does not qualify the complete Gate,
browser flow or activated service installation.

The accepted design consolidates the workspace into eight public package roles.
At the Runtime integration checkpoint `64f74c7a`, 29 workspace packages and four
standalone qualification-tool packages remain: 33 maintained packages in total.
The planned CLI consolidation includes nine workspace packages and those four
standalone tools, accounting for 13 existing package responsibilities.
The eight-package migration is not complete and registry availability is not
claimed. Core now lives at `crates/sdk/humanymous-core`, with signals, behavior,
model, scoring, integrity, challenge and watermark as contextual modules.
Platform lives at `crates/integrations/humanymous-platform`; its optional
`testing` and `windows-object` features are separate and disabled by default.
The Windows object API additionally requires Windows. Neither feature grants
provider authority or makes a fixture into a production observation.

Runtime now lives at `crates/sdk/humanymous-runtime`. Its contextual features are
`gateway` (enabled by default), `authority`, `audit`, `capture`, `network` and
`native-security`; select the role explicitly for a narrow component build.
Features can enable required sibling contexts. Selecting a role does not qualify
its native service, browser flow or deployment.

## Isolated final-path map

The isolated assembly has reached the following eight physical package paths.
This map describes staged source ownership, not a live checkout cutover,
registry publication or supported deployment matrix. Existing live-checkout
commands below retain their own package and source scope.

| Package | Isolated path | Responsibility |
| --- | --- | --- |
| `humanymous` | `crates/sdk/humanymous` | Curated facade and build profiles |
| `humanymous-core` | `crates/sdk/humanymous-core` | Shared safe detection, integrity and model contexts |
| `humanymous-runtime` | `crates/sdk/humanymous-runtime` | Gateway, authority, audit, capture and network contexts |
| `humanymous-platform` | `crates/integrations/humanymous-platform` | Explicit testing and Windows object roles |
| `humanymous-collector` | `crates/integrations/humanymous-collector` | Browser/WASM collection |
| `humanymous-adapters` | `crates/integrations/humanymous-adapters` | Safe adapter contexts and separate host export roles |
| `humanymous-cli` | `crates/apps/humanymous-cli` | Applications and qualification tools |
| `humanymous-browser` | `crates/apps/humanymous-browser` | Native browser model, bootstrap and desktop roles |

The isolated facade passed four selected profile tests and strict Clippy after
its move. The isolated collector passed 26 library cases and one fixture contract,
plus WASM checks and strict Clippy. Selected source-pin and panic-classifier
consumer checks are separate from actual browser execution. Earlier snapshot
gaps are not repaired retroactively by a later unchanged-input observation.

## Contributing and checking a component

Read the repository's contributor instructions before editing. Keep security
decisions in the shared authority; adapters translate lifecycle events rather
than copying policy or cryptography. Test exact bytes, invalid inputs and failure
paths as well as successful requests. Preserve accessible recovery and separate
local administration from public routes.

Run the relevant locked workspace tests and record their scope. Full enforcement
claims require the isolated Docker suite and real deployment pilots after Rust
wiring. Documentation describes evidence already obtained and labels unfinished
work explicitly. See [how this is built](../explanation/how-this-was-built.md).

Useful source-bound component commands are:

```console
cargo test --locked -p humanymous-core --lib
cargo test --locked -p humanymous-runtime
cargo test --locked -p humanymous-cli
cargo test --locked -p humanymous-runtime --no-default-features --features native-security --lib
cargo test --locked -p humanymous-native-cli
cargo build --locked -p humanymous-collector --target wasm32-unknown-unknown --release
```

The collector's native tests cover pure logic; a WASM build does not execute
browser APIs. The actual Firefox component lab is Docker-only and has a separate
result from full session/collector/Pass integration. Read the
[collector reference](../reference/browser-collector.md) before interpreting it.
