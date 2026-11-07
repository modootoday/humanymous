# Compatibility transition and final retirement

The latest user instruction prioritizes obsolete generated-copy deletion. Root
owns the tracked vendor skill/rule/agent/lesson cleanup allowlist, retaining the
canonical counterparts and separately reviewing settings/hooks/untracked state.
Provider discovery is a required post-cleanup gate, not an indefinite deletion
blocker. No mirror regeneration is authorized. Historical prerequisites below
must be read with this accepted ordering change; actual runtime qualification
remains unproven until fresh source-bound evidence exists.

Canonical content now lives in `.agents/{rules,skills,agents}`. Root `skills/`
contains the separate product-user skill and Rust maintenance tools. User-root skills are external read-only inputs;
do not mass-copy, uninstall or overwrite them. The transition manifest is an
allowlisted plan, not a deletion executable or a claim of completed retirement.

1. Validate canonical paths, agent pointers, LF bytes and skill metadata using the
   Rust tool. Capture source hashes, mirror drift and actual evals.
2. Inventory actual references to each generated surface and identify its current
   consumer. A running chat alone is not evidence of a dependency or an indefinite
   retirement blocker. Do not run legacy sync: it recursively replaces mirrors.
   Migrate live consumers to canon and record the precise remaining dependency.
3. Root migrates legacy workflow/adapter consumers to canonical agent paths and
   supplies a qualified Rust adapter generator if a provider still requires a
   physical mirror. Test fresh discovery, compatibility paths, hooks and session
   coordination without global-user configuration changes.
4. Root fixes CI sync/verify jobs on the same candidate source, validates exact
   blob bytes and provider evals, and resolves evidenced consumer dependencies.
   Build a deletion candidate after replacement evidence, remove allowlisted
   obsolete outputs, then run the whole candidate gate on the cleaned tree.
   Do not require the obsolete Go shipping tree to pass before its approved
   replacement can be removed; that circular condition is not a transition gate.
   Actual deletion remains within the deleting owner's claimed paths; root owns
   the cross-owner cleanup candidate and final integration.
5. Preserve unchanged foreign oracle/hook/session tools until separately qualified
   replacement and product gates exist. Debt is not permission for new foreign
   implementation or script strings inside Rust.

No vendor directory, hook or active board/claim has been removed by M11. A
deterministic routing proxy is not model activation; the Windows CLI pilot is
not the upstream Linux bwrap plugin harness. Missing dependency or sandbox
rejection stays blocked/unsupported, never --isolation none.
