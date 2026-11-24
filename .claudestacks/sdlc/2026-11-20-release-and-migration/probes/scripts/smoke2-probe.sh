#!/bin/sh
# Probe: the agentstacks smoke steps with an empty PATH (no git, no airsl, no tools at all), as in
# the musl Alpine container: init --yes, plan, doctor on the claude extension + guideline-rust
# copied into a fresh project.
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
W=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/release-migration
B="$W/target/debug/agentstacks"
rm -rf "$S/smoke2" && mkdir -p "$S/smoke2/ext" "$S/smoke2/packs"
cp -R "$W/extensions/claude" "$S/smoke2/ext/"
cp -R "$W/packs/guideline-rust" "$S/smoke2/packs/"
cd "$S/smoke2" || exit 1
echo "--- init --yes"
PATH=/nonexistent "$B" init --tool ext/claude --pack packs/guideline-rust --yes > init.out 2>&1
echo "exit=$?"
/usr/bin/tail -3 init.out
echo "--- plan"
PATH=/nonexistent "$B" plan
echo "exit=$?"
echo "--- doctor"
PATH=/nonexistent "$B" doctor
echo "exit=$?"
