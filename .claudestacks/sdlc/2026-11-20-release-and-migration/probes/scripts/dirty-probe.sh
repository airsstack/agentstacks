#!/bin/sh
# Probe: per-binary tags with a hand-edited release trigger. Does `dist plan` accept the edit
# without and with allow-dirty = ["ci"], what does each binary tag release, and does
# `dist generate` keep the edit?
set -u
P=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad/distprobe
cd "$P" || exit 1
summary() {
  python3 -I -c '
import json, sys
d = json.load(sys.stdin)
print(d["announcement_tag"], [(r["app_name"], r["app_version"]) for r in d["releases"]],
      sorted(k for k, a in d["artifacts"].items() if a["kind"] == "installer"))'
}
echo '$ dist --version'; dist --version
echo '$ grep -h ^version crates/*/Cargo.toml'; grep -h '^version' crates/*/Cargo.toml
sed -i '' '/^allow-dirty/d' dist-workspace.toml
echo '$ cat dist-workspace.toml'; cat dist-workspace.toml
echo '$ sed -n 41,47p .github/workflows/release.yml   # hand-edited trigger'
sed -n 41,47p .github/workflows/release.yml
echo '$ dist plan --tag=agentstacks-cli-v0.1.0   # no allow-dirty'
dist plan --tag=agentstacks-cli-v0.1.0 > out.txt 2>&1; echo "exit=$?"; tail -6 out.txt
sed -i '' 's/^pr-run-mode = "upload"/pr-run-mode = "upload"\nallow-dirty = ["ci"]/' dist-workspace.toml
echo '$ (allow-dirty = ["ci"] added under [dist])'
for t in agentstacks-cli-v0.1.0 claudevs-cli-v0.1.2; do
  echo "\$ dist plan --tag=$t"
  dist plan --tag="$t" --output-format=json 2>/dev/null > out.json; echo "exit=$?"; summary < out.json
done
echo '$ dist plan --tag=claudevs-v0.1.1   # library tag'
dist plan --tag=claudevs-v0.1.1 --output-format=json 2>/dev/null > out.json; echo "exit=$?"; summary < out.json
echo '$ dist generate; sed -n 44,47p .github/workflows/release.yml'
dist generate > out.txt 2>&1; echo "exit=$?"; tail -1 out.txt
sed -n 44,47p .github/workflows/release.yml
rm -f out.txt out.json
