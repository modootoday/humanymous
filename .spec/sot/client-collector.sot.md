---
id: sot:root/client-collector
title: Rust browser signal collection and generated WASM boundary
status: active
created: 2026-11-07T01:30:00Z
updated: 2026-11-07T01:30:00Z
verify:
  - cargo test -p humanymous-collector --locked
rel:
  governs: [crates/collector/**]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application]
---

# Browser collector migration

Port frozen internal/detect/{l1_static,l2_fingerprint,l3_integrity,collect,bridge}.go
without changing IDs, predicates, notes, verdict/confidence or registry weights.
Use one pure snapshot evaluator and one WASM browser I/O boundary. Reuse shared
Signal/ClientReport and registry constructors; never fork scoring. Identify the
new implementation as rust-wasm-0.1.0 rather than claiming Go's runtime version.

Browser access is authored Rust through pinned wasm-bindgen0.2.130/js-sys0.3.107
safe public APIs. Disable js-sys unsafe-eval feature. No inline_js, eval, Function
constructor, hand-authored JS loader or script strings in Rust. Standard upstream
generated binding/test glue is an external tool output with version/hash provenance.
Test runner Chromium/Edge defaults that append no-sandbox are prohibited; use a
runner/browser configuration without disabling sandbox or report it unexecuted.

Preserve JavaScript truthiness versus property existence, case-sensitive UA
checks, weak plugins/screen behavior, device-memory allowed set, native-code marker
and fresh iframe comparison. L1 CDP flags are consumed as before; their upstream
active producers are separate unported work, not measured by an absent flag.
Do not reinterpret old comments as new stronger automation claims.

DOM getter/call failures return collection failure, never an empty success report.
Missing optional APIs keep their frozen skip/default semantics. Bound enumerated
keys/strings to protect CPU/memory; malformed or excessive input fails collection.
An owned hidden same-origin iframe is removed on success/error. No permission
request, navigation, telemetry, raw key/text collection, credential or server plane
is added. Browser metadata remains client-asserted and tamperable at the T4 ceiling.

This slice produces L1-L3 partial ClientReport only. Environment.probed remains
false, behavior/advanced/guard remain unmeasured defaults, sessionId is not minted.
L4 events, environment/guard/advanced probes, generated bootstrap/distribution,
collector/RIT transport and recovery are mandatory later slices. Runtime routes
requiring a qualified collector stay fail-closed; an exported JSON function or
WASM compile is not an authenticated collector or human ALLOW.

Unit matrices must test all17frozen signal IDs, registry parity, benign baselines,
case/zero/absence boundaries and fresh-realm distinctions. Real browser WASM tests
must inspect actual APIs and iframe cleanup. Native snapshot tests, browser unit
tests and Docker product e2e have distinct evidence scopes; no result implies a
real human/keyboard/AT baseline or full browser-tool readiness.
