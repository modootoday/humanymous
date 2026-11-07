# Rust implementation and qualification

All newly authored project prose and source comments/docstrings use English.
This includes internal SoTs, plans, change notes, evidence summaries, authored
private notes, skills, README files and workflow labels. Preserve immutable raw
evidence, third-party quotations, literal UI titles and intentional multilingual
test data. Unicode is not a language violation. Existing non-English authored
prose is tracked debt, not a blanket scope exemption or permission to alter
recorded bytes and hashes.

The Rust `language-check SOURCE PRESERVATION_JSON` command reports bounded
language indicators in Markdown prose and Rust comment-only lines. Preservation
records bind exact path and line hash to a reason/category; no scope-wide waiver.
It deliberately does not reject Unicode generally or claim complete language
identification. Inline comments, fenced examples, workflow formats and languages
outside its indicator ranges still require review. Missing findings are not proof
of English conformance. Classify existing findings as debt before owner fixes.

The controlling convention is `.spec/sot/rust-migration.sot.md` and current status
is `.spec/sot/program-roadmap.sot.md`. Use Rust 1.99.1 / edition 2024 for all new
authored implementation and tests, including maintenance tools. Do not hide JS,
Python, shell, PS or other executable implementations inside Rust strings or
generated templates. Upstream generated glue and unchanged test-only oracles
require provenance and explicit retirement conditions, not a foreign-code waiver.

The user's explicit ASP.NET Core exception is limited to three authored C# files:
`adapters/dotnet/src/Humanymous.Native/NativeBinding.cs`,
`adapters/dotnet/src/Humanymous.Native/HumanymousMiddleware.cs`, and
`adapters/dotnet/pilot/Program.cs`. They provide ABI binding, middleware and the
real host pilot only. The controlling accepted decision is the managed-host
exception in `.spec/sot/rust-migration.sot.md`; this is not a general foreign-code
waiver. Scoring, cryptography, policy, credentials and authority stay in Rust.
Extra C# files, executable templates or generated foreign implementations require
a separate explicit decision. Hosted Kestrel/JIT/NativeAOT/GC and installation
qualification remain required, and comments/prose remain English.

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
