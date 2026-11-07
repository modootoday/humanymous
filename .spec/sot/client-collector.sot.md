---
id: sot:root/client-collector
title: Rust browser signal collection and generated WASM boundary
status: active
created: 2026-11-07T01:30:00Z
updated: 2026-11-07T03:48:54Z
verify:
  - cargo test -p humanymous-collector --locked
rel:
  governs: [crates/collector/**]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application]
---

# Browser collector migration

## First-party environment probe extension

Port web/js/env.js to an explicit asynchronous Rust environment_json API.
Reuse the shared Environment wire type and frozen scorer; no new detection rule.
Preserve privacy metadata truthiness, four bait classes, offscreen10px geometry,
60ms observation delay, offsetHeight/display predicate,400ms first-party fetch
abort and false collectorBlocked default. Fetch failures remain weak privacy
observations, never proof of a blocker or automation. HTTP error status alone
is not a fetch failure. Missing/broken DOM setup and malformed values fail the
probe rather than reporting probed=true with synthetic clean observations.

Critique reflected: the original best-effort DOM catch and leaked timeout on
fetch rejection are not a safe ownership model. Rust owns the bait and timers,
removes/cancels them on success or error and rejects cleanup failures. Use only
generated bindings and checked reflection, never executable strings. Fetch only
the fixed same-origin honeypot path with redirects refused and credentials
omitted; no arbitrary URL parameter, permissions or third-party tracker request.
This explicit API does not silently attach network activity to detect_json or
snapshot_json. Those reports remain partial until session transport integrates
the measured environment. Guard/advanced/bootstrap/Pass remain separate gates.

Required actual browser tests: DOM cleanup on success/failure, visible/hidden
bait, rejected fetch and HTTP error semantics, privacy getter truthiness,
same-origin request options and aborted slow fetch/timer cleanup. Synthetic
fixtures are not human/AT evidence or full product e2e. Optional API absence
uses original defaults; throwing required privacy getters reject collection.
Bound doNotTrack to256UTF16 units before copying it into Rust. Timers express
configured browser delays, not a wall-clock guarantee under event-loop stalls.
If browser cleanup APIs throw, return failure and attempt best-effort Drop cleanup;
no complete cleanup or trustworthy observation is claimed against a hostile realm.

## Passive behavior extension

Port the read-only oracle web/js/collector.js and features/{mouse,keystroke}.js
into one pure Rust behavior reduction context and a separate passive browser
listener owner. The existing humanymous-behavior crate owns summary-to-tensor
transforms; it does not already implement these event-to-summary reductions.
Do not fork the reducers or alter scoring to compensate for their limitations.

Preserve first600 sample/timeline caps (the oracle is not actually rolling),
uncapped aggregate event counts, exact event inclusion/trust rules, coalesced
fallback1,800ms no-move boundary,120ms pause,0.995 straightness,12entropy bins,
1500ms cadence boundary and all population-variance formulas. Preserve the
oracle's unusual latest-down-per-code pairing over the entire buffered window,
including possible negative dwell; do not silently replace it with causal pairs.
Flight remains press-to-press despite the older wire field's description.
These frozen heuristics are measurements, not proofs of AI/human identity.

Use an explicit owned BrowserCollector with passive listeners, no preventDefault,
finite/bounded API observations and one reusable pure Recorder. Event.key, text,
input values and clipboard contents are never read. Physical event.code is used
only for bounded transient pairing through per-instance opaque IDs; no code or
ordered key identity is serialized/logged. Clear the intern map and raw buffers
on stop/drop. The bounded intern table holds JS string handles and uses strict
identity/value comparison without mutable JS Map prototype calls. Validate the
physical code's128UTF16-unit budget before copying any string into Rust.
A new instance starts a fresh epoch; stop freezes the final summary.
Multiple registrations must be detachable and construction failure must undo
partial registration. Inactive callbacks are no-ops. If removal itself throws,
return failure and preserve a no-op callback lifetime rather than a dangling
freed Rust closure; bound this residual to that instance's six callbacks.

Malformed observations/reentrancy fail the report rather than synthesize empty
success. The legacy optional coalesced-events probe deliberately retains its
catch-and-fallback1 semantics; this exception is not full input provenance.
Behavior reports remain client-asserted. snapshot_json combines measured behavior
with the shared L1-L3 report but leaves environment/guard/advanced/session and
transport unqualified. The free detect_json remains a stateless L1-L3 API.

Critique reflected: no text/key export, bounded state, exact repeat-key behavior,
explicit callback cleanup failure and negative/unknown provenance; no new signal,
threshold, hard rule or a11y speed/motor requirement. Required tests include
independent analytic boundary vectors, differential tests against the unchanged
existing feature modules, actual browser dispatch/lifecycle/cleanup and error
paths. Synthetic dispatch is not a real human/AT baseline or full product e2e.

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

The stateless detect_json produces L1-L3 partial ClientReport only. Its behavior
remains at unmeasured defaults. BrowserCollector explicitly adds measured
behavior; environment.probed remains false, advanced/guard are unmeasured and
sessionId is not minted. Environment/guard/advanced probes, generated bootstrap/distribution,
collector/RIT transport and recovery are mandatory later slices. Runtime routes
requiring a qualified collector stay fail-closed; an exported JSON function or
WASM compile is not an authenticated collector or human ALLOW.

Unit matrices must test all17frozen signal IDs, registry parity, benign baselines,
case/zero/absence boundaries and fresh-realm distinctions. Real browser WASM tests
must inspect actual APIs and iframe cleanup. Native snapshot tests, browser unit
tests and Docker product e2e have distinct evidence scopes; no result implies a
real human/keyboard/AT baseline or full browser-tool readiness.
