---
id: plan:root/20261107224000
title: Source triage and narrow controlled-selector scanner correction
status: draft
created: 2026-11-07T22:40:00Z
updated: 2026-11-07T22:40:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/delivery]
---

# Actual annotation and source context

Root/Delivery report original1f87 advisory job128552668346 Trivy filesystem
GoCardless MEDIUM hit at runner controlled.rs42. Scanner log already masks the
matched value. No masked value was reconstructed/copied, no provider endpoint
contacted and no candidate credential tested. Do not claim credential validity
from a scan or publish a potential secret value. Other Go findings/retirement
debt belong to separate owners; no scanner-policy change is authorized.

Read historical1f87d310492b2bf23f62395c97b90e766af5f84a source privately: the
reported line is a fixed lowercase Rust controlled-fixture selector, exactly
joined to a declared function and matching grant selector in Platform's
cfg(all(test,windows)) controlled_barrier actual-test module. Exactly TWO current
Rust files contained it. This closed profile/grant path uses it as a test name,
not credential issuance, provider authentication, key configuration or a request
header. Static fixture context explains the detector hit; no issued credential
usage is evidenced. Raw source/log history remains unchanged.

# Minimal authorized correction and custody

New selector leaf: parent_remains_running_until_original_outer_deadline.
Native owns ONLY runner controlled.rs profile selector plus its existing exact
unit assertion (two replacements). Root separately owns/applied Platform test
function declaration and literal grant selector (two replacements), no body/
case/outcome/unsafe change. Native does not stage Root's Platform file. After
readback, both new selectors match and old-name references in those current Rust
source trees are ZERO. Historical source/evidence/spec observations are not
rewritten or requalified. No scanner ignore/disable/exclusion/threshold relaxation.

Case ID remains outer-live-timeout; ExpectedOutcome remains LiveDeadlineTermination;
closed argument parsing, actual Job/Child ownership, original hard deadline,
failure classification and fixture body remain the same. The rename does not
turn an outer negative into an inner test pass or production/native Ready proof.

native-selector-scan-triage-source.json pins historical blob/annotation context,
owned runner before/after hashes and Root-owned current counterpart hash. Source
checkpoint joins two owners' edits; final coherent source requires Root's separate
Platform commit before a combined validation allocation. No compatibility alias
or acceptance of the previous selector is added.

# Unit contract and validation still pending

Updated existing exact test:
controlled::tests::fault_profiles_have_fixed_selectors_and_cannot_weaken_ordinary_outcomes.
No added/removed tests. Existing controlled test inventory remains FIVE:
fixed_selectors_refuse_unknown_or_duplicate_arguments;
fixed_parent_requires_absolute_executable_digest_and_new_work_locator;
cli_profile_has_one_literal_selector_and_specific_artifact_prefix;
executable_hash_names_cannot_cross_profiles_or_escape_fixed_shape;
fault_profiles_have_fixed_selectors_and_cannot_weaken_ordinary_outcomes.

Source formatter/whitespace and spec checks only at this checkpoint. Tests,
typecheck/Clippy, scanner rerun and actual fixture host execution are NOT_RUN.
Root may allocate the smallest fresh single exact unit plus scoped type/lint
checks after BOTH commits; no actual timeout/Job/process fixture is requested.
Future scanner evidence must bind actual new source and raw scanner result;
source rename alone is not a claimed full security scan pass or new full CI green.
