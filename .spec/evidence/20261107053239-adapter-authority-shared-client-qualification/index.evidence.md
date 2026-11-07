---
id: evidence:root/20261107053239
title: Adapter authority shared client qualification
status: active
created: 2026-11-07T05:32:38Z
updated: 2026-11-07T08:16:26Z
rel:
  depends_on: [sot:root/adapter-authority]
files:
  - {path: authority-shared-client-results.json, sha256: 065c93adf2ff9c338971d9402e57826a1a52ed673d1ae5a7833dcbcbb99edd8c, bytes: 9537, store: repo}
  - {path: canonical-authority-source.json, sha256: b73d2b35e58e51c12d2ce0893d23e87c369b4a932bf521ee4ad6a56218ffae7f, bytes: 5247, store: repo}
  - {path: sealed-startup-source.json, sha256: dbdd5d4db9dac9d8122285c27bb235fdd1e48020a4ae37884cf46ecbc1307a64, bytes: 3078, store: repo}
  - {path: sealed-local-results.json, sha256: e2240677e28b0893050dd79c7dd670a4acac9bb69dcf7e0b2cd13b0f2cbdeebe, bytes: 1167, store: repo}
  - {path: sealed-revalidation-scope.json, sha256: dfc1e2de11efb2660320aa72ee9d65370fe868d9bd89febc80268d85af44c265, bytes: 2561, store: repo}
---

# Adapter authority shared client qualification

## What was measured

Rust1.99.1/edition2024 authority shared local client, credential-bound Node/nginx
profiles, native peer/pipe DACL boundary and generic management-transport seam.
Final Windows25 tests and isolated optimized LocalHandler3 pass. Final Linux
network-none Docker22 tests pass, with2explicit privileged/helper ignores excluded
from successes; the separate other-UID parent1 passes and actually invokes its
UID65534 helper. The nested helper is not another product case.

Actual negatives include scope/profile/nonce/body/restart/audit/expiry, wrong
server PID, real world-granted pipe refusal before any credential byte,144
nonreading peers followed by a healthy decision, fixture-management credential/
route/Origin refusal, canceled worker admission and explicit drain waiting.
Clippy and scoped formatting pass. Cargo known vulnerabilities0; preserve the
unmaintained ttf-parser advisory outside the authority dependency closure.

Attached sanitized facts contain exact owned runtime/test source hashes,
working-vs-Git-filtered blob equality, compiler/image identity, test denominators,
source overrides and explicit unqualified capabilities. All19 owned source/
manifest/test inputs are LF and match Git-filtered bytes. This is validated
working source until the separate canonical checkpoint receipt is attached.

## How to read it

Component evidence, not full product release, runtime admin or browser/human
acceptance. v0.1 has no session receipt channel; required raw TLS/H2/collector/RIT
remain422. Real runtime LocalHandler/session/one-use receipt, sealed audit owner,
external Windows host denial and full browser/Pass/Gate qualification remain open.
No credential, cookie, raw application entity, private trace or installed artifact
is attached publicly. Original failed/earlier runs remain in private execution
evidence, including fixture expiry/admission races, missing Docker workspace
members, lock refusal and shared executable contention. They are not rewritten
as successful runs. The final test container has recorded source overrides on
the pinned base image; image identity alone is not its complete source identity.
