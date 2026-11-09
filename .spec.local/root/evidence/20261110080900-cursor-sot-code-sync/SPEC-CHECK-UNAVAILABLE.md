# Residual: `spec check` unavailable / hung

Tip: `4c198f30771d937673525dd739cb7f7dcc7910c5`  
Observed: 2026-11-09T23:20:30Z

## Finding

Full and bounded `spec check` CLI runs hung and exited -1 (shells 854909,
804721, 854910). No useful rule findings returned. **Do not retry** for this
SoT/code honesty sync turn.

## Disposition

- Substitute gate: file-level SoT/plan/canvas/receipt honesty vs measured
  Cap0 state (product=0, BrowserReady=false, Cap free=false, AT/AUTH absent).
- Detection freeze preserved; no scoring-law rewrite.
- This residual is **not** product green and does not invent Cap free /
  BrowserReady / AUTH / AT / `pass_product_clean_room`.

Machine record: `SPEC-CHECK-UNAVAILABLE.json`.
