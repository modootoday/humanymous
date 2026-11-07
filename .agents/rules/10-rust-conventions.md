# Rust implementation and qualification

The controlling convention is `.spec/sot/rust-migration.sot.md` and current status
is `.spec/sot/program-roadmap.sot.md`. Use Rust 1.99.1 / edition 2024 for all new
authored implementation and tests, including maintenance tools. Do not hide JS,
Python, shell, PS or other executable implementations inside Rust strings or
generated templates. Upstream generated glue and unchanged test-only oracles
require provenance and explicit retirement conditions, not a foreign-code waiver.

Use standard Cargo layout and humanymous-* package names. Keep bounded-context
responsibilities together, separate pure decisions from I/O, and reuse scoring,
canonicalization and security primitives instead of reimplementing them in adapters.
Composition roots wire services. No forwarding-only fragmentation or catch-all modules.

Run locked scoped unit tests, fmt, Clippy and the actual boundary pilot. Full
integration uses workspace gates. Product e2e is Docker-only; native UI/service
pilots are distinct and require the real supported OS/arch. Preserve failures,
unsupported and ignored cases, mock results and source identity. Do not disable
sandboxing to make a failing browser or evaluation pass. No component green means
full product green. Detection freeze, fail-closed, dual-control, motor/speed-free
a11y, owned lab targeting and the T4 ceiling remain mandatory.
