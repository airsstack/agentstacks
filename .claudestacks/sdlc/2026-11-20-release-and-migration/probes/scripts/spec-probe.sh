#!/bin/sh
# Probes the spec's exact texts, extracted from spec.md (never retyped):
#  1. §4.1 dist-workspace.toml and the §4.2 trigger edit, through the §4.2 procedure, with dist 0.33.1.
#  2. §2 crate metadata, with `cargo publish --workspace --dry-run`.
#  3. §3 packs/CHANGELOG.md, through the two pack text gates of Makefile.toml (lines 173, 181).
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
W=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/release-migration
SPEC="$W/.claudestacks/sdlc/2026-11-20-release-and-migration/spec.md"
P="$S/distprobe"
block() { # fence-language → prints the first block of that language in the spec
  awk -v lang="$1" '$0 == "```" lang {on=1; next} on && $0 == "```" {exit} on {print}' "$SPEC"
}
cd "$P" || exit 1
echo "== 1. dist (§4.1, §4.2)"
dist --version
block toml | grep -v '^allow-dirty' > dist-workspace.toml
rm -f .github/workflows/*release.yml
dist generate > out.txt 2>&1; echo "\$ dist generate (without allow-dirty) exit=$?"
block yaml > trigger.yml
python3 -I - "$P/.github/workflows/release.yml" "$P/trigger.yml" <<'EOF'
import re, sys
wf, trig = sys.argv[1], open(sys.argv[2]).read()
s = open(wf).read()
new, n = re.subn(r"^on:\n(?:  .*\n)+", trig, s, count=1, flags=re.M)
assert n == 1, "no on: block"
open(wf, "w").write(new)
EOF
block toml > dist-workspace.toml
echo '$ cmp dist-workspace.toml <(§4.1 block)'; block toml | cmp - dist-workspace.toml && echo identical
echo '$ sed -n 36,52p .github/workflows/release.yml'; sed -n 36,52p .github/workflows/release.yml
summary() {
  python3 -I -c '
import json, sys
d = json.load(sys.stdin)
print(d["announcement_tag"], [(r["app_name"], r["app_version"]) for r in d["releases"]],
      sorted(k for k, a in d["artifacts"].items() if a["kind"] == "installer"),
      len([a for a in d["artifacts"].values() if a["kind"] == "executable-zip"]), "archives")'
}
for t in agentstacks-cli-v0.1.0 claudevs-cli-v0.1.2; do
  echo "\$ dist plan --tag=$t"; dist plan --tag="$t" --output-format=json 2>/dev/null > out.json; echo "exit=$?"; summary < out.json
done
echo '$ dist plan   # untagged, as on a pull request'; dist plan > out.txt 2>&1; echo "exit=$?"; grep -E "announcing" out.txt
echo '$ dist plan --tag=agentstacks-cli-v0.1.9   # version not in the manifest'; dist plan --tag=agentstacks-cli-v0.1.9 > out.txt 2>&1; echo "exit=$?"; grep -E "claims|×" out.txt | head -2
echo '$ dist generate; trigger kept?'; dist generate > out.txt 2>&1; echo "exit=$?"; sed -n '/^    tags:/,/^$/p' .github/workflows/release.yml | head -4
rm -f out.txt out.json trigger.yml

echo "== 2. crate metadata (§2) + publish dry run"
python3 -I - "$SPEC" "$P" <<'EOF'
import re, sys
spec, root = open(sys.argv[1]).read(), sys.argv[2]
for crate in ("agentstacks", "agentstacks-cli"):
    row = next(l for l in spec.splitlines() if l.startswith(f"| `{crate}` | `readme"))
    keys = re.findall(r"`((?:readme|keywords|categories) = [^`]+)`", row)
    path = f"{root}/crates/{crate}/Cargo.toml"
    text = open(path).read()
    text = text.replace("\n[lints]", "\n" + "\n".join(keys) + "\n\n[lints]", 1)
    open(path, "w").write(text)
    print(crate, keys)
EOF
for c in agentstacks agentstacks-cli; do printf '# %s\n' "$c" > "crates/$c/README.md"; done
grep -h -E '^(version|readme|keywords|categories)' crates/agentstacks/Cargo.toml crates/agentstacks-cli/Cargo.toml
grep -n 'path = "crates' Cargo.toml; grep -n '^publish' Cargo.toml
cargo publish --workspace --dry-run --allow-dirty > out.txt 2>&1; echo "\$ cargo publish --workspace --dry-run --allow-dirty exit=$?"
grep -E "Packaging|Packaged|warning|error" out.txt | grep -v "aborting upload" | sed "s#$P#<scratch>#"
rm -f out.txt

echo "== 3. packs/CHANGELOG.md through the pack text gates"
block markdown > packs/CHANGELOG.md
echo '$ head -3 packs/CHANGELOG.md'; head -3 packs/CHANGELOG.md
for n in 173 181; do
  cmd=$(sed -n "${n}p" "$W/Makefile.toml" | sed 's/ || rc=\$?$//')
  echo "\$ Makefile.toml:$n"; sh -c "$cmd"; echo "grep exit=$? (1 = no hit)"
done
