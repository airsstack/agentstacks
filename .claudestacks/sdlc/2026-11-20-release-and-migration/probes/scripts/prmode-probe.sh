#!/bin/sh
# Probe: untagged `dist plan` (what the release workflow runs on a pull request) with per-binary
# versions, under pr-run-mode = upload, plan and skip; and what `dist generate` writes for the
# pull_request trigger under skip.
set -u
P=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad/distprobe
cd "$P" || exit 1
for mode in upload plan skip; do
  sed -i '' -E "s/^pr-run-mode = \"[a-z]+\"/pr-run-mode = \"$mode\"/" dist-workspace.toml
  echo "\$ pr-run-mode = \"$mode\"; dist plan   # no tag"
  dist plan > out.txt 2>&1; echo "exit=$?"; grep -E "too many unrelated|Announce|announcing|error" out.txt | head -3
done
echo '$ (pr-run-mode = "skip") allow-dirty removed; dist generate; sed -n 41,46p .github/workflows/release.yml'
cp .github/workflows/release.yml release.yml.keep
sed -i '' '/^allow-dirty/d' dist-workspace.toml
dist generate > out.txt 2>&1; echo "exit=$?"
sed -n 41,46p .github/workflows/release.yml
cp release.yml.keep .github/workflows/release.yml
rm -f release.yml.keep out.txt
sed -i '' 's/^pr-run-mode = "skip"/pr-run-mode = "upload"\nallow-dirty = ["ci"]/' dist-workspace.toml
grep -n "pr-run-mode\|allow-dirty" dist-workspace.toml
