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
