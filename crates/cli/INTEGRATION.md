Root integrated crates/cli membership while this slice was being verified. It
inherits existing sha2; no new dependency is required. Binary: humanymous.
Provisioning only; no Core/Gate runtime or system service is fabricated.

Commands: doctor|install|rollback --artifact ABSOLUTE_PATH --destination
ABSOLUTE_PATH --sha256 LOWERCASE_DIGEST --local-management-verified [--dry-run].
The acknowledgement means the operator independently checked the supplied
artifact's local management and IPC ACL behavior; CLI does not establish that.
Parent must already be private and user-owned. No service activation/elevation.

Actual workspace cargo test -p humanymous-cli --locked (6 tests) and cargo clippy
-p humanymous-cli --all-targets --locked -- -D warnings passed on Windows.
Root still owns full workspace/release and Linux/macOS acceptance gates. See CLI
SoT and plan for exact prerequisites, transaction assumptions and residuals.
