---
layout: doc
title: Inspect a native integration plan
permalink: /tutorials/quickstart-monitor-mode.html
reviewed: 2026-11-07
---
# Inspect a native integration plan

**Diátaxis quadrant:** Tutorial. **Audience:** first-time evaluators.

This tutorial runs the implemented Rust planner without modifying your workspace.
It does not install a native SDK or activate protection. Use a reviewed checkout
and Rust 1.99.1; run from the repository root.

## Print a plan

```console
cargo run --locked -p humanymous-native-cli --bin humanymous-native -- init --framework electron --target x86_64-pc-windows-msvc --runtime-version 39.0.0 --dry-run
```

The runtime version is an example input token, not a supported Electron-version
claim. The target identifies the requested plan; it does not attest the host.
Dry run does not inspect or create a supplied workspace.

## Read the result

The JSON reports `operation: dry_run`, `publication: null`,
`integration_status: not_implemented`, `qualified: false` and no available
capabilities. These are expected limits, not a failed installation.

For Electron, the plan calls for a Rust Node-API addon in the trusted main
process, sandboxing and context isolation, with renderer native access disabled.
The CLI does not generate JavaScript glue or an addon, and does not start a
second CEF runtime. Other framework selections likewise remain plans.

Explicit `--write --workspace ABSOLUTE_PATH` can create a new manifest on the
currently implemented Windows path after path/reparse checks and directory
handle pinning. The writer does not verify workspace ownership or ACLs; a
trusted workspace without hostile same-user entry changes is a prerequisite.
Existing files are preserved; unsupported-platform writes are refused.
Publication and temporary-link cleanup have separate results: exit code 3 can
mean the destination was published but a temporary file remains. Inspect the
`publication` receipt before retrying; a nonzero exit is not proof of no writes.
No power-loss or directory-namespace durability is claimed. This tutorial
deliberately uses the read-only mode. Continue with the
[CLI reference](../reference/cli-config-policy.md) and [evaluator guide](../start-here/evaluator.md).
