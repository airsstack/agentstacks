#!/bin/sh
# Probe: dist 0.33.1 on a scratch copy of the workspace with all four crates at 0.2.0 and
# publish = true. Records the release trigger with and without tag-namespace = "v", what
# `dist plan` releases for v0.2.0, the archives and their build runners, and what it does with
# a packs-v0.1.0 tag.
set -u
P=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad/distprobe
cd "$P" || exit 1
echo '$ dist --version'; dist --version
echo '$ grep -h ^version crates/*/Cargo.toml'; grep -h '^version' crates/*/Cargo.toml
echo '$ cat dist-workspace.toml'; cat dist-workspace.toml
rm -rf .github/workflows/release.yml .github/workflows/v-release.yml
dist generate >/dev/null 2>&1
echo '$ ls .github/workflows'; ls .github/workflows
echo '$ sed -n 41,46p .github/workflows/v-release.yml'; sed -n 41,46p .github/workflows/v-release.yml
echo '$ grep -n "uses: ./.github/workflows/smoke.yml" .github/workflows/v-release.yml'
grep -n "uses: ./.github/workflows/smoke.yml" .github/workflows/v-release.yml
echo '$ dist plan --tag=v0.2.0 --output-format=json (releases, archives, runners)'
dist plan --tag=v0.2.0 --output-format=json 2>/dev/null | python3 -I -c '
import json, sys
d = json.load(sys.stdin)
print("releases:", [(r["app_name"], r["app_version"]) for r in d["releases"]])
for k, a in sorted(d["artifacts"].items()):
    if a["kind"] in ("executable-zip", "installer"):
        print(a["kind"], k)
for j in d["ci"]["github"]["artifacts_matrix"]["include"]:
    print("runner", j["runner"], j["targets"])
'
echo '$ dist plan --tag=packs-v0.1.0'; dist plan --tag=packs-v0.1.0 2>&1 | tail -2
echo '$ dist plan --tag=agentstacks-cli-v0.2.0 (releases)'
dist plan --tag=agentstacks-cli-v0.2.0 --output-format=json 2>/dev/null | python3 -I -c '
import json, sys
print("releases:", [(r["app_name"], r["app_version"]) for r in json.load(sys.stdin)["releases"]])'
echo '--- without tag-namespace:'
sed -i '' '/^tag-namespace/d' dist-workspace.toml
rm -f .github/workflows/v-release.yml
dist generate >/dev/null 2>&1
echo '$ sed -n 41,46p .github/workflows/release.yml'; sed -n 41,46p .github/workflows/release.yml
sed -i '' 's/^install-updater = false/install-updater = false\ntag-namespace = "v"/' dist-workspace.toml
rm -f .github/workflows/release.yml
dist generate >/dev/null 2>&1
