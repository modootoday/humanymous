# Bounded evidence materialization contract

Version0.1. Scope: future immutable third-party/source/theme/workspace unpacking
and individually owner-reviewed retirement candidates. This is a structural
contract, not a cleanup command or an execution allocation. No baseline changes,
global rule disablement, broad census or active foreign-source move is included.

## Storage boundaries

Canonical registered manifests, original raw logs, checksums and compressed archive
parts remain under the owning evidence bundle in `.spec.local`. Future unpacked
materializations use a unique absolute TEMP or explicitly declared scratch path
outside `.spec.local`, allocated to one owner/run. A scratch copy is not canonical
documentation and is never promoted to the repository's SoT by its location.
Original third-party bytes remain original; no formatter, language rewrite or LF
normalization applies to raw evidence. Existing historical executed paths remain
in receipts even after an independently authorized retirement.

Authored public JSON projections use canonical LF before attachment. The author
must verify the exact committed Git payload SHA256 and length against its evidence
index; a working-tree Spec pass is insufficient. Preserve the first mismatch and
original working projection privately before correcting only owned metadata or
authored projection line endings. This does not authorize normalizing original
execution logs, archives or third-party inputs.

## Required per-candidate record

A future bounded planner must name exactly these fields, with missing or unknown
values refusing retirement:

- candidate ID, exact resolved absolute materialization path and owning lane,
  provider, session and explicit disposition authority;
- original source checkpoint, immutable upstream/content identity and original
  historical executed path, with the original receipt reference;
- every producer, reader, build/test and nested-session handle that could still
  use that path, their actual terminal exit/status records, released target/lease
  and the owner's assertion of no remaining consumers;
- retained archive manifest path, format and extraction profile; finite ordered
  part list with exact byte lengths/SHA256, plus whole-archive identity;
- source-tree member manifest (relative path, kind, exact bytes/SHA256 for regular
  files, link target and executable/mode metadata where relevant), traversal and
  case-collision decisions, and total member/byte limits;
- actual archive creation and read-back verification receipts, including safe
  bounded extraction to a new scratch path and equality against the member
  manifest; archive existence or compression success alone is insufficient;
- retained private location/access policy for archives and logs, proposed action,
  dependency/retention constraints, and separately authorized rollback/recovery.

Counts or a matching aggregate size do not prove member coverage. A large archive
may be split into finite registered compressed parts; every part must be present
and verified. A hash of path strings does not prove captured file bytes. Sensitive
source/logs retain their existing private handling; compression is not redaction.

## State and ownership rules

States are `proposed`, `owner-confirmed`, `terminal-consumers-confirmed`,
`archive-verified`, `retirement-authorized`, and `retired`. Observing a stale board
claim or an idle chat does not establish a terminal consumer. State transitions
record exact receipts and never imply authority over another owner's paths.

Only an exact candidate with confirmed owner, terminal consumers, verified archive
and explicit disposition authority can reach retirement authorization. The source
planner must not recurse outside its declared path/byte/member caps, follow links
outside the selected root, kill processes, release foreign claims, move active
sources, or delete anything. A later separately authorized mutator requires its
own bounded actual tests and before-action revalidation. This contract implements
neither planner nor mutator and grants no new filesystem cleanup authority.

Scratch materializations are immutable for their selected run. Mutation/change,
unavailable metadata, unknown child handles, missing archive parts or failed
read-back verification stop the transition. A same-user concurrent replacement
is a residual identity risk; path/metadata observation is not an immutable snapshot
or permission to reuse a cache or execute archived contents.

## Wiring and validation

The owning agent-maintenance SoT governs this contract. A future implementation
belongs in the existing Rust1.99.1/edition2024 tool with explicit request and
metadata-only planning output; shared graph changes require their existing owner.
Validation remains scoped: pure transition/path/coverage/accounting tests first,
actual finite archive/part/read-back tests next, and source-bound hosted/container
gates only when their behavior requires them. No global private-tree scan is
required to prepare this contract or to validate one candidate.

## Critique reflected

Blue architecture requires original/canonical/scratch identities to stay separate.
Red review requires exact owner and terminal handle proofs: an archive alone cannot
justify deleting a live workspace. SRE review requires verified rehydration and
preserved historical paths, not a best-effort cleanup. Privacy review retains
private access rules and raw originals. Evaluation review requires canonical Git
payload checks and labels unexecuted planner/mutator behavior as unqualified.

Local design critique: ready as a bounded source contract, not an implemented
retirement tool. P1 risks of premature active-source deletion and silent archive
loss become mandatory refusal conditions above. P2 residuals include external
retention policy, OS metadata fidelity and concurrent hostile replacement; these
must remain explicit in any later executable design. The reported large private
Spec failure count is a root-supplied observation, not a new measured census here.

## Accepted metadata-only planner source slice

Root accepted the v0.1 contract for a planner in the existing maintenance crate.
`evidence-layout-plan REQUEST_JSON` now has a source implementation, not executed
qualification. Its strict version1 request uses exactly `schema_version`,
`candidate_id`, `repository_root`, `materialization_path`, `retained_root`,
`scratch_path`, `owner`, `source`, `consumers`, `archive`, `coverage`, `members`,
`readback`, `retention` and `proposed_action`. Nested DTOs deny unknown fields.
The Rust request model and finite fixture in the maintenance crate define the
exact field types. The only action is `retire_materialization`; it is a proposal,
never a mutator instruction. Supplied state/timeout/arbitrary action fields fail.

The shared selected-input request capture provides a2MiB true-EOF JSON boundary,
known-link/metadata refusal and one original cooperative30-second clock through
planning. Diagnostics omit submitted values. Only the request file is opened;
there is no candidate/archive/receipt lookup, directory traversal, archive
execution, extraction, move, deletion, process kill or lease release. Recorded
observation time excludes output serialization/stdout and is not a hard deadline.

Count caps are5000 members and64 each of handles, archive parts, lease and
dependency receipts. Each declared regular file/archive part is at most512MiB;
combined declared source-plus-archive bytes are at most2GiB, using checked sums.
These are metadata declarations, not measured actual content bytes. A source file
may be empty; a submitted archive part must be nonempty. Parts are ordered from0,
unique under the retained root, with exact declared total and whole identity.
One part must match the declared whole hash. Supported format labels are tar_zstd
and zip; no decoder is invoked. The member manifest fingerprint is domain-tagged,
length-framed SHA256 of sorted supplied path/kind/byte/hash/link/mode metadata.
Matching a submitted read-back fingerprint proves only self-consistent assertions.

Paths must be absolute and lexically nonescaping; Windows device/UNC prefixes are
refused. Retained archives must be under a declared repository `.spec.local` child.
Scratch must be outside that canonical tree and not overlap protected roots.
Archive retention must not fall under the proposed retirement subtree. Selected
members use slash-relative nonescaping paths, reject case-folded duplicates,
non-directory ancestors and unsupported link escapes. Lexical checks establish no
resolved filesystem identity, hostile-race protection or native alias guarantee.

Exit0 means a structurally consistent proposal was emitted. All actual verification
and retirement/execution authority fields stay false, even if submitted receipts,
terminal status and complete-coverage assertions agree. Such a plan stays
`proposed`; it never advances to owner-confirmed, terminal-consumers-confirmed,
archive-verified, retirement-authorized or retired. Semantic failures emit
`refused` plus static codes and exit1; malformed/oversized/refused request capture
exits2 with sanitized diagnostics. Output contains metadata/counts/fingerprints,
not submitted file contents or executable actions.

Authored gates are9 pure units and4 actual CLI cases. No Cargo/runtime slot has
been used for this slice. Proposed next allocation is the existing Windows
validation-maintenance target with fresh commit/free-memory admission,
jobs1/incremental0/offline/locked: complete library (expected73, exact listing
required), evidence_layout_cli4 plus selected_inputs_cli6/windows_doctor3
regressions, all-target check/strict Clippy and scoped fmt. Existing prior29-case
integration coverage is not implied by the expanded33-case source inventory.
Named120s profile binding, actual archive bytes/safe read-back, OS identity,
independent owner/terminal facts, hosted/provider evaluation and M11 completion
remain separate open gates. No cleanup authority is requested by this source.

### Actual bounded Windows qualification

Source577e1940 ran without overlays in a unique immutable Git-archive workspace
outside `.spec.local`, using the separately allocated absolute target
`C:/workspaces/humanymous/target/maintenance-evidence-plan-577e1940`. The create-new
lease bound source/workspace/target to codex/rust-agents-m11. Actual library listing
was73 and all73 passed with zero failures/ignored/filtered. PlannerCLI4 plus shared
selectedCLI6 and doctorCLI3 passed with the same zero exclusions. All-target check,
strict Clippy and scoped fmt exited0 on first attempt, within the360-second budget
(about41.4seconds). All42 archived inputs matched the source freeze and before/after
hashes. Terminal status and original lease bytes were retained, then the owned
marker was removed and target returned. Raw evidence is evidence:root/20261107172013.
This qualifies the metadata-only planner component, not archive contents/read-back,
OS/ownership/terminal assertion proof, retirement authority, full33 integration,
named120s binding, provider inference or the full M11 goal.
