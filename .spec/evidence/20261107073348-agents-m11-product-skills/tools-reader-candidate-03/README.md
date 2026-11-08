# Reader composition and actual package boundary candidate 03

NOT_COMPILED / NOT_EXECUTED / NOT_INTEGRATED. This candidate is against the
observed Root-owned tools-integration-5f1010d4 assembled source after language
and candidate02 integration, not an unmodified Git revision. Every observed
original is retained byte-for-byte as a .base attachment with its exact hash.
Root source, Cargo and isolated snapshot were only read, never edited.

## Concrete composition changes

1. Remove I/O child-module declarations from pure language/product/catalog/registry
   modules. Keep every existing pure implementation and test in its owning file.
2. Declare those same four physical reader modules only in command.rs. Replace
   their super imports with explicit imports of the SINGLE canonical pure parser:
   crate::maintenance::language/product/sessions::catalog/sessions::registry.
   Readers stay at their existing physical paths; no source copy or new package.
3. Remove only obsolete root-only product/catalog/registry inspect(root) functions
   and imports made unused by those removals. Existing language::inspect pure
   function and product::load used by package staging remain intact. Actual new
   source/content request commands keep their current semantics/receipts.
4. Replace the product-package integration root's whole maintenance/mod.rs link
   with real subprocess calls to the existing Cargo-selected maintenance binary's
   stage-eval command. This is an actual executable boundary, not a stub or copied
   package implementation. Both exact original test IDs/assertions are retained.

The package boundary therefore has ZERO maintenance source-module imports.
This is smaller than linking package/inventory/product/model/catalog and avoids
recursively pulling all14 modules and their library units into this integration
target. Those implementation modules still compile once per owning maintenance
binary/unit role. There is no new library facade, duplicated parser, forwarding
crate, blanket allow, fake call, ignored test or cfg-disabled required test.
Root decides whether to adopt this actual boundary alternative to source linking.

## Exact source map

All paths below are relative to Root's CLI src/qualification directory. Attachment
names flatten slashes to double underscores; candidate.patch uses real paths.

| Source location | Candidate action |
| --- | --- |
| maintenance/language.rs:2-3 | Remove source child declaration only |
| maintenance/product.rs:2-3 and165 inspect(root) | Remove reader child and obsolete inspect; preserve load/validate and every unit body |
| maintenance/sessions/catalog.rs:2-3,5,127 inspect(root) | Remove reader child and obsolete filesystem inspector/imports; keep canonical parser/projection validation and all units |
| maintenance/sessions/registry.rs:2-3,5,64 inspect(root) | Same separation; keep canonical registry validator and every unit |
| maintenance/command.rs imports and37/71/77/83 command arms | Own four private reader module declarations and directly call them; drop now-unused pure module imports |
| maintenance/language_source.rs:2 | Import canonical language parser explicitly |
| maintenance/product_selected.rs:2 | Import canonical product parser explicitly |
| maintenance/sessions/catalog_selected.rs:2 | Import canonical catalog parser explicitly |
| maintenance/sessions/registry_selected.rs:2 | Import canonical registry parser explicitly |
| tests/maintenance_product_package.rs:3-5 and verify_bundle | Remove whole maintenance module link; call real binary, deserialize its actual receipt locally, preserve verification and refusal checks |

The two boundary IDs are unchanged:
product_bundle_contains_paired_suite_and_refuses_existing_output;
developer_bundle_contains_separate_five_case_suite. The original3/5 expected eval
case counts, per-file byte/hash checks, product/developer exclusion, nonqualified
receipt flags, existing-output refusal and before/after receipt equality remain.
New assertions check receipt schema/kind, finite file count and output path safety.
The local Deserialize structs describe receipt DATA only, not a mirrored package
implementation or public fixture API.

## Unit ownership and required census

All original76 library cases remain owned by the maintenance binary test root's
same maintenance/mod.rs and physical parser/helper source modules. This patch
does not remove, cfg out or ignore any of them; it stops compiling them a second
time through product-package's whole-module import. Source-level original test
functions in the nine implementation files compare unchanged before/after.
Root must verify exact --list membership under its final source/role before
running. No count inferred here replaces that required census.

Seven new reader parser units relocate in the SAME maintenance binary unit role:
maintenance::language::source::tests becomes
maintenance_command::language_content::tests (four existing function IDs);
maintenance::product::selected::tests becomes
maintenance_command::product_content::tests (one);
maintenance::sessions::catalog::selected::tests becomes
maintenance_command::lane_content::tests (one);
maintenance::sessions::registry::selected::tests becomes
maintenance_command::thread_content::tests (one). unit-ownership.json lists
every preserved function ID and old/new prefix. No new reader case is discarded.

The product-package target should list exactly its two original boundary cases,
not two plus the entire library census. Preserve the full33 original integration
IDs/23 boundary plus10 owned-filesystem/provider denominator across actual roles.
This source-composition change neither runs nor qualifies any owned-filesystem
case; those native/filesystem tests remain approved HOSTED-ONLY. No local pilot,
compiler, fmt, Clippy or --list was executed for this candidate.

## Explicit source/run contract and remaining decisions

Before approved hosted execution the runner MUST set
HUMANYMOUS_MAINTENANCE_PACKAGE_ROOT to the absolute concrete selected source
root from Root's frozen source/run manifest. No CARGO_MANIFEST_DIR parent, CWD,
HOME, vendor mirror or default repository discovery exists. Missing variable,
relative root or linked/reparse ancestor fails the required test; no skip.
The executable is env!(CARGO_BIN_EXE_humanymous-agent-maintenance), the existing
explicit Cargo binary role. The test launches no shell and changes no live Git,
claims or provider configuration. Each successful output is a fresh owned temp
artifact; a failed assertion retains output/diagnostics. Successful output alone
is removed after all checks, as in the original tests.

The root variable is EXPLICIT SELECTION, not authenticated source identity.
Root's outside runner still must bind immutable source extraction, exact binary,
finite source-input manifest and before/after closure to its receipt. These tests
exercise the current real stage-eval behavior; they do not magically implement
the separately pending finite staging reader contract or authenticate a caller's
Git declaration. That limitation is unchanged, not hidden by a synthetic fixture.
Root's content-only successful-command versus qualified-identity decision is
unmodified. New selected reader receipts still separate content completion from
unsupported identity. Final adoption, test feature/role joins and actual gates
remain Root-owned.
