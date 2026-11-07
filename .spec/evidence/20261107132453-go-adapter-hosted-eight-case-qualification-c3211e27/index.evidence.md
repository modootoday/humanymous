---
id: evidence:root/20261107132453
title: Go adapter hosted eight case qualification c3211e27
status: active
created: 2026-11-07T13:24:53Z
updated: 2026-11-07T13:27:35Z
files:
  - {path: summary.json, sha256: 9b1a56f3d34039bb21191191cc0ebe6372c4f523dfe132dc16a50ce7435b5b21, bytes: 3625, store: repo}
---

# Go adapter hosted eight case qualification c3211e27

## What was measured

Rust Gate Go integration adapter, bounded net/http HTTP conformance only.
Actual [hosted run42375310198](https://github.com/modootoday/humanymous/actions/runs/42375310198)
uses source c3211e27ba941f3f11809e420d07d61ececad623. Preflight job128411569841
passed Go10/lab20/delivery74 units with zero failed/ignored/filtered, Unix runtime
actor compilation, selected all-target strict Clippy and workflow policy.
Product job128412147497 completed successfully on GitHub Ubuntu24.04 with the
actual isolated Docker lab and pinned untouched upstream go-httpbin fixture.

| Case | Executed owner checks | Result |
| --- | ---: | --- |
| binary-allow | 4 | pass |
| dependency-down | 5 | pass |
| cancellation | 6 | pass |
| incomplete-body | 6 | pass |
| forged-provenance | 5 | pass |
| origin-bypass | 5 | pass |
| upgrade | 5 | pass |
| response-semantics | 4 | pass |

Totals:8scheduled/evaluated/passed,40known checks,0failed/not-run/unknown cases.
Owner exit0, bounded requirement satisfied, valid receipt, no diagnostic failures,
owned cleanup observed and source unchanged. Root independently checked GitHub
head/jobs and recomputed the sole typed summary. Go owner separately fetched
both completed job logs and recomputed exact ordered IDs and check counts.

## How to read it

summary.json is the lab's sole allowlisted public projection, without raw bodies,
headers, credentials or configuration. Its private ledger digest is
5ab341d113866c367fff963681289978e324891af070496bd05821f8564240a4.
Lab source manifest is406eac69f317c702637e4e1adbe05ed64ad83a2df20706e3b35ea57e604dcaa9;
delivery manifest is4d486bb77299062ecacf9fe604c623f7c65bbe0db0711bc54ceadf9ba1f41201.
The trusted lab verified the actual source/image/fixture/configuration/engine
tuple internally. Individual tuple values are not exported in this projection;
public logs cannot independently reconstruct them. The custody-layout image
from the separate mount probe is not the runtime image. Raw CLI-decoded job logs
and external job metadata are retained privately as evidence:root/20261107132454;
Root additionally retains its raw capture under evidence:root/20261106194410.

Earlier438/448 build failures retain8NOT_RUN each. The15f attempt retains8failed
actor starts,0evaluated and8unknown assertion counts. The b482 mount probe observed
mode refusal on image and fresh-volume paths; c321 then observed acceptance on
all8paths with the same strict UID/type/0700 predicate. These are distinct runs.

This is a narrow synthetic trusted-session HTTP compatibility profile. It does
not qualify a native Go SDK, sibling frameworks, browser/human provenance,
accessible recovery,204/304,streaming or post-origin rollback. It is Rust Gate
integration with a Go service, not legacy Go core development. Full-product
qualification and release readiness remain false; no full-product gate is waived.
