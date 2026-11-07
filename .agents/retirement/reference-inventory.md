# Observed retirement consumers

This is a source-reference inventory, not proof of runtime use or permission to
delete outside the owner's claim. Observed on the current 2026-11-07 worktree.

| Surface | Concrete consumer | Required transition |
|---|---|---|
| Vendor skill mirrors | `.github/workflows/frozen-go-oracle.yml` agent layout step executes sync, mirror diff and legacy verify | CI owner retires this consumer on the cleaned candidate and uses source-bound Rust checks |
| Persona pointers | `scripts/agents/sync-adapters.sh` enumerates `.agents/personas`; legacy verify requires six files | Root retires generator/validator consumers; preserve named canonical roles |
| Persona pointers | `skills/agent-maintenance/src/inventory.rs` previously required six pointers | New source requires six canonical roles and validates compatibility pointers only when present; tests pending |
| Vendor sync | root `CLAUDE.md` instructs recursive sync; `GEMINI.md` describes mirrored discovery | Root updates vendor entry points before final removal |
| Mirror and Go-pointer registration | `.agents/manifest.yaml`, provider matrix and retirement manifest | Update projections with the same qualified removal candidate |
| Workflow role names | `.agents/workflows/feature-loop.yaml` and `design-panel.yaml` use a `personas` field | Role-name fields alone do not prove filesystem pointer use; inspect actual workflow runtime before removal |

Historical lessons mentioning old sync are recorded history, not instructions to
run it now. No script/workflow/vendor tree was edited or deleted by this inventory.
Running chats are not treated as an indefinite dependency without concrete evidence.
Fresh provider discovery and actual paired evaluation remain outstanding.
