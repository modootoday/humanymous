# Red/blue sequential wargame — protocol digest

Agent canon: rule `61-redblue-wargame` + skill `red-blue-wargame-round`.  
Copy or link this digest from a private series dir under `plan/<series-id>/` if useful.

## Shape

```text
RED attack artifact → Docker MEASURE → SCRATCH evidence + LEDGER → BLUE disposition
```

**Default: no git commits during the series.** Formalize with `git-coord` only after user OK.

## Planes

| Plane | Red | Docker |
|-------|-----|--------|
| Core | `test/redteam` (+ `cmd/redteam`) · 3 registries | `make attack` + `make e2e-assert` |
| Gate | `test/gate/e2e.mjs` | `make gate-e2e` |
| Pass | `test/redteam/pass_*` | compose pass-test |

No `test/wargame/` trees. Detection freeze unless user spends it. Defense-only.

## Dispositions

`fix` · `hold` · `fp` · `defer`

## Anti-patterns

Empty-commit theater · host-only e2e “done” · wrong plane · red tuned to hide gap · shared hard-reset
