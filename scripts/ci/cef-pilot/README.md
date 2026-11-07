# CEF native component runner

`--configure-toolchain` writes only the discovered Visual Studio CMake/Ninja
directories to the existing GitHub Actions GITHUB_PATH file for subsequent
workspace compilation. It requires a provisioned Windows toolchain and never
installs or executes an embedded shell script.

Run from the repository root on Windows:

`cargo run --manifest-path scripts/ci/cef-pilot/Cargo.toml --locked -- target/cef-native-evidence`

The Rust runner finds Visual Studio CMake/Ninja without embedded shell code,
builds both root-workspace CEF DLLs with `--locked`, locates the exact pinned
upstream application manifest through Cargo metadata and provisions the renamed
upstream bootstrap in Cargo's generated debug directory. Existing mismatched
artifacts are refused; no profile or personal browser directory is supplied.

It explicitly executes the required `native-pilot-tests` feature case with
`HUMANYMOUS_CEF_PILOT_BUNDLE` and requires exactly one successful, non-ignored
test receipt. That test checks four security override rejections before UI or
profile creation. The ordinary workspace test harness has no ignored pilot.
This runner is a mandatory separate native component gate, not permission to
remove/skip the pilot or relax standard-user native accounting.

The JSON receipt and raw build/test logs are bound to the workflow's source SHA;
receipt includes actual DLL/runtime/executable hashes.
The runner compares native/tool source hashes before and after execution, records
dirty local snapshots honestly, and rejects dirty or mismatched-SHA CI checkouts.
An unknown command-line option is refused before creating an output directory.
It does not qualify GUI,
keyboard/IME/AX, full sandbox policies, human behavior or product e2e. Those gates
remain required. This runner supports Windows x64 only at this stage; non-Windows
execution fails rather than manufacturing a successful empty harness.
