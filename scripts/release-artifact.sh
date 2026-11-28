#!/bin/sh
# Release one artifact stream: preflight everything locally, then trigger the
# release-artifact workflow, which mints and pushes the single annotated tag.
#
# If the workflow runner is unavailable, mint the tag by hand instead. Run the
# same checks the workflow would run, then tag exactly once and push alone:
#   sh scripts/release-gate.sh --repo . <stream-last-tag> HEAD
#   airsl run --policy trusted scripts/release/agreement.lua <manifest> <changelog> <version>
#   git tag -a <stream-tag> -m <stream-tag>
#   git push origin refs/tags/<stream-tag>
# then walk the smoke steps (init, update, apply, doctor) from the workflow by hand.
#
# Usage: scripts/release-artifact.sh [--dry-run] <artifact> <version>
#
# With --dry-run (or DRY_RUN=1) the wrapper stops before triggering the
# workflow; every refusal still fires. --self-test exercises the refusal
# paths against fixture repositories and never touches the network.

set -u

fail() {
  printf 'release-artifact: %s\n' "$1" >&2
  exit "${2:-1}"
}

AIRSL_BIN="${AIRSL_BIN:-airsl}"
GH_BIN="${GH_BIN:-gh}"

# Absolute locations, resolved once: the wrapper is invoked from any
# directory, and the self-test runs fixtures in temp dirs.
SELF="$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd)/$(basename -- "$0")"
SCRIPT_DIR="$(dirname -- "$SELF")"

manifest_for() {
  case "$1" in
    core | sdlc | journal | guideline-rust | guideline-python)
      printf 'packs/%s/pack.toml' "$1"
      ;;
    claude | musecode | opencode)
      printf 'extensions/%s/extension.toml' "$1"
      ;;
    *)
      return 1
      ;;
  esac
}

changelog_for() {
  case "$1" in
    core | sdlc | journal | guideline-rust | guideline-python)
      printf 'packs/%s/CHANGELOG.md' "$1"
      ;;
    *)
      printf 'extensions/%s/CHANGELOG.md' "$1"
      ;;
  esac
}

self_test() {
  cases=0
  passed=0
  BINPATH="$(dirname "$(command -v git)"):$(dirname "$(command -v airsl)")"
  # 1: dirty tree is refused before anything else.
  dir=$(mktemp -d)
  git -C "$dir" init -q -b main
  git -C "$dir" config user.email gate@test
  git -C "$dir" config user.name gate
  printf x > "$dir/untracked.txt"
  cases=$((cases + 1))
  if (cd "$dir" && "$SELF" --dry-run core 9.9.9 >/dev/null 2>&1); then
    printf 'release-artifact self-test: dirty tree accepted\n' >&2
  else
    passed=$((passed + 1))
  fi
  rm -rf "$dir"
  # The rest need a clean repo with one pack.
  dir=$(mktemp -d)
  git -C "$dir" init -q -b main
  git -C "$dir" config user.email gate@test
  git -C "$dir" config user.name gate
  mkdir -p "$dir/packs/core"
  printf '[pack]\nname = "core"\nversion = "1.0.1"\napi = 1\ndescription = "d"\n' > "$dir/packs/core/pack.toml"
  printf '# Changelog\n\n## 1.0.1\n\n- Fixed.\n' > "$dir/packs/core/CHANGELOG.md"
  git -C "$dir" add -A
  git -C "$dir" commit -qm seed
  # 2: version mismatch is refused.
  cases=$((cases + 1))
  if (cd "$dir" && "$SELF" --dry-run core 9.9.9 >/dev/null 2>&1); then
    printf 'release-artifact self-test: version mismatch accepted\n' >&2
  else
    passed=$((passed + 1))
  fi
  # 4: existing tag is refused (tag the seed first).
  git -C "$dir" tag -a "packs-core-v1.0.1" -m "packs-core-v1.0.1"
  cases=$((cases + 1))
  if (cd "$dir" && "$SELF" --dry-run core 1.0.1 >/dev/null 2>&1); then
    printf 'release-artifact self-test: existing tag accepted\n' >&2
  else
    passed=$((passed + 1))
  fi
  # 5: missing airsl is refused with the pointer.
  cases=$((cases + 1))
  out=$(cd "$dir" && AIRSL_BIN=no-such-airsl "$SELF" --dry-run core 1.0.2 2>&1)
  case "$out" in
    *"not found on PATH (install it to run the release checks)"*) passed=$((passed + 1)) ;;
    *) printf 'release-artifact self-test: airsl refusal missing: %s\n' "$out" >&2 ;;
  esac
  # 3: refuses_missing_entry -- version matches but the changelog has no entry.
  sed -i.bak 's/version = "1.0.1"/version = "1.0.2"/' "$dir/packs/core/pack.toml" && rm "$dir/packs/core/pack.toml.bak"
  git -C "$dir" add -A && git -C "$dir" commit -qm bump
  cases=$((cases + 1))
  out=$(cd "$dir" && "$SELF" --dry-run core 1.0.2 2>&1)
  case "$out" in
    *"no '## 1.0.2' entry"*) passed=$((passed + 1)) ;;
    *) printf 'release-artifact self-test: refuses_missing_entry failed: %s\n' "$out" >&2 ;;
  esac
  git -C "$dir" reset -q --hard HEAD~1
  # 6: a clean dry run verifies and stops before the workflow.
  cases=$((cases + 1))
  git -C "$dir" tag -d "packs-core-v1.0.1" >/dev/null
  out=$(cd "$dir" && GH_BIN=false "$SELF" --dry-run core 1.0.1 2>&1)
  case "$out" in
    *"dry run: verified"*) passed=$((passed + 1)) ;;
    *) printf 'release-artifact self-test: dry run failed: %s\n' "$out" >&2 ;;
  esac
  # 7b: dispatch_rejects_version_mismatch -- a mismatched version never reaches gh.
  cases=$((cases + 1))
  stubdir2=$(mktemp -d)
  printf '#!/bin/sh\necho "$@" >> "$GH_LOG"\nexit 0\n' > "$stubdir2/gh-stub"
  chmod +x "$stubdir2/gh-stub"
  out=$(cd "$dir" && GH_LOG="$stubdir2/gh.log" GH_BIN="$stubdir2/gh-stub" PATH="$BINPATH:/usr/bin:/bin" "$SELF" core 9.9.9 2>&1);
  if [ $? -ne 0 ] && [ ! -f "$stubdir2/gh.log" ]; then
    passed=$((passed + 1))
  else
    printf 'release-artifact self-test: dispatch_rejects_version_mismatch failed: %s\n' "$out" >&2
  fi
  rm -rf "$stubdir2"
  # 7: refuses_failed_dispatch -- a failing dispatch stops before watching.
  cases=$((cases + 1))
  stubdir=$(mktemp -d)
  printf '#!/bin/sh\necho "$@" >> "$GH_LOG"\ncase "$1 $2" in "workflow run") exit 1;; *) exit 0;; esac\n' > "$stubdir/gh-stub"
  chmod +x "$stubdir/gh-stub"
  out=$(cd "$dir" && GH_LOG="$stubdir/gh.log" GH_BIN="$stubdir/gh-stub" PATH="$BINPATH:/usr/bin:/bin" "$SELF" core 1.0.1 2>&1)
  if [ $? -ne 0 ] && grep -q "workflow run" "$stubdir/gh.log" 2>/dev/null && ! grep -q "run watch" "$stubdir/gh.log"; then
    passed=$((passed + 1))
  else
    printf 'release-artifact self-test: refuses_failed_dispatch failed: %s\n' "$out" >&2
  fi
  rm -rf "$stubdir"
  # 7b: a successful dispatch resolves the run id and watches it. The stub
  # emulates gh faithfully: `run list` applies the caller's --jq filter with
  # the real jq, exactly as gh would.
  cases=$((cases + 1))
  stubdir=$(mktemp -d)
  cat > "$stubdir/gh-stub" <<'STUB'
#!/bin/sh
echo "$@" >> "$GH_LOG"
if [ "$1 $2" = "workflow run" ]; then
  exit 0
fi
if [ "$1 $2" = "run list" ]; then
  runs='[{"createdAt": "2999-01-01T00:00:00Z", "databaseId": 123, "headBranch": "main"}]'
  expr=""
  prev=""
  for arg in "$@"; do
    if [ "$prev" = "--jq" ] || [ "$prev" = "-q" ]; then expr="$arg"; fi
    prev="$arg"
  done
  printf '%s' "$runs" | jq -r "$expr"
  exit $?
fi
if [ "$1 $2" = "run watch" ]; then
  test "$3" = "123"
  exit $?
fi
exit 0
STUB
  chmod +x "$stubdir/gh-stub"
  out=$(cd "$dir" && GH_LOG="$stubdir/gh.log" GH_BIN="$stubdir/gh-stub" PATH="$BINPATH:/usr/bin:/bin" "$SELF" core 1.0.1 2>&1)
  if [ $? -eq 0 ] && grep -q "run watch 123" "$stubdir/gh.log" 2>/dev/null; then
    passed=$((passed + 1))
  else
    printf 'release-artifact self-test: watch path failed: %s\n' "$out" >&2
  fi
  rm -rf "$stubdir" "$dir"
  printf 'release-artifact: %s/%s self-test cases passed\n' "$passed" "$cases"
  [ "$passed" = "$cases" ]
}

dry_run=0
if [ "${1:-}" = "--dry-run" ]; then
  dry_run=1
  shift
fi
if [ "${1:-}" = "--self-test" ]; then
  self_test
  exit $?
fi
if [ "${DRY_RUN:-0}" = "1" ]; then
  dry_run=1
fi
if [ $# -ne 2 ]; then
  fail "usage: $0 [--dry-run] <artifact> <version>" 2
fi
artifact="$1"
version="$2"

case "$version" in
  [0-9]*.[0-9]*.[0-9]*) ;;
  *) fail "'$version' is not a version (expected MAJOR.MINOR.PATCH)" ;;
esac

manifest=$(manifest_for "$artifact") || fail "unknown artifact '$artifact'"
changelog=$(changelog_for "$artifact")
tag="packs-$artifact-v$version"
case "$artifact" in
  core | sdlc | journal | guideline-rust | guideline-python) ;;
  *) tag="ext-$artifact-v$version" ;;
esac

[ -n "$(git status --porcelain)" ] && fail "dirty tree (commit or stash first)"
command -v "$AIRSL_BIN" >/dev/null 2>&1 || fail "$AIRSL_BIN not found on PATH (install it to run the release checks)"
if ! out=$("$AIRSL_BIN" run --policy trusted "$SCRIPT_DIR/release/agreement.lua" "$manifest" "$changelog" "$version"); then
  fail "$out"
fi
case "$artifact" in
  core | sdlc | journal | guideline-rust | guideline-python) subdir="packs/$artifact" ;;
  *) subdir="extensions/$artifact" ;;
esac
if subjects=$(git log --format=%s "HEAD" -- "$subdir" 2>/dev/null | head -50); then
  # shellcheck disable=SC2086
  set -- $subjects
  suggestion=$("$AIRSL_BIN" run --policy trusted "$SCRIPT_DIR/release/suggest.lua" "$@" 2>/dev/null || true)
  [ -n "$suggestion" ] && printf 'release-artifact: %s\n' "$suggestion"
fi
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  fail "$tag already exists locally"
fi
if [ "$dry_run" = "0" ]; then
  if remote=$(git ls-remote --tags origin "$tag" 2>/dev/null); then
    [ -n "$remote" ] && fail "$tag already exists on origin"
  else
    printf 'release-artifact: warning: could not list origin tags (offline?); the workflow re-verifies before pushing\n' >&2
  fi
else
  printf 'release-artifact: note: remote tag check skipped (dry run)\n' >&2
fi
if [ "$dry_run" = "1" ]; then
  printf 'release-artifact: dry run: verified %s %s\n' "$artifact" "$version"
  exit 0
fi
command -v "$GH_BIN" >/dev/null 2>&1 || fail "gh not found on PATH (install it to trigger the workflow)"
started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
"$GH_BIN" workflow run release-artifact.yml --ref main -f "artifact=$artifact" -f "version=$version" || fail "gh workflow run failed"
run_id=""
attempts=0
while [ -z "$run_id" ] && [ "$attempts" -lt 20 ]; do
  sleep 3
  attempts=$((attempts + 1))
  run_id=$("$GH_BIN" run list --workflow release-artifact.yml --branch main --limit 5 --json databaseId,createdAt,headBranch --jq "[.[] | select(.headBranch == \"main\" and .createdAt >= \"$started_at\")] | first | .databaseId // empty" 2>/dev/null)
done
[ -n "$run_id" ] || fail "no workflow run found after dispatch"
echo "release-artifact: watching run $run_id"
"$GH_BIN" run watch "$run_id" --exit-status || fail "release workflow failed"
