---
id: evidence:root/20261107235307
title: CLI installation old library semantic inventory
status: active
created: 2026-11-07T23:53:07Z
updated: 2026-11-08T00:05:02Z
rel:
  depends_on: [sot:root/cli-installation, sot:root/structural-development]
files:
  - {path: summary.json, sha256: c7d2cabaa0e2ec0bccb5fb5c4a9a60c252b3726a485da21c2aa5e45ae6e1ab5f, bytes: 4945, store: repo}
  - {path: clock-precision-erratum.json, sha256: f1de27de603e55f5bacd954d8b0abe389b19923f7c00363164bdb2215c50d436, bytes: 3834, store: repo}
---

Root allocated six OLDlibrary --list commands on immutable source
5d68cd39f1d44b30d0df2804675d83cd28025d7e, retained comparison B. No overlays,
source corrections or retries. This observes test registration; NOtest function
was executed and no test pass is claimed.

Clock precision erratum: clock-precision-erratum.json preserves the original raw
command JSON and reproduces PowerShell Math.Min integer-overload rounding. The
nominal90s window was logged as90.0390521..90.3159935s from captured initial elapsed,
not an exact90s threshold; settlement deadlines were similarly rounded. All six
actual observations remained below90s (maximum57.3472881s), with no observed
overrun, timeout or rerun. This does not qualify an exact90s cutoff. Future monitors
must capture initial elapsed once and use explicit double bounds/arguments or
declare recorded precision. Original63raw files are unchanged; the erratum is one
additional provenance attachment, not rewritten timing evidence.

| Old package | Windows registered names | Exact expected-name differences |
| --- | --- | --- |
| humanymous-native-cli | 11 | 0 |
| humanymous-attacker | 27 | 0 |
| humanymous-defender | 9 | 0 |
| humanymous-agent-maintenance | 76 | 0 |
| humanymous-pages | 37 | 0 |
| humanymous-cli | 26 | 0 |

All186names match the source map. All187lexical case-source hashes match actual
immutable B before-snapshot bytes; the one absent case is the expected Unix-only
CLI permission test. Attacker's27includes25nonignored unit candidates and two
existing ignored native cases (one portable, one Windows-only). Discovering an
ignored name is not a native pass. Maintenance's76old LIBnames are separate from
the accepted33integration/bin-relocation cases.

Each command was locked/offline/jobs1/incremental0, --lib --all-features -- --list,
with retained actual Process handles and concurrent stdout/stderr copy tasks.
All six actual exits were0 and both output tasks completed; metadata preparation
also exited0. No observation timeout or resource stop occurred. Maximum actual
context observation57.35s stayed below90s. Original Stopwatch work161.837s includes
pre/post preparation, below390s; actual marker cleanup recorded198.506s by UTC
attribution, below450s total. UTC is attribution, not a replacement monotonic clock.

All3128source files match literal5d68Git blob bytes in both immediate snapshots,
with0differences. Selected metadata closure234packages includes10local/224registry
packages;9444registry files and66compiler/target-library files match before/after,
9510rows total. Seventeen environment variables match. Existing HOMEconfig inputs
and source-contained config scope were captured; a separate post-only ancestor
audit found0present config files and is not retroactive before-snapshot evidence.
This is not an attestation of every linker/OS/config input or a full read closure.

All seven command handles settled, exclusive target marker was removed and target/
source B returned intact. No Cargo/rustc was observed at return. The trusted Cargo
Stopwatch monitor is not a native Job/hostile-child/global CA-LIFETIME controller,
and elapsed time or an empty process observation does not prove all descendants
cleaned. Six executable hashes are post-run observations only, not artifact leases.

Private evidence:root/20261107235309 indexes63raw files: commands/process/exit/
output records, exact expected/actual lists, source/dependency/environment hashes,
source-map byte comparison, admission and cleanup. summary.json pins the results.
No15bin/new-layout selector discovery, actual test/native/service/account/SDK/
Docker/CI/release work occurred. New layout, runtime/Ready, shipping and product
qualification remain separate gates; local services/accounts were unchanged.
