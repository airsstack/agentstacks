#!/bin/sh
# Probe: does `agentstacks init --tool claude --pack core --pack guideline-rust` resolve official
# names once the official repository has a packs-v0.1.0 tag? The official URL is redirected to a
# local bare copy of this worktree's HEAD through a throwaway global git config (insteadOf), so
# nothing touches the real repository.
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
BIN=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/release-migration/target/debug/agentstacks
git --git-dir="$S/official.git" tag -f packs-v0.1.0 HEAD >/dev/null
printf '[url "file://%s/official.git"]\n\tinsteadOf = https://github.com/airsstack/agentstacks\n' "$S" > "$S/gitconfig"
rm -rf "$S/initproj" && mkdir -p "$S/initproj" && cd "$S/initproj" || exit 1
export GIT_CONFIG_GLOBAL="$S/gitconfig"
echo "--- init"
"$BIN" init --tool claude --pack core --pack guideline-rust < /dev/null
echo "init exit=$?"
echo "--- agentstacks.toml"
cat agentstacks.toml
echo "--- plan (tail)"
"$BIN" plan | tail -4
echo "plan exit=$?"
echo "--- --version"
"$BIN" --version
