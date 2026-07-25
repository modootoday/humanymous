---
name: review-changes
description: >
  Use when reviewing uncommitted or branch changes for detection freeze, security,
  PII, and test gaps before merge. Prefer over ad-hoc skim when the user asks for
  a pre-merge or freeze review.
when_to_use: >
  "review my changes", "pre-merge review", "detection freeze review", "what did I break".
---

# Review changes

## Focus

1. Detection freeze regressions  
2. Fail-closed / dual-control  
3. PII in logs  
4. Missing Docker gate when needed  
5. Docs over-claim  
6. Gate forking scoring / god-files  

## Method

`git status` + `git diff` → severity-ordered findings with paths → **detection behavior changed: yes/no**.
