---
id: evidence:root/20261116060831
title: Workspace gates and Cap0 re-measure after canonization
status: active
created: 2026-11-16T06:08:31Z
updated: 2026-11-16T06:08:31Z
rel:
  depends_on: [plan:root/20261116060213, sot:root/milestones]
---

# Workspace gates and Cap0 re-measure after canonization

## What was measured

Same Linux cloud container and Rust 1.99.1 as [[evidence:root/20261116055601]].
Measured on `codex/rust` after the lane-count fix, the rustfmt pass and the
lab-runner Clippy fix. In this run the Docker daemon was started locally (29.9.2,
overlayfs). It was used for nothing beyond Cap0 status probes; no Cap=1 product
run happened.

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass (exit 0) after the rustfmt commit |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | pass (exit 0) after fixing 5 lints in `test/lab/runner` (`cap0_chain.rs`, `until_complete_status.rs`); before the fix, exit 101 |
| `cargo test --workspace --locked --no-fail-fast` (uid 0) | 112 binaries; 1383 passed, 13 failed, 27 ignored; exit 101 |
| `humanymous-cli --lib` as an unprivileged user (offline copy of tracked files) | 15 passed, 0 failed. The 10 uid-0 failures are the intended non-root fail-closed refusal |
| `humanymous-agent-maintenance --test git_coord` | fails: `pwsh` absent (environmental) |
| `humanymous-lab-runner` bin tests | 169 passed, 1 failed: `unlock_a_cap_exit0_binds_into_n06_readiness_cap0` requires an untracked private `.spec.local` closer. The test is not hermetic |
| `humanymous-runtime --lib` `gateway::tests::tls` | flaky, 1 failure in 5 isolated reruns (`actual_tls_coalesced_request_is_refused_before_audit_or_origin`) |
| `spec check --no-local` | 0 errors, 0 warnings |
| `lab-runner until-complete-status` | `status=incompletes_remain`, product 0, `pass_product_clean_room` 0, `ready_for_cap1=false`. Open: AT-HANDOFF, AUTH-HANDOFF, Cap-free-restore-GRANT, BrowserReady, pass_product_clean_room, docker_clean_room_full_matrix_green, red_blue_validate_product, docker_e2e_validate_product, Unlock-C, apps_humanymous_browser |
| `lab-runner cap0-chain --inspect-only` | 4 blockers absent (AT-HANDOFF, AUTH-HANDOFF, Cap-free-GRANT, TRANSFER manifest); `cap_free=false`, `cap_burn=0` |

### TLS drain finding (source observation)

In `serve_tls_until` (`crates/sdk/humanymous-runtime/src/gateway/application/tls.rs`),
when shutdown arrives while a connection is still pending, the drain branch
propagates that connection's own refusal error with `??`. The server then
returns `Err("TLS complete request framing failed")`. The plaintext path
(`gateway/application.rs`) maps any finished drain to `Ok(())` and fails only
on drain or deadline timeout. The coalesced-request test sends `stop` right
after the client sees the closed connection. So it races the server's own
completion of the refused connection. Which side is wrong (code or test) is a
contract decision under [[sot:root/runtime-application]] ("drain errors never
become successful Stopped"). The owning lane (`cursor-integration`, `crates/sdk/**`)
must make that decision; no source change was made here.

## How to read it

Host and component evidence only. No milestone exit gate is met by it. It shows
that format and lint gates are green on this source. The remaining test
failures are 10 environmental (uid 0), 1 environmental (`pwsh`), 1 non-hermetic
private-state test and 1 timing race. The Cap0 board is unchanged:
`goal_complete=false`.
