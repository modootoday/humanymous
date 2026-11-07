# Legacy Go compatibility pointer

Current implementation rules are in `10-rust-conventions.md` and
`.spec/sot/rust-migration.sot.md`. Go source is a read-only differential oracle
until the parity/foreign-source retirement gate. Existing Go tests may measure
that oracle but never substitute for Rust tests or authorize new Go code.
Keep this filename while old adapters reference it; removal is a root integration
step after the retirement manifest and compatibility checks pass.
