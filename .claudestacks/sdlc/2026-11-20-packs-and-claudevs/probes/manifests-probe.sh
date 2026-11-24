#!/bin/sh
# Probe: load the exact pack.toml / native hooks.toml texts of the spec through the real
# agentstacks binary and the shipped claude extension, with stub content files.
SP=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad
WT=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/plugin-pack-claudevs-move
OUT=$WT/.claudestacks/sdlc/2026-11-20-packs-and-claudevs/probes/manifests-results.md
BIN=$SP/as-target/debug/agentstacks
P=$SP/manifests
rm -rf "$P"
mkdir -p "$P"
cd "$P" || exit 1
git init -q
cp -R "$WT/extensions/claude" ext-claude

skill() { # pack name
  mkdir -p "packs/$1/skills/$2"
  printf -- '---\nname: %s\ndescription: stub\n---\nstub\n' "$2" > "packs/$1/skills/$2/SKILL.md"
}
agent() {
  mkdir -p "packs/$1/agents"
  printf -- '---\nname: %s\ndescription: stub\n---\nstub\n' "$2" > "packs/$1/agents/$2.md"
}
lua() {
  mkdir -p "packs/$1/$(dirname "$2")"
  printf 'return function(event) return nil end\n' > "packs/$1/$2"
}

for pk in core sdlc journal guideline-rust guideline-python; do
  mkdir -p "packs/$pk"
  # The only edit: airsl 0.1.5 is not released, so the probe pins the installed 0.1.4.
  sed 's/">=0.1.5"/">=0.1.4"/' "$SP/tomls/$pk.toml" > "packs/$pk/pack.toml"
  printf 'stub readme\n' > "packs/$pk/README.md"
done
for s in context-handoff discuss enforce-doctor orchestrate process-guidelines snapshot-load snapshot-save; do skill core $s; done
mkdir -p packs/core/skills/discuss/references && printf 'stub\n' > packs/core/skills/discuss/references/protocol.md
for a in coder explorer reviewer; do agent core $a; done
for f in hooks/rearm.lua hooks/enforce.lua hooks/doctor.lua scripts/style.lua scripts/handoff_check_hook.lua scripts/comm_check_hook.lua; do lua core $f; done
printf 'return { t = function() end }\n' > packs/core/hooks/enforce_test.lua
mkdir -p packs/core/native/claude/skills/core-statusline
cp "$SP/tomls/core-native-hooks.toml" packs/core/native/claude/hooks.toml
printf -- '---\nname: core-statusline\ndescription: stub\ndisable-model-invocation: true\n---\nRun {{var project_dir}}/{{root}}/scripts/statusline-install.lua\n' > packs/core/native/claude/skills/core-statusline/SKILL.md
for s in adr design distill execute intent milestone plan roadmap setup status triage; do skill sdlc $s; done
for a in artifact-reviewer chain-reader task-briefer; do agent sdlc $a; done
lua sdlc scripts/attention.lua
for s in capture helped link note recall review vault-setup; do skill journal $s; done
for a in capture curator note recall; do agent journal $a; done
lua journal scripts/session-start.lua
for g in rust python; do
  sk=guidelines; [ "$g" = python ] && sk=py-guidelines; skill "guideline-$g" "$sk"
  printf '{"stack": "%s", "skill": "{{skill %s}}"}\n' "$g" "$sk" > "packs/guideline-$g/enforcement.json"
done

run() {
  echo "\$ agentstacks $*"
  "$BIN" "$@" 2>&1
  echo "exit=$?"
}
{
  echo "# Pack manifests through the real loader"
  echo
  echo "Run 2026-11-23 in $P. Binary: agentstacks built from HEAD 80fd75f (the worktree), airsl 0.1.4."
  echo "Each pack.toml is the spec's text with one edit: \`\">=0.1.5\"\` → \`\">=0.1.4\"\` (0.1.5 is not released)."
  echo "Content files are stubs. To get past the cross-pack name clash rule (set/clash.rs) this run names two stub skills journal/vault-setup and guideline-python/py-guidelines; the spec removes that rule. Output pasted verbatim."
  echo
  echo '~~~~text'
  for pk in core sdlc journal guideline-rust guideline-python; do echo "--- packs/$pk/pack.toml"; cat "packs/$pk/pack.toml"; done
  echo "--- packs/core/native/claude/hooks.toml"; cat packs/core/native/claude/hooks.toml
  run init --tool ./ext-claude --pack ./packs/core --pack ./packs/sdlc --pack ./packs/journal --pack ./packs/guideline-rust --pack ./packs/guideline-python --yes
  echo '$ find .claude -type f | sort'
  find .claude -type f | sort
  echo '$ cat .claude/settings.json'
  cat .claude/settings.json
  echo '$ cat .claude/agentstacks/guideline-rust/enforcement.json'
  cat .claude/agentstacks/guideline-rust/enforcement.json
  echo '$ cat .claude/skills/core-statusline/SKILL.md'
  cat .claude/skills/core-statusline/SKILL.md
  run plan
  echo '~~~~'
  echo
  echo "## Two proposals of one install path"
  echo
  echo "A second project: pack \`a\` ships skill \`x\`; pack \`b\` ships \`native/claude/skills/x/SKILL.md\`."
  echo "Both make the claude extension propose \`.claude/skills/x/SKILL.md\`."
  echo
  echo '~~~~text'
  Q=$SP/collision
  rm -rf "$Q"
  mkdir -p "$Q/packs/a/skills/x" "$Q/packs/b/native/claude/skills/x"
  cd "$Q" || exit 1
  git init -q
  cp -R "$WT/extensions/claude" ext-claude
  printf '[pack]\nname = "a"\nversion = "0.1.0"\napi = 0\ndescription = "a"\n' > packs/a/pack.toml
  printf '[pack]\nname = "b"\nversion = "0.1.0"\napi = 0\ndescription = "b"\n' > packs/b/pack.toml
  printf -- '---\nname: x\ndescription: stub\n---\nstub\n' > packs/a/skills/x/SKILL.md
  printf -- '---\nname: x\ndescription: stub\n---\nnative stub\n' > packs/b/native/claude/skills/x/SKILL.md
  mkdir -p packs/b/skills/w && printf -- '---\nname: w\ndescription: stub\n---\nstub\n' > packs/b/skills/w/SKILL.md
  for f in packs/a/pack.toml packs/b/pack.toml packs/a/skills/x/SKILL.md packs/b/native/claude/skills/x/SKILL.md packs/b/skills/w/SKILL.md; do echo "--- $f"; cat "$f"; done
  run init --tool ./ext-claude --pack ./packs/a --pack ./packs/b --yes
  echo '~~~~'
  echo
  echo "## The cross-pack name clash rule (today)"
  echo
  echo "A third project: packs \`c\` and \`d\` both ship skill \`setup\`."
  echo
  echo '~~~~text'
  K=$SP/clash
  rm -rf "$K"
  mkdir -p "$K/packs/c/skills/setup" "$K/packs/d/skills/setup"
  cd "$K" || exit 1
  git init -q
  cp -R "$WT/extensions/claude" ext-claude
  for pk in c d; do
    printf '[pack]\nname = "%s"\nversion = "0.1.0"\napi = 0\ndescription = "%s"\n' "$pk" "$pk" > "packs/$pk/pack.toml"
    printf -- '---\nname: setup\ndescription: stub\n---\nstub\n' > "packs/$pk/skills/setup/SKILL.md"
  done
  run init --tool ./ext-claude --pack ./packs/c --pack ./packs/d --yes
  echo '~~~~'
} > "$OUT"
cp "$0" "$WT/.claudestacks/sdlc/2026-11-20-packs-and-claudevs/probes/manifests-probe.sh"
grep -n 'exit=\|skip\|refus\|error\|already' "$OUT" | head -30
