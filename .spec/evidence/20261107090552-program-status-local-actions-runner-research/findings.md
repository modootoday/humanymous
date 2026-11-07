# Local GitHub Actions execution research

Observed 2026-11-07. Three read-only research agents independently investigated
Rust executors, established emulators, and Rust task/validation tools. Sources
below are official repositories, manifests and documentation. No candidate was
installed or executed. Documentation claims are not security certification or
measured compatibility with Humanymous workflows.

## Genuine Rust execution candidates

| Tool | Verified role | Qualification decision |
|---|---|---|
| [wrkflw](https://github.com/bahdotsh/wrkflw) | Rust CLI/TUI executing workflow YAML; MIT; JS/Docker/composite actions, matrices/services and reusable workflow support documented | First Rust evaluation candidate. Explicitly select Docker/Podman: automatic runtime selection can fall back to host execution. Native Windows behavior, permissions, secrets and exact reusable-workflow semantics need empirical qualification. Wrong-OS container success cannot qualify Windows/macOS. |
| [Local CI](https://github.com/redwoodjs/local-ci) | Rust runner available from Cargo source, using the official runner with emulated service APIs | Experimental alternative. Published npm execution remains TypeScript; native Rust packaging is deferred and described as parity testing. Cargo license is FSL-1.1-MIT, not an unconditional present-day MIT grant. Windows support was not established. |
| [Pernosco gha-runner](https://github.com/Pernosco/gha-runner) | MIT Rust library and gha_local example with Docker backend | Too limited as the main workflow executor: README limits supported expression syntax, pre/post actions and container/Docker workflows. Useful implementation reference, not a proven production replacement. |
| [Velnor](https://github.com/tailrocks/velnor) | Apache-2.0 Rust Linux self-hosted runner/control plane | Different role: GitHub still parses and schedules the workflow. It is not an offline local Actions emulator and does not validate native Windows/macOS. |

The wrkflw repository lists v0.8.0 as its latest release during inspection.
Root verified current main through the GitHub API at commit
553ecf535f45a59d035ec87ee19b43702e130b3b. Cached raw README and rendered
repository views disagreed on service support; the commit-bound README confirms
services on Docker/Podman but not other runtimes or jobs in called workflows.
Do not attribute these main-branch capabilities to the release without checking
its exact source. Workflow-level concurrency/cancellation and OS context remain
limitations even when feature headings advertise broader support.
Rendered release dates omitted a year in some views; no year is inferred.
Private remote reusable workflows and fine-grained GitHub authorization are
documented limitations. Docker availability must fail closed instead of silently
moving a workflow to the developer host. A future pilot must bind the release
or source commit, binary digest, container image digest and exact workload.

## Established reference and official runner

[nektos/act](https://github.com/nektos/act) is a maintained Go implementation,
not Rust. It is a useful optional reference for selected trusted Linux workflow
rehearsals. Its official [limitations](https://nektosact.com/not_supported.html)
include ignored concurrency, job permissions, timeout, job continue-on-error and
deployment-environment scoping, plus absent OIDC URL. Its runner images are not
the same as GitHub hosted VMs. [Native host mappings](https://nektosact.com/usage/runners.html)
require the corresponding real OS; WSL cannot qualify Windows or macOS.

[actions/runner](https://github.com/actions/runner) executes actual GitHub jobs on
self-hosted machines but requires GitHub registration/service coordination. It
is not an offline YAML emulator. No registration, token exchange or runner
service installation is authorized by this research. Upstream binaries written
in another language are distinct from authored Humanymous implementation.

Gitea [act_runner](https://docs.gitea.com/1.26/usage/actions/overview/) serves the
Gitea platform and is not a drop-in local GitHub authority. Docker socket access
and host mounts remain privileged capabilities regardless of executor language.

## Rust tools that do not emulate Actions

| Tool | Role and relevant limit |
|---|---|
| [just](https://just.systems/man/en/) | Thin cross-platform command runner, CC0-1.0. Reuses commands but does not provision or lock the toolchain by itself. |
| [cargo-make](https://sagiegurari.github.io/cargo-make/) | Rust TOML task graph, Apache-2.0. Disable automatic plugin installation for controlled gates; not workflow YAML emulation. |
| [mise](https://mise.jdx.dev/dev-tools/mise-lock.html) | Rust tool/environment/task manager, MIT. Windows and WSL need separate installations. Lock/checksum coverage depends on backend; locked does not mean offline or fully hermetic. |
| [moon](https://moonrepo.dev/docs/guides/ci) | Rust-core task graph/cache system, MIT. More infrastructure than this Cargo-centric repository currently needs; input declaration still requires review. |
| [action-validator](https://github.com/mpalmer/action-validator) | Rust static schema/glob validation, GPL-3.0-only. Executes no job; native Windows qualification was not established. |
| [zizmor](https://docs.zizmor.sh/) | Rust CI security static analysis, MIT. Executes no workflow. Offline mode omits network-dependent audits. |
| [Cicada](https://github.com/cicadahq/cicada) | Archived 2023-09-27; uses its own TypeScript pipeline definitions. Excluded as an Actions replacement. |

## Proposed Humanymous architecture

The preferred authority remains the repository's existing Rust validation
commands, shared by local Windows, local WSL/Linux and GitHub workflows. Make
package/features/target/profile explicit and report compilation, linking and
execution separately. This avoids assuming that a YAML emulator reproduces the
compiler, native SDK, network namespace, service manager or GUI session.

Evaluate wrkflw as an optional Linux/Docker workflow rehearsal only after the
direct-command benchmark establishes a baseline. Compare its overhead and
semantic behavior against act or direct invocation; do not add it as a required
gate merely because its implementation is Rust. Keep genuine GitHub checks for
macOS and other unavailable OS/architecture tuples, workflow permission/event
semantics, OIDC and publication attestations. No publication is authorized.

The user's local benchmark authorization supersedes the previous blanket
hosted-only restriction. It does not authorize spoofing GITHUB_ACTIONS or
RUNNER_ENVIRONMENT. Components that currently require hosted provenance need
an explicit local-benchmark profile, truthful source/environment receipts and
their owning safety boundaries. Existing disposable SCM/account constraints
must not be bypassed by choosing an emulator.

Acceptance before adoption: pinned provenance; trustworthy dependency/image
sources; no silent host fallback; correct failure/skip accounting; preserved
secrets and timeout behavior; supported feature matrix; and a measured replay
of the exact selected Humanymous workflow. These gates remain unexecuted.
