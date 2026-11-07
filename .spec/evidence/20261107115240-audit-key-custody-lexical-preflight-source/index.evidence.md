---
id: evidence:root/20261107115240
title: Audit key custody lexical preflight source
status: active
created: 2026-11-07T11:52:40Z
updated: 2026-11-07T11:52:40Z
rel:
  governs: [crates/audit/src/storage/framing/preflight]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody lexical preflight source

## What was measured

Source-only lexical resource scanner following root's accepted S1 delta; no
manifest/dependency feature changes. No Cargo workload while Pages holds the
heavy build. Five preflight tests and one compatibility decoder regression are
authored, compilation/execution NOT_RUN. Formatting/diff checks pass.

The scanner stores64 container counters on stack, no owned JSON/string tree.
Checked string-byte totals include keys and exact UTF8 escape/surrogate output
length for valid strings. Values exclude map keys; each array has its own item
counter. Numeric literals are delimited without machine numeric conversion.
The existing Record/Checkpoint decoder remains the sole semantic/syntax decoder.
Preflight is currently test-compiled only, awaiting the private read-only bounded
inspection consumer; compatibility decode does not adopt resource policy.

Source inspection of serde_json de.rs shows deserialize_ignored_any calls
ignore_value, unlike deserialize_any/parse_any_number. The authored regression
pins unknown +/-1e400 and huge integer fields accepted as default Record, while
Value1e400 refuses. Other tests cover exact decoded-byte budgets, per-array and
total/depth refusal, arithmetic overflow/no mutation, and92 record/180 checkpoint
fixture count parity with an independent owned Value traversal oracle.

## How to read it

Resource-check success is not JSON validity, a parsed record, authentication,
trusted state or authority. Invalid grammar is intentionally left for the same
existing decoder to reject. No S2/S3 production recovery, authenticated EOF,
allocation/RSS measurement or product qualification follows from this source.
