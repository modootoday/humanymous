---
title: How the self-correcting behavioral model works
description: "Why the optional behavioral model on the humanymous Core engine changes no verdicts: a weight-0, score-exempt residual that self-calibrates from an independent human oracle, protects accessibility cohorts, auto-rolls-back on the safe side only, and is promoted by redeploy — never a runtime endpoint."
keywords: ["weight-0 score-exempt signal","monitor-first machine learning detection","self-calibration adaptive conformal","independent human oracle labels","model collapse avoidance","accessibility cohort false-positive gate","canary auto-rollback safe direction","redeploy to promote","behavioral biometrics bot detection","humanymous Core engine"]
---

# How the self-correcting behavioral model works

**Diátaxis quadrant:** Explanation.
**Audience:** evaluators and detection developers asking the fair question "does adding a machine-learning model change what this system blocks?" The short answer is **no**, and this page explains the design choices that make that true by construction.

> **Note:** This repository is a reference implementation, not a production-hardened build. Where a property depends on the operator's configuration or on a future decision, this page says so rather than overclaim.

The Core engine can load an optional behavioral model that scores the aggregate behavioral features it already computes and emits a single signal, `l4.ml.behavioral`. Everything distinctive about it follows from one constraint: it must be able to *learn* without ever being allowed to *decide*.

## It annotates; it does not vote

`l4.ml.behavioral` is **weight-0 and score-exempt**. It carries a calibrated probability for the audit trail, but it contributes nothing to the risk score and matches no hard rule. A request scored with the model loaded and the same request scored with no model loaded reach the **same verdict** — allow, challenge, or deny — every time. When no model is present the signal is not even emitted.

This is why enabling the model is safe: it cannot raise your block rate, cannot lower it, and cannot be the reason any individual is challenged or denied. It is an observation layer over the frozen detection engine. Making it *affect* a verdict — giving the signal a weight — is a separate, deliberate change to the detection policy (a "detection-freeze" decision), not a side effect of turning the model on.

## Monitor-first: self-calibration moves an annotation, not a verdict

The model's fire threshold — the probability at which it annotates a session as model-suspicious — **self-calibrates** from live outcomes so that the realized false-positive rate on confirmed humans tracks an operator budget (default 0.5%), even as traffic shifts. This is an online, distribution-robust controller, not a retraining loop.

Because the thing it adjusts is the threshold of a **weight-0** signal, self-calibration can only ever change the annotation, never a verdict. "The engine tunes itself" is true of this one audit annotation and *only* that; the model's actual weights change solely through an offline trainer plus a redeploy (below). There is no production loop that rewrites the model in place.

## The oracle is a solved challenge — never the model's own output

A model that trains on labels it produced itself collapses: it drifts toward its own biases and quietly loses the rare, novel cases that matter most in security. So no label ever comes from the model. Labels come from an **independent** adjudicator:

- A **solved [humanymous Pass](../help/why-am-i-seeing-this.md)** — an interactive challenge designed to resist automation — is strong evidence of a **human**.
- A **failed** Pass is **not** treated as a bot. People fumble challenges, and mislabeling them would poison the data and, worse, penalize exactly the users a false-positive path exists to protect. Failed attempts inform only a rate-of-solving safety check, never a training label.
- For drift detection, the model is measured against the **frozen engine's own verdict** as the reference it must not silently diverge from.

Confirmed humans are the scarce, valuable signal — including humans the model or engine was inclined to over-flag — and they are the only training labels the online oracle produces.

## Accessibility cohorts: fairness enforced in the gate, not hoped for

A behavioral model built on motor microstructure is most likely to over-flag precisely the people who generate little of it: keyboard-only, switch-access, and other assistive-technology users. An aggregate false-positive number hides that harm behind the pointer-using majority.

So collected traces carry a coarse **input-modality cohort** (keyboard / pointer / default), and the offline trainer will emit a candidate **only if** the human false-positive rate is ≈ 0 **within every cohort**, not merely overall. A candidate that regresses a minority cohort is discarded. This is a property of the *offline promotion gate* — a fairness check the artifact must pass before it can ship — not a runtime guarantee about any single session. It ties into the first-class [accessibility path](../help/challenge-accessibility.md): the challenge, and the model that watches it, are built around not punishing people for how they interact.

## Autonomous only in the safe direction

An armed **canary** puts a freshly deployed model on probation and, on the first breach of the human-FP budget (or a drift or poisoning alarm), **disables the model** — reverting to the heuristics-only baseline — with no human in the loop. That autonomy is defensible precisely because the signal is weight-0: the worst an unattended trip can do is make detection *more conservative* (drop an audit annotation), never break a verdict. The model can be removed automatically; it can never be *promoted* automatically.

## Why promotion is a redeploy

Putting a **new** model in front of traffic is the one move that could matter if the signal were ever weighted, so it is deliberately **not** automated and **not** a runtime endpoint. Promotion is an operational redeploy: sign the artifact, ship a new engine image pinned to that signed digest, and let a rolling deploy converge every replica on the same model. The reasons:

- **Honest control.** A verdict-affecting model change is a high-blast action that deserves separated duties (a distinct requester and approver). The Core engine's operator surface is a single shared token, which cannot express that — so a "dual-control promote" there would be ceremony, not control. Separated-duty approval lives on the reverse proxy's admin plane, where it is real.
- **Auditability.** An immutable, signed artifact baked into a deployment means "the model running" equals "the digest you shipped." A live swap endpoint would let the running model drift away from the deployed image, which is exactly what on-call must be able to trust.
- **Convergence and rollback you already own.** A rolling deploy pins every replica to one digest and rolls back by redeploying the previous tag — better ergonomics than a bespoke in-process swap that one operator knows about.

If this residual is ever given a weight and starts affecting verdicts, the promotion becomes a proxy-admin action with separated-duty approval that triggers an engine reload — still never a Core-native mutation API.

## What this is not

The behavioral model does not defeat the detection ceiling. A sufficiently coherent, human-assisted, or fully humanized session can score like a person; the honest mitigations for that are elsewhere (rate limiting, cross-session reputation, and the [attested-route step-up](../how-to/configure-attested-routes.md)), not this model. It is a residual that sharpens observation of the murky middle, evaluated on your own traffic — not a claim to stop everything.

## Related

- [Operate the behavioral model](../how-to/operate-behavioral-model.md) — the operator task guide.
- [Detection engine internals](./detection-engine-internals.md) — the frozen scoring core this residual sits beside.
- [Data processing & personal-data inventory](../reference/data-processing-inventory.md) — what the training-trace file contains and its privacy posture.
