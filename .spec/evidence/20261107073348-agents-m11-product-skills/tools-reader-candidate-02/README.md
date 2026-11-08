# Concrete mutable-reader candidate 02

Base3137c905ac4e52b8b345fa2306af9e4ef4d42b48; NOT_COMPILED, NOT_EXECUTED,
NOT_INTEGRATED. Root's live source/Cargo and isolated Tools tree are untouched.
The new candidate directly addresses the ten reported all-targets compile embeds:
product5, catalog4, registry1. No new package, framework, foreign executable
template, canonical document copy or ambient relocated path is introduced.

## Actual files and exact original replacement map

| Original skills/agent-maintenance location | Replacement attachment |
| --- | --- |
| src/product.rs:174-214 compiled entrypoint/four references | src__product.rs retains the same test ID with a synthetic four-resource graph, resolved-link and resource-count assertions; product_selected.rs adds actual fixed five-file content checking |
| src/sessions/catalog.rs:142-162 four compiled lane/projection inputs | src__sessions__catalog.rs retains test ID with existing inert parser tokens and synthetic lane; catalog_selected.rs checks actual four selected originals plus declared lane-count assertion |
| src/sessions/registry.rs:73-76 compiled THREADS registry | src__sessions__registry.rs retains test ID with invented stable UUID/title; registry_selected.rs checks actual explicit external THREADS bytes |
| src/selected_inputs/request.rs:100-174 existing capture | src__selected_inputs__request.rs factors that exact implementation into capture_source_file with caller-tightened byte cap and mandatory absolute path; legacy request capture retains its old explicit relative-request behavior |
| src/selected_inputs/mod.rs:5 | src__selected_inputs__mod.rs exports the new capture only crate-internally |
| src/main.rs original skill-check/lane-check/thread-check match arms and usage | src__main.rs introduces product/lane/thread-content-check-request ABSOLUTE_REQUEST_JSON; old root-only variants explicitly refuse rather than bypass finite source pins |

candidate.patch is the actual unified diff, checked against preserved original
.base bytes in an owned evidence-only tree. It was NOT applied. Six complete
original/replacement pairs and three actual selected-reader modules are attached.
The main.rs hunk is independent in purpose from candidate01's language hunk;
Root must compose them, not overwrite one replacement file with the other.

Root's relocated import map is mechanical: these base-relative crate::inventory
and crate::selected_inputs references become crate::maintenance::inventory and
crate::maintenance::selected_inputs in the reported private binary/test roots.
Place product_selected.rs beside product.rs, and catalog_selected.rs /
registry_selected.rs beside their owning sessions modules. Rebind command.rs
paths to Root's existing private maintenance module; do not add production CLI
library fixture exports. No claim of compilation under either layout is made.

## Fixed requests and actual semantics

Three JSON examples pin hashes/lengths from the exact base, without copying the
mutable input contents. Each schema denies duplicate/unknown/missing fields.
Catalog request requires schema, absolute repo_root, source_commit,
expected_lane_count, and exactly4 ordered input pins. Product requires the same
except lane count and exactly5 pins. Registry requires its single pin's fields
at the top level. Every pin requires exact canonical path, owner=agents-meta,
presence=required, byte length and lowercase SHA256. source_commit is40/64 lowercase
hex. This owner label is declared producer ownership, never authenticated identity.
Optional/live mode is unsupported in these fixed required-source profiles.

The readers reuse FrozenSelection's absolute-root/traversal/control/case-collision
validation and the existing selected-input capture implementation, not a new I/O
framework. New source capture refuses relative paths before any CWD lookup.
No recursive scan occurs: the canonical four/five/one paths are exact finite
sets. Fixed input path mismatch, root wildcard/control, unknown fields, missing
file, reparse/link/metadata change, invalid UTF-8 or hash/length mismatch refuses.
Actual hash and parser consume the SAME captured bytes, without reopens or LF
normalization. Missing/changed input cannot be replaced by the synthetic units.

Each declared length further tightens actual capture to max(1, declared length),
bounded by256KiB lane JSON,256KiB product entry,2MiB other files. Product declared
text total is at most8MiB. Request cap is2MiB. Each command uses one original30s
cooperative SystemClock; EOF probes are accounted by ReadBudget. The command
receipt adds request reads to selected actual reads and refuses above10MiB+6;
per-profile fixed counts and tighter individual caps remain effective. Existing
5000/64/2GiB/512MiB maxima are not loosened or used as directory scan authority.

Catalog preserves projection parity and expected actual lane-count checking;
example count is25 at base, unlike the invented one-lane unit. Registry preserves
actual non-empty, stable-ID/title/unique/responsibility validation. Product
preserves metadata, four actual references, portable graph resolution and8MiB
semantic cap. These content checks CAN complete on matching declared bytes in a
trusted checkout; content_checks_complete records that result independently.
Mismatch/errors cannot report completed content checking.

Source authentication and race-resistant provider identity remain UNSUPPORTED:
all receipts keep source_commit_verified/provider_identity_qualified/qualified_complete
false, with explicit identity_status. Catalog/registry also keep writer_authorized
false and existing live/legacy qualifiers false. Product keeps runtime_qualified
false. CLI currently exits1 even for completed content checks because it does not
promote unsupported identity to qualification; malformed/missing requests exit2.
Root must decide the final trusted-content-only command role versus qualified
source role and authenticate/freeze its actual source manifest before a qualified
pass. A source-shaped string and metadata equality are insufficient.

## Three private-context unused-import diagnostics

selected_inputs_private_mod.rs is a separate exact proposed replacement for
Root's PRIVATE selected_inputs/mod.rs after applying the tighter capture change.
It removes unused reexports observe, FileObservation/Stability and
CliReceipt/decode_request. The implementations and parser unit tests stay intact.
files becomes crate-visible, not a public CLI fixture library. The existing six
owned-filesystem cases must import files::observe directly through the actual
private maintenance context; selected_inputs_test_import.rs gives that exact
Rust import block. Apply that block only at Root's
src/qualification/maintenance/qualification_tests/selected_inputs.rs:2-4.
All six test bodies and IDs remain unchanged. Do NOT apply this private-context
export narrowing to the old standalone package without migrating its consumers.
This is explicit ownership/consumer adaptation, not broad allow or deleted tests.

## Required gates and remaining frontier

Three new selected-reader parser unit tests and the three rewritten synthetic
units are NOT_RUN. Required actual checks are the three explicit request commands
against Root's final immutable source/manifest, plus missing/mismatch/changed/
link/deadline/cap negatives. Preserve the exact eight-target33-case old contract:
23 process/integration cases plus selected_inputs6/evidence_layout_cli4 actual
filesystem/provider contexts. No target, ID, expected denominator or assertion
may disappear because a synthetic unit passes. No old proof transfers here.

The candidate resolves the ten source embeds by construction, but only an actual
Root compile can confirm that. Candidate01 language remains separately pending;
the remaining git_coord original-script include needs its explicit external
producer execution contract. Inventory/routing/staging/live session and other
remaining accepted reader sites are NOT_IMPLEMENTED here. Existing legacy
product::load used by staging is NOT replaced by this fixed content-check API;
Root must not treat it as finite source-bound staging or allow it to bypass its
separately pending required contract. Full reader integration is still open.

## Root's subsequent verified lint changes take precedence

Root reported actual isolated language-role cargo check and strict Clippy pass
with its narrower role/test exports and exact RequestError formatting changes.
Those confirmed fixes supersede the separate private-mod/test-import proposal
attachments here. Do not apply those alternatives or replace complete Root files
blindly. Integrate only the actual capture cap/function hunk, its one internal
export, selected-reader modules, test fixture replacements and source command
arms. Preserve Root's existing lint/role edits and candidate01 language integration.
The complete replacements are review references against3137c905, not permission
to overwrite later source. No Root compile pass is attributed to candidate02.
