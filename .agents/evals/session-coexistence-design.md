# New-lane coexistence design v0.3

Status: draft for root review; no mutator or operational catalog change.

## Brief

Two new user-approved chats need convention-audit and performance ownership.
Legacy25 claims remain legacy-owned; the global writer mode must not change.
Only uniquely named SoT/plan/evidence paths are eligible. No product code, graph,
Pages/rules, git-ops, model or load-test grant follows. Existing Rust session
writes are deliberately blocked in legacy-coexistence. Existing strict parsing
also refuses an actual annotated legacy scope. This design preserves that denial.

## Typed inputs and catalog split

Proposed future CatalogV2 contains schema_version2, writer_mode legacy-coexistence,
canonical lanes and an explicit legacy_projection_ids set. Canonical membership
is the union of the fixed legacy25 and the two bounded new IDs. Markdown projects
the union; existing PS/bash allowlists project only the explicit legacy subset.
Reject duplicate IDs, unknown kinds, projection IDs absent from the union and
new IDs incorrectly advertised as legacy-supported. Do not infer support from a
lane's kind or silently ignore projection drift. The existing schema1 binary does
not implement this contract and cannot be advertised as claim-ready for it.

Proposed NewLanePolicy is a closed enum with exactly ConventionAudit and
Performance. Each maps to a root-reviewed ExactGrant value: its exact named SoT,
exact plan filename, an explicit set of public evidence files and an exact private
bundle root. Public descendants are not implicitly granted.
Reject prefixes, case aliases, parent/root escapes, added code paths and broader
wildcards. No wildcard is allowed within any selected filename or bundle root;
only a terminal /** for an explicitly admitted exact private evidence directory
is permitted. Every granted scope must be disjoint from every admitted owner
scope under the unchanged overlap solver; names alone do not prove separation.
The five-field Request remains lane/provider/session/goal/paths; the
requested set must equal the policy's bounded set, not merely intersect it.

The original v0.1 keyword plan/evidence globs are rejected: a composite basename
can contain both convention-audit and performance-optimization or another owner's
program-roadmap/program-status keyword. This is an actual glob intersection,
not a reason to relax the solver. Ancestor/nested evidence roots also conflict.

ConventionAudit's exact plan/bundle roots remain unassigned until previewed and
root-reviewed; no example wildcard grants ownership. The separately accepted
temporary legacy Performance loan has these observed exact scopes:
- .spec/sot/performance-optimization.sot.md
- .spec/plan/20261107135643-performance-optimization.plan.md
- .spec/evidence/20261107135643-performance-optimization/index.evidence.md
- .spec/evidence/20261107135643-performance-optimization/baseline.json
- .spec.local/root/evidence/20261107135643-performance-optimization/**

The initial exact public subtree loan was subsequently narrowed by root to the
two explicit files above; the owner's actual updated claim was re-read. Additional
public children require a new root scope review, never prefix-based permission.
The private fixed subtree is separately admitted by root against existing owners.
That claim is project-coordination/codex/rust-performance-baseline, acquired by
the performance owner using the unmodified existing PS compatibility protocol.
It is not a CatalogV2/Rust mutator result. No .agents metadata, product code or
benchmark source path is included; later target/code scope requires a separate
root assignment. The global catalog/mode and legacy25 remain unchanged.

## Literal legacy scope handling

LegacyScope is ParsedPositiveScopes or Unknown with the original claim-file hash
and diagnostic. Preserve raw bytes/BOM, strict JSON fields, lane/provider/session,
timestamp and paths. The actual CI expression containing EXCEPT and semicolon
clauses is Unknown under current grammar. Do not split it heuristically, discard
exceptions or treat an unreadable/unknown claim as disjoint. Unknown blocks every
automatic new grant. All positive parsed patterns use the existing conservative
case-folded glob intersection with bounded work; overlap or budget exhaustion
blocks. Existing occupancy blocks even when the owner is the same session.

An owner may separately propose an explicit positive superset of its annotated
claim, keyed by exact raw claim hash. Root reviews that envelope and records the
source decision; it must contain all possible claimed paths, never only a convenient
disjoint portion. A changed hash, unresolved ambiguity or missing owner review
invalidates the envelope and retains Unknown. This is a reviewed arbitration
input, not a permissive parser, rewrite of another owner's claim or runtime proof.

## Atomicity finding and temporary arbitration

Legacy writers do not honor the Rust storage lock or perform the same cross-scope
admission. Their list/board projections enumerate only legacy IDs; legacy release
can remove a supplied lane filename without exact owner verification. Therefore
a Rust create_new plus before/after snapshot alone cannot prove safe coexistence:
an old writer can change overlapping ownership immediately after revalidation.
This is a trusted-repository coordination boundary, not OS authorization.

Verdict: BLOCK automatic new-lane mutator readiness on the present protocol.
Keep chats read-only unless root accepts and actually performs this temporary
arbitration procedure:

1. Root declares a bounded claim-change barrier and obtains acknowledgments from
   every active claim writer. Existing code work may continue within unchanged
   scopes. A free git-ops mutex alone does not establish this barrier.
2. Root snapshots every literal legacy claim, operational new-lane claim and
   relevant catalog/policy bytes, refusing symlinks, oversized/unknown entries,
   invalid owners/timestamps and unresolved Unknown scope. Resolve annotated
   claims only through the exact-hash reviewed positive envelopes above.
3. Check the exact new policy scopes against the entire admitted snapshot and
   current path ownership. Confirm no same-lane file exists and both chat identity
   and owner session match the intended grant. Root records these precise facts.
4. Under the barrier, create only the new claim with create_new and durable grant
   intent/result receipts. Never replace legacy claims or rewrite ACTIVE.md.
   Re-read the complete snapshot before acknowledging ownership. Any discrepancy
   retains a recovery obligation and never announces a successful grant.
5. Root records that the new unique scopes are reserved against future legacy
   grants and only then ends the barrier. All future conflicting legacy grant
   changes must be routed through root review. Without that ongoing procedural
   guarantee, remain read-only; a one-time snapshot is insufficient.
6. Release requires exact lane/provider/session and original grant identity.
   Append durable intent before removal, preserve failure recovery and other
   claims, and never use the legacy unowned release command for new lanes.

No implementation is authorized by this draft. Root-mediated execution itself
still needs an explicit accepted procedure and reviewed tool/command boundary;
this document is not an executable grant. Global writer-mode migration and
automatic legacy takeover are outside scope.

## Verification and critique reflected

The companion session-coexistence-cases.json defines acceptance vectors, not
executed tests. Before any implementation, root must accept the typed schema,
barrier/envelope procedure and error recovery. Then author Rust1.99.1/edition2024
pure catalog/scope tests and actual isolated two-process storage tests in an
allocated target. Required cases include unknown legacy scope, ordinary positive
overlap, scope escape, occupied/create_new race, wrong-owner release, changed
snapshot/envelope, symlink, lock retention and preservation of every legacy byte.
No new PS/bash implementation or foreign executable strings are allowed.

Blue architecture: isolate policy/schema and never expand product ownership.
Red review P0: a new private lock cannot serialize old writers; the barrier and
ongoing arbitration are mandatory, otherwise block. SRE P1: ACTIVE projection
rewrites can hide new rows; do not use that board as grant authority or mutate it.
Independent convention/performance P1: substring glob namespaces overlap; exact
root-reviewed filenames/directories plus conservative intersection are mandatory.
Evaluator P1: catalog acceptance and a read command are not successful claims;
require actual terminal receipts. Compliance: retain only coordination identities
and hashes; never ingest secrets or unrelated user-global configuration. These
perspectives were reviewed inline without spawning agents. No scoring/a11y/T4
contract changes result.
