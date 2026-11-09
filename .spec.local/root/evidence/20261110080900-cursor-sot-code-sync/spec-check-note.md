# spec check residual

Attempts to run `spec check` on the edited SoT/plan paths hung and terminated
with no usable stdout/stderr (exit 4294967295 / empty capture). Cap=0 SoT/code
honesty edits were applied from measured tip `4c198f30` without a green
`spec check` receipt this turn. Re-run `spec check` under Root/tooling when the
graph checker is healthy; do not treat this absence as product green.
