# Installation and signed service planning

Read the installed release's matching operator documentation and verify the
artifact bytes against an independently trusted digest/key. Do not bootstrap trust
from a key shipped beside an untrusted artifact. Pin an exact OS/architecture and
verify the qualification receipt before making a support claim.

The current reference CLI binary is `humanymous`. Its provisioning-only commands
are `doctor`, `install` and `rollback`, with `--artifact`, `--destination`,
`--sha256`, `--local-management-verified` and optional `--dry-run`. `doctor` is
read-only; `install --dry-run` is a plan. `--local-management-verified` is an
operator acknowledgement, not authentication proof. In this boundary `rollback`
removes the verified managed installation; do not call it a version downgrade.
Do not suggest `--help` or `--version` without checking that release: the inspected
reference parser does not implement those generic flags.

For the separate signed-service boundary, `service-plan`, `service-install` and
`service-upgrade` take `--manifest`, `--signature`, `--trust-key`, `--artifact` and
`--destination`. Signature and public trust key are lowercase hex (128/64 chars).
The manifest is the exact signed UTF-8 `humanymous.release.v1` descriptor; unknown
or duplicate members, wrong host/digest/signature and replayed sequence fail closed.
Do not reformat signed bytes. Do not generate a key or sign a descriptor on behalf
of an operator merely to satisfy a missing trust prerequisite.

Plan first. Service mutations require an explicitly authorized `--activate`;
`--activate` and `--dry-run` are mutually exclusive. Destination must be an owned
private root; names are restricted to `humanymous-lab-` in this reference boundary.
No automatic elevation, new host users, autostart enablement or adoption of an
unrelated service is supported. A service artifact must implement the declared
native service protocol; an ordinary stdin runtime cannot be relabeled as one.

Use a reviewable plan identifying old/new signed generations, sequence, artifact
hash, native service identity, permissions and readiness criteria. Do not mutate
if state is unknown, the lock/journal is stale or trust/credentials are missing.
Retain the failure and pending record for explicit recovery; do not delete it to
force installation. A fixture returning `product_qualified=false` remains a fixture.

For current source details, consult the matching release of the project's
[CLI reference](https://github.com/modootoday/humanymous/tree/main/crates/cli).
Source documentation must not override the bytes and capabilities actually installed.
