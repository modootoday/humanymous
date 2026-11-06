#!/usr/bin/env bash
set -euo pipefail
# Run from docs/. This validates authored prose; it never writes documentation.
destination=''
baseurl=''
while (($#)); do
  case "$1" in
    --destination) destination="$2"; shift 2 ;;
    --baseurl) baseurl="$2"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done
test -n "$destination" || { echo '--destination is required' >&2; exit 2; }
test -f _config.yml && test -s Gemfile.lock
export BUNDLE_FROZEN=true
bundle check
python3 scripts/check-site.py source
python3 -m unittest discover -s scripts -p 'test_*.py'
theme_dir=$(mktemp -d)
trap 'rm -rf -- "$theme_dir"' EXIT
git clone --quiet --depth 1 --branch v1.5.0 https://github.com/modootoday/jekyll-theme-modootoday.git "$theme_dir/theme"
test "$(git -C "$theme_dir/theme" rev-parse HEAD)" = ba2b23f0e1639f92ccefbb882f43092bfe5b16b7
bundle exec jekyll build --destination "$destination" --baseurl "$baseurl" --strict_front_matter
ruby "$theme_dir/theme/exe/modootoday-check" . "$destination" --strict
python3 scripts/check-site.py built --site "$destination" --baseurl "$baseurl"
