# Evaluation boundaries

The legacy trigger dataset and rules are measured by the Rust `routing` command.
Its output is a regex intent proxy, not model activation or answer quality.
Rust-era cases add authority and WASM implementation requests plus a conceptual
near miss. The model pilot's cases and behavioral rubric are defined once in
`skills/agent-maintenance/src/evaluation.rs`; do not copy examples into skill bodies.

Model cases were defined before the pilot: two positive tasks (real authority
peer missing despite mock success; actual browser missing despite native units)
and one conceptual TLS explanation near miss. Positives require the pinned Rust
toolchain, locked Cargo verification and an actual boundary pilot. The near miss
must not invoke implementation. Both arms use identical behavioral graders;
claimed skill names are not proof of an observed skill read.

The modoo.today `codex-plugin-eval` is the required separate upstream plugin
measurement path. The original catalog bundle omitted shared imports. The full
package is now acquired at pinned commit
`1c4f32595d9cf3f07a393b8c561b9ab3b7f94981` in private evidence; acquisition
is not a runtime result. Retain its exact hash, provision bwrap without weakening
isolation, and report dependency failure separately. Do not fill missing harness
imports with ad-hoc mocks or claim a standalone Windows CLI pilot is that suite.

The recorded M11 pilot stopped at its observed token ceiling after two baseline
and one treatment answers. The treatment skill read was policy-blocked. Do not
compute a positive delta, tune the skill to the sandbox error, rerun around the
policy or classify missing answers as quality failures. A final qualified rerun
needs complete plugin provenance, two runs per arm for all three cases, recorded
model/runtime/usage, actual discovery/read evidence and a valid sandbox boundary.

The separate `product/` suite evaluates the product-user skill with two positive
cases and one negative, two runs per arm. The Rust `paired-check RESULT_JSON`
command checks its twelve-run denominator and complete behavioral grading against
the pinned upstream result shape. It refuses a delta for partial/missing/null or
contaminated results. This accounting never establishes source/runtime provenance
on its own, and synthetic unit fixtures are not model evaluation evidence.

Observed failure to read the target skill, or activating it for a negative case,
is a discovery outcome rather than missing run data. Such runs remain in the
behavior denominator and are reported in discovery_failures plus expected/actual
target-read counts. Missing read observations, baseline skill contamination or
cross-plugin reads invalidate accounting. A numeric arm difference alone is not
evidence that the skill caused improvement, especially when discovery failed.

The developer suite is isolated under `developer/`: two operations-guide cases,
two explicitly invoked release-guide cases and an unrelated negative, twenty
subject runs. `stage-eval ROOT NEW_OUTPUT developer` stages its own plugin;
`paired-check RESULT_JSON developer` enforces that distinct case/run denominator.
The later allocated batch passed36 pure tests, all-target check/Clippy and fmt;
developer-after.json binds those tested bytes. Hosted execution remains pending.
No current8132117 CI result qualifies these later working-tree edits.
