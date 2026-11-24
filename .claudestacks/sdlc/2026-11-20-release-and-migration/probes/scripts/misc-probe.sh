#!/bin/sh
# Probes: (1) dist plan with no tag (a pull request) lists both apps; (2) crates.io category slugs
# exist; (3) `sh -n` passes a good script and fails a broken one; (4) claudevs-cli's published
# categories and keywords.
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
cd "$S/distprobe" || exit 1
echo '$ dist plan --output-format=json   # no tag, as on a pull request'
dist plan --output-format=json 2>/dev/null | python3 -I -c '
import json, sys
d = json.load(sys.stdin)
print("releases:", [(r["app_name"], r["app_version"]) for r in d["releases"]])
print("archives:", len([a for a in d["artifacts"].values() if a["kind"] == "executable-zip"]))
'
echo '$ crates.io categories'
for c in command-line-utilities development-tools development-tools::testing config; do
  printf '%s: ' "$c"
  curl -s -A "release-probe" "https://crates.io/api/v1/categories/$c" | python3 -I -c 'import sys,json; d=json.load(sys.stdin); print(d["category"]["slug"] if "category" in d else d.get("errors"))'
done
echo '$ sh -n on a good and a broken script'
printf 'main() {\n  echo ok\n}\nmain "$@"\n' > "$S/good.sh"
printf 'main() {\n  if true; then\n    echo ok\n}\nmain "$@"\n' > "$S/bad.sh"
sh -n "$S/good.sh"; echo "good exit=$?"
sh -n "$S/bad.sh"; echo "bad exit=$?"
echo '$ sh -n on the existing pointer scripts'
sh -n /Users/hiraq/Projects/airsstack/airsl/install/install.sh; echo "airsl install.sh exit=$?"
sh -n /Users/hiraq/Projects/airsstack/claudestacks/install/install.sh; echo "claudestacks install.sh exit=$?"
echo '$ claudevs-cli on crates.io: categories, keywords'
curl -s -A "release-probe" "https://crates.io/api/v1/crates/claudevs-cli" | python3 -I -c 'import sys,json; d=json.load(sys.stdin); print([c["slug"] for c in d["categories"]], [k["keyword"] for k in d["keywords"]])'
