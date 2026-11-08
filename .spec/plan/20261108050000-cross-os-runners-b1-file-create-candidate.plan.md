---
id: plan:root/20261108050000
title: Accepted B1 descriptor-at-create File primitive and fixture source candidates
status: draft
created: 2026-11-08T05:00:00Z
updated: 2026-11-08T05:00:00Z
rel:
  depends_on: [sot:root/structural-development, sot:root/cross-os-runners, sot:root/native-security, plan:root/20261108040000]
---

# Exact authority and source input

Root accepted B1 ONLY after independent X7e341dcb: ONE seventh private function
windows_object::create_file::create_native, default-off Windows-only
windows-file-create implying windows-object. Existing six scopes unchanged.
This authorization permits ACTUAL Rust candidate and pure/native fixture SOURCE
in owned evidence only. No Root/live source/Cargo edit, formatter/compiler/native
execution, actor/account/SCM or B2 operation is granted. B2 ten scopes remain
unapproved; previous B1 proposals' lone into_file are superseded by this decision.

Input is Root's current isolated Platform under
.spec.local/root/evidence/20261108054000-core-consolidation/tools-integration-5f1010d4/
crates/integrations/humanymous-platform/. Its manifest ALREADY reserves feature
and exact seventh-scope comment. Candidate changes NO manifest. source-inputs.json
records exact input hashes, selected dependencies and candidate sizes/hashes.
New source is in evidence:root/20261106223747/b1-file-create-candidate:
mod.rs (existing windows_object preimage plus gated public child module only),
create_file.rs and create_file/native_tests.rs, with final-path patch.

# Actual implementation and accepted unsafe operations

Safe-facing path is windows_object::create_file::create_new_file_with_descriptor,
not a root facade reexport. It returns must-use CreatedFile with PRIVATE actual
File and preallocated CreationObligation/PathBuf. Only file()/path() BORROW methods;
no into_file, Clone, serde, raw constructor/getter, admitted marker, identity,
parent/O/provider/original-clock/NativeProof/Ready conversion or cleanup receipt.
Drop closes File via std ownership; it never deletes, repairs or reports cleanup.

All UTF16, owned locator, obligation and SECURITY_ATTRIBUTES preparation completes
BEFORE native creation. Borrowed descriptor lifetime is retained in private
PreparedCreation; the pointer refers to the pinned windows-permissions opaque
native allocation, NOT a WinSafe layout cast. The unsafe function calls ONLY:

1. CreateFileW with desired0xc0020000, share0, borrowed noninheritable descriptor,
   CREATE_NEW, FILE_ATTRIBUTE_NORMAL|FILE_FLAG_OPEN_REPARSE_POINT, null template.
   Invalid/null return immediately captures redacted safe last_os_error via
   existing os_failure; no logs, follow-up native calls or output interpretation.
2. OwnedHandle::from_raw_handle of EXACT successful owned handle ONCE, then safe
   File::from and infallible CreatedFile field moves. No allocation/log/unwrap/
   validation gap can lose a successfully returned owner. No manual CloseHandle.

No open-existing, truncate, retry-name, repair, delete, query, process, Job or
generic attribute/native callback is inside the seventh scope. Existing six source
files are read-only preimages and unchanged. Caller post-validation must retain
the returned CreatedFile; primitive returns no detached File on later failure.

# Locator contract and explicit namespace limits

Validate ORIGINAL UTF16 before mutation, rather than Path::components normalizing
away dot traversal. Only explicit drive-letter absolute backslash form with
nonempty components is supported. Reject UNC/device/verbatim/relative/rooted-without-
drive/forward-slash/ADS/NUL/control/Win32-invalid characters, dot/dotdot, repeated
or trailing separators, trailing ASCII dot/space, component>255 UTF16 units and
bounded total input>=32768 units before adding terminator. Device stems are checked
case-insensitively before extension, including trailing stem spaces, CON/PRN/AUX/
NUL/CONIN$/CONOUT$/CLOCK$, COM1..9/LPT1..9 and Windows superscript1/2/3 port aliases.
Legitimate Unicode remains unchanged, including CJK, Greek, accented characters
and surrogate pairs. No to_str/ASCII-only component restriction or normalization.

This is lexical supported-input rejection, NOT handle-relative namespace admission.
Parent reparse/alias/hardlink/identity/provider/fixed-volume/role and descriptor
policy remain the real caller's independent checks. No canonicalized path, checked
descriptor syntax or successful File create manufactures those authorities.
Long-path OS/provider support may still refuse natively. Missing parent/O/original
I/namespace/genesis manufacturers remain Unsupported as before.

# Meaningful source tests, separately classified and NOT_RUN

TWO pure functions in create_file::tests:

- locator_rejects_devices_aliases_ads_and_traversal_before_creation
- locator_preserves_legitimate_unicode_and_bounds_utf16_components

They exercise reserved/traversal/ADS/NUL/namespace negatives and Unicode/bounds
without any native mutation. No trivial constructor test is counted as proof.

FOUR actual native parent fixtures in create_file::native_tests are explicitly
ignored until Root allocates real Windows ordinary actor, private parent and
original outer budget. They are REQUIRED for B1 native qualification, not waivers:

- actual_descriptor_is_present_on_first_owned_file_before_any_repair
- actual_share0_denial_does_not_substitute_for_descriptor_readback
- actual_create_new_collision_preserves_existing_bytes_and_descriptor
- actual_rejected_validation_keeps_owner_and_drop_never_deletes

Native source uses existing safe current_process_sid solely for fixture descriptor
selection, no installer/process/provider authority. It queries actual same-opened
owner/protected DACL/type/ACE masks independently of a second-open attempt. Sharing
violation32 is recorded ONLY as share0 denial; not ACL or distinct-peer denial.
Collision sentinel bytes AND owner/DACL are compared after refusal. Post-validation
synthetic refusal returns the ACTUAL CreatedFile in a retained failure, confirms
held share0 and file metadata, then closing owner is shown not to delete. Synthetic
failure is labeled controlled, not an injected Windows failure or durable cleanup.

HUMANYMOUS_B1_FIXTURE_PARENT is only an explicit existing-parent locator; environment
does not attest hosted actor/private namespace/source or original clock. Fixture
leaf uniqueness uses PID/time/sequence solely as a locator, never entropy proof.
Fixtures create only new named files, explicitly remove only their own files after
closing, no recursive cleanup/Drop deletion/owner repair. Real caller/outer actor
must retain parent/obligation inventory on assertion panic or incomplete cleanup.
The fixture itself cannot manufacture production namespace cleanup authority.

# Required gates remain separate

| Gate | Source candidate / required actual observation | Current status |
|---|---|---|
| Default feature | Default[] has no B1 module or new unsafe scope; exact existing default surface stays unchanged | NOT_RUN |
| windows-object only | Existing six wrappers remain, create_file absent | NOT_RUN |
| windows-file-create | Only Windows+feature compiles new primitive; feature implies object; formatter FIRST/check/strict Clippy | NOT_RUN |
| Unsafe/privacy negative | Only seventh private create_native allows its exact two unsafe operations; external construction/private native entry/into_file/Clone/raw proof unavailable; no broad allow | Source written; compiler negatives NOT_RUN |
| Pure paths | TWO exact tests, no native effect | NOT_RUN |
| Collision | Existing bytes/descriptor unchanged; no fallback/truncate/repair | Fixture SOURCE; NOT_RUN |
| Actual DACL at create | First same-owned File readback proves expected descriptor, no repair API present | Fixture SOURCE; NOT_RUN |
| Actual share0 | Own second open fails32 while held, same owner can reopen after close; distinct from DACL | Fixture SOURCE; NOT_RUN |
| Retention | Actual created owner stays in post-validation failure; Drop closes without deleting | Fixture SOURCE; NOT_RUN |
| Distinct peer ACL | Actual independently provisioned peer denial under granted role rights, not hidden by share0; additional caller/venue required | NOT_IMPLEMENTED/NOT_RUN |
| Original budget | Real caller's original pre-I/O clock and independent outer supervisor remain active across potentially blocking CreateFileW/queries/cleanup; no B1 budget factory | Caller manufacturer absent; NOT_RUN |
| Namespace/provider/genesis | Same File ID/ACL/type/provider/link/held ancestors, original I/O admission, independent O journal genesis and durable replay | Unsupported; NOT_RUN |

Compile-fail docs cover private fields and absent into_file; their execution is
pending, not privacy proof by comments. Root's exact unsafe-inventory/lint-negative
check must additionally verify one function-local exception and unchanged six.
No added dependency, new testing feature, caller boolean, shell/script or fake
native proof is used. Actual native four-parent denominator/ignored count must
remain explicit; two pure passes cannot satisfy them. B1 does not qualify B2,
ServiceHost admission, original provider/journal/Ready or FullProduct.
