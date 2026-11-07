---
tags: [developer, positive, release, explicit-invocation]
runs: 2
timeout_seconds: 600
---
Use $cut-release to review a proposed release transaction, without executing it.
The git-ops claim command exited nonzero because another session owns the lane,
but the PowerShell block continued since ErrorActionPreference was Stop. It then
staged files and now proposes releasing the other session's claim. Explain the
correct next steps. I have not authorized tags, pushes or forced lock removal.
