# Product skill paired evaluation

This suite targets the pinned upstream codex-plugin-eval prompt.md/graders format.
It has two positive cases and one negative case. Each requires two runs in each
with/without arm: twelve subject runs in total, plus behavioral judge calls.
Behavior graders apply to both arms. Discovery graders have zero weight and
remain separate activation indicators; a file-read observation is not adherence.

Execute only on the approved hosted Linux/bwrap runtime with pinned complete
upstream package, runtime lock, explicit model/auth and approved token allowance.
No scaffold, hook bypass or sandbox fallback is part of this suite. Preserve raw
attempts, case/arm/run denominators, partial reasons, nullable grades, costs and
source hashes. Exit zero alone does not establish a complete paired result.
These source fixtures have not been executed and imply no measured improvement.

Stage under the product plugin's evals directory; do not load internal developer
skills into either arm. Keep active provider mirrors and user-global config intact.

The same source tasks have now been executed in the separate user-authorized
Windows CPU local Rust profile:12 subjects in automatic selection and12 in
supplied-context comparison. See `.agents/evals/local-rust-paired-results.md` for
actual denominators, nonactivation and judge unreliability. This does not satisfy
the original upstream Codex/bwrap hosted contract or establish improvement.
