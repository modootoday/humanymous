---
id: plan:root/20261106225056
title: AWS one shot validation rollout
status: active
created: 2026-11-06T22:50:56Z
updated: 2026-11-06T22:50:56Z
rel:
  depends_on: [sot:root/aws-test-environments]
---

# AWS validation rollout

- [x] Three read-only agents researched Lightsail, ECS/Fargate/EC2/Batch/CodeBuild and policy/security boundaries.
- [x] Canonical comparison, IAM, lifecycle, budget, evidence and cleanup contract.
- [x] Separate AWS-policy-safe cases from unrestricted existing red catalog.
- [ ] Finish actual Rust authority/Core/Gate/browser/state and isolated Compose integration.
- [ ] Select concrete AWS profile, account/region, supported cases and budget.
- [ ] Confirm scoped OIDC IAM/roles and independent expiry janitor.
- [ ] Implement exact resource graph and bounded lifecycle templates.
- [ ] Run actual owned AWS pilot, export immutable artifacts and verify cleanup inventory.

No AWS account/resource API or paid provisioning was performed. Current support
is an accepted design, not a passed provider tuple. EC2 is the closest match to
canonical Compose topology; Fargate/Lightsail are separate managed profiles.
