# Observed retirement consumers

This is a source-reference inventory, not proof of runtime use or permission to
delete outside the owner's claim. Observed on the current 2026-11-07 worktree.

| Surface | Concrete consumer | Required transition |
|---|---|---|
| Vendor skill mirrors | No tracked files returned by `git ls-files` for the four vendor skill directories | Root cleanup is present in this worktree; fresh discovery remains unqualified |
| Former CI sync consumer | `.github/workflows/frozen-go-oracle.yml` now runs Rust canonical `check` and all-target `cargo check` | Prior sync/mirror-diff/legacy-verify entry is historical; actual runtime tests are a separate gate |
| Former generators and validators | Neither sync-adapters nor verify-agents-layout shell/PowerShell files remain tracked or present in the inspected scripts directory | Do not recreate these consumers or run historical recipes |
| Persona pointers | Six `.agents/personas/*.md` compatibility files remain; Rust inventory requires six canonical roles and validates pointers only when present | Actual pointer-free process test exists but is unqualified on current source; preserve named canonical roles |
| Former vendor entry points | Root `CLAUDE.md` and `GEMINI.md` are absent and untracked | Prior mirror instructions are historical; absence is not provider discovery evidence |
| Canonical registration | `.agents/manifest.yaml` version6 and rule50 point providers at canonical paths with discovery_qualified=false | Keep explicit unqualified discovery state until observed traces exist |
| Go convention pointer | `.agents/rules/10-go-conventions.md` remains tracked and is still a retirement candidate | Inspect remaining references before owner-authorized removal; it is absent from manifest's active rule list |
| Workflow role names | `.agents/workflows/feature-loop.yaml` and `design-panel.yaml` retain `personas` fields; the inspected workflow-runner source has no literal personas or canonical-role path match | This bounded text search does not establish runtime role resolution; role names alone do not prove pointer-file use |

Historical lessons mentioning old sync are recorded history, not instructions to
run it now. No script/workflow/vendor tree was edited or deleted by this inventory.
These observations refresh the earlier pre-cleanup table; they describe inspected
worktree files rather than a committed candidate or a passing runtime result.
Running chats are not treated as an indefinite dependency without concrete evidence.
Fresh provider discovery and actual paired evaluation remain outstanding.

Legacy Go core retirement is inventoried separately in `legacy-go-consumers.md`
and the exact336-path/hash `legacy-go-inventory.json`. Those proposals preserve
the Rust Go-origin adapter and external upstream fixture. No foreign source was
deleted; root/path owners execute independently reviewed removal batches.
The separate `legacy-go-owner-routing.json` preserves all314 baseline rows and
routes owner review; it is not an authoritative parity-status replacement.
