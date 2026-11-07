---
id: evidence:root/20261107053239
title: Adapter authority shared client qualification
status: active
created: 2026-11-07T05:32:38Z
updated: 2026-11-07T20:09:39Z
rel:
  depends_on: [sot:root/adapter-authority]
files:
  - {path: authority-shared-client-results.json, sha256: 065c93adf2ff9c338971d9402e57826a1a52ed673d1ae5a7833dcbcbb99edd8c, bytes: 9537, store: repo}
  - {path: canonical-authority-source.json, sha256: b73d2b35e58e51c12d2ce0893d23e87c369b4a932bf521ee4ad6a56218ffae7f, bytes: 5247, store: repo}
  - {path: sealed-startup-source.json, sha256: dbdd5d4db9dac9d8122285c27bb235fdd1e48020a4ae37884cf46ecbc1307a64, bytes: 3078, store: repo}
  - {path: sealed-local-results.json, sha256: e2240677e28b0893050dd79c7dd670a4acac9bb69dcf7e0b2cd13b0f2cbdeebe, bytes: 1167, store: repo}
  - {path: sealed-revalidation-scope.json, sha256: dfc1e2de11efb2660320aa72ee9d65370fe868d9bd89febc80268d85af44c265, bytes: 2561, store: repo}
  - {path: hosted-42354873001.json, sha256: feaebd98451516d314b97ed8e6e18ac91be9ecfce0bee6013df48622698db73d, bytes: 1827, store: repo}
  - {path: hosted-42355168729-partial.json, sha256: 18b3c1d5b84c2716c8221747709d991a35632630f4255131c950b7c6cc53bd8a, bytes: 1453, store: repo}
  - {path: hosted-42355168729-linux-authority.json, sha256: c57523c51f4eec4684251240e9f4e5276bc06f41f9f0096fa5db80a4b7e23e43, bytes: 3900, store: repo}
  - {path: hosted-42355168729-native-actors.json, sha256: 8836ddf829e2870ce429923e97df7818a06d090ab987cb331b63e3c10b38278e, bytes: 2538, store: repo}
  - {path: hosted-42355168729-final.json, sha256: 700e5c6809383fc61e0b9edc691cd5d38c38006a01188f62e9bda238e9e39f61, bytes: 3552, store: repo}
  - {path: current-windows-pure.json, sha256: a498fbccea7dda669ec149088a3b18e753583babe7a3d523fc1cc79d75520996, bytes: 2281, store: repo}
  - {path: current-windows-ipc.json, sha256: 375d5f6f1cbc17703e2604aa83fe3c08a88ecffb28819efd810884f79b3b0331, bytes: 2781, store: repo}
  - {path: independent-expiry.json, sha256: f675dc5e9c606a12ccb109c21b58d164bf514d48a2c25fa18054641bdc0cc110, bytes: 2458, store: repo}
  - {path: sealed-observer-ownership.json, sha256: 6dac8d86b73fb1c2d482140d5af61a2c62add1ae22dea14161966cd33c2f1b0f, bytes: 2706, store: repo}
  - {path: encoded-key-first-failure.json, sha256: 768d4447b3696ddc1171f4ae8c1a46043ee81374d7210b36515d03df37c01c7d, bytes: 1669, store: repo}
  - {path: encoded-key-retry.json, sha256: bc879112fcbb7a2f91c65b5a0dda4c219da2fdc06e71b9afdce6878333e0baa6, bytes: 2456, store: repo}
  - {path: authorization-kernel-r2.json, sha256: 4d9c7e0a6b80edaf6cb810db527cc1061113b8ed4e4690abfdddc540c9ddac92, bytes: 9106, store: repo}
  - {path: structural-handoff.json, sha256: b248088d83c0c7ae4bf06609e2a64e35fa66c0ccdb5b38a300577c7c7da91f3b, bytes: 5543, store: repo}
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
