# Cross-OS runner research provenance

Observed 2026-11-07 KST by three read-only subagents:
- /root/hosted_runner_research: official hosted labels, OS/architecture,
  container/nested virtualization, privileges, billing and private connectivity.
- /root/dedicated_runner_research: ephemeral/JIT lifecycle, ARC boundaries,
  runner groups, public-PR security, Apple/Docker licensing and OIDC separation.
- /root/crossos_matrix_research: current checkout gaps and per-surface executable
  qualification cases for CLI/audit/IPC/adapters/browser/wargame/releases.

Primary-source URLs and accepted synthesis are in sot:root/cross-os-runners.
No third-party targets were probed. Research is not remote execution evidence.

Read-only gh API observation:
- repos/modootoday/humanymous: public, main default, not archived.
- actions/runners: total_count=0, runners=[]. Organization runners may require
  separate account visibility/configuration; absence is not fleet provisioning.
- actions/permissions: enabled=true, allowed_actions=all,
  sha_pinning_required=false.
- actions/permissions/workflow: default_workflow_permissions=write,
  can_approve_pull_request_reviews=true. New probe overrides to contents:read;
  repository-wide defaults were not changed.
- Remote workflow list at observation: ci, codeql(disabled_inactivity), release,
  Dependabot Updates, pages-build-deployment. Rust migration workflow was not
  in that default-branch list.

Remote execution artifacts will be attached after branch push/run, with exact
commit and job URLs. No Mac execution or remote runtime qualification is implied
by local Windows/WSL results.
