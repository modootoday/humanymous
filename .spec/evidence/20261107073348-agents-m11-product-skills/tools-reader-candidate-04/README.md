# Bounded explicit stage-eval candidate 04

NOT_COMPILED / NOT_EXECUTED / NOT_INTEGRATED. Based on six exact observed
Root-isolated tools-integration-5f1010d4 files after candidate03/Root's lint and
format corrections. Original bytes are retained as .base attachments. Root
source/Cargo/snapshot were not edited and no compiler or native pilot ran.

## Actual implementation and replacement map

| Root CLI src/qualification location | Actual candidate change |
| --- | --- |
| maintenance/package.rs Receipt and plan_developer/stage | Keep existing product planner and both old unit bodies. Developer planner consumes captured selected texts, never a directory. stage_prepared validates/writes prepared files only, retains fresh create_new/hash/size/exclusion semantics and one command clock |
| maintenance/product.rs old load(root) | Remove now-unused recursive filesystem reader and imports; pure validate/Bundle/all six units remain unchanged |
| maintenance/command.rs stage-eval arm/import/usage | Command-owned package_stage module; stage-eval ABSOLUTE_REQUEST_JSON ABSOLUTE_NEW_OUTPUT; old root/kind operand signature has no fallback |
| maintenance/selected_inputs/files.rs checked_path visibility | Reuse existing ancestor/reparse/type implementation crate-internally; body unchanged |
| maintenance/selected_inputs/mod.rs | One crate-internal checked_path export; Root's prior lint/test exports retained |
| New maintenance/package_stage.rs | Actual bounded capture/manifest hash verification, pure-plan composition, output admission and fresh writer call |
| New maintenance/package_selection.rs | Fixed operation request schema, exact kind/skill/eval profile checks and four synthetic parser units; command-owned child, no I/O framework |
| tests/maintenance_product_package.rs | Same two IDs and original assertions; explicit per-kind request inputs, actual subprocess, source receipt hashes/sizes/root/case/exclusion checks and source-hash-mismatch-before-output assertion |

candidate.patch is an actual unified diff against the preserved assembled-source
originals, with the two actual new Rust files. It was applicability-checked only
against owned preserved copies, not applied. Complete replacement files are review
references; Root must merge with any subsequent source changes, not overwrite them.

## Fixed request and finite selection

Required JSON fields: schema=humanymous.stage-eval-source.v1, absolute repo_root,
source_commit (40/64 lowercase hex declaration), kind (product/developer),
input_selection_complete=true, expected_skill_names and files. Every pin requires
path/owner/presence=required/bytes/lowercase SHA256. Duplicate or unknown JSON
fields, duplicate/case-colliding paths, traversal, wildcard/ambiguous paths, relative
root, missing pins and false declared completeness refuse. Owner labels are
repository for LICENSE and agents-meta for skill/eval documents, not authenticated
provider identities; Root freezes this concrete label choice before adoption.

LICENSE is always required. Product selects skills/SKILL.md and only explicitly
listed skills/references/*.md; its actual metadata name must match the single
declared expected skill. Existing product graph validation catches missing,
escaping or orphan selected references. Developer selects only .agents/skills/
documents under explicitly named skills; each listed skill requires its SKILL.md,
and captured metadata name must match its folder. No script/source executable,
rules tree, chat registry or other plugin's inputs can enter either artifact.

Both profiles require their suite README and all three original documents per
fixed case: prompt.md, graders/behavior.md and graders/discovery.md. Product has
install-plan/readiness/unrelated-ui (3); developer has ci-identity/ci-accounting/
release-prepare/release-failed-claim/unrelated-layout (5). Other eval cases and
opposite-profile documents refuse. There is no recursive walk or resource
discovery. A selected finite profile is not proof that the entire canonical tree
has been enumerated: canonical_tree_complete stays false. Root's frozen outside
manifest must establish its required actual canonical closure.

Request <=2MiB; source inputs <=510 (two generated metadata files retain output
<=512), each <=2MiB and product entry <=256KiB. Declared source text <=8MiB; the
prepared artifact INCLUDING generated metadata still must satisfy original
512-files/8MiB limits before creating output. Each actual capture further tightens
its cap to max(1, declared length), retaining growth probes. Actual request plus
source reads are accounted against2MiB+8MiB+511; every read and writer checkpoint
uses the SAME original30s cooperative clock. Existing tighter caps stay intact;
no5000/64/2GiB/512MiB ceiling becomes scan authority. Blocking syscall/hard
lifetime and CA-LIFETIME/provider guarantees remain OPEN, not claimed solved.

The existing selected-input capture implementation reads the bytes, checks
ancestor links/reparse flags and metadata before/open/after, and returns that
same capture. Hash/length verification precedes UTF-8 parsing; LF violations
refuse without normalization. All selected text and required semantic checks
finish before output creation. Missing/changed/invalid/mismatched inputs therefore
leave no output. No later reopen supplies parsed input content.

## Output and receipt contract

Output is an explicit absolute fresh path with an existing checked parent. The
existing checked_path helper is reused, then canonical source/parent containment
is checked. Source overlap/ancestor output, existing or unknown output status,
links/reparse points and active .agents/.codex/.claude/.grok/.gemini path components
refuse. Only an admitted NEW artifact root is created; original source/provider
config is never a writer target. Existing create_new file writes, per-file sync,
receipt hashing and refusal-to-overwrite behavior remain. Write/deadline failures
retain partial owned output; no silent cleanup/retry or cancellation proof.
These are cooperative trusted-environment checks, not race-resistant OS authority.

The existing humanymous.skill-package.v1 receipt gains an additive required source
object in this candidate, persisted and printed identically. It records the
request SHA256, explicit root/declared source commit, EVERY captured input's raw
hash/size/path, actual read bytes and pre-write elapsed time. Declared completeness
and matching selected contents are true only after all checks. Canonical-tree,
Git-source, provider-identity and qualified-complete flags remain false with
unsupported_source_identity status. Existing runtime/release flags remain false.
Root must review the additive receipt compatibility/version choice; the immutable
old33-case qualification contract is not rewritten. A commit-shaped value and
caller pins do not authenticate Git source or close hostile replacement races.

The command returns success for a completed bounded experimental artifact while
keeping all qualification flags false, preserving the old staging action's
success meaning. This does NOT decide success semantics for other content-check
commands or promote any runtime/provider proof. Root retains that decision.

## Actual required hosted tests and preserved census

Keep the exact two original IDs in maintenance-product-package. The runner now
must supply absolute request paths via HUMANYMOUS_MAINTENANCE_PRODUCT_STAGE_REQUEST
and HUMANYMOUS_MAINTENANCE_DEVELOPER_STAGE_REQUEST. No environment/default repo
fallback exists; missing configuration fails the required case. The request's
selected absolute root still receives the original ancestor/link/reparse checks.
The executable stays the exact Cargo-selected maintenance binary role.

Original output byte/hash, expected3/5 cases, exclusion, runtime/release false,
existing-output refusal and unchanged receipt assertions remain. New assertions
compare every source pin and raw request SHA, exact successful actual read count,
root/declared commit, unchanged request bytes and honest unsupported flags. A
corrupted source SHA in an owned temporary request must fail before output
creation inside EACH of the same two cases (no new integration IDs). Failure
artifacts/diagnostics remain; successful owned outputs/negative request files alone
are removed. These native/filesystem cases remain HOSTED-ONLY and NOT_RUN here.

All original76 library +7 reader +10 owned-FS cases from Root's actual93 census
remain required and unchanged in identity; four new pure selection units are
additional, not substitutes. All33 original integration IDs remain required.
Static changed-file test function comparisons and unit-ownership.json are not an
executed --list/test pass. Root must rerun actual source-bound count/fmt/strict
Clippy and approved hosted boundaries after adoption.

Example requests contain hashes only, not copied mutable canon. Their explicit
placeholder source root intentionally cannot act as a fallback. Root must bind
them to its actual frozen extraction with a newly recorded request hash. Supplied
pins/declared completeness do not independently establish complete tree coverage,
authentic source, publication readiness or broader skill evaluation improvement.

Applicability verification initially found a malformed generated diff caused by
trimming a blank context line, before any application. The original malformed
patch is privately registered as evidence:root/20261107073349,
tools-reader-patch-check-04/first-corrupt.patch, SHA256
96fa437ca2404273964496529ce23951771d7de78baa371ecff8230581af8f61.
The corrected diff preserves context bytes and passes applicability check against
owned copies only. This is not a compiler failure or executed implementation test.
The hashes-only example profiles contain16product inputs/28028bytes and40developer
inputs/65367bytes (14 named developer skills) from the observed isolated source.
All56 raw selected documents had no CR bytes; none were normalized or copied into
CLI source. The sample commit remains a declaration, not source authentication.
