---
title: Operate the behavioral model
description: "How to load, observe, canary, and promote the optional behavioral model on the humanymous Core engine. The model produces an audit-only, weight-0 signal — enabling it changes no verdicts — and promotion is a redeploy, not a runtime endpoint."
keywords: ["behavioral model artifact","ml-bundle","audit-only signal","weight-0 residual","self-calibration false-positive budget","shadow candidate model","canary auto-rollback","signed model admission ed25519","offline retrain gate","redeploy to promote","humanymous Core engine"]
---

# Operate the behavioral model

**Diátaxis quadrant:** How-to.
**Audience:** operators and detection developers running the standalone Core engine (`bin/server.exe`) who want to load, observe, canary, and promote the optional behavioral model. Defensive, own-deployment only.

> **Note:** Defensive, local-only, self-target-only, educational. This repository is a reference implementation, not a production-hardened build. Every number you observe is reference-measured on your machine, not a guarantee.

The behavioral model is an **optional** add-on to the Core engine. It scores the aggregate behavioral features the engine already computes (pointer and keystroke timing/motion statistics, event-cadence counts) with a small policy-specific model and emits one signal, `l4.ml.behavioral`.

Read this first, because it changes how you should think about everything below: that signal is **weight-0 / score-exempt**. It contributes an *audit annotation* only — it is **not** a cause of any verdict. Turning the model on does not change what gets allowed, challenged, or denied; a build with no model loaded and a build with the model loaded produce byte-identical verdicts. The model exists so you can observe, self-calibrate, and evolve a behavioral residual **alongside** the frozen engine without spending detection freeze. (Giving it a non-zero weight — making it affect verdicts — is a separate, deliberate detection-freeze decision, not something this page does.)

For the engine itself and how to read a verdict, start with [Run the standalone detection engine](./run-detection-engine.md). For where the model's data lands on disk and its privacy posture, read [Data processing & personal-data inventory](../reference/data-processing-inventory.md) **before** you enable trace collection.

A note on words: in this project a **"bundle"** is the client-side detection loader that Gate injects into your HTML (see [The `/__hmn/` control plane and the injected detection bundle](../explanation/control-plane-and-bundle.md)). The thing this page loads is a **behavioral model artifact** — a different object entirely. The flag happens to be spelled `-ml-bundle`; treat it as a literal token, not the client bundle.

## When to use this

Use this when you want a behavioral residual scored and observed next to the frozen engine, and you want the shadow → canary → redeploy path to evolve it over time — without touching detection freeze. You do **not** need it for baseline detection; the engine blocks automation on its own.

## Prerequisites

- A running Core engine (`bin/server.exe` on `127.0.0.1:8443`, per [Run the standalone detection engine](./run-detection-engine.md)).
- An operator bearer token configured (`-ops-token`) so you can read `/api/mlcorrect`.
- A behavioral model artifact. Build one with `make ml-model` (a synthetic bootstrap — see [Retrain offline](#retrain-a-candidate-offline)). For a signed production load you also need an Ed25519 public key, a detached signature, and optionally a pinned digest.

## Enable the model

### Development (unsigned)

```bash
bin/server.exe -ml-bundle configs/ml/behavioral.json
# or: HMN_ML_BUNDLE=configs/ml/behavioral.json bin/server.exe
```

`-ml-bundle` is the path to the signed behavioral-model artifact. The artifact's feature-schema pin is **always** enforced; a signature is not required in this unsigned dev path. If the artifact is missing, malformed, or built against a different feature schema, the engine **logs and runs on heuristics** — a bad model never takes the edge down and never blocks a human on model plumbing (fail-open on the model). With no `-ml-bundle` at all, the signal is simply never emitted.

### Production (signed admission)

Configure a public key to make the signature **required** on the model that actually faces traffic:

```bash
bin/server.exe \
  -ml-bundle configs/ml/behavioral.json \
  -ml-pubkey configs/ml/model-pub.pem \      # PKIX Ed25519 → signature now REQUIRED
  -ml-bundle-sig configs/ml/behavioral.json.sig \
  -ml-bundle-digest 3f9c...                   # optional: pin the exact sha256
```

With `-ml-pubkey` set, an artifact with a missing or invalid signature is **refused**, and the engine falls back to heuristics rather than serve an unverifiable model. A pinned `-ml-bundle-digest` is refused on any mismatch. This is what keeps "the model in effect" equal to "the artifact you signed and deployed."

### Tune the human-false-positive budget

```bash
-ml-fp-budget 0.005   # default
```

This is the realized human false-positive rate the model's fire threshold maintains. It self-adjusts **only** from oracle-confirmed humans (see [How self-correction works](../explanation/self-correcting-behavioral-model.md)); a smaller budget makes the residual less eager to flag, a larger one more sensitive. Because the signal is weight-0, this only tunes the *annotation* threshold, never a verdict.

## Verify it is running

`/api/mlcorrect` is a read-only, operator-token-gated Core endpoint (no personal data). A missing or wrong token answers `404` by design — that is deny-by-default, not a bug.

```bash
curl -sS -H "Authorization: Bearer $HMN_OPS_TOKEN" http://127.0.0.1:8443/api/mlcorrect
```

- No model loaded → `{"enabled": false, ...}` (a marker, not a `404`).
- Model loaded → `enabled: true` plus `activeBundle` (`version` + `digest`), `fireThreshold`, `calibration`, `drift`, `passSolveRate`, `shadow`, and `canary`. Confirm `activeBundle.digest` is the artifact you meant to deploy.

## Promote a new model (shadow → canary → redeploy)

Promotion is a **redeploy**. There is deliberately **no runtime endpoint** that swaps the model in a running engine — the reasons are in [How self-correction works](../explanation/self-correcting-behavioral-model.md#why-promotion-is-a-redeploy). The in-process automation owns only the *safe* (rollback) direction.

1. **Shadow the candidate.** Run it in parallel with the live model; it is scored but **never served**:

   ```bash
   -ml-shadow-bundle configs/ml/candidate.json
   ```

   Read the `shadow` block of `/api/mlcorrect`: how far the candidate's scores diverge, and — at the live threshold — whether it would flag *more humans* or *more of what the live model passes*. High divergence toward flagging humans is a reason **not** to promote.

2. **Roll it out under a canary.** Sign the candidate, then deploy a **new Core image/tag** with the candidate as `-ml-bundle` (plus the signed-admission flags) and the canary armed:

   ```bash
   -ml-canary -ml-canary-max-fp 0.02 -ml-canary-probation 200
   ```

   During probation the model is auto-disabled (reverted to heuristics) on the **first** breach of the human-FP ceiling (`-ml-canary-max-fp`), on a drift alarm, or on a Pass-solve poisoning anomaly. If it observes `-ml-canary-probation` oracle-confirmed humans with no breach, it **graduates**. Watch `canary` and `calibration` on `/api/mlcorrect` through the window.

## Roll back

- **Forward rollback (a bad model got out):** redeploy the previous image tag. In-process, an armed canary already auto-reverts the model to heuristics and logs `ml-canary: AUTO-ROLLBACK …`; the redeploy makes the good artifact durable.
- **There is nothing to page for here.** Because the signal is weight-0, an auto-rollback changes no verdicts — see the [on-call quick reference](../reference/on-call-quick-reference.md).

## Retrain a candidate offline

```bash
make ml-model                                   # synthetic bootstrap artifact
make ml-retrain ANCHOR=gold.jsonl ORACLE=traces.jsonl [BASE=configs/ml/behavioral.json]
```

`make ml-retrain` runs the offline trainer, which writes a candidate **only if** it clears the promotion gates: human false-positive ≈ 0 **per accessibility cohort** (not just in aggregate), no regression against the red-team catalog, and a time-ordered evaluation. **A run that fails a gate writes nothing.** Labels come only from an independent oracle (a solved [humanymous Pass](../explanation/control-plane-and-bundle.md) is a confirmed human) — never from the model's own output.

## Collect training traces (lab only)

```bash
-ml-trace-dir /var/lib/hmn/ml-traces    # OFF by default
```

When set, the engine appends a labeled trace — the session's **aggregate** behavioral summary plus an input-modality cohort — for each **solved** Pass (a confirmed human) to a JSONL file, for offline retraining. A failed Pass is never recorded as a bot.

> **Privacy caveat.** This writes labeled behavioral summaries to disk in plaintext. It is not covered by the engine's pseudonymization or erasure primitives. Enable it only on an operator-owned lab host, and **record it in your data inventory first** — see [Data processing & personal-data inventory](../reference/data-processing-inventory.md), which describes exactly what the file contains, its retention, and why per-subject erasure does not reach it.

## Troubleshooting

| Symptom | Where to look | What it means |
|---|---|---|
| `/api/mlcorrect` shows `enabled:false` after `-ml-bundle` | startup log `ml-bundle: verify/stage failed …` | schema, digest, or signature mismatch → running on heuristics |
| Signed artifact refused | log `ml-pubkey`/`ml-bundle-sig` read or verify failure | no valid signature under a configured key → running on heuristics (intended) |
| `canary` reports a rollback | log `ml-canary: AUTO-ROLLBACK …` | model auto-reverted on a human-FP / drift / poisoning breach; inspect `calibration` vs `drift`; redeploy a known-good tag |
| `shadow` shows high divergence | `/api/mlcorrect` `shadow` block | the candidate would behave very differently — do **not** promote it |
| `fireThreshold` moving over time | `/api/mlcorrect` `fireThreshold` | normal self-calibration; alarm only on a canary breach |

## Related

- [Run the standalone detection engine](./run-detection-engine.md) — build and run the Core, read a verdict.
- [How the self-correcting behavioral model works](../explanation/self-correcting-behavioral-model.md) — why it changes no verdicts, and why promotion is a redeploy.
- [Data processing & personal-data inventory](../reference/data-processing-inventory.md) — the trace file, before you enable it.
- [On-call quick reference](../reference/on-call-quick-reference.md) — what to check (and not page for) when the canary trips.
