# Local Rust paired profile v1

The user's2026-11-08 request explicitly authorizes actual autonomous, no-paid-API
local model evaluation. This profile is distinct from the historical upstream
Codex/bwrap hosted harness; it does not bypass that harness, modify its accounting
or claim that its provider, sandbox, discovery or full M11 gates passed.

Runtime: pinned mistral.rs0.9.5 Windows CPU binary, official release archive
SHA25664d567c299a5c97ef265aa0362649aafb968af47706810df640f74fe8ea4702a.
Model: official Qwen/Qwen3-1.7B-GGUF revision90862c4b9d2787eaed51d12237eafdfe7c5f6077,
Qwen3-1.7B-Q8_0.gguf,1,834,426,016bytes,
SHA256061b54daade076b5d3362dac252678d17da8c68f07560be70818cace6590cb1a.
Matching official tokenizer/config revision70d244cc86ccca08cf5af4e1e306ecf908b1ad5e
is recorded separately. Downloads use public artifacts, no paid API/auth token.

The Rust driver in the existing maintenance crate accesses only the loopback
OpenAI-compatible endpoint and finite allowlisted source skill files. No shell,
host mutation, network tool, arbitrary path, credentials, installation or release
action is available to the model. Treatment receives only skill metadata and a
read_skill function; the model must request the body itself. Baseline has neither
catalog nor tools. Product catalog and developer catalog remain separate.

The product catalog contains the product SKILL.md plus its four references. The
developer catalog is explicitly limited to github-actions-ops and cut-release,
the two skills targeted by these five developer cases; this is not a test of the
full plugin inventory. Each of three product and five developer cases receives
two runs per arm,32 subject runs total. Run order reverses per repeat; matched
arms use greedy temperature0 without an RNG seed. Windows CPU rejects explicit
seeding (first startup failure retained); bitwise determinism is not asserted.
Tool rounds are capped at5,
calls per round4, subject output768tokens per call, per-case cooperative budget600s
and entire driver3600s. Exact model requests/responses, output, actual returned
skill-body hashes and observed usage remain in raw receipts. API prices are zero;
local electricity/time are not priced by this profile.

Existing behavior rubrics grade both arms. A separate call to the SAME local model
receives task, rubric and answer without explicit arm metadata and returns a
boolean plus evidence. This is a non-independent, imperfectly blinded small-model
judge, not externally validated quality truth. Missing/invalid grades stay null;
incomplete generation is not an automatic behavior failure or a passing result.
Read traces measure exposure, not adherence or causal benefit. Raw answers require
review alongside score deltas; no positive effect is assumed or manufactured.

Resource profile: Windows CPU,4threads, max_sequences1, context8192, paged attention
off, prefix cache0 and no GPU allocation. One model process plus one sequential
driver only. Fresh host admission requires commit<85%/free physical>20GiB; retain
the host emergency stop commit>=88% or free<16GiB and an8GiB model-process private
memory monitored stop budget, not an OS-enforced hard limit. Model initialization and any compiler work need coordinated distinct
physical targets/leases; no WSL/Docker/backend change or foreign process kill.
Only the verified owned process tree may be stopped on its budget/guard.

This is an actual local exposure/behavior experiment, not a claim that this1.7B
model represents larger coding agents. Final receipts must identify complete and
partial denominators, discovery/nonactivation, judge limitations, source/model
hashes, time/usage/resources, retained first failures and returned leases.

## Exploratory supplied-context follow-up

The first discovery-profile product treatments did not request a skill-body read.
Preserve that actual nonactivation result; it cannot establish applied-skill benefit.
The user's requested actual before/after comparison therefore also gets a separate
explicit supplied-context ablation, added after this preliminary observation.
It is exploratory, not a preregistered confirmatory result.

`local-skill-exposure-eval` repeats all32 subjects with fresh matched baseline
requests. Positive treatment receives the expected skill entry and needed product
references selected by the fixture's known target, in system context; baseline
receives none. Negative cases receive catalog metadata only, no skill bodies.
This is oracle-routed exposure, not model-selected activation. Context exposure is tagged supplied_context in
the exact file/hash records and is never relabeled a model-requested tool read or
automatic activation. No extra tools or host actions become available. Both
positive and negative cases retain the same tasks and behavior rubrics. Distinct
output directories, request logs and summaries preserve both experiments rather
than overwrite a failed discovery result. Judge limitations remain unchanged.
