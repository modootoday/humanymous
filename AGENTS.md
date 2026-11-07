# AGENTS.md — humanymous

Cross-tool always-on rules (Claude Code · Grok Build · OpenAI Codex · Gemini CLI).
This file is the provider-neutral project entry point. Keep it **short** — details load on demand.

## Mission

Raise the cost of automation. Apache-2.0 **reference** build: honesty over over-claim. T4 (coherent real browser + human pace) is an intentional ceiling.

## Hard constraints

1. **Detection freeze** unless the user explicitly changes scoring (weights, thresholds, hard-rule predicates, verdict bands).
2. **Fail-closed** enforcement when required signals are missing; dual-control for erasure / kill switch / permanent bans.
3. **A11y**: never gate Pass/CHALLENGE on motor richness or speed.
4. **Defense-only**: red catalog targets this deployment only — no third-party probing.
5. **No folklore**: every shipped tell/rule traces to a SoT (or a plan delta that becomes SoT).

## Progressive disclosure (read more only when needed)

Rust migration canon: `.spec/sot/rust-migration.sot.md`; execution and evidence
live in `.spec/` and private `.spec.local/`. Before editing, run `spec context <path>`;
before committing, run `spec check`. Rust conventions in that SoT are mandatory.

| If you are working on… | Read next |
|------------------------|-----------|
| Rust layout / scoring / signals | `.agents/rules/10-rust-conventions.md` + owning crate and `.spec/sot/` |
| Runtime, edge proxy, audit, admin | `crates/runtime`, `crates/authority`, `crates/audit` + owning SoT |
| Public docs (Diátaxis) | `docs/AGENTS.md` |
| Attack catalog / e2e | `test/AGENTS.md` |
| Rust client WASM | `crates/collector` + `.spec/sot/client-collector.sot.md`; `web/` is legacy oracle |
| Spec / design before code | skill `design-sot` → `.spec/sot/`, `.spec/plan/` |
| Deferred / misdocumented claims | `sots/38-truth-debt-remediation.md` if present + `.agents/rules/80-truth-debt.md` |
| Hard-won multi-session lessons | `.agents/lessons/HARD-WON.md` (Claude memory is **not** visible to other providers) |
| Full verification | skill `red-blue-validate` (**Docker only**) |
| Local Docker, WSL Docker or act | `.agents/rules/96-docker-engine-affinity.md`; verify the intended engine before execution or image cleanup |
| Multi-round red/blue wargame (attack→e2e→guard) | skill `red-blue-wargame-round` + rule `61-redblue-wargame` |
| Pass challenge round only | skill `pass-wargame-round` (SoT-36) |
| Multi-perspective panel | `.agents/agents/` + `.agents/workflows/design-panel.yaml` |
| Prior LLM sessions on this machine | skill `survey-provider-history` |
| SemVer tag / ghcr release / release.yml failure | skill `cut-release` (user-invoked publish) or `github-actions-ops` + rule `93` + `docs/how-to/cut-a-release.md` |

Current canon: `.spec/sot/`, `.spec/plan/`; raw evidence is private `.spec.local/`.
Old `sots/`, `plan/`, Go and foreign-language harnesses are historical oracles,
not instructions to author new non-Rust code. Public method: `docs/explanation/how-this-was-built.md`.

## Done means

| Scope | Gate |
|-------|------|
| Authored implementation/tests | Rust **1.99.1 / edition 2024** only; no executable foreign script strings or generated foreign templates as a workaround |
| Rust changes | scoped `cargo test --locked`, fmt and Clippy; integration candidate uses workspace gates in Rust migration SoT |
| Rust WASM collector | `cargo check --locked -p humanymous-collector --target wasm32-unknown-unknown` plus actual isolated browser pilot |
| **Product e2e** | **Docker only**; skill `red-blue-validate`; report exact source/image/topology and actual runner, never a stub as full product success |
| Native tools/UI | real OS/process/GUI pilot in addition to unit tests; unsupported, ignored, mock and component results remain separate |

Host/loopback `node test/e2e/runner.mjs` or host `node test/gate/e2e.mjs` is **not** completion authority (misses L5 topology / lab isolation). See `.agents/rules/60-e2e-docker-only.md`.

## Skills & agents

- Canon: `.agents/skills/`, `.agents/agents/`, `.agents/rules/`; repository-root `skills/SKILL.md` is the separate product-user skill, alongside Rust maintenance/evaluation tooling.
- `.agents/personas/` and generated vendor mirrors are compatibility surfaces pending retirement. Follow `.agents/retirement/README.md` and its concrete consumer inventory; running chats alone are not an indefinite blocker. Do not run legacy recursive sync. Remove only owner-authorized candidates after consumer migration and required discovery evidence.
- Verify canon: `cargo run --locked --manifest-path skills/agent-maintenance/Cargo.toml -- check .`
- Preserve session coordination scripts until their Rust replacement and compatibility gates are qualified; their use is not permission for new foreign implementation.
- Workspace Cargo/toolchain and whole roadmap edits belong to root integration. Keep public evidence sanitized and source-bound; observe LF in worktree **and Git blob** bytes.

Cloudflare: use `cf` CLI unless the target project has Wrangler configuration.

## Multi-provider / nested sessions (overlap)

Several LLM chats may run on this repo **at once**. Before non-trivial edits:

1. `pwsh -File scripts/agents/session-board.ps1 list` (or `bash scripts/agents/session-board.sh list`)
2. **Claim a work lane** — skill `coordinate-sessions` / `.agents/sessions/LANES.md`
3. Work only under claimed paths
4. **Git writes are serial + attributed** — before `add`/`commit`/`pull`/`push`/`stash`/branch switch:

   ```powershell
   pwsh -File scripts/agents/git-coord.ps1 preflight
   pwsh -File scripts/agents/git-coord.ps1 claim -Provider <claude|grok|codex|gemini>
   git add <only-your-paths>
   pwsh -File scripts/agents/git-coord.ps1 commit -Provider <claude|grok|codex|gemini> `
     -Subject "feat(scope): summary" -Body "optional why…"
   # injects vendor GitHub-linked Co-Authored-By (avatars):
   #   claude → Claude <noreply@anthropic.com>
   #   grok   → Grok <grok@x.ai>  (community de-facto; avatar may be absent)
   #   codex  → codex <codex@openai.com>
   pwsh -File scripts/agents/git-coord.ps1 release -Note "…"
   ```

   Commit message canon: `.agents/sessions/COMMIT-CONVENTION.md` (rule `92`).
   Use only identities from the canonical registry; prefer its GitHub-linked
   identity where one exists, and never invent an email to force an avatar.

5. Release the **work lane** when done or switching providers; incomplete → board handover + `handover-pack`

Same work lane = one writer. Different work lanes may dirty the tree in parallel;  
**`git-ops` is a global mutex** for the index/HEAD/remote.  
`detection-core` ↔ `gate-edge` mutex on scoring seams.  
Details: `.agents/rules/90-session-overlap.md`, `91-git-contention.md`,  
`.agents/sessions/PROTOCOL.md`, `GIT-PROTOCOL.md`.

## Ambiguity

If requirements conflict or scope is unclear: **stop and ask** before large edits (agents default to silent assumption otherwise).

## Cross-provider note

Claude Code may have rich `~/.claude/.../memory/*.md` for this repo. **Grok, Codex, and Gemini do not load that tree.** Prefer `.agents/lessons/` and the **session board** (`.agents/sessions/ACTIVE.md`) for anything that must survive provider switches or stacked chats.
