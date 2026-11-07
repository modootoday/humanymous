# Canonical provider inputs and discovery evidence

All providers use one project source: root/nested `AGENTS.md`, `.agents/rules`,
`.agents/skills`, `.agents/agents` and `.agents/lessons`. Product-user instructions
live separately in `skills/SKILL.md`. Do not regenerate vendor mirrors or infer
automatic discovery from this intended layout.

| Input | Canonical path | Verification boundary |
|---|---|---|
| Project rules | root/nested `AGENTS.md` | Actual provider context and selected instructions |
| Developer skills | `.agents/skills/*/SKILL.md` | Fresh discovery/read traces and paired behavioral evaluation |
| Review roles | `.agents/agents/*.md` | Read canonical files; perspective does not authorize spawning |
| Durable lessons | `.agents/lessons` | Preserve history and promote portable lessons through the owning lane |
| Product skill | `skills/SKILL.md` and references | Isolated product evaluation without internal developer skills |
| Attribution | `.agents/sessions/COMMIT-CONVENTION.md` | Canonical provider identity; never invent an address |

Generated vendor skill/rule/agent/lesson copies are being removed by the root
cleanup owner. Their absence is accepted by the Rust inventory. Optional surviving
compatibility files are inspected for unsafe links and drift, never recreated.
Non-generated settings, hooks and untracked local state remain separately owned;
their presence does not qualify discovery or authorize deletion.

Fresh provider discovery after cleanup is not yet qualified. Preserve observed
paths/hashes, runtime versions, denied/missing reads and both behavior arms.
An explicit file read, declared skill name or static routing match alone cannot
establish automatic activation or quality improvement. Keep the environment's
actual trust, sandbox and approval policy; do not change it to force a passing eval.
