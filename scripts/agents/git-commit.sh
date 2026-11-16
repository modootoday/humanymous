#!/usr/bin/env bash
# Wrapper: conventional commit. No co-author trailer (rule 92); the provider
# argument is validated and reported, and attribution goes in the PR body.
# usage: git-commit.sh <provider> <subject> [body]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
PROVIDER="${1:-}"
SUBJECT="${2:-}"
BODY="${3:-}"
[[ -n "$PROVIDER" && -n "$SUBJECT" ]] || {
  echo "usage: $0 <claude|grok|codex|gemini|cursor> <subject> [body]" >&2
  exit 1
}

case "$PROVIDER" in
  claude|grok|codex|gemini|cursor) ;;
  *) echo "unknown provider: $PROVIDER" >&2; exit 1 ;;
esac

if ! echo "$SUBJECT" | grep -qE '^(feat|fix|docs|test|ci|build|refactor|perf|harden|security|chore)(\(.+\))?!?: '; then
  echo "subject must be Conventional Commits, e.g. feat(scope): summary" >&2
  exit 1
fi

MSG_FILE=$(mktemp)
{
  echo "$SUBJECT"
  echo ""
  if [[ -n "$BODY" ]]; then
    echo "$BODY"
    echo ""
  fi
} >"$MSG_FILE"

git commit -F "$MSG_FILE"
rm -f "$MSG_FILE"
echo "committed (provider $PROVIDER: name it in the pull request body)"
