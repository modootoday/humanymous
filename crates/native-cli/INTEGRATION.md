# Native scaffold consumer integration

Owner: browser-ui / rust-browser-native-lifecycle, explicitly expanded by root.
This package is the native scaffold/packaging consumer. Operational service CLI
crates/cli remains a separate owner. Manifest generation does not qualify an SDK,
framework adapter, sandbox, native input or automation blocking.

The library test_support module is compiled only for test+Windows because its
only library consumer is storage/windows/tests.rs through cfg(windows) storage.
Integration tests/command.rs declares its own path-based test_support module and
still compiles/runs on its supported platforms. No test, runtime permission or
publication gate was removed; no allow(dead_code) suppression was added.

This fixes the source cause reported by hosted run42348518353/source63f69ae:
Linux library-test Clippy flagged unused NEXT/Directory/new/count. That failure
remains historical. New Linux Clippy verification is not_run and must be supplied
by the hosted CI owner after root source closure. The current root graph window
forbids new local Cargo; no native-cli test/process was executed for this fix.

Contracts remain sot:root/native-embedding and plan:root/20261107042614. The
browser checkpoint change:root/20261107073933 records the ownership/source delta;
it does not replace those contracts. Shared Cargo/lock/toolchain and push/dispatch
belong to root; no dependencies or features changed here.
