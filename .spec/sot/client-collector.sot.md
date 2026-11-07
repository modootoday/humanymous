---
id: sot:root/client-collector
title: Rust browser signal collection and generated WASM boundary
status: active
created: 2026-11-07T01:30:00Z
updated: 2026-11-07T04:56:14Z
verify:
  - cargo test -p humanymous-collector --locked
rel:
  governs: [crates/collector/**]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application]
---

# Browser collector migration

## Isolated actual-runtime pilot contract

The non-default runtime-pilot feature consumes pilot/fixture.json as the one
machine-readable scope/path/status contract and pilot/webdriver.json as typed
WebDriver configuration. Actual browser requests use BrowserSession and an
unmodified BrowserCollector report; no synthetic Fetch or benign-report rewrite
qualifies this profile. Browser/gate namespaces are separate; a lab-owned strict
HTTP proxy preserves the browser-visible origin while forwarding only the owned
API inventory to the actual loopback runtime. The proxy is not original client
TLS/H2/header provenance. Its actual hit counters and backend/egress isolation
must be independently measured by the lab, not inferred from a configured pref.
Missing required fresh collector capability remains a pre-consumption422 without
ACK and closes the client epoch. This is distinct from accepted client-asserted
report ingestion200. The feature test alone does not prove the full product,
witness independence, human/AT support, cookies or streaming. Those gates remain
required; runtime execution of this new source is currently not_run.

## Owned Fetch transport contract

BrowserSession owns actual Request/Headers/AbortController and one whole-request
timer through complete bounded response consumption. Use same-origin,
credentialsomit, redirecterror, no-store and no-referrer. Initial requests64KiB,
responses1MiB buffered, bootstrap8192bytes; streams/cookies remain unsupported.
Sign final browser method/URL/Content-Type/Authorization and bytes with the shared
codec. Reject reserved/transport-owned headers and encoded paths. Normalized
exposed response headers do not establish raw-header provenance. Check ACK only
after body completion and cleanup. Rust future/owner drop invalidates and aborts;
JavaScript Promise GC alone is not cancellation. Explicit close during a flight,
no automatic retry and no key persistence are mandatory. Real runtime/session
join and qualified fresh collector provenance require separate actual evidence.

## Browser session transport implementation contract

The browser_auth_only client consumes an exact provisioned Scope, never trusting
bootstrap to choose installation/tenant/origin/revision/authority. Strict JSON
bootstrap has protocol/session/key/counter/expires and all5 scope fields, no
duplicates/unknown fields, at most8192 bytes, canonical32byte base64url key and
session, initialcounter0, futureJSsafe expiry. Key/ID/token buffers zeroize on
drop/invalidation. Request signing reuses shared integrity browser codec and
seed(key,lastcounter); only one flight is allowed. Rollback/expiry, ambiguous
network/body failure, timeout/abort/drop/redirect invalidate rather than retry or
guess counter. Invalid input before a flight does not consume a counter.

Response acknowledgement was accepted by the runtime owner: only a
consumed counter receives protocol/counter/MAC headers, bound to pending token,
counter/bucket and observed status. Response status alone is not consumption
proof. Initial profile uses same-origin/credentialsomit/redirecterror and bounded
buffered bodies; cookies, streams and fresh collector provenance stay separate
required follow-ups, not silently claimed supported.
ACK bytes are humanymous.browser/0.1/ack newline, decimal byte-length/colon/
pending request token/newline, decimal actual status/newline, signed using
compute_token(seed(sessionkey,n),bytes,n,bucket). Gateway strips all origin
x-humanymous-* response headers and issues ACK only after consumption, including
consumed4xx/5xx. ACK establishes progress only, not ALLOW/response-body/collector
attestation. Client advances only after bounded complete body plus valid ACK;
an ACK cannot make body failure/abort/drop/timeout safe to retry.

## Explicit bounded console-preview observation

An explicit once-per-collector probe ports the legacy Error.stack getter with
console.debug and prototype Proxy ownKeys with console.groupEnd. Synthetic
objects contain no application body, URL, argument or input. The synchronous
post-call result matches the original sampling boundary; callbacks invoked later
remain inert and live under JS-managed closure lifetime. Both probes separately
record attempted/unsupported/failed and triggered. Absence/failure is not a
negative detection or a successful qualified plane. No universal CDP, current
Chrome, automation identity or human guarantee follows from either result.
It does not attach DevTools, enable a debugging port, navigate or run script
source. Default construction does not perform these active console operations.
The legacy groupEnd call can change developer-console grouping; it is not a
passive or invisible operation. Synthetic positive callback tests do not prove
behavior of a particular real CDP client.
Only successful attempted observations may feed the existing unchanged CDP
flag evaluator; no scoring weights/predicates/bands or new tells are added.
Once explicitly requested, an incomplete CDP attempt rejects this collector's
snapshot instead of silently substituting a negative flag. Repeated calls return
the cached attempt; stop rejects later active probes.

## Owned script observer

Observe document childList/subtree changes with an owned MutationObserver.
Count only directly added nodes with tagName SCRIPT and falsy dataset.hm,
matching legacy marker semantics. Never inspect script text/URL/arguments or
descendants. The shared guard Epoch owns counts and coverage. Script coverage
alone keeps reported=false until eval and Function coverage really exist.
Bound each delivery to4096 records/16384 nodes; malformed acquisition, overflow
or reentrancy invalidates this guard acquisition. Explicit stop drains pending
records before disabling/disconnecting, then freezes. Drop disables/disconnects;
failed disconnect retains an inert callback rather than dropping a reachable
WASM closure. No script execution or global eval/Function replacement here.

## Guard coverage and compatibility

BrowserCollector now acquires real Object/Array own-property counts and captured
Function.prototype.toString checks for fetch/XHR/JSON/console before passive
listener registration. Native source text is bounded32768UTF16 units before
copying, inspected transiently and discarded; no source text/arguments are exported.
Use the exact legacy braced-native marker with ECMAScript whitespace, not Rust's
broader Unicode whitespace. Throwing/budget-invalid required observations reject
capture. Failed native toString invocation is a negative check as in the legacy
guard; console absence/failure means consoleDisabled=true.

snapshot_json includes these partial guard observations; reported stays false
because eval/Function/MutationObserver hooks are not installed by this slice.
stop attempts both passive-listener cleanup and guard freeze, reporting failures;
once guard freeze succeeds, later global mutations cannot rewrite that guard.
The free detect_json remains stateless. Capture is not proof of pristine startup,
untampered realm, full guard coverage or authenticated session qualification.
The whitespace set follows [ECMAScript whitespace and line terminators](https://tc39.es/ecma262/multipage/ecmascript-language-lexical-grammar.html#sec-white-space),
checked2026-11-07; U+FEFF is included and U+0085/U+180E/U+200B are excluded.

Port injector.js as owned runtime instrumentation, not a fabricated guard report.
Separate pure epoch/counters from WASM hooks, mutation observer and native/prototype
capture. Reuse shared Guard. Baseline own-property counts are bounded16384;
eval/function/script counters are nonnegative exact JS integers (max2^53-1).
Overflow poisons the epoch and rejects reports. Stop freezes the final snapshot;
inactive callbacks cannot add counts. A new epoch has new counters/baseline.
Never receive/store/log dynamic source strings, script text, URLs or arguments
in the pure state: it accepts event kinds only. Native checks preserve original
fetch/XHR/JSON native conjunction and console interpretation; prototype pollution
compares counts at read time and clears again if counts return to baseline.

reported=true requires measured native/prototype baseline plus installed eval,
Function and script-observer coverage. Missing hooks must remain reported=false;
pure caller-supplied state is not an authenticated runtime or client attestation.
Hook cleanup must restore only references still owned by this epoch, disconnect
observer and retain any still-referenced Rust callback safely; no global blind restore.

Compatibility finding: injector.js's comment that replacing global eval affects
only indirect eval is incorrect. Direct-eval evaluation requires identity with
the realm intrinsic eval; a wrapper/Proxy changes that identity and lexical
behavior. Do not automatically deploy global replacements under an unchanged
behavior claim. Browser instrumentation must specify this compatibility delta,
preserve forwarding return/throw/this and Function construction behavior, and
prove owned-realm lifecycle/application tests before collector bootstrap activation.
Forwarding caller-owned opaque arguments is distinct from authoring/executing a
JS source string in Rust. No new eval/Function source strings, scoring rule or
unsupported complete-CDP-detection claim is authorized by this design.

Forwarding hooks are an explicit enable_guard_hooks opt-in, never implicit in
BrowserCollector construction. A realm must expose writable own data eval and
Function plus Proxy/FinalizationRegistry. Proxy apply and construct count only
event kinds, immediately forward opaque arguments/this/newTarget and propagate
the exact return/throw. No source string is authored or stored. JS-owned managed
WASM closures remain alive for externally retained proxy references; disabling
stops counts but retained proxies still forward. Restore only a property whose
current own data value is the installed proxy, preserving current descriptor
flags; never overwrite a replacement/accessor/deletion by another owner. Roll
back partial installation. Full dynamic coverage is a client observation, not
proof of pristine intrinsics, human identity, code attestation or protected route
readiness. Explicit opt-in accepts the direct-eval lexical compatibility delta.
Within this WASM module, overlapping hook activation in the same realm is
rejected (bounded64 active realms) to avoid nesting and restoring stale proxies.
Other independently injected modules remain outside this ownership guarantee.

Checked2026-11-07: [direct-eval identity rule](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-function-calls-runtime-semantics-evaluation),
[Proxy internal call/construct rules](https://tc39.es/ecma262/multipage/ordinary-and-exotic-objects-behaviours.html#sec-proxy-object-internal-methods-and-internal-slots).

## Advanced probe asynchronous ownership prerequisite

Environment and advanced observations share one browser timer owner. A deadline
must distinguish a Promise fulfilled with undefined from timeout, preserve
rejection as a separate outcome and cancel its timer after every settlement.
Use a per-timer opaque object sentinel passed to the native Promise resolver;
do not compare only an undefined value, allocate Rust callbacks retained by the
browser or duplicate timeout implementation per API. Timer/cleanup setup failure
is an explicit error. Browser event-loop stalls are not wall-clock guarantees.
This prerequisite does not mark advanced.probed or qualify any new plane.

The advanced migration must separately cover device counts/kinds without labels,
voices with owned event cleanup, EME and GPU800ms fallbacks, WebGL/audio lifecycle,
connection/battery, locale/display/memory and existing wire defaults. Preserve
frozen server interpretation, no permission prompts or raw device identifiers.
DRM absence alone is not a new blocking rule. Source advanced.js fingerprint-ID
and WebRTC ICE/session correlation remain distinct required responsibilities.

Implement collectAdvanced as an explicit async advanced_json API, using shared
Advanced and preserving connectionRtt=-1/batteryLevel=-1 plus original optional
defaults. probed means the attempt ran, not that every optional surface exists.
Do not merge it into qualified session reports implicitly. Optional API absence
and rejection keep original per-surface defaults; malformed/oversized returned
data and cleanup/timer infrastructure failure reject the probe. Use800ms for
formerly unbounded media/battery/GPU-info/memory operations as well as EME/GPU;
memory timeout is not an observed exception and must not set measureMemThrows.
No arbitrary sites, license requests, device IDs/labels, voice names, permission
prompts or local addresses are collected. Audio sample rate uses an unplayed
context and awaits close; voice waiting adds/removes its own native Promise
resolver listener and never replaces onvoiceschanged. Each stage owns cleanup.

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
behavior and measured guard baseline/script observation; environment.probed
remains false and advanced is not automatically joined. Explicit forwarding hooks
are opt-in and sessionId is not minted. Environment/advanced probes are standalone;
generated bootstrap/distribution,
collector/RIT transport and recovery are mandatory later slices. Runtime routes
requiring a qualified collector stay fail-closed; an exported JSON function or
WASM compile is not an authenticated collector or human ALLOW.

Unit matrices must test all17frozen signal IDs, registry parity, benign baselines,
case/zero/absence boundaries and fresh-realm distinctions. Real browser WASM tests
must inspect actual APIs and iframe cleanup. Native snapshot tests, browser unit
tests and Docker product e2e have distinct evidence scopes; no result implies a
real human/keyboard/AT baseline or full browser-tool readiness.
