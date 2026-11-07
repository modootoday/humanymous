# Apache adapter integration frontier

## Accepted eight-package handoff (2026-11-08 KST)

The confirmed baseline in sot:root/structural-development supersedes the separate
humanymous-adapter-apache/native package targets in the historical naming proposal
below: Apache safe/native responsibilities join humanymous-adapters. Root alone
selects destination modules, graph, paths and atomic integration. No move, rename,
manifest/ref change or compatibility shim has been performed by this handoff.
The public consumer face remains humanymous; internal host roles stay isolated.

Evidence:root/20261107200931 inventories23 unchanged source/runtime/build inputs
at8affb45aeb4d31330fda8f2e5135043c6b252c05 with exact Git blob and SHA256/size.
This INTEGRATION file and the owning SoT are deliberately excluded mutable
handoff metadata. The inventory is a migration input snapshot, not new runtime
qualification. See the owning SoT's eight-package handoff for safety/dependency
hazards, gates, leases and ordered next actions. Historical b40045/45 evidence
and original raw commands/hashes remain unchanged.

## Adapter-family naming migration handoff (2026-11-08 KST)

Target convention comes from sot:root/structural-development, not a Cargo syntax
requirement. This inventory does not execute a rename. Root must migrate graph,
source, lock, workflows, artifacts and current documentation together. The two
Apache packages are publish=false; this is not a crates.io publication request.

The user's later clarification supersedes preserving implementation-specific
public loader/import names: the public consumer identity is humanymous (Rust
facade, npm, Python import, Ruby require and native product loader; Humanymous
for .NET conventions). humanymous-adapter-* names are internal Rust packages.
The implementation-specific names below are LEGACY inventory, not a proposed
permanent public interface. No namespace ownership or publication is asserted.

| Surface | Current identity | Coordinated migration impact |
| --- | --- | --- |
| Safe package/default library | humanymous-apache-adapter / humanymous_apache_adapter | Target package humanymous-adapter-apache; without a library-name override the Rust identifier becomes humanymous_adapter_apache. Native Cargo dependency key and host.rs import must agree. |
| Native package/default library | humanymous-apache-native / humanymous_apache_native | Target package humanymous-adapter-apache-native. Its cdylib/rlib has no explicit [lib].name, so the default Rust identifier and Linux build artifact become humanymous_adapter_apache_native and libhumanymous_adapter_apache_native.so. |
| Shared framework dependency | humanymous-framework-adapter / humanymous_framework_adapter | Target humanymous-adapter-framework / humanymous_adapter_framework. Native optional dependency keys, dep: feature entries and imports in host, coordinator and pilot modules must migrate atomically with the provider. |
| Explicit internal pilot binary | humanymous-apache-pilot | [[bin]].name is explicit and does not follow a package rename. It is a validation tool, not the public consumer facade; its own mapping must keep --bin commands, Docker install source and executable paths consistent. |
| LEGACY Apache runtime module paths | /opt/httpd/modules/libhumanymous_apache_native.so and libhumanymous_apache_probe.so | Propose one public humanymous loader basename with the platform extension under an Apache/ABI/target-specific directory. Root's ABI canon must choose the exact basename and layout; old implementation-specific filenames are not the permanent public API. Probe artifacts belong in a separate non-public test directory/manifest. |
| Apache host entry/config | humanymous_module, LoadModule humanymous_module, HumanymousConfig | Propose retaining humanymous_module as the explicitly resolved Apache module-descriptor entry symbol, a host-entry suffix exception rather than a second product brand. HumanymousConfig already uses the unified brand. Root must accept this compatibility map; the module descriptor uses generated Apache ABI layouts, not a generic native-product C ABI. |
| Profile/provenance registry | AdapterProfile::Apache; apache; humanymous.apache/0.1.0-experimental.1; apache-normalized | Shared service/provenance identities do not derive from Cargo names. Preserve unless their owning accepted contracts explicitly migrate. Apache has no Python/Ruby import-module alias to rename. |

Current command/recipe consumers are .github/workflows/apache-validation.yml,
adapters/apache/Dockerfile.native and coordinator-contract.json. Both -p package
selectors change; the explicit --bin selector need not. The Dockerfile currently
copies the native default build filename for both production and probe builds.
The pilot LoadModule fixture, module/probe hash reads and pilot-contract.json
artifact paths must match the chosen installed-name compatibility map. The local
nonce-scoped humanymous-apache-pilot image tag is not a registry publication or
an automatically inferred package name. ABI/header receipts and module hashes
must be regenerated and checked against the actual selected artifacts.

Proposed atomic public-face move: the public Rust facade exposes Apache capability
without requiring consumers to import internal adapter crates; the installed
Apache loader becomes humanymous plus the platform extension, while runtime/OS/
architecture/ABI selection lives in paths and a verified manifest. Exact basename
prefixes/extensions, facade export and host entry-symbol exceptions are decisions
for root's ABI canon. Do not combine the Apache module implementation with a
different native product ABI merely because they share the humanymous brand.
Two incompatible DLLs/shared objects must never overwrite one destination:
separate host/ABI/version/target directories and manifest identities are required;
production/probe must also remain isolated. No global alias or loader fallback
is implied. Meaningful versioned humanymous_*_v1 function names may remain where
the separately owned native ABI contract requires them; Apache's module table
does not become that ABI through a filename change.

One coordinated checkpoint must update the facade, package/dependency/import
graph, source and locks, installed module layout, LoadModule configuration,
artifact hash readers, image/workflow recipes, current contracts/docs and claims.
Compatibility tests must load the intended public file and verify its manifest,
entry symbol, generated host ABI, ownership/error/unwind/thread/lifetime rules,
three-MPM operation and cleanup before promotion. A renamed filename alone proves
none of these properties. No public namespace registration or publication occurs
as part of this handoff.

Root owns Cargo.toml/Cargo.lock and any crate path/claim remap; CI owns catalog/
policy changes, and Pages owns public docs. No directory relocation follows from
this inventory. Shared native-abi/native-security provider names stay outside
the host-adapter rename. Historical b400 commands, source/manifests, hashes and
45-case receipt stay unchanged; they do not qualify renamed artifacts. After a
coherent root checkpoint, scoped fmt/check/tests/Clippy and actual Apache loader,
three-MPM and cleanup evidence are required before promoting the renamed tuple.

## Qualified component and retained history

The requested Apache component scope is complete at exact source
b400094480483d9abb8328234f9de12520b71b65. Root registered the native crate and
approved its narrow generated ABI. Genuine hosted run42374566498/job128409407533
passed safe12/native13 tests, checks and warnings-denied Clippy; built actual
production/probe modules; passed all45MPM cases (15each); and verified pilot,
container/image and separate ledger cleanup. Evidence:root/20261107130407 binds
the source/manifest/image/five artifact identities and complete outcomes.
Independent prefork CLI exit0, owned masterexit0/reap and group isolation were
observed. This qualifies the pinned H1 explicit-Content-Length component tuple
only; synthetic frozen ALLOW is not full product/browser provenance. Later
source, SSL/H2 and broader version/module matrices are not implicitly qualified.

## Historical implementation and qualification checkpoints

Safe std-only lifecycle/replay crate is registered. Root-assigned A passed9pure
units and all-target Clippy with warnings denied; evidence:root/20261107082159.
Root has registered the native workspace member and lock entry. Actual hosted
source8042c6bc built production/probe modules and completed strict Clippy, then
ran all45MPM pairs:23passed/22failed/0not_run. That source remained unqualified.
Root-assigned D passed native DEFAULT all-target check/Clippy without headers or
bindgen; evidence:root/20261107085316 binds identical before/after graph/source.

Required shared seam (owners only): AdapterProfile::Apache with trusted config
name apache; humanymous.apache/0.1.0-experimental.1; exact apache-normalized;
peer/connectionId/capture/stream null, rawTls/originalH2/rawHeaderOrder false.
framework-adapter::encode now has Apache => apache-normalized, and current
Pending/Configuration/Admission plus authority::BoundResponse remain sole
decision/IPC owners. Apache must never use Node identity as fallback.

The earlier root-reported six pure passes are historical; the current independent
nine-pass receipt binds all four safe source/manifest hashes and exact commands.
Root accepted isolated native ABI; see adapters/apache/ABI-SAFETY.md. Safe crate
still forbids unsafe. No handwritten C/C++ or executable foreign templates.

Approved graph window registered separate crates/apache-adapter/native package
humanymous-apache-native, cdylib/rlib, opt-in httpd feature, default empty so
non-Linux workspace checks remain safe-only. Dependencies: Apache safe crate,
framework-adapter, authority, integrity, tokio(workspace), zeroize(workspace),
rustix1.1(fs,process). Build dependencies bindgen=0.73.2, sha2(workspace),
serde_json(workspace). Build script runs generation only for Linux httpd feature.
The native manifest is frozen after root's window; dependency changes require
another explicit integration window.
The approved native pilot uses optional runtime audit/signals/getrandom/hex/sha2/
serde_json dependencies, all already present in the root graph. Binary path is
native/src/bin/pilot.rs with required feature pilot. Explicit pilot-probes feature
is required on the component module build to exercise real nested host requests;
production module must also build without those test-only hooks. Runner accounts
45 required scenario/MPM pairs, preserving unreached cases as not_run. Its pilot
feature and selected httpd feature compiled on actual hosted source8042c6bc.
Evidence:root/20261107111302 retains independent module/ABI/image identities and
the failed actual45outcome. Outer owned-ledger cleanup passed; pilot cleanup
remained false. Source d65da705 then passed the actual coordinator11units and
reported event14/15, worker8/15 and prefork1/15. Evidence:root/20261107113711
retains these failed outcomes. The candidate fixes pinned MPM query selection
and test-only rejected lookup handling, adding three safe capability tests.
At that checkpoint12safe/11coordinator were expected but unrun for those bytes.
Subsequent33/12and35/10actual results remain failed historical evidence. The
final source isolates each foreground master/helper process group at birth,
checks real kernel ownership after pidfd capture and signals only owned pidfds.
This prevents the pinned prefork shutdown group signal from reaching sibling
fixture masters; prior numeric group identities were not recorded.

Hosted pilot interface: adapters/apache/pilot-contract.json. Product-lab owns
Compose orchestration; Apache owns only the root/CI-delegated standalone
apache-validation.yml, CI owns catalog/policy, and root controls push/dispatch.
Public docs delta: Apache has a source-bound passed45case hosted component
tuple, with true cleanup and fullProductSuccessfalse. Root owns roadmap and
Pages owns public docs; exact qualification never extends to newer source by
proximity to the passing commit.
