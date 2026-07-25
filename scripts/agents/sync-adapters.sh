#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

write_marker() {
  local dir="$1" kind="$2"
  mkdir -p "$dir"
  cat >"$dir/GENERATED.md" <<EOF
# GENERATED — do not edit

Mirrored from \`.agents/${kind}\` by \`scripts/agents/sync-adapters\`.
Edit \`.agents/\` then re-run sync.
EOF
}

copy_tree() {
  local src="$1" dst="$2"
  if [[ ! -d "$src" ]]; then
    echo "warn: missing source $src" >&2
    return 0
  fi
  mkdir -p "$dst"
  find "$dst" -mindepth 1 -maxdepth 1 ! -name 'GENERATED.md' -exec rm -rf {} +
  cp -a "$src"/. "$dst"/
}

for rel in .claude/skills .grok/skills .gemini/skills .codex/skills; do
  write_marker "$ROOT/$rel" skills
  copy_tree "$ROOT/.agents/skills" "$ROOT/$rel"
  echo "Synced skills -> $rel"
done

for rel in .claude/rules .grok/rules; do
  write_marker "$ROOT/$rel" rules
  copy_tree "$ROOT/.agents/rules" "$ROOT/$rel"
  echo "Synced rules -> $rel"
done

AGENTS_DST="$ROOT/.claude/agents"
write_marker "$AGENTS_DST" personas
if [[ -d "$ROOT/.agents/personas" ]]; then
  find "$AGENTS_DST" -mindepth 1 -maxdepth 1 ! -name 'GENERATED.md' -exec rm -rf {} +
  for f in "$ROOT/.agents/personas"/*.md; do
    [[ -f "$f" ]] || continue
    name="$(basename "$f" .md)"
    {
      echo "---"
      echo "name: $name"
      echo "description: Project persona $name (from .agents/personas). Use for multi-perspective design panels."
      echo "---"
      echo
      cat "$f"
    } >"$AGENTS_DST/$name.md"
  done
  echo "Synced personas -> .claude/agents"
fi

if [[ -f "$ROOT/.agents/hooks/claude-settings.fragment.json" ]]; then
  mkdir -p "$ROOT/.claude"
  # Fragment is already valid settings-shaped { hooks: ... }
  cp "$ROOT/.agents/hooks/claude-settings.fragment.json" "$ROOT/.claude/settings.json"
  echo "Wrote .claude/settings.json (hooks)"
fi

if [[ -f "$ROOT/.agents/hooks/grok-project-safety.json" ]]; then
  mkdir -p "$ROOT/.grok/hooks"
  cp "$ROOT/.agents/hooks/grok-project-safety.json" "$ROOT/.grok/hooks/project-safety.json"
  echo "Synced Grok hooks -> .grok/hooks/project-safety.json"
fi

if [[ -f "$ROOT/.agents/hooks/codex-hooks.json" ]]; then
  mkdir -p "$ROOT/.codex"
  cp "$ROOT/.agents/hooks/codex-hooks.json" "$ROOT/.codex/hooks.json"
  echo "Synced Codex hooks -> .codex/hooks.json"
fi

if [[ -d "$ROOT/.agents/lessons" ]]; then
  write_marker "$ROOT/.claude/lessons" lessons
  copy_tree "$ROOT/.agents/lessons" "$ROOT/.claude/lessons"
  echo "Synced lessons -> .claude/lessons"
fi

echo "Done. Canon remains under .agents/ and AGENTS.md (+ nested AGENTS.md)."
