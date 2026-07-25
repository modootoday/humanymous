# Scoring freeze golden corpus (SoT-38 §9.1)

Frozen `SessionReport` → `Score` snapshots for today's detection policy.

- **Asserted by:** `internal/scoring/freeze_golden_test.go`
- **Update only** with a deliberate detection change:

  ```bash
  UPDATE_SCORING_GOLDEN=1 go test ./internal/scoring -run TestFreezeGoldenCorpus
  ```

  That commit must carry `!` / `BREAKING CHANGE:` when verdicts move (SoT-37).

Do not hand-edit JSON risk scores without re-running Score().
