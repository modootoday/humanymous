# Rust engine wargame lab

This runs synthetic/frozen observation reports through the existing Rust scoring
engine. It does not send attacks, collect browser signals, verify challenge
credentials or enforce HTTP traffic. See `sot:root/rust-wargame` for runtime
identity binding and egress requirements that block a future network runner.

Before root workspace integration, create an isolated snapshot (the destination
must not already exist):

```powershell
pwsh -File test/rust-wargame/prepare-workspace.ps1 -Destination <private-new-directory>
cargo test --manifest-path <private-new-directory>/Cargo.toml -p humanymous-attacker -p humanymous-defender
cargo clippy --manifest-path <private-new-directory>/Cargo.toml -p humanymous-attacker -p humanymous-defender --all-targets -- -D warnings
cargo run --manifest-path <private-new-directory>/Cargo.toml -p humanymous-attacker -- --seed 42
docker build -t humanymous/rust-wargame:local <private-new-directory>
docker compose -f test/rust-wargame/compose.yaml run --rm engine-pilot
python test/rust-wargame/verify-report.py <saved-report.json> [same-seed-report.json]
```

The first Cargo command creates the snapshot lockfile used by the Docker build.
Runtime uses `network_mode: none`; dependency downloads happen only during build.
No `--target`, URL or runtime option exists. Invalid CLI arguments exit 2; failed
engine/regression or JSON output writes exit 1. Exit 0 means supported observation
regressions passed, while `runtime_coverage_complete` remains false. Read totals
and every unsupported/error/skip row; a successful simulation is not full Gate e2e.

Seed only permutes scheduled cases. Identical source/lockfile and seed reproduce
the report byte-for-byte. Human zero-motor, long-window accessibility CHALLENGE,
and coherent-observation T4 ALLOW remain separate controls. No measured human
false-positive rate is inferred from these synthetic observations.
`human_controls_all_allow` is false when a human/accessibility control is
challenged, denied, errored, skipped or unavailable, even if frozen regression
expectations match. This keeps an existing heuristic CHALLENGE visible.

The image and both non-publishable crates are private lab tooling. Do not add
them to normal product builds, runtime images, npm or public container releases.
