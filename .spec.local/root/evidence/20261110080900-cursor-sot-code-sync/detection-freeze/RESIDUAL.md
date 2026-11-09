# Detection-freeze residual (SoT/code honesty sync)

Observed: 2026-11-09T23:15:00Z  
Tip (pre-commit): `4c198f30771d937673525dd739cb7f7dcc7910c5`  
Branch: `codex/rust`

## Scope

Detection freeze forbids changing scoring weights, thresholds, hard-rule
predicates, or verdict bands in SoT to “match” divergent code.

## Measurement this turn

| Surface | Observation |
| --- | --- |
| `sot:root/model-training-correction` | Still states production weights/thresholds/predicates/verdict bands are frozen; training output never activates a model |
| `sot:root/pass-protocol` | No new detection rule or verdict claimed |
| `sot:root/rust-wargame` | Explicitly does not alter weights/thresholds/rules |
| This Cap=0 SoT/code sync | Updated **status/honesty/roadmap/clean-room/Cap/product** boards only |

## Residual

No measured scoring-law rewrite was performed. No silent detection SoT edit was
made to chase code. If future tooling finds concrete scoring SoT ≠ code
divergence, record it under a new evidence epoch and prefer **code realign to
SoT** (or keep an explicit truth-debt residual). Do not rewrite detection law
from Cap0 tip-bind hygiene.

## Non-claims

- tip_bind.ok ≠ Pass protocol green
- Cap0 honesty board ≠ detection unfreeze
- Component/wargame EXIT0 ≠ verdict-band change
