#!/bin/sh
# Probe: with pr-run-mode = "plan", which jobs of the generated release workflow run on a pull
# request? Prints the `if:` and `needs:` of the build and smoke jobs.
set -u
P=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad/distprobe
cd "$P" || exit 1
cp .github/workflows/release.yml release.yml.keep
sed -i '' '/^allow-dirty/d' dist-workspace.toml
sed -i '' -E 's/^pr-run-mode = "[a-z]+"/pr-run-mode = "plan"/' dist-workspace.toml
dist generate > out.txt 2>&1; echo "generate exit=$?"
cp .github/workflows/release.yml release-planmode.yml
echo '$ grep -n -E "^  [a-z-]+:$|^    if:|^    needs:|publishing" release-planmode.yml'
grep -n -E '^  [a-z-]+:$|^    if:|^    needs:|publishing ==|publishing:' release-planmode.yml
cp release.yml.keep .github/workflows/release.yml
rm -f release.yml.keep out.txt
sed -i '' 's/^pr-run-mode = "plan"/pr-run-mode = "upload"\nallow-dirty = ["ci"]/' dist-workspace.toml
