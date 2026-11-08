---
id: plan:root/20261107235818
title: Delivery candidate dependency completeness contract
status: draft
created: 2026-11-07T23:58:18Z
updated: 2026-11-07T23:58:18Z
---

# Delivery candidate dependency completeness contract

## Brief and authority

Version0.1; proposed implementable contract, not accepted or executed qualification.
Root explicitly assigned source-independent Delivery design after Tools freeze work
began. No source/Cargo/resource changes, compiler, Docker, hosted watch or dispatch
are authorized by this document. Detection, a11y and enforcement remain unchanged.
The private-pipes candidate is not a Core/Gate shipping replacement. Its successful
observed86-package scan at1f87 proves neither this contract nor dependency completeness.
The retained shipping-role plan remains a separate prerequisite to shipping.

## Evidence basis and limits

Current image_scan projection requires one nonempty rustbinary inventory, checks
source/image identity and all reported vulnerabilities, and deliberately leaves
inventory_completeness_verified, shipping_qualified and full_product_qualified false.
The candidate builds one selected gateway executable with defaults disabled using
cargo-auditable0.7.7 and Rust1.99.1; future package placement must follow Tools freeze.
No old source path in this plan authorizes a replacement package or executable.

Pinned upstream revision3d3bb574f97dc09a264cfc666ec5d40790f6b3ba:

- [auditable-serde format semantics](https://github.com/rust-secure-code/cargo-auditable/blob/3d3bb574f97dc09a264cfc666ec5d40790f6b3ba/auditable-serde/src/lib.rs): formats0/1 may overapproximate;0 can misclassify proc macros;8 uses a different producer. An embedded graph is a producer assertion, not independent compilation evidence.
- [wrapper setup](https://github.com/rust-secure-code/cargo-auditable/blob/3d3bb574f97dc09a264cfc666ec5d40790f6b3ba/cargo-auditable/src/cargo_auditable.rs) and [injection](https://github.com/rust-secure-code/cargo-auditable/blob/3d3bb574f97dc09a264cfc666ec5d40790f6b3ba/cargo-auditable/src/rustc_wrapper.rs): the workspace wrapper embeds metadata and may complete without injection on unsupported paths. Build exit0 alone is insufficient.
- [extractor](https://github.com/rust-secure-code/cargo-auditable/blob/3d3bb574f97dc09a264cfc666ec5d40790f6b3ba/auditable-extract/src/lib.rs) locates executable audit sections; decompression/schema/graph validation remain additional obligations.
- [Cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html) identifies packages with opaque IDs; package lists alone are not per-artifact compilation graphs.
- [Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html) can compile/link native code and generate inputs. Their Rust host dependencies must not be mistaken for all native code they introduce.

The fixed upstream sources were read during this design. Current Cargo reference
pages explain interfaces, not a claim that a new compiler/tool version was tested.

## Four separately retained views

1. Intent I: exact source commit/tree/manifest digest, selected manifest and opaque
   package ID, binary target name/kind/source path, platform triple, host triple,
   explicit default-feature boolean and feature list, release/profile/cfg flags,
   lock/config/tool/wrapper hashes and source/archive provenance.
2. Build B: actual successful normal Cargo build JSON, actual compiler invocation
   transcript and artifact hashes, plus metadata for resolving identities. B is
   independent of the embedded metadata producer; never synthesize B from E or S.
3. Embedded E: bounded extraction from the final executable's actual bytes,
   compressed-section and decoded-document digests, exact format revision, root,
   identities, runtime/build roles and dependency edges.
4. Scan S: original successful all-package/all-severity Trivy JSON for that exact
   image, tool/database identity, binary target and observed package multiset.

All originals remain private, registered with hash AND byte length. Public output
contains finite reason codes, counts and digests; no raw compiler arguments, home
paths, registry credentials or unbounded scanner text. Decoded JSON and original
tool stdout are distinct byte representations. Missing originals cannot be repaired
by copying hashes from public logs. Source mutation invalidates the receipt.
Initial parser bounds:100000 compiler units,1000000 total dependency edges,
64MiB each Cargo/transcript/scan JSON,1MiB compressed audit section and16MiB
decoded audit JSON, nesting depth64. Reject duplicate keys and integer overflow.
Stream records within those aggregate bounds; do not allocate from an unchecked
declared length. Root may lower limits after actual measurement; raising them is
a reviewed contract change. Parent build timeout/process-tree custody comes from
the existing allocated runner, not an observer-created background process.

## Actual build and artifact joins

Use one fresh isolated target directory and one selected executable, without
cached/fresh artifacts or shared target reuse. Preserve exact command argv and
bounded successful Cargo --message-format=json stream including build-finished.
No --workspace/--all-targets/test/dev expansion may substitute for the selected build.
Require one successful selected compiler-artifact executable, joined by opaque
package ID, target kind/name/source, profile, features and target. Rehash the output
before and after packaging; inspect/extract the container executable without running
it, and require byte equality to the Cargo artifact. Join the inspected image ID,
source labels, scanned image ID and binary digest. Labels alone are not a join.

Archive checksums do not bind mutable extracted registry/Git source directories.
Before compilation, create a controlled source store from verified archives/Git
objects and record the actual extracted file closure, including local/path crates,
patch/source replacements and external include inputs. Record path/type/size/digest,
reject out-of-store links or unmeasured external paths, and compare after the build.
Do not claim an archive-only before/after census measured these extracted inputs.
Actual manifests, .cargo configuration, target specifications, compiler/wrapper
binaries, allowlisted build-affecting environment and generated/link inputs also
belong to the retained provenance record. Missing closure is a finite provenance
failure and keeps build_artifact_graph_verified=false, even if graph joins match.
Ordinary Cargo/build-script execution can read undeclared host inputs; hashes alone
are not a hermeticity proof. Actual admission must state its isolated filesystem
and network boundary, residual reads and generated-input ownership. Uncontrolled
reads remain unqualified provenance, never an implicit complete-source claim.

Implement a Rust observer as RUSTC_WRAPPER around Cargo's actual compiler chain;
cargo-auditable retains its pinned RUSTC_WORKSPACE_WRAPPER role. No shell wrapper.
Record argv as OS arguments, cwd, allowlisted Cargo identity variables, effective
target/cfg/features, --extern artifact references, crate types/output locations,
child executable identities, terminal exit and output hashes. Preserve semantics,
exit/signal propagation and argument ordering. Reject unapproved extra wrappers,
response files or unparsed graph-affecting flags until explicitly supported.
Observer cannot claim to observe injected inner arguments it did not see. Admission
must test this precise chain with the pinned auditable implementation and prove
that its transformation preserves the recorded --extern graph; otherwise B is
unqualified. This is an actual compilation receipt, not merely a wrapper log file.

Join every relevant invocation/output to Cargo artifact/metadata identity using
exact manifests, declared targets and output paths, not crate-name suffixes.
Renamed --extern names map to actual artifact paths and full package identities.
Same package may produce different host/target/feature units. Keep them distinct.
Compiler query invocations have an explicit non-artifact class; unknown units fail.
Every edge from the selected executable must resolve exactly once. Reject absent,
ambiguous, foreign, failed, cached-only, mutated or duplicate unit records.

## Expected graph and role algorithm

Start at selected executable unit. Traverse actual target --extern edges to normal
target Rust libraries. A proc-macro edge enters a host-only compilation role; its
transitive graph is build provenance, not target-runtime membership. Build-script
executables and their host libraries likewise enter build provenance. Host==target
does not merge these roles. If a package is independently reached as target runtime
and host tool, retain both units and project package membership to runtime.
Record all observed build/proc-macro units for provenance and separate tool audits;
excluding their runtime membership does not waive findings reported by Trivy.
Do not exclude a normal target crate because its functions might be optimized out.
This graph describes compiler input package coverage, not surviving machine code.

Package identity K is name/version plus exact Cargo source identity and registry
archive checksum, Git revision/tree or local source manifest digest. Embedded
formats may collapse source details. Project K to an E/S join key only if injective
for this build; same name/version from multiple sources refuses ambiguous equality.
No basename/URL suffix guesses or whole-lock inventory. Retain full K privately.

Expected target package set R is the normal target closure plus selected root,
collapsed only after unit-role validation. Expected build set H is host-only units.
E must have exactly one matching root, valid bounded acyclic edges, known format,
and an unambiguous source/identity projection. Require E.runtime == project(R).
For E.build, require each identity to resolve to retained actual H; unexplained
extras/missing entries or role misclassification refuse completeness. Older formats
that misclassify proc macros cannot be silently reinterpreted as a newer format.
Known metadata overapproximation is a failed/unqualified contract result, not an
allowlist or count adjustment. A future producer correction needs separate review.

Trivy may expose less role/source information than E. Admission fixtures and actual
candidate output must establish its exact pinned projection P: E -> rustbinary rows,
including whether root/build packages are emitted. Require S == P(E) as identities
and multiplicities with exactly one expected binary inventory. P is fixed before
candidate verification and cannot be derived to fit each scan. Unsupported format,
unknown root policy or lossy collisions leave scanner_inventory_complete=false.
All reported vulnerabilities across ALL scan results still fail the CVE gate,
including UNKNOWN severity and host/build records. No suppressions for projection.

## Native, generated and toolchain coverage

Rust package equality does not enumerate bundled C/assembly, copied files, generated
code provenance, statically linked libc/compiler runtime, sysroot crates or dynamic
loader dependencies. Record build-script outputs/link directives and final executable
format/architecture/interpreter/NEEDED libraries; they are discovery evidence only.
For every native/system component require source/archive/version/checksum/license,
build recipe and link-to-final-artifact evidence, plus applicable vulnerability scan
coverage. Rust std/compiler runtime need pinned toolchain provenance and separate
advisory disposition. Missing native/linker attribution remains unknown, not empty.
Scratch, static linking, no NEEDED entries and zero Trivy OS packages prove none of
these inventories complete. Unsupported scanner coverage is explicit unqualified
coverage; no permanent false-positive suppression or broad ignore may hide it.

## Result schema and fail-closed states

New proposed schema humanymous.image-dependency-completeness/1 records I/B/E/S
attachment hashes/sizes, artifact/image/binary joins, platform/tool identities,
runtime/build counts, expected/embedded/scanner graph digests, native/toolchain
coverage and finite failure codes. Record independent booleans:
build_artifact_graph_verified, embedded_runtime_inventory_complete,
scanner_inventory_complete, native_inventory_complete, toolchain_inventory_complete.
inventory_completeness_verified is their conjunction. No generic boolean may erase
an unknown class. Missing/invalid records produce false plus reason and nonzero
required-gate exit, while preserving the first failure evidence.
Candidate-only CVE success retains its current narrow observed status; this new
required completeness gate must not turn that success into shipping qualification.
Even all completeness booleans true leave shipping/full_product false until the
separate accepted role, lifecycle, product topology and release requirements pass.

## Implementation slices after separate authorization

1. Pure Rust bounded schemas, opaque identity joins and graph/projection verifier
   within Delivery's final frozen private qualification module. No helper package,
   public SDK API or duplication of Cargo resolution logic.
2. Private Rust compiler observer role in the same Tools package, default-off and
   excluded from production release. Root must freeze its bin/dispatch identity,
   self-executable restrictions and wrapper composition before source implementation.
3. Pinned safe Rust binary extractor/decompressor/schema validation, installed from
   reviewed locked sources; no new version/dependency granted by this design.
4. Candidate builder retains normal Cargo JSON/transcripts/artifact and container
   binary evidence; verifier consumes retained files offline. Image scan joins S.
5. Add separate required completeness gate; Root owns workflow insertion/required
   graph. Preserve legacy Core/Gate guards and publication=false throughout.

## Meaningful unit and actual verification plan

Pure graph tests use small declared graphs: renamed dependency; two versions;
same-version distinct sources; host==target proc macro with host dependencies;
same package in both runtime/build roles; unused workspace feature; alternate target;
build-script native output; missing/duplicate/foreign/cyclic edge; mutated artifact;
failed build/no build-finished; fresh-only cache; stripped/missing/truncated/bomb audit
section; unsupported format; root/role mismatch; scanner dropped/extra/duplicated
package; source-key collision; unknown native component; all-severity finding.
Include an unchanged verified archive with a modified extracted source, external
include escape, source replacement/config drift and unmeasured generated input;
each must refuse provenance despite otherwise identical package names and counts.
Expected membership is hand-declared independently, never generated by the tested
algorithm. Test finite public diagnostics do not echo private path/canary text.

Actual admission, under a separate Root compiler/container allocation, uses a tiny
owned Rust fixture with runtime, proc-macro and build-script roles and a same-package
dual role; compile real pinned wrapper chain, compare actual Cargo JSON/transcript,
extract final executable and exercise copied/tampered/missing-artifact negatives.
No unit transcript alone qualifies observer composition. Keep first failures.

Then build the exact candidate in standard hosted Docker for one explicit platform,
retain original I/B/E/S and native/toolchain inventories, inspect final binary bytes
and require every join above. Never replace this with a second host-only build or
cross-compile execution claim. A real format/metadata/scanner mismatch is a reported
failure to fix in its owner, not a synthetic fixture update. Repeat for each claimed
platform. Run separate Docker-only accepted role/product/lifecycle gates before any
shipping claim; do not deploy, publish, exchange OIDC credentials or cut a tag.

## Critique reflected and readiness

Single-author blue/red/SRE/DPO/evaluator review using loaded repository personas;
not an independent panel. Adversarial verdict: needs-work until observer/tool
composition and native provenance are actually demonstrated.
P0 circular metadata proof is prevented by independent actual compilation B and
final-byte joins. P0 host-role confusion is prevented by unit-level traversal.
P1 metadata supersets, ambiguous names and unsupported scanner projections refuse
qualification. P1 Rust-only coverage cannot certify native/toolchain content.
P1 copied logs, cache reuse and image labels alone cannot establish custody.
P1 public logs cannot expose private compiler arguments. No current candidate
result is promoted. This design is ready for owner review, not shipping or source
implementation; Root must accept exact slices and Tools identities first.
