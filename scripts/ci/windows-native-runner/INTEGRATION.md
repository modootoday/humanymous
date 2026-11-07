# Native runner integration

Rust1.99.1/edition2024 independently locked tool; root Cargo is unchanged.
Build: `cargo build --locked --manifest-path scripts/ci/windows-native-runner/Cargo.toml`.

From the repository root, invoke the built executable with literal arguments:

```
humanymous-native-runner native --report ABSOLUTE_NEW_REPORT --binaries ABSOLUTE_ROOT/target/debug
humanymous-native-runner standard-user --report ABSOLUTE_NEW_REPORT --binaries ABSOLUTE_ROOT/target/release
```

The report parent must already exist. `native` works on Windows/Linux/macOS with
an actual non-elevated installer token/non-root UID. `standard-user` is Windows
only and refuses before provisioning unless GitHub-hosted and elevated. Local
nonadministrator machines never create accounts. Report paths are not secrets.

`--binaries .../release` builds/tests the release profile; other directories use
debug. Paths and SHA256 must equal actual Cargo JSON product artifacts, so a
prebuilt/stale release binary cannot be relabeled. Prepare compiles and executes
all harnesses for runtime, authority, CLI and native-security only, including the
named actual-process integration assertions. This is not the all-workspace probe
replacement and does not waive the legacy all-harness requirement. The parent
standard-user mode prepares first, then creates a Users-only local account,
uses native CreateProcessWithLogonW suspended launch into a kill-on-close job,
clears the environment to an explicit minimal block, checks actual SID/token
and workspace write denial, and deletes the account/scratch before reporting.
Its privileged execution still requires actual disposable hosted qualification.
The narrowly allowed FFI lives only in this CI tool, with ownership/pointer
invariants documented in actor.rs and cross-os-runners SoT. No shell source.

Schema1, kind=native, current source SHA, reviewed-document digest, source-file
manifest, actual OS/arch/token, Cargo manifests, per-harness SHA/counts/exit and
real CLI snapshot checks are emitted. Source changes during execution fail.
Product algorithms/signing/scoring are not copied: source-bound integration
harnesses execute their existing actual native children and shared APIs.

Exit2 means component execution completed but full delivery is blocked. The
report explicitly misses local-only-admin and documentation-examples, with
documentation_examples_executed=false and full_runtime_qualified=false. Exit1
means infrastructure/assertion/refusal error; unknown/ignored/filtered tests fail.
No path returns full-qualified exit0 yet. Raw build stderr/messages use the report
stem with `.build-stderr.log`, `.build-messages.jsonl`, `.tests-stderr.log` and
`.tests-messages.jsonl`; retain privately. Credential-bearing runtime stdout is
parsed in memory and never copied to report/debug output. The document hash is
an identity reference, never a claim that reviewed examples executed.

The completed local Windows debug source snapshot ran10harnesses/51tests,
zeroignored, with required runtime/authority native-process assertions. It remains
blocked and dirty-source scoped. Linux/macOS and hosted account/job execution
are not yet observed; request exact-source Actions through root/CI owner. OS
service activation belongs to CLI; browser, positive raw TLS/H2, actual remote
machine peer denial and Docker product qualification remain separate blockers.

No push/tag/publish/main changes are performed by this lane.
