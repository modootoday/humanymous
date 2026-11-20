---
layout: doc
title: Review behavioral model operations
permalink: /how-to/operate-behavioral-model.html
reviewed: 2026-11-19
---
# Review behavioral model operations

**Diátaxis quadrant:** How-to. **Audience:** model and service reviewers.

No qualified Rust model-operation command or automatic learning loop is claimed.
Before a pilot, define training input provenance, bounded resources, poisoning
controls, rollback and human-impact measures.

Include sparse interaction, keyboard-only access and assistive technology in
controls. Keep model errors distinct from verdicts. Migration work does not
authorize changes to frozen scoring or motor-based recovery requirements.
See [model limits](../explanation/self-correcting-behavioral-model.md).

## Promote or roll back a staged model

The runtime admin plane can promote and roll back a behavioral model. These
are operator actions on the admin listener, and each one is audited:

1. A verified candidate must be staged first. Staging checks the digest pin,
   the signature when a public key is configured, the bundle shape and the
   feature schema. Staging and training never serve a model.
2. `GET models` shows the staged, active and previous bundle versions and
   digests.
3. An operator sends `POST models/promote` with the staged digest. This
   creates a pending action. A different principal with the approver role
   commits it through `approvals/<id>`.
4. An operator sends `POST models/rollback` with the active digest (empty when
   no model is active). The previous model is served again, or no model.

Each change is recorded in the audit log before it takes effect. If that record
cannot be written, the request fails with 503 and nothing changes. A digest
that does not match answers 409, and so does a request with nothing to promote
or roll back; these refusals are decided before anything is recorded. If the
model lifecycle is unavailable, the request fails with 503 and nothing is
recorded.

A served model only adds an annotation with weight 0. It does not change any
score, verdict, threshold or signal. The runtime service binary does not yet
attach this plane, so this is a component capability, not a deployed one.
