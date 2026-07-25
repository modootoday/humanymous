---
name: cut-release
description: >
  Use when cutting a SemVer release per SoT-37 (tag, notes, images). Manual publish
  only — do not push tags unless the user explicitly asks to publish.
disable-model-invocation: true
when_to_use: >
  "cut a release", "tag v0.", "SoT-37", "prepare release notes".
---

# Cut release

## Steps

1. CI green (go + detector-vs-bots as applicable).
2. Commits since last tag → MAJOR/MINOR/PATCH (pre-1.0 clause if `0.y.z`).
3. Detection policy breaks need `!` / BREAKING marker.
4. Follow `docs/how-to/cut-a-release.md`; annotated tag `vX.Y.Z`.
5. Release builds engine/gate only — never distribute red image.

Canon: `sots/37-versioning-convention.md`, `cliff.toml`.
