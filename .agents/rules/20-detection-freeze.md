# Detection freeze (default)

Unless the user **explicitly** requests a scoring or verdict change:

- Do not change signal `weight` or `expected_human` values in `crates/signals/src/observation/registry.rs` and its embedded `registry.json`, or confidence/score behavior in `observation/signal.rs` and `observation/verdict.rs` under that crate.
- Do not change hard-rule predicates, order, or labels in `crates/scoring/src/promotion` in a way that alters production verdicts.
- Do not move `challenge_at` / `deny_at` bands in `crates/scoring/src/policy.rs`; preserve their public serde names `ChallengeAt` / `DenyAt`.
- String id renames must keep **byte-identical** public ids (or be a deliberate MAJOR/policy event).
- Structural refactors are fine when behavior-preserving and covered by existing golden / attack gates.

The former `internal/signals/registry.go` and `internal/scoring/policy.go` are
read-only historical oracles, not the current implementation owners. This pointer
update does not authorize any registry, default, predicate, value or public-ID change.

If a detection change is requested: update SoT first, then code, then full red-blue gate; treat as policy-version event.
