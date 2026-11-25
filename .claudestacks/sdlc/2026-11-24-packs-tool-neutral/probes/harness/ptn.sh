set -e
R=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode
W=$1; rm -rf "$W"; mkdir -p "$W/proj/ext" "$W/proj/packs"
cp -R $R/extensions/claude $R/extensions/opencode "$W/proj/ext/"
for p in core sdlc journal guideline-rust guideline-python; do cp -R $R/packs/$p "$W/proj/packs/"; done
cd "$W/proj"; git init -q
set +e
$R/target/debug/agentstacks init --tool ext/claude --tool ext/opencode --pack packs/core --pack packs/sdlc --pack packs/journal --pack packs/guideline-rust --pack packs/guideline-python --yes > "$W/init.out" 2>&1
echo exit=$?
