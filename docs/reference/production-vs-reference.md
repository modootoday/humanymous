---
layout: doc
title: Implementation and release status
permalink: /reference/production-vs-reference.html
reviewed: 2026-11-07
---
# Implementation and release status

**Diátaxis quadrant:** Reference. **Audience:** adopters and reviewers.

Reviewed 2026-11-07. This branch is an incomplete Rust migration of a reference
project. No production-hardening, universal detection or completed replacement
release is claimed.

| Surface | Qualification state |
| --- | --- |
| Rust libraries | Shared detection, integrity, audit and state components exist; the public `humanymous` facade has explicit optional adapter features; unit and namespace checks are narrower than deployed parity |
| HTTP runtime | Experimental loopback HTTP/1.1 with bounded complete requests, sessions, request integrity and durable audit before origin dispatch |
| Artifact and service provisioning | Original `doctor`, `install`, `rollback` manage an explicit artifact; separate signed `service-*` commands implement owned lab-service lifecycle with full runtime qualification still false |
| Native integration CLI | `humanymous-native init` emits an unqualified framework plan; explicit manifest writing currently requires Windows |
| Browser collector | Rust/WASM browser reads, passive behavior and separate environment/advanced APIs exist; authenticated runtime integration and recovery remain incomplete |
| Laboratory browser | Windows Meari/CEF prototype has actual native rendering and window controls; full browser readiness, external navigation and SDK qualification remain separate |
| Node/Bun and nginx adapters | Rust Node-API and Linux nginx implementations exist with bounded historical pilots; changed-source platform, packaging and deployment qualification remain separate |
| Cross-platform packages | Source-bound native component runs exist; they do not qualify every service, browser or framework on those operating systems |
| Documentation | Fixed-source pinned-theme builds, rendered-link checks, archive round trips and representative browser observations exist; complete source review and accessibility qualification remain separate |
| Standards Draft v0 | Independent developing proposal; no external submission or adoption |

Core consolidation at `c67d07c0`, Platform consolidation at `89715c04` and Runtime
consolidation at `64f74c7a` reduce the workspace from 41 packages to 29. Four
standalone qualification-tool packages are separate from that workspace count:
the maintained-package total changes from 45 to 33. The accepted target is eight public package
roles; consolidation remains in progress and does not establish registry
publication or supported deployment tuples. The current package paths are
`crates/sdk/humanymous-core` and `crates/integrations/humanymous-platform`.
Platform defaults are empty; `testing` and `windows-object` are independent
optional features, with the object API additionally restricted to Windows.
Testing support cannot mint provider authority or readiness.

Runtime's six contexts now share `crates/sdk/humanymous-runtime`: `gateway`
(the default feature), `authority`, `audit`, `capture`, `network` and
`native-security`. Four role executables and 17 explicitly declared integration
targets remain distinct. The bounded Windows checkpoint records all-target,
all-feature check, strict Clippy and formatting success, plus 177 selected pure
cases passing with none failed or ignored from a 366-case inventory. It is not
a full 366-case run, native execution, Docker product result or actual browser
qualification. A separate browser run on older source cannot qualify this source.

The follow-up receipt `8e51e558` records one positive import and ten expected
private-access compiler refusals placed inside the actual sibling modules.
It also records checks of four source-only crate archives: 522 extracted files
were unchanged, Runtime and a temporary consumer passed all-target/all-feature
check and strict Clippy, and the default consumer's separate pure run succeeded. The
normal/build dependency tree did not enable Platform's `testing` feature.
These archives omitted lock files and skipped Cargo package verification;
they are not normal `cargo package` verification, registry availability,
native execution or a full library test result.

At source `42ecee7d`, the Core component result includes 160 library tests and
17 integration tests, with 15 SGD tests including three file cases. Formatting,
check and strict Clippy passed; independent review is retained at `b71f2f00`.
This does not establish unchanged-Go bit parity, bootstrap deployment, model
activation or live detection performance.

The matched Runtime dependency experiment at source `5d68cd39`, retained at
`cc4758` and independently reviewed at `90b125d2`, measured two cold-artifact
CLI `cargo check` pairs. Wall time fell by 45.385% and 46.455%, with artifact
events falling from 117 to 62; six warm checks rebuilt no artifacts. These are
bounded check measurements, not full-build or Runtime latency improvements.

Lab source `f8aec286` passed 44 default Windows cases and the separately selected
checkout case that is ignored by default. The corrected canonical source-root
environment passed after an initial failure, which remains in the record.
Formatting, check and strict Clippy passed, but complete tool prehash and hostile
custody are not established. The separate WASM check and strict Clippy success
on unchanged source inputs also does not execute a browser or qualify deployment.

Tools relocation and the accepted adapters design of one Rust library with seven
mutually exclusive export roles remain unimplemented plans at this checkpoint.
Draft module/root inventories are not executed-test denominators or available
registry packages.

A first pure public-role data parser is implemented at source `2e9934fe`.
Its scoped Windows component run passed 19 parser tests, with 79 other library
tests filtered out, retained at `e8f71220`;
later test-only stronger array controls are not covered by that run.
A separate source `be9eec6d` run passed the one selected stronger-array-control
test, with 97 library tests filtered out, retained at `8c8fc504`; formatting,
check and strict Clippy also passed. This closes that test-isolation gap, not a
rerun of all 19 parser cases or all 98 library cases.
It describes bounded caller-supplied data rather than a
public listener, native preparation, provider availability or a Ready producer.
No public endpoint or registry-ready API is promised by that acceptance. Existing
private-loopback startup configuration remains a separate implemented interface.

Historical Go behavior is a migration comparison baseline, not evidence that the
new runtime is ready. A future release must name exact supported tuples,
capabilities, artifact digests and completed checks. Until then, use an isolated
evaluation environment and retain your existing production controls.

Workspace inventories and component test counts belong to their exact source
checkpoints. Neither a workspace member count nor a passing component suite is
a supported-platform matrix or proof of full-product parity. Native browser
process tests and framework deployment pilots need their own execution evidence;
a documentation preview does not exercise them.

At source `e696e6a5`, the public Rust crate `humanymous` has no default adapter
features. Explicitly enabling `adapter-go` exposes `humanymous::adapters::go`
by re-exporting the existing owning Go-origin integration types and modules.
This is a Rust API surface for external Go HTTP origins, not a Go-language SDK
or evidence of execution inside a Go host. Internal namespace migration,
laboratory imports and CLI admission need their own accepted source checkpoint.

CI source `c3d81a7c`, with results retained at `86d8d68c`, pins Node 24 action
families and Ubuntu 24.04 runners and corrects release-note newline handling.
Its recorded local validation passed 107 pure tests, check, strict Clippy,
formatting, delivery policy and actionlint. These checks do not execute the
hosted actions or prove that historical annotations and owning job failures
have disappeared. A successful workflow syntax check is not a release or
production Pages deployment.

The pure Pass challenge implementation at source `ae57c13` has a recorded
Windows component run with 37 passing Rust tests: 35 library tests, including
27 Pass tests, and two existing attestation/proof-of-work vector tests.
Separately, all 16 unchanged Go baseline tests passed at source `981d25c`.
These results do not supply independent Pass seed/challenge vectors or qualify
runtime issuance, authenticated instances, single use, replay protection or
assistive-technology behavior. The public alignment puzzle is deliberately
derivable; solving it is not proof of humanity and requires no motor-speed test.

Hosted [run `42355168729`](https://github.com/modootoday/humanymous/actions/runs/42355168729)
targets source `8132117`, which precedes that local integration checkpoint.
It completed with failure: five of 19 jobs succeeded, including both-baseurl
Pages builds; source-review inventory and native/product gates still failed.
These separate outcomes cannot qualify later source changes.

At documentation source `269607366e4b2e37b47a974595b406b66642caae`, both the
site root and `/humanymous` builds produced 88 pages and 165 public files.
Rendered checks and two separate archive restorations passed for each build.
The restored files were then served on literal loopback addresses and inspected
in the in-app browser: representative desktop and 390-pixel mobile layouts,
navigation menus and keyboard skip-to-content behavior were observed.
The security-disclosure file matched its archived bytes over HTTP.
These observations apply to that source, not later edits. They are not a full
browser-page corpus, assistive-technology review, WCAG conformance result or
production deployment. Complete source-review reconciliation remains pending.

The standalone HTTP runtime uses plaintext loopback HTTP/1.1. Library source
also exposes `ServiceHost::bind_tls` with installation-owned TLS configuration
and a same-connection captured request owner. This optional TLS/H1 path accepts
only buffered native routes and HTTP/1.1 ALPN; it rejects browser profiles and
does not enable HTTP/2 capture. Six selected Windows TLS cases at runtime source
`1646c4821a5da2208a65d349656a10ec657ebeb5` passed, with 46 other library cases
filtered out. Both recorded identity cases returned CHALLENGE/403 and did not
invoke the origin. This establishes neither TLS ALLOW-to-origin behavior nor
complete ingress, browser or product qualification.
A readiness declaration is not deployment evidence,
and CDN re-termination still hides the original client's TLS handshake.
Browser recovery, public administration and arbitrary streaming proxy
compatibility remain unavailable. Routes requiring unavailable capture or a
qualified collector stay unready.
See [runtime configuration](cli-config-policy.md), [collector API](browser-collector.md)
and [native browser boundaries](native-browser.md) for the exact implemented scope.
