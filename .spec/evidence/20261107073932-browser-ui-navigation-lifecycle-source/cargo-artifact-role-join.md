# Actual Cargo artifact to role preservation join

This delta addresses the remaining CA-BROWSER-ROLE-CALLER-01 gap after Root's
adoption of d08dc778. A successful Cargo exit and a later byte copy did not bind
the guessed `target/debug/humanymous_browser.dll` to the selected role. That gap
remains a recorded first failure; no previous receipt becomes native qualification.

`cargo-artifact-role-join.json` pins seven fresh current Root preimages and full
candidate postimages. Four files change; three are byte-identical. The complete
seven-file candidate/preimage trees are retained under
`.spec.local/root/evidence/20261108134000-browser-ui-artifact-ledger/`.
`cargo-artifact-role-join.patch` applies only against those exact Root preimages.
All seven matched Root's frozen tree when checked; patch applicability passed.
Neither Root's files nor any live source was written or executed.

## Implemented join

The existing locked Cargo metadata call now uses the existing strict JSON decoder
with an 8 MiB accepted-input limit. The context-specific `browser_identity`
selector requires one workspace Browser package at the exact expected manifest,
its opaque package ID verbatim, workspace membership, local package source, one
library target with the exact name/source path and the existing rlib/cdylib types.
It retains the target-directory and metadata digest. It does not parse or invent
the package ID from a name/version string. Workspace matching uses the current
directory representation; filesystem canonicalization is reserved for containment
at actual preservation so Windows extended-prefix spelling does not manufacture
a different expected metadata path.

Each actual role command writes a create-new private log. After successful command
completion, the driver requires unchanged declared source files and unchanged
metadata identity. The bounded existing text reader and strict JSON decoder then
select exactly one compatible `compiler-artifact` row and exactly one DLL output.
The row must match the metadata package ID, manifest, target name/source/types/kind,
exact role features, `profile.test=false`, no executable and a non-fresh compiler
output. Bootstrap features are exactly `cef-bootstrap`; desktop features are
exactly `cef-desktop,native-ui`. No pilot-test or combined-role feature is accepted.

The ledger must contain a final successful `build-finished` record and the actual
command exit must be observed zero. Missing, failed, duplicate, conflicting or
post-finish JSON records refuse the join. A Browser-shaped row belonging to a
different package refuses it; unrelated dependency artifact rows may precede the
Browser row. Malformed/oversized JSON, missing output, multiple DLLs, wrong target,
wrong role, cached `fresh=true`, executable/test-profile output, parent traversal
and absolute paths outside the metadata target directory also refuse it. Cached
artifacts are intentionally unsupported by this candidate; cache qualification
would need an explicit later decision. Bounds are 8 MiB ledger, 256 KiB JSON line
and 65,536 JSON rows. Non-JSON Cargo progress text remains private.

`retain_role` now reads the **selected compiler-artifact filename**, checks actual
canonical containment in the selected target directory, and feeds that actual
path to the existing create-new/sync/hash preservation. It no longer derives a
DLL path from `debug` or the target name. The next role begins only after this
join and preservation complete. The private staged receipt includes the actual
metadata/ledger/source digests, package/manifest/target, role/features/profile,
selected path, build completion and copied-byte hash/size. The preserved role
files still supply fresh runtime bundles and final packaged hash comparisons.
SDK resource provisioning is separate from this role DLL selection.

## Privacy, custody and verification limits

The existing public projection function is byte-equivalent to the frozen Root
preimage after trimming its surrounding blank lines. Ledger content, package IDs,
paths and profile details remain inside the private `cargo` receipt. Public schema
2 still exposes only its existing sanitized per-role build diagnostics and staged
hash/size fields. No raw logs or new generic parser/framework/dependency is added.
Root's module/entry visibility, reviewed package-local contract include and old
contract provenance are preserved. Producer/replica refresh remains Root pending.

Four pure Rust regressions were authored: exact metadata/opaque identity selection
with negative fixtures; successful bootstrap and desktop output joins at an
actual filename different from the guessed debug path; adversarial artifact and
completion fixtures; and private ledger/path exclusion from the safe projection.
They cover wrong metadata, package, manifest, target, role/features, profile,
duplicate/conflicting/missing/stale/output escape and unsuccessful/missing build
completion. None ran. Root's separate same-byte-copy regression and its explicit
three-file/directory cleanup remain unchanged, including the removal of recursive
silent Drop cleanup. Byte-copy fixtures are not PE export/import or runtime proof.

Static source review confirmed removal of the guessed DLL path from preservation,
the ledger call before copy, retained private join data and unchanged public
projection/copy-test cleanup. Applicability against the exact Root preimages
passed. No formatter, compiler, Cargo metadata invocation, test, SDK provisioning,
DLL build, PE inspection, hosted process/browser pilot or publication ran.

Root must rejoin its final source/metadata/lock epoch, inspect/apply the delta,
format and type-check it, run the pure fixtures and separate copy unit, refresh the
reviewed contract and then run both real hosted SDK roles and actual native pilots.
An exclusive owned build target remains an execution prerequisite; a ledger and
copy receipt alone do not prove exclusion of arbitrary external file writers or
signed upstream distribution provenance. Actual CEF/IME/AX/DPI/sandbox/UI and
distribution qualification remain open, with Rust-only and mandatory sandbox intact.
