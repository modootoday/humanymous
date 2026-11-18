---
id: sot:root/abuse-similarity
title: Similarity to confirmed abuse patterns
status: draft
created: 2026-11-18T06:00:00Z
updated: 2026-11-18T06:00:00Z
verify: cargo test --locked -p humanymous-core --lib model::similarity
rel:
  governs: [crates/sdk/humanymous-core/src/model/similarity/**]
  depends_on: [sot:root/rust-migration, sot:root/model-training-correction]
---

# Similarity to confirmed abuse patterns

Document version 0.1.0. The owner asked on 2026-11-18 for three things:

- cross-check new sessions against behavior patterns that were already blocked
  for abuse;
- escalate toward blocking from that similarity and how often the pattern
  recurs;
- let an administrator set the allow, challenge and block criteria in the
  admin panel.

This SoT fixes the safe shape of that feature. Under the detection freeze it
ships in **shadow mode only**. Promotion into enforcement is a separate owner
decision, recorded in the needs-owner issue.

## Model

- **Vector space.** The fixed `bv1` behavior feature vector (27 dimensions,
  `crates/sdk/humanymous-core/src/behavior/features/schema.rs`). A schema hash
  change invalidates the store.
- **Admission.** Only independently confirmed abuse enters the store:
  - a measured red-team catalog session against this deployment;
  - an operator-confirmed review.

  Automatic enforcement results, shadow outcomes, model probabilities and weak
  CHALLENGE failures are not admissible. This stops a false positive from
  reinforcing itself.
- **Clusters.**
  - A confirmed vector joins the nearest cluster within standardized distance
    2.0. Otherwise it starts a new cluster.
  - Each cluster keeps only a count, a per-dimension mean and a Welford
    variance (floored at 0.01), plus the times of later matches.
  - The store keeps no raw vectors and no subject identifiers.
- **Similarity.** For the nearest cluster with at least 5 confirmed members,
  similarity is `exp(-d²/2)`, where `d` is the root-mean-square standardized
  distance. Probing is read-only.
- **Recurrence.** A recorded match feeds the cluster's recurrence count only. It
  never moves the mean or variance.

## Accessibility rule

The system measures only "near a confirmed abuse cluster". It never measures or
uses "far from typical human behavior". Atypical motor or speed patterns,
including assistive technology, therefore cannot raise an outcome. The policy
format has no field that could express the forbidden direction. Unknown fields
are refused.

## Policy bounds

An operator policy is versioned JSON with:

- a recurrence window of 1 hour to 7 days (the ban strike decay);
- 1 to 16 rules.

Each rule names an action and three minimums: similarity, cluster recurrence
and subject repeats. Subject repeats count prior matches by the same IP or
fingerprint key, as supplied by the runtime. The most severe matching rule
wins; no match means no change.

Operators cannot lower these floors:

| Action | Min similarity | Min cluster recurrence | Min subject repeats |
|---|---|---|---|
| challenge | 0.80 | 0 | 0 |
| temporary block | 0.90 | 3 | 2 |
| permanent-ban proposal | 0.95 | 10 | 3 |

Consequences:

- Behavior similarity alone can reach at most CHALLENGE. Any block also needs
  repetition by the same subject.
- A temporary block goes through the existing progressive strike path in
  `gateway/ban.rs`.
- A permanent ban is only ever a proposal for the existing dual-control queue
  (`Action::PermanentBan`). No automatic or single-operator path creates one.

The floors are lab defaults for shadow evaluation. They are not production
thresholds.

## Retention and erasure

- A cluster not reinforced by confirmed evidence for 30 days expires.
- Match times older than the policy window are dropped.
- An operator can retire one cluster. The runtime decides whether retirement
  needs dual control; erasure of the whole store follows the existing
  dual-control erasure rule.
- Policy changes are audited with the SHA-256 digest of the canonical policy
  JSON.

## Shadow accounting and preview

The shadow ledger compares the policy outcome with the verdict the frozen
engine actually served. It counts:

- stricter, looser and agreeing outcomes;
- the outcome distribution;
- the number of behavior events observed when a non-trivial outcome was
  reached, in buckets of 16, 64, 256 and more. This measures time to detection.

Before an operator saves a policy, the admin panel can replay held
de-identified samples under it. The preview never changes a verdict.

## Stages

1. **Kernel (this slice).** A pure module in `humanymous-core` covering the
   store, policy validation, the shadow ledger and preview, with unit tests.
2. **Offline separation check.** Generate catalog sessions in Docker and
   measure whether bot families separate from human controls in `bv1` space.
   If they do not separate, stop and record the negative result.
3. **Runtime shadow wiring.** Add an admin API and console view for the policy
   (bounded, audited, with preview) and record shadow statistics next to the
   served verdict. Verdicts are unchanged.
4. **Promotion (owner only).** Lifting the detection freeze and connecting
   outcomes to challenge or ban requires an owner decision. It then goes
   through gate, canary auto-rollback and dual control.

## Limits

- The T4 ceiling (a coherent real browser at human pace) is unchanged. A
  careful operator can shape behavior away from known clusters.
- Similarity is evidence about behavior, not identity.
- Feedback poisoning is limited by admission control and canary rollback, not
  eliminated.
