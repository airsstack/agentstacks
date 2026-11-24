#!/bin/sh
# Probe 1: init with official names against the real public repository today (no packs-v tag).
# Probe 2: a smoke-able plan with no airsl on PATH: claude extension + guideline-rust from paths.
# Probe 3: claudevs --version and the smoke fixtures it uses exist here.
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
W=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/release-migration
rm -rf "$S/notag" "$S/smokeproj" && mkdir -p "$S/notag" "$S/smokeproj/ext" "$S/smokeproj/packs"
cd "$S/notag" || exit 1
echo "--- 1 init official names, real repo, no tag"
"$W/target/debug/agentstacks" init --tool claude --pack core < /dev/null
echo "exit=$?"
cp -R "$W/extensions/claude" "$S/smokeproj/ext/"
cp -R "$W/packs/guideline-rust" "$S/smokeproj/packs/"
cd "$S/smokeproj" || exit 1
echo "--- 2 init + plan, paths, PATH without airsl"
PATH=/usr/bin:/bin "$W/target/debug/agentstacks" init --tool ext/claude --pack packs/guideline-rust < /dev/null | tail -2
echo "init exit=$?"
PATH=/usr/bin:/bin "$W/target/debug/agentstacks" plan | tail -1
echo "plan exit=$?"
echo "--- 3 claudevs"
"$W/target/debug/claudevs" --version
ls -d "$W"/crates/claudevs/tests/fixtures/exec-args-plugin "$W"/crates/claudevs/tests/fixtures/dead-script-plugin "$W"/crates/claudevs/tests/fixtures/escape-plugin "$W"/crates/claudevs/tests/fixtures/bad-matcher-plugin "$W"/crates/claudevs/tests/fixtures/hooks-array-plugin "$W"/crates/claudevs/tests/fixtures/minimal-plugin "$W"/crates/claudevs/tests/fixtures/project-branch-plugin "$W"/crates/claudevs/tests/fixtures/matcher-routing-plugin
