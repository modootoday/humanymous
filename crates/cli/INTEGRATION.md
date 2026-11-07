# CLI integration

Binary: humanymous. Workspace metadata/lints, Rust1.99.1, publish=false.
Normal dependencies: sha2 and humanymous-native-security. Windows test-only
fixtures use pinned windows-permissions and junction. No authored shell source
or runtime helper executable remains in this crate. Native identity, drive and
handle privacy are shared rather than copying audit cryptography or ACL parsers.

Commands: doctor|install|rollback --artifact ABSOLUTE_PATH --destination
ABSOLUTE_PATH --sha256 LOWERCASE_DIGEST --local-management-verified [--dry-run].
The acknowledgement is an operator precondition, not runtime/IPC qualification.
Existing private user-owned parents are required; Unix requires mode0700and
non-root UID. Windows requires a fixed drive, no impersonated/elevated/enabled-
administrator token and the explicit installer DACL policy. SYSTEM/Administrators
exceptions never apply to strict audit/custody profiles. No permission repair.

Implementation and tests are contextualized under src/installation. Windows
CLI6and native-security6tests passed. The isolated Linux Docker lab executes the
real CLI for doctor, dry-run snapshots, install/repeat/rollback, bad digest,
unwritable parent and mutated-file refusal. These are bounded provisioning
results: no service activation, artifact execution, elevation, signed-release
provenance or full Core/Gate installation is claimed.

Run cargo test -p humanymous-cli -p humanymous-native-security --locked and
warnings-denied Clippy. The exact updated source must be qualified on all six
native tuples. Norms: sot:root/cli-installation and sot:root/native-security.
