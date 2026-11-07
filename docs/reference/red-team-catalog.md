---
layout: doc
title: Evaluation catalog and evidence
permalink: /reference/red-team-catalog.html
reviewed: 2026-11-07
---
# Evaluation catalog and evidence

**Diátaxis quadrant:** Reference. **Audience:** researchers and release reviewers.

The catalog exercises an owned lab, not every current automation product. The
Rust replacement's complete Docker catalog has not yet established parity.
Historical results must retain their original implementation and topology label.

Each case needs a capability premise, expected decision and origin effect,
artifact/configuration identity and observed outcome. Preserve allows,
challenges, denials, errors and skips as separate counts. Test ordinary users,
authenticated machine clients and coherent browsers as controls.

Catalog expansion does not authorize changing frozen detection policy. A passing
case supports only the tested condition. See [evaluation rules](red-team-rules-of-engagement.md)
and [reporting](../explanation/transparency-report.md).

## Current Rust evidence modes

| Mode | Source-level scope |
| --- | --- |
| Pure engine simulation | Frozen/synthetic observation replay; the seed changes case order, not probabilistic coverage or browser behavior |
| Supervisor-owned network report | A lab supervisor owns the actual runtime child, authenticated audit recovery and counted origin; caller-supplied external targets are not supported |
| Report verification | Checks scoped report consistency, not a new network run or independent observation |

The simulation CLI accepts only an optional unsigned ordering seed. It does not
accept a runtime executable or target URL. A replay case labelled human or
accessibility remains a fixture, not an actual participant. A synthetic sparse,
long-window control can match an expected CHALLENGE and pass regression checks;
that does not establish successful accessible completion or a qualified recovery
flow. Keep unavailable/error/control rows in the scheduled denominator.

The Rust verifier's current source reads a bounded report file and distinguishes
simulation replay from network-ledger checks. Exit 0 means the scoped checks
passed. Exit 2 means a consistent scoped acceptance failure, or a request for
full qualification that the report does not establish. Exit 1 covers malformed,
inconsistent, oversized or unreadable input. `--require-full` must not be omitted
and then described as a full-product gate. A valid ledger may still contain
unsupported cases and explicitly false human/assistive/full-product flags.

Current verifier follow-through is source-authored with changed-source execution
pending. Replaying a simulation against current code does not silently validate a
historical binary. A self-consistent network ledger does not prove that an
untrusted author executed its asserted topology. Bind actual results to the
owned Docker supervisor, exact binary/source identity, audit custody and origin
measurements. HTTP 403 alone is not an audited DENY, and a collection receipt is
not a policy verdict. No catalog or verifier pass authorizes scoring changes or
probing an unrelated deployment.

Simulation report comparison is semantic JSON comparison against a fresh replay:
object ordering and equivalent integer/float representations are not canonical
byte commitments. The current Value-based simulation decode is not a strict
duplicate-key wire validator. Network reports use typed decoding from original
bytes for their declared fields, but this report checker is not the strict
experimental adapter parser or a generic Draft v0 canonicalization test.
Retain the original report and its hash alongside any normalized summary.

Network ledger validation recomputes the catalog order, expected statuses/effects,
class totals and audited policy-verdict counts. It checks declared process
identity continuity and nonoverlapping audit sequence numbers. It does not reopen
the original WAL, recheck its authentication keys or independently attest the
reported binary hashes. Those responsibilities belong to the actual supervisor
and audit reader before projection. Keep their source-bound raw evidence and
custody context; hex-shaped hashes and consistent totals alone cannot authenticate
an externally supplied ledger.
