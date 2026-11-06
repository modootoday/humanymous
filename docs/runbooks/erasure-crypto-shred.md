---
layout: doc
title: Prepare an erasure pilot
permalink: /runbooks/erasure-crypto-shred.html
reviewed: 2026-11-07
---
# Prepare an erasure pilot

**Diátaxis quadrant:** Runbook. **Audience:** privacy operators and audit reviewers.

The Rust erasure path is not operationally qualified. Use synthetic subjects and
an isolated data set when preparing a test; do not execute against visitor data.

Inventory subject-linked ciphertext, keys, replicas, exports and retention rules.
Define authorized independent approvals and the expected audit-continuity result.
Test cancellation, restart and storage failure around the irreversible boundary.
Verify that the intended subject's data is inaccessible while unrelated subjects
and valid audit proofs remain intact.

Key deletion alone does not establish removal from every backup or export.
Document residual copies and purpose-specific handling. See [privacy review](../start-here/compliance-dpo.md).
