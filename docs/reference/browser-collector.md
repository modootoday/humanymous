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
| `probe_cdp_json()` | Explicit once-per-epoch synthetic console-preview observations, with separate attempted/unsupported/failed states |
| `stop()` | Freeze the summary, clear transient pairing/buffers and detach owned callbacks; failures remain visible |

Guard baseline includes bounded native/prototype observations and an owned
MutationObserver. Its script counter inspects directly added `SCRIPT` nodes;
it does not descend into inserted containers, and ignores scripts with a truthy
`data-hm` marker. It does not inspect script payloads. Complete hook
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

`probe_cdp_json()` performs active synthetic console operations only when called;
default construction does not run it. Its `console.groupEnd` call can change
developer-console grouping. Repeated calls return the cached attempt, and a
stopped collector rejects a new probe. An incomplete requested observation
rejects `snapshot_json()` rather than becoming a negative detection flag.
The result does not identify every CDP client or attest automation or human use.

The passive implementation does not read input text, clipboard values or
`Event.key`. Physical key codes are used transiently for epoch-local pairing and
are not exported as key names or traces. Clearing owned buffers is not proof of
cryptographic erasure from the browser or operating system.

## Reproduce a component check

The Rust `session::Session` API implements a separate volatile client state
machine. `from_json` checks a bounded strict bootstrap against independently
provisioned scope. `begin` reuses shared request canonicalization and allows
only one pending request. `complete` advances its counter only after a valid
consumption acknowledgement bound to the pending request and observed status;
bad acknowledgement, expiry or clock rollback closes the epoch.

This state machine does not perform Fetch, read a response body, own an Abort
signal or join the actual server wire. Its caller must invalidate on ambiguous
network/body/timeout/abort/drop outcomes and call `complete` only after bounded
body completion. The state API cannot independently observe those transport
events. An acknowledgement proves counter progress, not ALLOW, response-body
integrity or qualified collector provenance. Cookie-free transport, runtime
integration and accessible recovery remain separate work.

### Browser transport follow-through under source review

Source exposes WASM `BrowserSession` and `BrowserResponse` exports separately
from the pure `Session` state API. At canonical source `667e98f`, the Firefox
component suite passed 66 cases with no failures, ignored or filtered cases.
These include real browser Request/Response APIs with a synthetic Fetch fixture;
they do not establish the positive browser-to-runtime-to-origin join. The first
cleanup attempt failed to create a thread under its 256-PID container limit;
the recorded successful attempt used a 1,024-PID limit. These historical
component results do not qualify later source changes or ordinary human use.

| Working-source interface | Declared responsibility, not a qualification result |
| --- | --- |
| `BrowserSession.connect(scope_json, timeout_ms)` | Same-origin bootstrap against independently provisioned scope, with an 8 KiB response bound |
| `fetch(method, target, headers_json, body)` | One buffered signed Fetch operation using cookie omission, redirect refusal and no automatic retry; request body bound 64 KiB |
| `close()`, `is_closed()` | Explicit invalidation/abort and local epoch status; ignoring or garbage-collecting a JavaScript Promise alone is not cancellation |
| `BrowserResponse.status`, `headers_json()`, `bytes()` | Buffered status, normalized exposed headers and copied body bytes after complete-body/ACK processing; not a streaming Response or raw header capture |

The client source's response-body bound is 1 MiB, while the current runtime's
origin-response bound is 64 KiB. One does not enlarge the other's support.
Runtime source separately selects a cookie-free `browser_auth_only` profile,
bootstrap and consumed-counter ACK owner. Its origin-request builder preserves
application Authorization for this profile, and local/origin response paths
insert status-bound ACK headers. Native TCP fixtures at runtime source
`adb8e3ed` exercised the shared client/server model. They do not run browser Fetch
or qualify collector provenance. Neither those native fixtures nor the historical
Firefox component suite establishes a qualified browser-to-origin flow.

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
