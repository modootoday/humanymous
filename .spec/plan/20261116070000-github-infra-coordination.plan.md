---
id: plan:root/20261116070000
title: GitHub infrastructure coordination for cloud and scheduled sessions
status: active
created: 2026-11-16T06:54:38Z
updated: 2026-11-16T07:46:00Z
rel:
  depends_on: [plan:root/20261116060213, evidence:root/20261116081000, sot:root/common-automation-rules]
---

# GitHub infrastructure coordination for cloud and scheduled sessions

## Goal

On 2026-11-16 the owner decided to move the local-only coordination parts of
humanymous onto GitHub infrastructure, the same way as the sibling projects. They
then asked for the work to proceed after measurement. This plan holds the
measured inventory and the phases. Rule `.agents/rules/97-github-coordination.md`
is the always-on part.

## Measured inventory (source `631fc67`, Linux cloud container)

| Local dependency | Measurement | GitHub replacement | Class |
| --- | --- | --- | --- |
| `.agents/sessions/ACTIVE.md` board | gitignored; absent in a fresh clone | open PR or branch naming one lane; one per lane | moves |
| `lanes.json` / `LANES.md` (27 lanes) | tracked, public | definitions stay; occupancy becomes PR or branch | stays (definitions) |
| `THREADS.md`, handover-pack | tracked | PR bodies, Issues, Actions runs | moves |
| `git-coord.ps1` | needs `pwsh`, which the container lacks; `git_coord` test fails without it | remote as arbiter, merge queue after phase 2 | moves; scripts kept until a Rust replacement qualifies (goal unit 33) |
| `git-coord.sh preflight` | ran in the container | still recommended | stays |
| `.agent-runs/hooks` | gitignored, local | Actions logs for CI actions | local; raw tool-call audit may hold paths and prompts, so it is never published |
| `.spec.local/` | 158 force-added files untracked; consumers unchanged without them ([[evidence:root/20261116081000]]) | public `.spec/evidence` summaries; CI refuses tracked files | private |
| Windows AT, CEF, Unlock A receipts | need a Windows desktop | none | local only (class H) |
| Rule 96 local Docker engine | the container ran its own `dockerd` | container engine, or CI `docker` job, recorded as distinct venues | moves (rule 96 section) |
| Rule 60 Docker-only e2e | product lab images are scaffolds ([[evidence:root/20261116062411]]) | CI `docker` and `browser-product` jobs become required checks once a real harness exists | blocked on goal unit 35 |
| Provider chat history | local per machine | `.agents/lessons/` summaries | local; may hold personal data |
| Owner decisions | chat | Issues labelled `needs-owner` | moves |

## Steps

| # | Step | Class | State |
| ---: | --- | --- | --- |
| 1 | Untrack `.spec.local` and add the CI guard | A | done 2026-11-16 |
| 2 | Rule 97, with pointers from AGENTS.md and rules 90, 91 and 96 | A | done 2026-11-16 |
| 3 | Labels as `.github/labels.json` (owner applies, C2.7); file the open owner decisions as `needs-owner` Issues | A (file), P (apply) | labels file committed 2026-11-16; Issues are routine since the owner's 2026-11-16 unattended direction |
| 4 | Commit `.github/rulesets/*.json` for `main` and `codex/rust` plus `v-tags`; required checks `validate / ready`, `Public surface`, `Secret scan`, later the product e2e job | A to draft, P to apply | files committed 2026-11-16; owner applies |
| 5 | PR flow: `<provider>/<slug>` branches, PR into `codex/rust`, the owner merges (Phase 1). Rules 90, 91, 95, 97 and `GIT-PROTOCOL.md` updated | A (branch, push, PR), P (merge) | active from 2026-11-16 by owner direction; enforced once the ruleset is applied |
| 6 | Replace `git-coord`/`session-board` scripts with Rust, then retire the board files | A after 5 | open (unit 33) |
| 7 | Cloud environment setup script that copies the CI toolchain steps | A | open |

## Record

- 2026-11-16: steps 1 and 2 landed. No ref, label, Issue or ruleset was created
  on GitHub.
- 2026-11-16: the owner adopted the common automation rules v1.2
  ([[sot:root/common-automation-rules]]), confirmed `codex/rust` as the base
  and the owner noreply commit identity, and directed unattended work: topic
  branch push and pull requests are routine; merge, release, tag, secret values
  and ruleset or settings application stay with the owner and are recorded in
  the pull request body or `needs-owner` Issues. Settings files, `Public
  surface`, `Secret scan` and `Settings drift` landed on a `claude/` branch.
