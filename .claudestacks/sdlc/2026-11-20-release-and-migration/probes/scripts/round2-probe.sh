#!/bin/sh
# Round-2 design probes, answering the spec review.
#  a. README example with core: init --tool claude --pack core --pack sdlc --pack guideline-rust
#  b. airsl's real pointer script: version rules and the no-downloader path, under sh and dash
#  c. the smoke matrix jq program on real dist plan JSON (both binary tags)
#  d. where dist's generated shell installer puts binaries
#  e. snapshot store path in the core pack and the plugin; vault.lua whole-file diff minus comments
#  f. the "List releases" doc section names no order (control: the latest-release section does)
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
W=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/release-migration
C=/Users/hiraq/Projects/airsstack/claudestacks
A=/Users/hiraq/Projects/airsstack/airsl
B="$W/target/debug/agentstacks"

echo "== a. README example (official URL -> bare copy of HEAD tagged packs-v0.1.0, via insteadOf)"
printf '[url "file://%s/official.git"]\n\tinsteadOf = https://github.com/airsstack/agentstacks\n' "$S" > "$S/gitconfig"
rm -rf "$S/readmeproj" && mkdir -p "$S/readmeproj" && cd "$S/readmeproj" || exit 1
echo '$ agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust < /dev/null'
GIT_CONFIG_GLOBAL="$S/gitconfig" "$B" init --tool claude --pack core --pack sdlc --pack guideline-rust < /dev/null > out.txt 2>&1
echo "exit=$?"; tail -3 out.txt
echo '$ agentstacks plan | tail -1'
GIT_CONFIG_GLOBAL="$S/gitconfig" "$B" plan > out.txt 2>&1; echo "exit=$?"; tail -1 out.txt
echo '$ agentstacks init --tool claude --pack sdlc --pack guideline-rust   # without core, fresh dir'
rm -rf "$S/readmeproj2" && mkdir -p "$S/readmeproj2" && cd "$S/readmeproj2" || exit 1
GIT_CONFIG_GLOBAL="$S/gitconfig" "$B" init --tool claude --pack sdlc --pack guideline-rust < /dev/null > out.txt 2>&1
echo "exit=$?"; cat out.txt

echo "== b. airsl's pointer script ($A/install/install.sh), fake curl that always fails (exit 22)"
mkdir -p "$S/fakecurl" && printf '#!/bin/sh\nexit 22\n' > "$S/fakecurl/curl" && chmod +x "$S/fakecurl/curl"
for shell in /bin/sh /bin/dash; do
  for v in "" latest v0.1.5 0.1.5 "0.1 5" "a/b" v 0.2.0-rc.1; do
    echo "\$ AIRSL_VERSION='$v' PATH=<fakecurl>:/usr/bin:/bin $shell install.sh"
    AIRSL_VERSION="$v" PATH="$S/fakecurl:/usr/bin:/bin" "$shell" "$A/install/install.sh" 2>&1 | sed "s#$S#<scratch>#g"
  done
  echo "\$ PATH=/nonexistent $shell install.sh   # no curl, no wget"
  PATH=/nonexistent "$shell" "$A/install/install.sh" 2>&1; echo "exit=$?"
done

echo "== c. smoke matrix jq program on dist plan JSON"
cd "$S/distprobe" || exit 1
cat > "$S/matrix.jq" <<'EOF'
[.releases[] as $r
 | $r.artifacts[]
 | select($all.artifacts[.].kind == "executable-zip")
 | {archive: ., target: $all.artifacts[.].target_triples[0],
    app: $r.app_name, bin: ($r.app_name | sub("-cli$"; "")), version: $r.app_version}]
EOF
echo '$ cat matrix.jq'; cat "$S/matrix.jq"
for t in agentstacks-cli-v0.1.0 claudevs-cli-v0.1.2; do
  echo "\$ dist plan --tag=$t --output-format=json | jq -c '. as \$all | <matrix.jq>'"
  dist plan --tag="$t" --output-format=json 2>/dev/null | jq -c ". as \$all | $(cat "$S/matrix.jq")"
done
echo '$ jq keys of .releases[0] and of one executable-zip artifact (tag agentstacks-cli-v0.1.0)'
dist plan --tag=agentstacks-cli-v0.1.0 --output-format=json 2>/dev/null | jq -c '(.releases[0] | keys), (.releases[0].artifacts), (.artifacts["agentstacks-cli-x86_64-unknown-linux-musl.tar.xz"] | del(.assets))'

echo "== d. dist build --artifacts=global: the generated shell installer's install dir"
rm -rf target/distrib
dist build --tag=agentstacks-cli-v0.1.0 --artifacts=global > out.txt 2>&1; echo "exit=$?"
ls target/distrib/ | grep installer
grep -n -E 'LOCAL_BIN|\.local/bin|_install_dir=|INSTALL_DIR' target/distrib/agentstacks-cli-installer.sh | head -12
rm -f out.txt

echo "== e. snapshot store path; vault.lua minus comment lines"
cd "$W" || exit 1
echo '$ grep -n "snapshots/" packs/core/skills/snapshot-save/SKILL.md (and the plugin)'
grep -n 'snapshots/' packs/core/skills/snapshot-save/SKILL.md | head -3
grep -n 'snapshots/' "$C/plugins/claudestacks/skills/snapshot-save/SKILL.md" | head -3
grep -v -E '^\s*--' packs/journal/scripts/lib/vault.lua > "$S/v1.lua"
grep -v -E '^\s*--' "$C/plugins/claudestacks-journal/scripts/lib/vault.lua" > "$S/v2.lua"
echo '$ diff <pack vault.lua without comment lines> <plugin vault.lua without comment lines>'
diff "$S/v1.lua" "$S/v2.lua"; echo "diff exit=$?"

echo "== f. GitHub REST docs: order of List releases"
R="$W/.claudestacks/sdlc/2026-11-20-release-and-migration/probes/rest-releases.md"
echo '$ sed -n 11,40p rest-releases.md | grep -n -i -E "sort|order|created_at"'
sed -n 11,40p "$R" | grep -n -i -E 'sort|order|created_at'; echo "grep exit=$?"
echo '$ grep -n -i "sorted by" rest-releases.md   # control'
grep -n -i 'sorted by' "$R"
