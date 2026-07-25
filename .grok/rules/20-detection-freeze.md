# Detection freeze (default)

Unless the user **explicitly** requests a scoring or verdict change:

- Do not edit signal weights, confidence, or expected-human values in `internal/signals/registry.go`.
- Do not change hard-rule predicates, order, or labels in a way that alters production verdicts.
- Do not move `ChallengeAt` / `DenyAt` bands in `internal/scoring/policy.go`.
- String id renames must keep **byte-identical** public ids (or be a deliberate MAJOR/policy event).
- Structural refactors are fine when behavior-preserving and covered by existing golden / attack gates.

If a detection change is requested: update SoT first, then code, then full red-blue gate; treat as policy-version event.
