# AWS container validation research provenance

Checked 2026-11-07 KST. Three read-only subagents:
- hosted_runner_research: Lightsail Container Service deployment/HTTPS/localhost,
  region-specific support, ECR private access, persistence and hourly cleanup.
- dedicated_runner_research: ECS RunTask/Fargate, EC2 Compose, Batch/CodeBuild,
  terminal-state evidence, lifecycle, CPU/storage, OIDC and cost boundaries.
- crossos_matrix_research: AWS policy case filtering, IAM/task isolation,
  metadata/egress negatives, cleanup and actual product qualification gaps.

AWS Core connector was used only for official documentation search. Primary
source synthesis and URLs are in sot:root/aws-test-environments. No AWS account
identity or resource inventory was queried; no resource, deployment, role,
network, payment or billing change occurred. All AWS execution tuples are not_run.

Important resolved distinctions: same-task localhost is not isolation; managed
HTTPS is not original raw capture; ACTIVE/RUNNING/HTTP 200 is not test success;
disabled Lightsail service is still billable; tags do not implement expiry;
private ECR support has a dedicated authoritative guide despite older FAQ text.
AWS security policy requires a bounded allowed-case profile; excluded flood/DoS
cases cannot be reported as full suite passes merely because targets are owned.
