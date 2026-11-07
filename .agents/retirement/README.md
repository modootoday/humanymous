# Compatibility transition and final retirement

Canonical content now lives in `.agents/{rules,skills,agents}`. Root `skills/`
contains Rust maintenance tools. User-root skills are external read-only inputs;
do not mass-copy, uninstall or overwrite them. The transition manifest is an
allowlisted plan, not a deletion executable or a claim of completed retirement.

1. Validate canonical paths, agent pointers, LF bytes and skill metadata using the
   Rust tool. Capture source hashes, mirror drift and actual evals.
2. Existing chats retain discovered vendor skill paths. Preserve generated files
   while chats run. Do not run legacy sync: it recursively replaces mirror contents.
3. Root migrates legacy workflow/adapter consumers to canonical agent paths and
   supplies a qualified Rust adapter generator if a provider still requires a
   physical mirror. Test fresh discovery, compatibility paths, hooks and session
   coordination without global-user configuration changes.
4. Root fixes CI sync/verify jobs on the same candidate source, validates exact
   blob bytes and provider evals, and confirms no active session depends on the
   retired surface. Only then remove manifest-listed obsolete outputs.
5. Preserve unchanged foreign oracle/hook/session tools until separately qualified
   replacement and product gates exist. Debt is not permission for new foreign
   implementation or script strings inside Rust.

No vendor directory, hook or active board/claim has been removed by M11. A
deterministic routing proxy is not model activation; the Windows CLI pilot is
not the upstream Linux bwrap plugin harness. Missing dependency or sandbox
rejection stays blocked/unsupported, never --isolation none.
