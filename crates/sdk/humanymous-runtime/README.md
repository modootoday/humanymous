# humanymous-runtime

Safe contextual service implementation for Humanymous. Context features select
`gateway`, `authority`, `audit`, `capture`, `network` and `native-security`.
The default gateway includes its prerequisite contexts. Production readiness
still requires actual native custody, durable audit, witness and runtime gates.

Four separately supervised executable roles retain their existing names.
Integration tests and frozen fixtures remain context-qualified. Test-data replica
provenance is recorded in `tests/fixtures/provenance.json`; no implementation is
copied from another package. All authored Rust is subject to workspace unsafe
forbid and frozen detection rules.
