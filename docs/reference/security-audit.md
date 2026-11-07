---
layout: doc
title: Security evidence and audit status
permalink: /reference/security-audit.html
reviewed: 2026-11-07
---
# Security evidence and audit status

**Diátaxis quadrant:** Reference. **Audience:** security reviewers and adopters.

No completed independent audit or production certification of the Rust replacement
is claimed. Unit checks, dependency advisory scans and deployment pilots answer
different questions and must be reported separately.

## Implemented audit boundaries

| Rust surface | What it establishes and what remains separate |
| --- | --- |
| `record` and `checkpoint` | Canonical event commitments, record HMAC and signed tree heads; producers must pseudonymize data before constructing records |
| `verification::verify` | Checks supplied records and checkpoints, with optional HMAC; inspect the actual verification scope before claiming an external anchor |
| `verification::verify_with_witness` | Checks witness signatures on supplied checkpoints; an empty checkpoint list does not establish a witnessed history |
| `storage::Wal` and `log::Log` | Durable local append and authenticated replay; rollback resistance needs a trusted head retained outside the local WAL |
| External witness client/process | A separately supervised co-signer protocol; a trusted broker still owns process/peer authentication, pipe provenance, deadlines and reaping |
| `sink::DurableSink` | Authenticated bounded local export and a receipt; no remote delivery, WORM storage or automatic personal-data sanitization is implied |

The compatibility verifier can return `OK: true` with class `hmac-unchecked`
when no HMAC key was supplied. Its native `Report.scope` records checked records,
HMAC coverage, checkpoints and witnessed checkpoints, but is omitted from the
compatibility JSON serialization. Preserve that scope separately in a review
report. A successful result without supplied checkpoints is not proof of an
independently retained anchor.

Audit records and checkpoints have distinct frozen byte rules. Selected record
strings use the record canonicalizer's normalization; checkpoint signing bytes
use a fixed-order JSON representation without NFC normalization. Neither rule
should be applied to request-body bytes or the developing standards profile by
analogy. See [continuity verification](../how-to/verify-audit-log.md).

Filesystem durability is also explicit: successful Unix file-and-directory sync
differs from Windows file sync with rename-namespace power-loss durability
unconfirmed. A configured strict witness-durability requirement must not be
silently downgraded. Component persistence is not independent process isolation
or complete deployment qualification.

A reviewable report names the exact source and artifact, covered interfaces,
threat assumptions, tools/database dates, findings and unresolved conditions.
Unexecuted checks remain pending. A clean advisory scan only means that its
database found no matching known advisory in the scanned dependency set.

Required boundaries include missing evidence, origin bypass, local management,
replay, audit storage failure and destructive-action approval. Report sensitive
findings through [security disclosure](security-disclosure.md).
