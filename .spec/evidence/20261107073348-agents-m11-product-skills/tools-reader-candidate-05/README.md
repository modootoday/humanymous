# Reviewed fixed-input request materializer candidate 05

NOT_COMPILED / NOT_EXECUTED / NOT_INTEGRATED. Three exact observed originals
from Root's isolated tools-integration-5f1010d4 are preserved as .base attachments.
Only owned evidence was authored. No Root/private snapshot, live source, Cargo,
workflow, canonical script or legal text was edited; no compiler/native run.

## Actual code and concrete command

Candidate command argument order:

`humanymous-agent-maintenance prepare-stage-request ABS_ALLOWLIST REVIEWED_SHA256 ABS_FROZEN_ROOT EXPECTED_SOURCE_COMMIT product|developer ABS_NEW_REQUEST`

| CLI src/qualification/maintenance location | Concrete candidate |
| --- | --- |
| package_selection.rs | Add Serialize to the SAME Kind/Request/Pin types; add shared raw verify_pin predicate; original strict decode and all four units retained |
| package_stage.rs | Share that exact raw predicate instead of its inline duplicate; selection and existing output_path become visible only to command siblings; existing staging behavior unchanged |
| command.rs | Own package_materialize module and explicit six-operand command; no defaults/ambient roots |
| New package_materialize.rs | Review-hash check, explicit epoch/kind/root binding, bounded actual raw capture and fresh request write/sync/read-back; four pure units |

No core parser is copied: the materializer deserializes through the existing
deny-unknown/duplicate Request type, then invokes existing selection::decode
after explicit root binding and again on the exact outgoing bytes. All original
profile/case/skill/path/cap predicates remain owned by that one implementation.
The shared verify_pin is used by both staging and materialization.

## Owner-reviewed allowlist and explicit root policy

The allowlist has candidate04's v1 staging fields, source epoch and exact raw
pin expectations. The caller supplies its independently reviewed raw SHA256.
The materializer first captures/hash-compares those exact allowlist bytes.
Unknown/duplicate JSON fields fail, never disappear through generic JSON rewriting.
Wrong review hash, source commit or kind refuses before selected source reads.

repo_root is either already identical to ABS_FROZEN_ROOT or EXACTLY the reviewed
marker `__FROZEN_CANONICAL_ROOT__`. Only that marker authorizes binding to the
explicit caller-selected root. No other old/different root is overwritten, and
there is no CWD, package-parent, live-worktree, Git/network or directory discovery.
The marked allowlist is deliberately NOT a runnable staging request; after
binding, the existing strict decoder requires a concrete absolute safe root.

Each actual selected file is captured with existing capture_source_file and
the reviewed length as a tighter cap. Raw bytes must match the reviewed length
AND SHA before any outgoing pin is assigned. The assigned length/hash is derived
from that same capture and necessarily equals the reviewed pin. There is NO
refresh, normalize, retry or changed-LICENSE exception. The pure unit exercises
both CRLF and same-length legal-text changes and checks the reviewed pin remains
unchanged. Actual legal/source bytes are never written by this command.

The example allowlists preserve candidate04's exact16 product/40 developer input
pins,3/5 case documents and14 developer skill names for THAT reviewed closure;
only the explicitly reviewed root marker differs. LICENSE remains11353 raw bytes,
SHA256 28153ab6c3197253563f1ae0932a89a758d2d0f9c667d59da2e41c01c9cbfeb1.
This agrees with Root's frozen canonical legal pin; no legal bytes were normalized
or copied into compiled CLI. New request JSON is authored UTF-8/LF metadata,
not a reinterpretation of the source legal hash. Do not reuse the old request
SHA for the marked allowlist or for the generated/root-bound request.

## Bounded output and receipt

Existing per-file/profile limits stay unchanged: allowlist/request <=2MiB,
<=510 source inputs, product entry <=256KiB, other inputs <=2MiB, source text
<=8MiB. Same original30s cooperative clock spans allowlist, all selected reads,
serialization, write/sync and bounded exact read-back. Actual content reads are
accounted up to12MiB+512, including allowlist, source, outgoing request read-back
and growth probes. This new operation has one additional bounded request read;
it does not loosen staging's limits or authorize scanning.

Output admission reuses stage's existing output_path: explicit absolute fresh
file, existing checked parent, outside canonical source, active-provider paths
excluded. create_new refuses existing/unknown paths; no overwrite. No file is
created before ALL reviewed pins match and final schema/size checks pass.
After write/sync/close, the existing capped capture reads it back; exact byte
equality and its raw SHA are checked. Failures retain partial owned output and
return non-success. No environment is changed and staging is not executed.

Successful JSON schema: humanymous.stage-request-materialization.v1. Fields:
kind; reviewed_allowlist_sha256; reviewed_root; repo_root; declared_source_commit;
request_path; request_sha256; request_bytes; inputs; actual_read_bytes;
elapsed_micros; raw_pins_match_review/materialization_complete=true. The receipt
keeps canonical_tree_complete/source_commit_verified/owner_identity_verified/
provider_identity_qualified/qualified_complete/staging_executed=false and explicit
unsupported_source_identity status. SHA equality is not owner authentication,
whole-tree closure, Git authenticity, a frozen filesystem provider, CA-LIFETIME
or a package-test pass. The outside sealed-source/runner gate remains required.

## Concrete Delivery handoff

Delivery agreed an explicit existing runner-owned fresh directory under runner.temp
with a run/attempt-specific allocation, not CWD/package root. Root owns the final
directory and command integration. Use distinct absent product/developer request
file paths and separate later stage output paths. After an OUTSIDE canonical-root
closure/seal gate, successful terminal materializer exit and request SHA receipt
verification, bind only actual absolute receipt.request_path values:

| Receipt kind | Existing hosted package test environment |
| --- | --- |
| product | HUMANYMOUS_MAINTENANCE_PRODUCT_STAGE_REQUEST |
| developer | HUMANYMOUS_MAINTENANCE_DEVELOPER_STAGE_REQUEST |

Then existing tests run `stage-eval ABS_REQUEST ABS_NEW_OUTPUT` through their
exact Cargo-selected binary. Never set these envs to placeholder examples or
partial/nonterminal output. Preserve the actual exit code, allowlist and request
raw hashes, root/epoch seal, binary identity and artifact provenance. Delivery
received this concrete candidate invocation/receipt schema, pending Root review;
no workflow patch, execution or publication is implied.

## Required remaining checks

All Root's original97 actual-listed cases and33 integration IDs stay required;
the four new pure materializer units are additions, not substitutes. Existing
selection test IDs and stage request behavior remain unchanged. The actual two
package subprocess cases,10 owned-FS/provider cases and coordinator's separately
required include-ignored hosted gate remain HOSTED-ONLY/NOT_RUN here. Canonical
coordinator script remains external, source-pinned and unmodified; no foreign
implementation/template is created. Root's script pin8781B/SHA4244f101... is
outside this materializer's allowed profile and cannot enter a staged request.

Required actual admission/verification: Root reviews the new allowlist hashes,
materializer CLI/source/feature closure, exact --list additions, fmt/strict Clippy,
then approved hosted materialization plus valid staging and changed LICENSE /
other pin / wrong epoch / wrong kind / unrelated root / existing output negatives.
No pure green may replace those boundaries. Source authority and CA-LIFETIME
remain open; this candidate makes no new provider claim.
