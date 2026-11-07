---
layout: doc
title: Rust browser collector API and limits
description: Implemented observation exports, epoch ownership and qualification gaps.
permalink: /reference/browser-collector.html
reviewed: 2026-11-07
---
# Rust browser collector API and limits

**Diátaxis quadrant:** Reference. **Audience:** browser integrators and evaluators.

The Rust/WASM collector reads actual browser APIs and has source-bound component
tests. It is not yet a qualified, authenticated collection-to-origin flow.
No observation establishes universal natural-person presence.

| Export | Current responsibility |
| --- | --- |
| `detect_json()` | Stateless browser capture and partial detection report; capture failures return an error |
| `environment_json()` | Explicit bounded first-party environment/privacy observations, without session attestation |
| `advanced_json()` | Optional advanced observations, separate from a qualified session report |
| `BrowserCollector` | Own one passive-listener epoch, partial guard baseline and script-mutation observation |
| `behavior_json()` | Reduced behavior features; no input text, key identities or ordered raw samples |
| `snapshot_json()` | Combine browser observations, measured behavior and partial guard state |
| `enable_guard_hooks()` | Explicitly opt in to managed `eval`/`Function` forwarding hooks for this collector epoch |
| `stop()` | Freeze the summary, clear transient pairing/buffers and detach owned callbacks; failures remain visible |

Guard baseline includes bounded native/prototype observations and an owned
MutationObserver for relevant script changes. Complete hook
coverage is not established merely by collecting a baseline; the current report
retains `reported: false` when required hooks are missing. A caller must not
promote that into complete instrumentation or authenticated startup provenance.

The default constructor does not replace `eval` or `Function`.
`enable_guard_hooks()` is an explicit compatibility choice: forwarding direct
`eval` changes its lexical-scope behavior. Test that change against the host
application before opting in. Hooks reject overlapping ownership, retain proxy
callbacks while installed, and restore only references they still own when
stopped. Measured opt-in hook coverage is distinct from the default partial
baseline; neither mode attests code integrity or human presence.

The passive implementation does not read input text, clipboard values or
`Event.key`. Physical key codes are used transiently for epoch-local pairing and
are not exported as key names or traces. Clearing owned buffers is not proof of
cryptographic erasure from the browser or operating system.

## Reproduce a component check

Request-signing byte codecs and canonicalization are being integrated separately.
A pure signing vector or WASM build does not establish authenticated bootstrap,
session transport or a recovered browser request. Do not infer those from the
collector exports listed above.

```console
cargo test --locked -p humanymous-collector
cargo build --locked -p humanymous-collector --target wasm32-unknown-unknown --release
docker compose -f deployments/lab/compose.collector.yaml --profile collector build
docker compose -f deployments/lab/compose.collector.yaml --profile collector run --rm --no-deps collector-browser
```

The Docker lab uses actual Firefox/WebDriver with isolated networking and no
sandbox-disabling flag. Headless/WebDriver observations are not an ordinary
human baseline; successful component tests do not establish the browser's
complete sandbox policy. Retain source/image identity, every failure and skipped
case. The full loader, session-bound transport, request integrity, recovery and
ordinary-browser/assistive-technology controls have separate acceptance gates.
See [accessible recovery](../help/challenge-accessibility.md).
