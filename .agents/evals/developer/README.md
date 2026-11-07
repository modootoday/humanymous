# Developer guide paired evaluation

This suite is separate from the product-user plugin. It covers two GitHub Actions
operations cases, two explicitly user-invoked release preparation cases and one
unrelated negative. Two runs per with/without arm require twenty subject runs.
Behavior grading applies to both arms; skill discovery is a zero-weight indicator.
Release cases explicitly invoke the skill because it disables automatic model
invocation. No case authorizes publication, a remote message or a repository edit.

Use the pinned complete upstream prompt.md/graders harness and preserve its
runtime lock, sandbox, source bindings, attempts and full denominators. The current
Rust `paired-check RESULT_JSON developer` command has a separate twenty-run
profile; product data cannot satisfy it. Staging now includes this suite only in
the developer plugin. The subsequent allocated batch passed36 pure tests,
all-target compile/Clippy and fmt with unchanged before/after source hashes.
Actual hosted packaging and model execution remain pending. Source fixtures are not
observed activation or measured improvement.

Accounting keeps observed nonactivation/misrouting separate from missing data:
discovery_failures and target-read counts retain those failures without dropping
behavior runs. A complete accounting result is not a passing discovery result or
proof that a numeric arm difference was caused by the skill.

The same source tasks have now been executed in the separate user-authorized
Windows CPU local Rust profile:20 subjects in automatic selection and20 in
supplied-context comparison. See `.agents/evals/local-rust-paired-results.md` for
actual repeated-read/truncation failures and judge unreliability. Original hosted
Codex/provider qualification remains open; no benefit delta is inferred.
