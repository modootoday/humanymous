---
id: sot:root/aws-test-environments
title: AWS one shot container validation profiles
status: active
created: 2026-11-06T22:50:55Z
updated: 2026-11-06T22:50:55Z
verify: manual
rel:
  depends_on: [sot:root/container-test-environments, sot:root/cross-os-runners]
---

# AWS one-shot validation

Version 0.1.0. Official-source research checked 2026-11-07 KST by three read-only
agents and the AWS documentation connector. No account lookup, deployment,
resource creation or charge occurred. All AWS execution tuples remain not_run.

## Decision and comparison

One-time remote validation is feasible. Prefer an ephemeral Linux EC2 VM with
Docker Compose when the existing network/volume topology matters. Use ECS
standalone RunTask/Fargate for bounded application or provider compatibility,
and Lightsail Container Service only for temporary HTTP deployment smoke.
Preserve evidence types from the container-environments canon. None substitutes
for native Windows/macOS testing or qualifies unfinished Rust product features.

| Profile | Suitable work | Qualification boundary |
| --- | --- | --- |
| aws-ec2-compose | Canonical Docker topology, actual Rust images and recovery | AWS-policy-safe cases only; explicit VM/IAM/metadata/egress cleanup |
| aws-fargate-task | Actual Linux x64/arm64 image and bounded application task | No privileged Docker/Compose host; separate task trust zones |
| aws-batch | Many independent seeded attempts with durable reports | Explicit queue/start/run deadlines; Fargate or EC2 backend limits remain |
| aws-codebuild | Managed image builds and Docker-capable Linux CI | Privileged configuration and compute mode constraints; not native OS proof |
| lightsail-http | Deploy exact image/version, perform HTTP assertions, export, delete | Managed HTTPS termination and restricted topology; not a one-shot job API |
| aws-native | Dedicated actual supported OS hosts | EC2 Mac host minimum billing makes short ad-hoc runs expensive |

## Lightsail Container Service

It is a continuously managed deployment service, not an exit-code-oriented task
runner. An external controller must deploy, verify exact version/image/run
identity, run assertions, export results and delete the service. ACTIVE/healthy
does not mean tests passed; failed replacement can leave the older deployment
active. The container schema does not import Compose networks/volumes/namespace
or no-egress policies. Same-service containers can communicate through localhost;
do not put hostile clients beside loopback administration or independent witness.
Public ingress is HTTPS only, not arbitrary TCP/UDP. LB termination means backend
observations cannot be original client TLS/H2 capture. [L1-L3]

Linux images are documented. The exact managed CPU architecture was not resolved
from the reviewed primary text: lightsailctl ARM support is client tooling, not
runtime proof. Observe actual uname/architecture/image digest in any future run.
Tokyo and Seoul have container-specific support evidence in the regional alias
guide; account quota/capacity/permissions are still unverified. [L4]

Private ECR is supported by a dedicated guide/API using a same-region repository,
service image-puller role and repository policy. This supersedes the general
FAQ's conflicting public-registry-only wording, not a promise of arbitrary
private registry credentials. Preserve unrelated repository policy statements.
No persistent volume or custom subnet/SG/egress-control declaration exists in
the reviewed container API. External durable WAL/head design and isolation are
therefore separate unqualified requirements. [L2, L5]

Billing is hourly with a monthly cap and power/scale dependence. Disabled services
still incur charges: deletion is required. Do not promise sub-hour prorating or
a three-month free trial without current account eligibility. Export first:
service deletion removes deployment/image/log data, while certificates/domains,
ECR artifacts/policies and external reports have separate lifecycles. [L6, L7]

## ECS RunTask and Fargate

RunTask supports work that terminates, but API success/RUNNING is not test
success. Validate returned failures and expected task count, poll with bounded
backoff to STOPPED, preserve task-definition revision, platform, actual image
digest, stopCode/reason and role-specific container exit statuses. Server-sidecar
normal shutdown is different from harness completion. Verify exact-SHA report,
required case inventory, denominators and artifact digest independently. Missing
reports, capacity failure, timeout and interruption are not passed attempts. [E1]

Fargate uses awsvpc and does not allow privileged/Docker daemon execution as a
Compose host. Same-task containers share localhost/network namespace. Separate
attacker, origin, authority and witness trust zones into distinct tasks with
verified SG/routing policies; otherwise direct-origin/local-admin tests are
invalid. No public IP/LB is required for a private-VPC pilot. Image/log/artifact
endpoint connectivity belongs to the control plane, not attacker egress. [E2, E3]

Linux supports x86_64 and ARM64 subject to platform/region availability; Windows
container families are x86_64 and do not prove Windows ARM or macOS native
behavior. ALB HTTPS and NLB TLS terminate TLS; NLB TCP passthrough can preserve
the encrypted stream but not every TCP/source-IP property automatically. Capture
attributes and actual evidence rather than infer them from load-balancer names.
Fargate ephemeral storage does not survive task replacement as a durable WAL.
EFS/EBS options require explicit storage profiles and crash/lock/truncate/reopen
tests; encrypted storage alone does not establish audit continuity. [E2-E5]

awslogs buffering can lose records and is not the security WAL or complete
artifact authority. Upload bounded report/manifest to a run-specific S3 prefix
and independently check completeness. stopTimeout concerns graceful shutdown,
not the run deadline. A TTL tag does not stop a task. Use an external controller
and janitor. Batch additionally needs bounded RUNNABLE/STARTING/image-pull time
outside execution timeout. CodeBuild privileges depend on compute type; Linux
Docker capability must not be generalized to Windows/Lambda/Mac modes. [E6-E8]

## Policy, IAM, cost and cleanup

AWS permits security assessments on listed owned services but prohibits or
requires separate handling for particular simulations. Do not run the existing
flood/DoS/resource-exhaustion catalog merely because the target is in the same
account or Compose network. Maintain aws-safe-bounded cases; mark excluded cases
unsupported_aws_policy and never call that subset a full catalog pass. Review
AWS policy for C2/covert/event and DDoS cases before execution. No third-party
targets, public scanner endpoints or provider-shared attack floods. [S1]

Use GitHub OIDC with restricted repository/ref or protected-environment subject
and audience, least-privilege run-controller IAM and narrowly scoped PassRole.
Separate execution/image-pull role, application role, report writer and janitor;
attacker tasks must have no AWS API credentials. Same-task role access is not
container isolation. Prevent metadata credential access and verify it negatively;
IMDSv2/hop limits/private subnet labels alone are not proof. [S2-S4]

Record account/region, budget, owner/run identity, expiry and exact resource ARNs
before execution. finally and external janitor must stop/terminate tasks/VMs,
verify deletion and compare owned resources against the initial inventory.
Include EBS/delete-on-termination, ENIs, LB, EIP/NAT/endpoints, ECR images/policy,
logs and S3 retention. Never delete shared baseline resources by broad name/tag.
SSM can coordinate EC2 without inbound SSH, but exit propagation and output
truncation must be handled; upload full reports before destruction. [E9]

Fargate pricing starts during image pull and has Linux/Windows minimum billing
periods, not just harness CPU time. Sum NAT, IPv4, endpoints, LB, ECR, logs and S3.
Lightsail disabled billing and EC2 Mac Dedicated Host minimum 24-hour allocation
are separate traps. No fixed dollar estimate or free-tier promise is canonical;
confirm current regional/account pricing before provisioning. [P1-P3]

## Required evidence and execution gates

Required cases: aws-resource-identity, aws-image-digest, aws-task-terminal-state,
aws-harness-exit, aws-artifacts-complete, aws-no-public-admin,
aws-no-origin-bypass, aws-attacker-no-egress, aws-attacker-no-metadata,
aws-task-role-isolation, aws-tls-termination-profile, aws-state-restart,
aws-witness-separation, aws-deadline-stop, aws-cleanup-inventory,
aws-policy-case-inventory and aws-budget-prerequisite.

Lightsail additionally requires exact deployment version, rejection of stale
deployment identity, body/header/target/response compatibility and deletion
verification. Report unsupported raw-capture/WAL/no-egress boundaries honestly.
Cloud-native readiness, frontend URL or stdout marker alone is insufficient.
Before implementation: complete actual Rust runtime interfaces, choose profile
and policy-safe case set, verify account/region/role/budget, then implement owned
resource lifecycle and conduct the bounded remote pilot. Research is not that
authorization or execution evidence.

## Critique reflected and sources

Blue: shared Rust authority and no invented capture. Red: same-task localhost,
direct origin and metadata escape checks. SRE: terminal lifecycle, queue deadlines,
orphan cleanup and cost inventory. DPO: synthetic data/redacted artifacts.
Evaluator: exclude unavailable/provider-policy cases without inflating success.

- L1 https://docs.aws.amazon.com/lightsail/latest/userguide/amazon-lightsail-container-services.html
- L2 https://docs.aws.amazon.com/lightsail/2016-11-28/api-reference/API_Container.html
- L3 https://docs.aws.amazon.com/lightsail/latest/userguide/amazon-lightsail-container-services-deployments.html
- L4 https://docs.aws.amazon.com/lightsail/latest/userguide/amazon-lightsail-route-53-alias-record-for-container-service.html
- L5 https://docs.aws.amazon.com/lightsail/latest/userguide/amazon-lightsail-container-service-ecr-private-repo-access.html
- L6 https://docs.aws.amazon.com/lightsail/latest/userguide/amazon-lightsail-faq-containers.html
- L7 https://docs.aws.amazon.com/lightsail/latest/userguide/amazon-lightsail-deleting-container-services.html
- E1 https://docs.aws.amazon.com/AmazonECS/latest/developerguide/standalone-tasks.html ; https://docs.aws.amazon.com/AmazonECS/latest/APIReference/API_RunTask.html ; https://docs.aws.amazon.com/AmazonECS/latest/APIReference/API_DescribeTasks.html
- E2 https://docs.aws.amazon.com/AmazonECS/latest/developerguide/fargate-tasks-services.html
- E3 https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task-networking-awsvpc.html ; https://docs.aws.amazon.com/AmazonECS/latest/developerguide/security-network.html
- E4 https://docs.aws.amazon.com/elasticloadbalancing/latest/network/load-balancer-listeners.html
- E5 https://docs.aws.amazon.com/AmazonECS/latest/developerguide/fargate-task-storage.html ; https://docs.aws.amazon.com/AmazonECS/latest/developerguide/using_data_volumes.html ; https://docs.aws.amazon.com/efs/latest/ug/features.html
- E6 https://docs.aws.amazon.com/AmazonECS/latest/APIReference/API_LogConfiguration.html
- E7 https://docs.aws.amazon.com/batch/latest/userguide/job_states.html ; https://docs.aws.amazon.com/batch/latest/userguide/job_timeouts.html
- E8 https://docs.aws.amazon.com/codebuild/latest/APIReference/API_ProjectEnvironment.html
- E9 https://docs.aws.amazon.com/systems-manager/latest/userguide/troubleshooting-remote-commands.html
- S1 https://aws.amazon.com/security/penetration-testing/
- S2 https://docs.aws.amazon.com/IAM/latest/UserGuide/id_roles_create_for-idp_oidc.html
- S3 https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task-iam-roles.html
- S4 https://docs.aws.amazon.com/AmazonECS/latest/developerguide/control-access-with-tags.html
- P1 https://aws.amazon.com/fargate/pricing/
- P2 https://aws.amazon.com/lightsail/pricing/
- P3 https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-mac-instances.html
