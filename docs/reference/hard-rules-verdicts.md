---
layout: doc
title: Decisions and enforcement limits
permalink: /reference/hard-rules-verdicts.html
reviewed: 2026-11-07
---
# Decisions and enforcement limits

**Diátaxis quadrant:** Reference. **Audience:** integration and policy reviewers.

The migration preserves existing scoring weights, thresholds, hard-rule
predicates and verdict bands. This page does not define new policy or claim
full-engine Rust equivalence.

An allow permits the bound request under the configured policy. A challenge
requires a supported recovery step. A deny stops the request. Input, transport
and dependency errors remain distinct from those outcomes; they must not be
converted into an allow when required evidence is absent.

Decisions must retain their route, request, body, session and policy bindings.
They do not establish intent or universal human presence. Accessible recovery
must not depend on motor richness or speed. See [visitor help](../help/why-am-i-seeing-this.md).
