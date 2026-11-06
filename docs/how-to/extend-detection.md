---
layout: doc
title: Propose a detection change
permalink: /how-to/extend-detection.html
reviewed: 2026-11-07
---
# Propose a detection change

**Diátaxis quadrant:** How-to. **Audience:** research contributors.

Detection policy is frozen during migration. New weights, thresholds, hard-rule
predicates or verdict bands need explicit authorization and a reviewed contract;
an implementation refactor does not grant it.

For a research proposal, state the observation's provenance, counterexamples,
privacy cost and accessibility risks. Design synthetic owned-lab tests with
human controls and coherent-browser limits. Do not ship a folklore heuristic
or claim measured coverage before execution.
See [evaluation reporting](../explanation/transparency-report.md).
