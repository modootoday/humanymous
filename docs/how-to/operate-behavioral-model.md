---
layout: doc
title: Review behavioral model operations
permalink: /how-to/operate-behavioral-model.html
reviewed: 2026-11-20
---
# Review behavioral model operations

**Diátaxis quadrant:** How-to. **Audience:** model and service reviewers.

No qualified Rust model-operation command or automatic learning loop is claimed.
Before a pilot, define training input provenance, bounded resources, poisoning
controls, rollback and human-impact measures.

Include sparse interaction, keyboard-only access and assistive technology in
controls. Keep model errors distinct from verdicts. Migration work does not
authorize changes to frozen scoring or motor-based recovery requirements.
See [model limits](../explanation/self-correcting-behavioral-model.md).

## Stage a candidate at startup

The runtime gateway stages a candidate only when its private configuration
has an admin listener with a `models` object:

```json
"admin": {
  "listen": "127.0.0.1:8445",
  "operators": ["…"],
  "bans": {"…": "…"},
  "models": {
    "bundle": "/srv/humanymous/model/bundle.json",
    "public_key": "/srv/humanymous/model/bundle-key.pem",
    "signature": "/srv/humanymous/model/bundle.sig",
    "digest": "<64 lowercase hex characters>"
  }
}
```

Only `bundle` is required. Set `public_key` (a PKIX Ed25519 PEM key) to
require a signature over the exact bundle bytes, and `digest` to pin its
SHA-256. Without either, only the feature schema protects the bundle.

At startup the bundle is verified and staged. It is never served at startup.
The admin listener's readiness facts show `signed`, `staged` and, when nothing
was staged, the refusal class. A candidate that fails verification does not
stop the gateway: nothing is staged and the gateway runs without a model. A
configured public key that cannot be read or parsed stops startup, so a signed
setup never runs unsigned.

## Promote or roll back a staged model

The runtime admin plane can promote and roll back a behavioral model. These
are operator actions on the admin listener, and each one is audited:

1. A verified candidate must be staged first. Staging checks the digest pin,
   the signature when a public key is configured, the bundle shape and the
   feature schema. Staging and training never serve a model.
2. `GET models` shows the staged, active and previous bundle versions and
   digests.
3. An operator sends `POST models/promote` with the staged digest. This
   creates a pending action. A different principal with the approver role
   commits it through `approvals/<id>`.
4. An operator sends `POST models/rollback` with the active digest (empty when
   no model is active). The previous model is served again, or no model.

Each change is recorded in the audit log before it takes effect. If that record
cannot be written, the request fails with 503 and nothing changes. A digest
that does not match answers 409, and so does a request with nothing to promote
or roll back; these refusals are decided before anything is recorded. If the
model lifecycle is unavailable, the request fails with 503 and nothing is
recorded.

A served model only adds an annotation with weight 0. It does not change any
score, verdict, threshold or signal. The model routes exist only when the
`models` object is configured; otherwise they answer 404.
