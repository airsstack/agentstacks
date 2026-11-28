#!/bin/sh
# Manifest-discipline gate: every shipped change to an official pack or tool
# extension travels with a manifest version bump and a changelog entry, and no
# bump travels without a shipped change.
#
# Usage:
#   scripts/release-gate.sh --self-test
#     builds fixture repositories under a temp directory and asserts each
#     verdict; no network, no repository state needed.
#   scripts/release-gate.sh [--repo DIR] BASE HEAD
#     checks `git diff BASE...HEAD` in DIR (default `.`).
#
# Scope rule: a changed file counts as shipped unless it is release paperwork
# (the artifact's own manifest, its top-level CHANGELOG.md) or matched by the
# manifest's `exclude` globs. Globs follow the shapes the official manifests
# use today: exact names (`README.md`, root only) and `**/`-prefixed suffixes
# (`** slash-star _test.lua`). Anything the matcher cannot classify counts as
# shipped: the gate fails closed, never silent.
#
# Exit 0 when every touched artifact is disciplined; otherwise each violation
# is printed and the exit is 1.

set -u

fail=0
say() { printf '%s\n' "$*" >&2; }
violation() { say "release gate: $1"; fail=1; }

# version_of <repo> <rev> <manifest-path>: the `version` the manifest holds at
# <rev>, or empty when the file or the key is absent there.
version_of() {
  git -C "$1" show "$2:$3" 2>/dev/null | sed -n 's/^version *= *"\(.*\)" *$/\1/p' | head -1
}

# excluded <manifest-at-head> <artifact-relative-path>: 0 when the path is an
# excluded non-shipped file, 1 otherwise. Only the repo's own glob shapes are
# matched (`**/` prefix, `*` wildcards, exact names); anything else counts as
# shipped. The manifest lines feed a `while read` loop over a heredoc instead
# of a `for` loop on purpose: unquoted expansion there would let the shell
# glob the patterns themselves against the working tree.
excluded() {
  local manifest file glob suffix
  manifest=$1
  file=$2
  while IFS= read -r glob; do
    [ -z "$glob" ] && continue
    case "$glob" in
      '**/'*)
        suffix=${glob#'**/'}
        case "$file" in
          $suffix | */$suffix) return 0 ;;
        esac
        ;;
      *'/'*)
        case "$file" in
          $glob) return 0 ;;
        esac
        ;;
      *)
        [ "$file" = "$glob" ] && return 0
        ;;
    esac
  done <<EOF
$(sed -n 's/^exclude *= *\[\(.*\)\] *$/\1/p' "$manifest" | tr ',' '\n' | sed 's/^ *"//; s/" *$//')
EOF
  return 1
}

# check_range <repo> <base> <head>: the gate over one diff.
check_range() {
  local repo base head changed artifacts artifact manifest old_version new_version bumped changelog_added shipped rel manifest_head file
  repo="$1"
  base="$2"
  head="$3"
  changed=$(git -C "$repo" diff --name-only "$base...$head" -- . || exit 1)
  [ -z "$changed" ] && return 0
  artifacts=$(printf '%s\n' "$changed" | sed -n 's|^\(packs/[^/]*\)/.*|\1|p; s|^\(extensions/[^/]*\)/.*|\1|p' | sort -u)
  for artifact in $artifacts; do
    case "$artifact" in
      packs/*) manifest="$artifact/pack.toml" ;;
      extensions/*) manifest="$artifact/extension.toml" ;;
      *) continue ;;
    esac
    if [ ! -f "$repo/$manifest" ]; then
      continue
    fi
    old_version=$(version_of "$repo" "$base" "$manifest")
    new_version=$(version_of "$repo" "$head" "$manifest")
    if [ "$old_version" = "$new_version" ]; then
      bumped=""
    else
      bumped="yes"
    fi
    changelog_added=$(git -C "$repo" diff "$base...$head" -- "$artifact/CHANGELOG.md" | grep -c '^+[^+]' || true)
    shipped=""
    while IFS= read -r file; do
      [ "${file#$artifact/}" = "$file" ] && continue
      rel=${file#$artifact/}
      case "$rel" in
        pack.toml | extension.toml | CHANGELOG.md) continue ;;
      esac
      manifest_head="$repo/$manifest"
      if [ "$rel" = "pack.toml" ] || [ "$rel" = "extension.toml" ]; then
        continue
      fi
      if excluded "$manifest_head" "$rel"; then
        continue
      fi
      shipped="$shipped $rel"
    done <<EOF
$(printf '%s\n' "$changed" | grep "^$artifact/")
EOF
    shipped=$(echo "$shipped" | sed 's/^ *//')
    if git -C "$repo" diff "$base...$head" -- "$manifest" | grep -qE '^[+-].*exclude'; then
      shipped="$shipped $manifest#exclude"
      shipped=$(echo "$shipped" | sed 's/^ *//')
    fi
    if [ -n "$shipped" ] && [ -z "$bumped" ]; then
      violation "$artifact changed without a version bump:$shipped"
    fi
    if [ -n "$bumped" ] && [ -z "$shipped" ]; then
      violation "$artifact bumped to $new_version with no shipped change"
    fi
    if [ -n "$bumped" ] && [ "$changelog_added" = "0" ]; then
      violation "$artifact bumped to $new_version with no CHANGELOG entry"
    fi
  done
  return 0
}

# fixture <name> <setup...>: builds a two-commit repo; the caller commits base
# and head, then asserts the gate verdict.
fixture_repo() {
  local dir
  dir=$(mktemp -d)
  git -C "$dir" init -q -b main
  git -C "$dir" config user.email gate@test
  git -C "$dir" config user.name gate
  printf '%s' "$dir"
}

fixture_commit() {
  git -C "$1" add -A
  git -C "$1" commit -qm "$2"
  git -C "$1" rev-parse HEAD
}

self_test() {
  local cases passed dir base head out
  cases=0
  passed=0
  # gate_rejects_82d7561_shape: shipped SKILL plus adapter plus hook changes
  # across two artifacts, no bump, no entry -> fail naming both artifacts.
  dir=$(fixture_repo)
  mkdir -p "$dir/packs/sdlc/skills/plan" "$dir/packs/sdlc/hooks" "$dir/packs/core/skills/discuss"
  printf '[pack]\nname = "sdlc"\nversion = "1.0.0"\napi = 1\ndescription = "d"\n' > "$dir/packs/sdlc/pack.toml"
  printf '# Changelog\n' > "$dir/packs/sdlc/CHANGELOG.md"
  printf '[pack]\nname = "core"\nversion = "1.0.1"\napi = 1\ndescription = "d"\n' > "$dir/packs/core/pack.toml"
  printf '# Changelog\n\n## 1.0.1\n\n- x\n' > "$dir/packs/core/CHANGELOG.md"
  printf 'body\n' > "$dir/packs/sdlc/skills/plan/SKILL.md"
  printf 'check\n' > "$dir/packs/sdlc/hooks/check.lua"
  printf 'body\n' > "$dir/packs/core/skills/discuss/SKILL.md"
  base=$(fixture_commit "$dir" base)
  printf 'changed\n' >> "$dir/packs/sdlc/skills/plan/SKILL.md"
  printf 'changed\n' >> "$dir/packs/sdlc/hooks/check.lua"
  printf 'changed\n' >> "$dir/packs/core/skills/discuss/SKILL.md"
  head=$(fixture_commit "$dir" head)
  cases=$((cases + 1))
  out=$(sh "$0" --repo "$dir" "$base" "$head" 2>&1)
  case "$out" in
    *packs/sdlc*packs/core*|*packs/core*packs/sdlc*) passed=$((passed + 1)) ;;
    *)
      if sh "$0" --repo "$dir" "$base" "$head" >/dev/null 2>&1; then
        say "release gate self-test: gate_rejects_82d7561_shape passed, want failure"
      else
        say "release gate self-test: gate_rejects_82d7561_shape failed without naming both artifacts: $out"
      fi
      ;;
  esac
  rm -rf "$dir"
  # 2: shipped change with bump and entry -> pass.
  dir=$(fixture_repo)
  mkdir -p "$dir/packs/core/skills/discuss"
  printf '[pack]\nname = "core"\nversion = "1.0.0"\napi = 1\ndescription = "d"\n' > "$dir/packs/core/pack.toml"
  printf 'exclude = ["README.md", "**/*_test.lua"]\n' >> "$dir/packs/core/pack.toml"
  printf '# Changelog\n' > "$dir/packs/core/CHANGELOG.md"
  printf 'body\n' > "$dir/packs/core/skills/discuss/SKILL.md"
  base=$(fixture_commit "$dir" base)
  sed -i.bak 's/version = "1.0.0"/version = "1.0.1"/' "$dir/packs/core/pack.toml" && rm "$dir/packs/core/pack.toml.bak"
  printf 'body2\n' >> "$dir/packs/core/skills/discuss/SKILL.md"
  printf '\n## 1.0.1\n\n- Fixed.\n' >> "$dir/packs/core/CHANGELOG.md"
  head=$(fixture_commit "$dir" head)
  cases=$((cases + 1))
  if sh "$0" --repo "$dir" "$base" "$head" >/dev/null 2>&1; then passed=$((passed + 1)); else say "release gate self-test: clean bump failed"; fi
  rm -rf "$dir"
  # gate_accepts_paperwork_only: changelog-only change, no bump -> pass.
  dir=$(fixture_repo)
  mkdir -p "$dir/packs/core"
  printf '[pack]\nname = "core"\nversion = "1.0.0"\napi = 1\ndescription = "d"\n' > "$dir/packs/core/pack.toml"
  printf '# Changelog\n' > "$dir/packs/core/CHANGELOG.md"
  base=$(fixture_commit "$dir" base)
  printf 'notes\n' >> "$dir/packs/core/CHANGELOG.md"
  head=$(fixture_commit "$dir" head)
  cases=$((cases + 1))
  if sh "$0" --repo "$dir" "$base" "$head" >/dev/null 2>&1; then passed=$((passed + 1)); else say "release gate self-test: gate_accepts_paperwork_only failed"; fi
  rm -rf "$dir"
  # gate_accepts_excluded_only: README plus test change, no bump, no entry -> pass.
  dir=$(fixture_repo)
  mkdir -p "$dir/packs/core"
  printf 'exclude = ["README.md", "**/*_test.lua"]\n[pack]\nname = "core"\nversion = "1.0.0"\napi = 1\ndescription = "d"\n' > "$dir/packs/core/pack.toml"
  printf 'docs\n' > "$dir/packs/core/README.md"
  printf 't\n' > "$dir/packs/core/x_test.lua"
  base=$(fixture_commit "$dir" base)
  printf 'more docs\n' >> "$dir/packs/core/README.md"
  printf 't2\n' >> "$dir/packs/core/x_test.lua"
  head=$(fixture_commit "$dir" head)
  cases=$((cases + 1))
  if sh "$0" --repo "$dir" "$base" "$head" >/dev/null 2>&1; then passed=$((passed + 1)); else say "release gate self-test: gate_accepts_excluded_only failed"; fi
  rm -rf "$dir"
  # gate_rejects_exclude_edit_without_bump: exclude edit, no bump -> fail.
  dir=$(fixture_repo)
  mkdir -p "$dir/packs/core"
  printf 'exclude = ["README.md"]\n[pack]\nname = "core"\nversion = "1.0.0"\napi = 1\ndescription = "d"\n' > "$dir/packs/core/pack.toml"
  base=$(fixture_commit "$dir" base)
  printf 'exclude = ["README.md", "**/*_test.lua"]\n[pack]\nname = "core"\nversion = "1.0.0"\napi = 1\ndescription = "d"\n' > "$dir/packs/core/pack.toml"
  head=$(fixture_commit "$dir" head)
  cases=$((cases + 1))
  if sh "$0" --repo "$dir" "$base" "$head" >/dev/null 2>&1; then say "release gate self-test: gate_rejects_exclude_edit_without_bump passed, want failure"; else passed=$((passed + 1)); fi
  rm -rf "$dir"
  # gate_rejects_skill_change_without_bump: a SKILL change, an adapter change,
  # and a hook change each fail without a bump.
  dir=$(fixture_repo)
  mkdir -p "$dir/packs/sdlc/skills/plan"
  printf '[pack]\nname = "sdlc"\nversion = "1.0.0"\napi = 1\ndescription = "d"\n' > "$dir/packs/sdlc/pack.toml"
  printf 'body\n' > "$dir/packs/sdlc/skills/plan/SKILL.md"
  base=$(fixture_commit "$dir" base)
  printf 'changed\n' >> "$dir/packs/sdlc/skills/plan/SKILL.md"
  head=$(fixture_commit "$dir" head)
  cases=$((cases + 1))
  if sh "$0" --repo "$dir" "$base" "$head" >/dev/null 2>&1; then say "release gate self-test: skill change passed, want failure"; else passed=$((passed + 1)); fi
  rm -rf "$dir"
  dir=$(fixture_repo)
  mkdir -p "$dir/extensions/claude"
  printf '[extension]\nname = "claude"\nversion = "1.1.0"\napi = 1\ndescription = "d"\nentry = "adapter.lua"\n' > "$dir/extensions/claude/extension.toml"
  printf 'return {}\n' > "$dir/extensions/claude/adapter.lua"
  base=$(fixture_commit "$dir" base)
  printf 'return {1}\n' > "$dir/extensions/claude/adapter.lua"
  head=$(fixture_commit "$dir" head)
  cases=$((cases + 1))
  if sh "$0" --repo "$dir" "$base" "$head" >/dev/null 2>&1; then say "release gate self-test: adapter change passed, want failure"; else passed=$((passed + 1)); fi
  rm -rf "$dir"
  dir=$(fixture_repo)
  mkdir -p "$dir/packs/sdlc/hooks"
  printf '[pack]\nname = "sdlc"\nversion = "1.0.0"\napi = 1\ndescription = "d"\n' > "$dir/packs/sdlc/pack.toml"
  printf 'check\n' > "$dir/packs/sdlc/hooks/check.lua"
  base=$(fixture_commit "$dir" base)
  printf 'changed\n' >> "$dir/packs/sdlc/hooks/check.lua"
  head=$(fixture_commit "$dir" head)
  cases=$((cases + 1))
  if sh "$0" --repo "$dir" "$base" "$head" >/dev/null 2>&1; then say "release gate self-test: hook change passed, want failure"; else passed=$((passed + 1)); fi
  rm -rf "$dir"
  # vanity bump: version plus entry but no shipped change -> fail.
  dir=$(fixture_repo)
  mkdir -p "$dir/packs/core"
  printf '[pack]\nname = "core"\nversion = "1.0.0"\napi = 1\ndescription = "d"\n' > "$dir/packs/core/pack.toml"
  printf '# Changelog\n' > "$dir/packs/core/CHANGELOG.md"
  base=$(fixture_commit "$dir" base)
  sed -i.bak 's/version = "1.0.0"/version = "1.0.1"/' "$dir/packs/core/pack.toml" && rm "$dir/packs/core/pack.toml.bak"
  printf '\n## 1.0.1\n\n- Nothing.\n' >> "$dir/packs/core/CHANGELOG.md"
  head=$(fixture_commit "$dir" head)
  cases=$((cases + 1))
  if sh "$0" --repo "$dir" "$base" "$head" >/dev/null 2>&1; then say "release gate self-test: vanity bump passed, want failure"; else passed=$((passed + 1)); fi
  rm -rf "$dir"
  # missing entry: shipped change plus bump but no changelog entry -> fail.
  dir=$(fixture_repo)
  mkdir -p "$dir/packs/core/skills/discuss"
  printf '[pack]\nname = "core"\nversion = "1.0.0"\napi = 1\ndescription = "d"\n' > "$dir/packs/core/pack.toml"
  printf '# Changelog\n' > "$dir/packs/core/CHANGELOG.md"
  printf 'body\n' > "$dir/packs/core/skills/discuss/SKILL.md"
  base=$(fixture_commit "$dir" base)
  sed -i.bak 's/version = "1.0.0"/version = "1.0.1"/' "$dir/packs/core/pack.toml" && rm "$dir/packs/core/pack.toml.bak"
  printf 'body2\n' >> "$dir/packs/core/skills/discuss/SKILL.md"
  head=$(fixture_commit "$dir" head)
  cases=$((cases + 1))
  if sh "$0" --repo "$dir" "$base" "$head" >/dev/null 2>&1; then say "release gate self-test: missing entry passed, want failure"; else passed=$((passed + 1)); fi
  rm -rf "$dir"
  echo "release gate: $passed/$cases self-test cases passed"
  [ "$passed" = "$cases" ]
}

repo="."
if [ "${1:-}" = "--self-test" ]; then
  self_test
  exit $?
fi
while [ $# -gt 0 ]; do
  case "$1" in
    --repo)
      repo="$2"
      shift 2
      ;;
    *)
      break
      ;;
  esac
done
if [ $# -ne 2 ]; then
  say "usage: $0 [--self-test] [--repo DIR] BASE HEAD"
  exit 2
fi
fail=0
check_range "$repo" "$1" "$2"
if [ "$fail" != "0" ]; then
  exit 1
fi
echo "release gate: range clean"
