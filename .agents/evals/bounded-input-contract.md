# Bounded selected-input observation v1

Root accepted implementation and the defaults below. This is trusted-repository
cooperative content observation, not an immutable security snapshot or permission
to execute/reuse a cache. Existing canonical LF inventory and Delivery Git/working
provenance schemas remain unchanged. No new dependency, crate or graph edge.

| Limit | Accepted default |
|---|---:|
| Selected files |5000|
| Selected roots |64|
| Total actual content-read bytes, including positive probe reads |2147483648|
| Per-file content eligibility cap |536870912|
| Cooperative elapsed budget |30seconds|

`selected_inputs` separates model, reader and actual-file adapter responsibilities.
FrozenSelection owns a finite, nonempty caller-selected root/file inventory, checks
counts and lexical aliases/escapes, and binds order/paths/coverage declaration by
SHA256. It performs no dependency discovery, directory walk or cache census.
Observation rechecks the frozen counts against the actual batch limits, including
when a selection was constructed under a looser profile. Caller-declared complete
selection is coverage of that declared set only, not a resolved compiler/SDK graph.

ReadBudget uses a fixed64KiB buffer, streaming SHA256, checked cumulative actual
read accounting and one monotonic clock domain for the batch. The generic caller
must retain that clock; observed regressions fail. The actual-file observer starts
one SystemClock before provider I/O and retains it across every file. It checks
time before/after reads and between Interrupted retries. Other read errors retain
partial accounting and no full digest. Invalid reader byte counts mark accounting
unknown and stop subsequent reads from using that budget.

Reaching a content cap is not EOF. At the per-file cap, a real one-byte probe is
allowed only within remaining total actual-read budget. A positive byte counts
both as a probe and actual I/O, invalidates a full hash and stops that file. A
zero-byte probe observes EOF and consumes no bytes. At exact aggregate exhaustion,
no extra probe is made: completeness is unknown and no full hash is emitted.
Short reads are not EOF. No stdRead::take synthetic EOF is used.

The actual-file adapter checks selected ancestors without enumerating directories,
refuses known links/reparse points, nonregular/missing files and unavailable
metadata, and compares pre/post handle/path size and modified-time observations.
Observed change and unknown stability are distinct, and neither permits a full
file digest. Those checks are not adversarial replacement/alias/identity proof.
The current provider deliberately leaves provider_identity_qualified=false and
qualified_complete=false, even when content_read_complete and a caller's complete
selection declaration are true. The receipt also fixes immutable_snapshot,
execution_authorized and cache_authorized to false.

Blocking open/metadata/read operations can exceed30seconds before a cooperative
check returns. Injected clock tests prove state transitions, not an OS hard
deadline. Actual link fixtures need venue permission; an unavailable fixture is
a failed/unsupported boundary, never silently counted as qualified. No hostile
same-user race, native identity or cache-authorization guarantee is included.

L1 tests cover empty/exact-cap/cap+1/aggregate exhaustion, shared probe accounting,
short/Interrupted/error/invalid-count reads, deadline/regression, arithmetic overflow,
finite/duplicate/escaped/unknown-root selections and changed selection fingerprints.
L2 tests separately exercise real binary/CRLF and empty files, incomplete coverage,
shared limits, stricter observation counts, missing/directory/link inputs and
observed open-file mutation. Qualification is scoped to actual terminal results,
not the authored inventory or this accepted contract alone.

## Candidate real-input acceptance fixture

The existing CLI accepts `selected-inputs REQUEST_JSON`. Version1 request JSON
requires `schema_version`, finite `roots`, finite `files` and an explicit
`input_selection_complete` declaration. Unknown/duplicate fields, invalid relative
paths and oversized requests fail closed. Request capture is bounded to2MiB with
true EOF and checked before/after metadata. One original command clock continues
through request capture and every selected file; actual request reads consume the
same2GiB command budget. Results contain hashes/metadata/status, never file bodies.
Exit0 means complete content and declared selection within command budgets, not
provider identity qualification; partial content/coverage emits JSON and exit1.
Malformed/refused request preparation exits2 with value-sanitized diagnostics.
Evidence is saved by the caller; this command does not write repository evidence.

Performance reported the existing Audit154715 selected-before manifests as a
candidate for the next explicitly allocated CLI pilot: 177 source, 3181 registry
and 73 tool files (3431 total), 657,540,216 bytes, largest file 208,167,760 bytes.
These counts fit the default count and byte caps. Its separately observed registry
hash time was 26.8293511 seconds and tool hash time 18.911908 seconds, totaling
45.7412591 seconds before source hashing. Those earlier measurements are not a
run of this CLI and do not predict an exact CLI completion time.

If the CLI reaches its shared 30-second cooperative deadline on that selection,
it must retain partial accounting and report incomplete content. Budgets do not
reset per root. A full equal-coverage pilot needs an explicitly root-approved,
larger finite profile implemented and qualified first; the current request schema
does not accept limit overrides. Never automatically raise defaults to finish a
fixture. This candidate establishes no antivirus cause or speedup claim. No new
Windows, WSL or model execution was authorized by receipt of the candidate.

### Accepted future named profile

Root accepted the finite profile `audit154715-equal-coverage-120s` for the later
Audit154715 equal-coverage CLI pilot only. It permits a 120-second cooperative
command budget for the same 3431 selected files and 657,540,216 content bytes.
All existing file, root, aggregate-byte and per-file caps remain unchanged. One
command clock covers request preparation and every root/file; there is no reset
per root. Every other selection retains the 30-second default.

This is an accepted scope decision, not permission to run during the current
CEF-exclusive window. A future implementation must bind this exact profile name
to the frozen Audit154715 selection identity and retain that identity and effective
limits in its receipt. Counts and sizes alone cannot establish equal coverage.
Unknown names, arbitrary numeric timeout overrides and profile/selection mismatch
must fail closed. Until the frozen selection identity is attached and the named
profile is implemented and qualified, the existing strict request schema rejects
profile fields and continues to enforce 30 seconds. No executable 120-second
override is introduced by this contract checkpoint. The earlier measured
45.7412591 seconds remain historical phase timings, not CLI proof.
