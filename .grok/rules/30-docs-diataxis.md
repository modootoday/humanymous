# Documentation rules (user-facing)

- Public docs live under `docs/` and follow Diátaxis + SoT-29.
- Every page: first lines declare quadrant + audience when following the house style.
- **Do not** surface internal `SoT-NN` identifiers on reader-facing pages (maintainer derivation index is the exception).
- User docs **derive** from SoT; they are not the raw Korean/internal specs.
- Enforcement docs and blocked-user guidance ship together in spirit (false-positive path is first-class).
- After engine behavior changes that affect operators, update derivation targets in `docs/reference/versioning-derivation-index.md` when that file is in tree.
