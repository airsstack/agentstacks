#!/bin/sh
# Re-runs every design probe and writes its raw output into the chain's probes/ directory.
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
W=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/release-migration
PR="$W/.claudestacks/sdlc/2026-11-20-release-and-migration/probes"
B="$W/target/debug/agentstacks"

fence() { printf '```text\n'; cat; printf '```\n'; }

{
  echo "# dist 0.33.1: tags, trigger, releases, archives, runners"
  echo
  echo "Scratch copy of this workspace (HEAD 0fe1f61), all four crates set to 0.2.0, publish = true."
  echo "Script: \`dist-probe.sh\` (scratch). Run 2026-11-24."
  echo
  fence < "$S/dist-out.txt"
} > "$PR/dist-results.md"

{
  echo "# cargo publish --workspace --dry-run (cargo 1.94.1)"
  echo
  echo "Same scratch copy as dist-results.md (all four crates at 0.2.0, publish = true)."
  echo
  grep -E "Packaging|Packaged|Uploading|warning|error|exit=" "$S/publish-dryrun.txt" | sed "s#$S/distprobe#<scratch>#" | fence
  echo
  echo "## crates.io state (curl https://crates.io/api/v1/crates/<name>)"
  echo
  for c in agentstacks agentstacks-cli claudevs claudevs-cli; do
    printf '%s: ' "$c"
    curl -s -A "release-probe" "https://crates.io/api/v1/crates/$c" | python3 -I -c 'import sys,json; d=json.load(sys.stdin); print([v["num"] for v in d["versions"]] if "versions" in d else d.get("errors"))'
  done | fence
} > "$PR/publish-results.md"

{
  echo "# agentstacks and airsl on PATH (agentstacks 0.0.0, worktree HEAD 0fe1f61)"
  echo
  echo "Project: extensions/claude copied to ext/claude, packs/core to packs/core."
  echo
  cd "$S/noairsl" || exit 1
  {
    echo '$ PATH=/usr/bin:/bin agentstacks plan   # no airsl'
    PATH=/usr/bin:/bin "$B" plan 2>&1; echo "exit=$?"
    echo '$ PATH=<dir with an airsl that prints "airsl 0.1.4">:/usr/bin:/bin agentstacks plan'
    PATH="$S/oldairsl:/usr/bin:/bin" "$B" plan 2>&1; echo "exit=$?"
    echo '$ agentstacks plan | tail -3   # airsl 0.1.5 at ~/.local/bin/airsl'
    "$B" plan 2>&1 | tail -3
  } | fence
  echo
  echo "## Negative: claudevs never runs the airsl binary"
  echo
  cd "$W" || exit 1
  {
    echo '$ grep -rn "\"airsl\"" crates/claudevs/src crates/claudevs-cli/src'
    grep -rn '"airsl"' crates/claudevs/src crates/claudevs-cli/src; echo "exit=$?"
    echo '$ grep -rn "\"airsl\"" crates/agentstacks/src/process   # control'
    grep -rn '"airsl"' crates/agentstacks/src/process
  } | fence
} > "$PR/airsl-results.md"

{
  echo "# init with official names"
  echo
  echo "## 1. Today, against the real public repository (no packs-v tag)"
  echo
  sh "$S/smoke-probe.sh" 2>&1 | sed -n '/--- 1/,/exit=/p' | fence
  echo
  echo "## 2. With a packs-v0.1.0 tag (official URL redirected to a bare copy of HEAD via insteadOf)"
  echo
  sh "$S/init-probe.sh" 2>&1 | sed -n '/^--- init/p;/airsl    0.1.5/,$p' | fence
  echo
  echo "## 3. anonymous git ls-remote --tags https://github.com/airsstack/agentstacks"
  echo
  { git ls-remote --tags https://github.com/airsstack/agentstacks; echo "exit=$?"; } | fence
} > "$PR/init-results.md"

{
  echo "# smoke commands with an empty PATH (no git, airsl or other tools)"
  echo
  sh "$S/smoke2-probe.sh" 2>&1 | fence
  echo
  echo '$ claudevs --version (worktree build)'
  "$W/target/debug/claudevs" --version | fence
} > "$PR/smoke-results.md"

{
  echo "# GitHub Pages and archived repositories"
  echo
  echo "archiving.md (docs.github.com, fetched 2026-11-24) says nothing about Pages:"
  echo
  { echo '$ grep -n -i pages docs/archiving.md'; grep -n -i pages "$S/docs/archiving.md"; echo "exit=$?"; } | fence
  echo
  echo "Archived repositories with a workflow-built Pages site still serve it:"
  echo
  {
    for r in microsoft/autogen nhn/tui.editor; do
      gh api "repos/$r" --jq '[.full_name,.archived]|@tsv'
      gh api "repos/$r/pages" --jq '[.html_url,.build_type]|@tsv'
    done
    sh "$S/urls.sh" https://microsoft.github.io/autogen/ https://nhn.github.io/tui.editor/
    echo '$ gh api repos/airsstack/claudestacks/pages'
    gh api repos/airsstack/claudestacks/pages --jq '[.html_url,.build_type]|@tsv'
    echo '$ gh api repos/airsstack/agentstacks/pages'
    gh api repos/airsstack/agentstacks/pages 2>&1 | tail -1
    sh "$S/urls.sh" https://airsstack.github.io/airsl/install.sh https://airsstack.github.io/claudestacks/install.sh https://airsstack.github.io/agentstacks/install.sh
  } | fence
} > "$PR/pages-results.md"

cp "$S/docs/discover-plugins.md" "$PR/discover-plugins.md"
cp "$S/docs/archiving.md" "$PR/archiving.md"
sed -n '3284,3300p' "$S/docs/workflow-syntax.md" > "$PR/workflow-syntax-filter.md"
ls -la "$PR"
