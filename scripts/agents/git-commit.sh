#!/usr/bin/env bash
# Wrapper: conventional commit + mandatory Co-Authored-By provider trailer.
# usage: git-commit.sh <provider> <subject> [body]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
PROVIDER="${1:-}"
SUBJECT="${2:-}"
BODY="${3:-}"
[[ -n "$PROVIDER" && -n "$SUBJECT" ]] || {
  echo "usage: $0 <claude|grok|codex|gemini> <subject> [body]" >&2
  exit 1
}

# Emails must be GitHub-linked for avatars — see .agents/sessions/COMMIT-CONVENTION.md
case "$PROVIDER" in
  claude) DISPLAY="Claude"; EMAIL="noreply@anthropic.com" ;;
  grok)   DISPLAY="Grok"; EMAIL="grok@x.ai" ;;
  codex)  DISPLAY="codex"; EMAIL="codex@openai.com" ;;
  gemini) DISPLAY="Gemini CLI"; EMAIL="gemini-cli@users.noreply.github.com" ;;
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
  echo "Co-Authored-By: $DISPLAY <$EMAIL>"
  echo "X-Agent-Provider: $PROVIDER"
} >"$MSG_FILE"

git commit -F "$MSG_FILE"
rm -f "$MSG_FILE"
echo "committed with Co-Authored-By: $DISPLAY <$EMAIL>"
