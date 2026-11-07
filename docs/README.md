---
layout: doc
title: humanymous documentation
description: Scope, limits and implementation status of the humanymous reference project.
permalink: /
reviewed: 2026-11-07
---

# humanymous documentation

**Diátaxis quadrant:** Explanation. **Audience:** evaluators and prospective adopters.

humanymous explores how to raise the cost of automation using observations,
request integrity, accessible challenges and accountable enforcement. It is an
Apache-2.0 reference project. A coherent real browser operating at a person's
pace can remain indistinguishable from legitimate use; no universal human or
bot identification claim is made.

**This branch is undergoing a Rust migration.** A bounded loopback HTTP runtime,
artifact-provisioning commands, native integration planning and browser-collection
components are implemented. Full deployment combinations have not completed
release qualification. Source builds and component pilots are available for an
owned lab; they do not constitute a production installation.
See [implementation status](reference/production-vs-reference.md) before using
an old release or evaluating a new artifact.

## Choose a starting point

| Your task | Start here |
| --- | --- |
| Connect an application | [Integrator guide](start-here/integrator.md) |
| Prepare an operating environment | [Operator guide](start-here/operator.md) |
| Assess claims and limits | [Evaluator guide](start-here/evaluator.md) |
| Contribute implementation evidence | [Developer guide](start-here/developer.md) |
| Review personal-data handling | [Data-protection guide](start-here/compliance-dpo.md) |
| Recover from a challenge | [Visitor help](help/why-am-i-seeing-this.md) |

For a safe executable starting point, [inspect a native integration plan](tutorials/quickstart-monitor-mode.md).
It prints an explicitly unqualified manifest without installing an SDK or service.

## Research and review

The [automation evidence Draft v0](standards/draft-v0/index.md) proposes an
interoperability model for observations and request-bound decisions. It is an
independent project draft, with incomplete wire details and no external adoption,
submission or conformance claim.

Tests and demonstrations apply only to an owned, isolated deployment. Published
results need their setup, failures, skipped cases and human controls. A successful
documentation build says nothing about runtime detection performance.
