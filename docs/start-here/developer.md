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
At the Platform integration checkpoint `89715c04`, 34 workspace packages remain;
the eight-package migration is not complete and registry availability is not
claimed. Core now lives at `crates/sdk/humanymous-core`, with signals, behavior,
model, scoring, integrity, challenge and watermark as contextual modules.
Platform lives at `crates/integrations/humanymous-platform`; its optional
`testing` and `windows-object` features are separate and disabled by default.
The Windows object API additionally requires Windows. Neither feature grants
provider authority or makes a fixture into a production observation.

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
cargo test --locked -p humanymous-cli -p humanymous-native-security
cargo test --locked -p humanymous-native-cli
cargo build --locked -p humanymous-collector --target wasm32-unknown-unknown --release
```

The collector's native tests cover pure logic; a WASM build does not execute
browser APIs. The actual Firefox component lab is Docker-only and has a separate
result from full session/collector/Pass integration. Read the
[collector reference](../reference/browser-collector.md) before interpreting it.
