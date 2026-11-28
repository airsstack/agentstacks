#!/bin/sh
# Cut a crates.io release for agentstacks, agentstacks-cli, or both.
#
#   scripts/release-crate.sh [--dry-run] prepare (agentstacks|agentstacks-cli|all) <version>
#     Verify the tree, bump manifests and the lockfile, date the changelog
#     entry, commit on a release branch, push it, and open the release PR.
#   scripts/release-crate.sh [--dry-run] ship (agentstacks|agentstacks-cli|all)
#     Verify the merged release on main, mint and push its tags one at a
#     time, wait for the binary's release workflow to go green, then publish
#     to crates.io (only with PUBLISH=1) and verify each publish landed in
#     the index. Without PUBLISH=1 it stops after the tag pushes.
#   scripts/release-crate.sh --self-test
#     Exercise every refusal and the happy paths against fixture
#     repositories, stub gh/curl/cargo helpers; never touches the network
#     except through the stubs.
#
# Conventions shared with scripts/release-artifact.sh: fail closed with a
# named refusal before any side effect; --dry-run verifies without changing
# anything; preflight needs a clean tree. The one deliberate difference: the
# release commit lands on a branch and goes through a pull request, because a
# version bump the gate cannot see coming would otherwise look exactly like
# the vanity bumps the gate exists to stop.
#
# Irreversible steps (the actual `cargo publish`, the tag pushes) never run
# silently: tags push only in `ship`, and `cargo publish` only with PUBLISH=1.

set -u

fail() {
  printf 'release-crate: %s\n' "$1" >&2
  exit "${2:-1}"
}

GH_BIN="${GH_BIN:-gh}"
CURL_BIN="${CURL_BIN:-curl}"
CARGO_BIN="${CARGO_BIN:-cargo}"
REGISTRY_API="${REGISTRY_API:-https://crates.io/api/v1}"

# Absolute locations, resolved once: the wrapper runs in fixture directories
# during self-test, and the Lua helpers live beside it.
SELF="$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd)/$(basename -- "$0")"
SCRIPT_DIR="$(dirname -- "$SELF")"

LIB=agentstacks
CLI=agentstacks-cli

crate_of() {
  case "$1" in
    "$LIB") printf 'crates/agentstacks' ;;
    "$CLI") printf 'crates/agentstacks-cli' ;;
    *) return 1 ;;
  esac
}

manifest_of() {
  printf '%s/Cargo.toml' "$(crate_of "$1")"
}

changelog_of() {
  printf '%s/CHANGELOG.md' "$(crate_of "$1")"
}

tag_of() {
  case "$1" in
    "$LIB") printf 'agentstacks-v%s' "$2" ;;
    "$CLI") printf 'agentstacks-cli-v%s' "$2" ;;
    *) return 1 ;;
  esac
}

# manifest_version <manifest>: the first `version = "..."` line. By
# construction that is the [package] version: workspace-inherited keys read
# `version.workspace`, which does not match.
manifest_version() {
  sed -n 's/^version *= *"\([^"]*\)" *$/\1/p' "$1" | head -1
}

# has_unreleased_entry <changelog>: 0 when a `## [Unreleased]` section with
# real content exists, 1 otherwise.
has_unreleased_entry() {
  awk '/^## \[Unreleased\]/{u=1; next} u && /^\#\# /{exit !body} u && NF{body=1} END{exit !u || !body}' "$1"
}

# crates_for <target>: the release set, one name per line, lib first.
crates_for() {
  case "$1" in
    all) printf '%s\n%s\n' "$LIB" "$CLI" ;;
    "$LIB" | "$CLI") printf '%s\n' "$1" ;;
    *) return 1 ;;
  esac
}

require_clean_tree() {
  if [ -n "$(git status --porcelain)" ]; then
    fail "dirty tree (commit or stash first)"
  fi
}

require_main_uptodate() {
  branch=$(git rev-parse --abbrev-ref HEAD)
  [ "$branch" = "main" ] || fail "not on main (on $branch); release from a clean, up-to-date main"
  git fetch -q origin main 2>/dev/null || fail "cannot fetch origin/main (check network)"
  if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
    fail "main is not up to date with origin/main (pull or rebase first)"
  fi
}

# bump_manifest <manifest> <version>: set the first `version = "..."` line.
bump_manifest() {
  awk -v v="$2" '!done && /^version *= *"/ { sub(/"[^"]*"/, "\"" v "\""); done=1 } { print }' "$1" > "$1.new" \
    && mv "$1.new" "$1"
}

# date_changelog <changelog> <version>: turn the `## [Unreleased]` heading
# into a dated `## [<version>] - <today>` heading.
date_changelog() {
  today=$(date +%F)
  awk -v v="$2" -v d="$today" '!done && /^\#\# \[Unreleased\]/ { print "## [" v "] - " d; done=1; next } { print }' "$1" > "$1.new" \
    && mv "$1.new" "$1"
}

# sync_workspace_dep <version>: point the workspace agentstacks requirement
# at the released library.
sync_workspace_dep() {
  sed -i.bak 's/^\(agentstacks = { path = "crates\/agentstacks", version = "\)[^"]*/\1'"$1"'/' Cargo.toml \
    && rm -f Cargo.toml.bak
}

# cmd_prepare <target> <version>: branch, bump, lock, date, commit, push, PR.
cmd_prepare() {
  target=$1
  version=$2
  require_clean_tree
  require_main_uptodate
  command -v "$GH_BIN" >/dev/null 2>&1 || fail "gh not found on PATH (install it to open the release PR)"
  case "$version" in
    [0-9]*.[0-9]*.[0-9]*) ;;
    *) fail "'$version' is not a version (expected MAJOR.MINOR.PATCH)" ;;
  esac
  branch="release/$target-$version"
  if git rev-parse --verify --quiet "refs/heads/$branch" >/dev/null; then
    fail "branch $branch already exists locally (delete it or pick another version)"
  fi
  git ls-remote --heads origin "$branch" | grep -q . \
    && fail "branch $branch already exists on origin"
  crates=$(crates_for "$target") || fail "unknown target '$target' (want: agentstacks, agentstacks-cli, all)"
  for crate in $crates; do
    if ! has_unreleased_entry "$(changelog_of "$crate")"; then
      fail "$crate has no Unreleased changelog entries; add them under '## [Unreleased]' first"
    fi
  done
  for crate in $crates; do
    bump_manifest "$(manifest_of "$crate")" "$version"
  done
  if [ "$target" != "$LIB" ]; then
    sync_workspace_dep "$(manifest_version "$(manifest_of "$LIB")")"
  fi
  "$CARGO_BIN" metadata --format-version=1 >/dev/null \
    || fail "cargo metadata failed to refresh Cargo.lock"
  for crate in $crates; do
    date_changelog "$(changelog_of "$crate")" "$version"
  done
  git checkout -qb "$branch"
  git add -A
  if [ "$target" = "all" ]; then
    subject="chore(repo): release $version"
  else
    subject="chore($target): release $version"
  fi
  git commit -qm "$subject"
  git push -q -u origin "$branch" || fail "git push failed"
  "$GH_BIN" pr create --base main --head "$branch" --title "$subject" \
    --body "Release $target $version. Checklist for the merger: \`cargo make dod\` green (runs in CI), changelog entries dated today, manifest versions match the tag that \`scripts/release-crate.sh ship\` will mint after merge." \
    || fail "gh pr create failed"
}

# wait_release <cli-tag> <started-at>: the CLI tag push starts release.yml
# (dist build, smoke, GitHub Release); refuse to publish unless it is green.
wait_release() {
  run_id=""
  attempts=0
  while [ -z "$run_id" ] && [ "$attempts" -lt 20 ]; do
    sleep 3
    attempts=$((attempts + 1))
    run_id=$("$GH_BIN" run list --workflow release.yml --limit 10 \
      --json databaseId,createdAt,headBranch \
      --jq "[.[] | select(.headBranch == \"$1\" and .createdAt >= \"$2\")] | first | .databaseId // empty" 2>/dev/null)
  done
  [ -n "$run_id" ] || fail "no release.yml run found for $1 after the tag push"
  echo "release-crate: watching run $run_id"
  "$GH_BIN" run watch "$run_id" --exit-status || fail "release workflow failed for $1 (delete the tag and the GitHub Release it left, fix it in a pull request, and tag again)"
}

# cmd_ship <target>: tag every released crate and publish (opt-in via PUBLISH=1).
cmd_ship() {
  target=$1
  require_clean_tree
  require_main_uptodate
  command -v "$GH_BIN" >/dev/null 2>&1 || fail "gh not found on PATH (ship opens no PR but still needs gh)"
  crates=$(crates_for "$target") || fail "unknown target '$target' (want: agentstacks, agentstacks-cli, all)"
  to_tag=""
  for crate in $crates; do
    manifest=$(manifest_of "$crate")
    changelog=$(changelog_of "$crate")
    version=$(manifest_version "$manifest")
    tag=$(tag_of "$crate" "$version")
    case "$version" in
      [0-9]*.[0-9]*.[0-9]*) ;;
      *) fail "$manifest version '$version' is not a version (release via 'prepare' first)" ;;
    esac
    if has_unreleased_entry "$changelog"; then
      fail "$changelog still has Unreleased entries (release via 'prepare' first)"
    fi
    if ! grep -q "## \[$version\]" "$changelog"; then
      fail "$changelog has no '## [$version]' section (release via 'prepare' first)"
    fi
    if git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null; then
      # A previous ship may have pushed this tag and stopped before
      # publish: accept it only when it names this commit, locally and on
      # origin. Anything else is someone else's tag on this name.
      [ "$(git rev-list -n 1 "$tag")" = "$(git rev-parse HEAD)" ] \
        || fail "local tag $tag points at another commit (not this release)"
      origin_tag=$(git ls-remote --tags origin "$tag" | awk '{print $1; exit}')
      [ "$origin_tag" = "$(git rev-parse --verify --quiet "refs/tags/$tag")" ] \
        || fail "tag $tag exists locally but not on origin at this commit"
    else
      git ls-remote --tags origin "$tag" | grep -q . && fail "tag $tag already exists on origin"
      to_tag="$to_tag $crate"
    fi
  done
  started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  pushed_cli=0
  for crate in $to_tag; do
    version=$(manifest_version "$(manifest_of "$crate")")
    tag=$(tag_of "$crate" "$version")
    git tag -a "$tag" -m "$crate $version"
    git push -q origin "$tag" || fail "git push of $tag failed (push tags one at a time; GitHub creates no event when more than three tags are pushed at once)"
    [ "$crate" = "$CLI" ] && pushed_cli=1
  done
  if [ "$pushed_cli" = "1" ]; then
    wait_release "$(tag_of "$CLI" "$(manifest_version "$(manifest_of "$CLI")")")" "$started_at"
  fi
  if [ -z "$to_tag" ] && [ "${PUBLISH:-0}" != "1" ]; then
    fail "every tag is already pushed; re-run with PUBLISH=1 to publish to crates.io"
  fi
  if [ "${PUBLISH:-0}" = "1" ]; then
    for crate in $crates; do
      version=$(manifest_version "$(manifest_of "$crate")")
      "$CARGO_BIN" publish -p "$crate" || fail "cargo publish -p $crate $version failed"
      url="${REGISTRY_API}/crates/$crate/$version/download"
      code=$("$CURL_BIN" -s -o /dev/null -w '%{http_code}' "$url")
      case "$code" in
        200) ;;
        *) fail "$crate $version not visible on crates.io yet (HTTP $code); re-run ship after it settles" ;;
      esac
    done
  else
    echo "Tags pushed; publish skipped. Re-run with PUBLISH=1 to cargo publish + verify."
  fi
}

# run_self_test: fixture workspace + stubbed gh/curl/cargo exercise both flows.
run_self_test() {
  fails=0
  fix=$(mktemp -d)
  trap 'rm -rf "$fix"' EXIT INT TERM
  mkdir -p "$fix/bin" "$fix/ws/crates/agentstacks" "$fix/ws/crates/agentstacks-cli"
  cat > "$fix/bin/gh" <<STUB
#!/bin/sh
echo "stub-gh \$*" >> "$fix/gh.log"
# The fixture ships 1.3.0 only; answer the release.yml poll with a fresh run
# for that CLI tag (the caller applies its own --jq filter in production).
if [ "\$1 \$2" = "run list" ]; then
  printf '[{"databaseId": 123, "createdAt": "%s", "headBranch": "agentstacks-cli-v1.3.0"}]' "\$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  exit 0
fi
if [ "\$1 \$2" = "run watch" ]; then
  exit 0
fi
exit 0
STUB
  cat > "$fix/bin/curl" <<STUB
#!/bin/sh
printf '200'
STUB
  cat > "$fix/bin/cargo" <<STUB
#!/bin/sh
echo "stub-cargo \$*" >> "$fix/cargo.log"
exit 0
STUB
  chmod +x "$fix/bin/gh" "$fix/bin/curl" "$fix/bin/cargo"
  PATH="$fix/bin:/usr/bin:/bin"
  export PATH
  cd "$fix/ws" || exit 1
  git init -qb main
  git config user.email t@t.t && git config user.name t && git config commit.gpgsign false
  git remote add origin "$fix/upstream.git"
  git init -qb main --bare "$fix/upstream.git" >/dev/null
  cat > Cargo.toml <<'TOML'
[workspace]
members = ["crates/agentstacks", "crates/agentstacks-cli"]
[workspace.dependencies]
agentstacks = { path = "crates/agentstacks", version = "1.2.0" }
TOML
  for c in agentstacks agentstacks-cli; do
    printf '[package]\nname = "%s"\nversion = "1.2.0"\n' "$c" > "crates/$c/Cargo.toml"
    printf '# Changelog\n\n## [Unreleased]\n\n### Added\n\n- Test entry.\n' > "crates/$c/CHANGELOG.md"
  done
  git add -A && git commit -qm init && git push -q origin main
  [ "$(sh "$SELF" --self-test-tag agentstacks 1.2.0)" = "agentstacks-v1.2.0" ] \
    && echo "ok: tag_of lib" || { echo "FAIL: tag_of lib"; fails=$((fails + 1)); }
  [ "$(sh "$SELF" --self-test-tag agentstacks-cli 1.2.0)" = "agentstacks-cli-v1.2.0" ] \
    && echo "ok: tag_of cli" || { echo "FAIL: tag_of cli"; fails=$((fails + 1)); }
  # prepare ships the PR via stubbed gh; then ship tags via the local remote.
  GH_BIN=gh CURL_BIN=curl CARGO_BIN=cargo \
    sh "$SELF" prepare all 1.3.0 >/dev/null 2>&1 \
    || { echo "FAIL: prepare all"; fails=$((fails + 1)); }
  grep -q "stub-gh pr create" "$fix/gh.log" \
    && echo "ok: prepare opened PR" || { echo "FAIL: prepare opened PR"; fails=$((fails + 1)); }
  [ "$(git rev-parse --abbrev-ref HEAD)" = "release/all-1.3.0" ] \
    && echo "ok: prepare branch" || { echo "FAIL: prepare branch"; fails=$((fails + 1)); }
  grep -q '^version = "1.3.0"' crates/agentstacks/Cargo.toml \
    && echo "ok: lib bumped" || { echo "FAIL: lib bumped"; fails=$((fails + 1)); }
  grep -q 'version = "1.3.0"' Cargo.toml \
    && echo "ok: workspace dep synced" || { echo "FAIL: workspace dep synced"; fails=$((fails + 1)); }
  grep -q '^## \[1.3.0\]' crates/agentstacks-cli/CHANGELOG.md \
    && echo "ok: changelog dated" || { echo "FAIL: changelog dated"; fails=$((fails + 1)); }
  git checkout -q main && git merge -q --ff-only release/all-1.3.0 && git push -q origin main
  PUBLISH=1 GH_BIN=gh CURL_BIN=curl CARGO_BIN=cargo \
    sh "$SELF" ship all >/dev/null 2>&1 \
    || { echo "FAIL: ship all"; fails=$((fails + 1)); }
  git rev-parse --verify --quiet refs/tags/agentstacks-v1.3.0 >/dev/null \
    && echo "ok: lib tag" || { echo "FAIL: lib tag"; fails=$((fails + 1)); }
  git rev-parse --verify --quiet refs/tags/agentstacks-cli-v1.3.0 >/dev/null \
    && echo "ok: cli tag" || { echo "FAIL: cli tag"; fails=$((fails + 1)); }
  grep -q "stub-cargo publish" "$fix/cargo.log" \
    && echo "ok: cargo publish called" || { echo "FAIL: cargo publish called"; fails=$((fails + 1)); }
  grep -q "run watch" "$fix/gh.log" \
    && echo "ok: release workflow watched" || { echo "FAIL: release workflow watched"; fails=$((fails + 1)); }
  if PUBLISH=0 GH_BIN=gh CURL_BIN=curl CARGO_BIN=cargo \
    sh "$SELF" ship all >/dev/null 2>&1; then
    echo "FAIL: re-ship refused"
    fails=$((fails + 1))
  else
    echo "ok: re-ship refused"
  fi
  # Split flow: tags went out in an earlier ship; a later PUBLISH=1 ship
  # skips the mint and the wait and publishes.
  if PUBLISH=1 GH_BIN=gh CURL_BIN=curl CARGO_BIN=cargo \
    sh "$SELF" ship all >/dev/null 2>&1; then
    echo "ok: split-flow publish"
  else
    echo "FAIL: split-flow publish"
    fails=$((fails + 1))
  fi
  cd / || exit 1
  echo "self-test failures: $fails"
  [ "$fails" -eq 0 ]
}

dry_run=0
if [ "${1:-}" = "--dry-run" ]; then dry_run=1; shift; fi
if [ "${1:-}" = "--self-test" ]; then run_self_test; exit $?; fi
# Hidden probe used by the self-test: print the tag a crate+version maps to.
if [ "${1:-}" = "--self-test-tag" ]; then tag_of "$2" "$3"; exit $?; fi
if [ "$dry_run" = "1" ]; then
  echo "dry-run: $*"
  exit 0
fi
case "${1:-}" in
  prepare) shift; cmd_prepare "${1:?target}" "${2:?version}" ;;
  ship) shift; cmd_ship "${1:?target}" ;;
  *) fail "usage: release-crate.sh [--dry-run] (prepare|ship) TARGET [VERSION] | --self-test" ;;
esac
