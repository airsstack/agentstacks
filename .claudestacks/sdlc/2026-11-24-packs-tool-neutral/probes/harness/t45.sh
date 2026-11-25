# t4: background service (no --standalone), two turns, a plain export only in turn 2: does the
#     session get a "server restarted" message?
# t5: --standalone, the rendered journal-capture agent with an external_directory allow rule added
#     after the deny-all rule: can the subagent read and write outside the project?
SP=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad
OC=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/target/opencode-check/2.4.1/bin/opencode
MSGS='const t=require("fs").readFileSync(process.argv[1],"utf8").replace(/exit=\d+\s*$/,"");const j=JSON.parse(t);
console.log("info.id "+j.info.id+" messages "+j.messages.length);
j.messages.forEach(m=>console.log("  "+m.type+" "+(m.content||[]).map(c=>c.type+(c.name?":"+c.name:"")).join(",")+" "+(m.text||"").replace(/\n/g," ").slice(0,60)));'

D=$SP/tn/t4; SID=ses_zzzzprobe0000000000000004
rm -rf "$D"; mkdir -p "$D"; cp -R "$SP/ptn/proj" "$D/proj"
printf '{ "main": [ { "text": "first answer" } ], "child": [] }\n' > "$D/turn1.json"
cat > "$D/turn2.json" <<'EOF'
{ "main": [
    { "tool": "shell", "args": { "command": "E=$(mktemp \"${TMPDIR:-/tmp}/journal-XXXXXX\"); opencode session export \"$OPENCODE_SESSION_ID\" > \"$E\"; echo $? > export.rc; cp \"$E\" export-copy.json; rm -f \"$E\"; echo exported" } },
    { "text": "main done" } ], "child": [] }
EOF
echo '$ RUNMODE=" " sh harness/run3.sh <t4> <port> '$SID' turn1.json "first prompt"; then turn2.json "second prompt"'
RUNMODE=" " sh "$SP/tn/run3.sh" "$D" 47351 $SID "$D/turn1.json" "first prompt" 2>/dev/null
RUNMODE=" " sh "$SP/tn/run3.sh" "$D" 47352 $SID "$D/turn2.json" "second prompt" 2>/dev/null
grep -E 'exit=|killed' "$D/out.txt"
echo "export.rc: $(cat $D/proj/export.rc)"; echo "left in TMPDIR: $(ls $D/tmp | grep journal- || echo none)"
echo '$ node <msgs> proj/export-copy.json   # the in-session export of turn 2'
node -e "$MSGS" "$D/proj/export-copy.json"
echo '$ opencode session export --standalone '$SID'   # after both runs, service stopped'
HOME="$D/home" XDG_CONFIG_HOME="$D/home/.config" XDG_DATA_HOME="$D/home/.local/share" XDG_CACHE_HOME="$D/home/.cache" XDG_STATE_HOME="$D/home/.local/state" \
  "$OC" session export --standalone "$SID" > "$D/after.json" 2>/dev/null; echo "exit=$?"
node -e "$MSGS" "$D/after.json"

D=$SP/tn/t5; SID=ses_zzzzprobe0000000000000005
rm -rf "$D"; mkdir -p "$D/tmp"; cp -R "$SP/ptn/proj" "$D/proj"; printf 'outside\n' > "$D/tmp/fixed.txt"
A="$D/proj/.opencode/agents/journal-capture.md"
sed -i '' 's|^permissions: \[{"action":"\*","effect":"deny","resource":"\*"},|permissions: [{"action":"*","effect":"deny","resource":"*"},{"action":"external_directory","effect":"allow","resource":"*"},|' "$A"
echo; echo '$ grep ^permissions .opencode/agents/journal-capture.md   # patched'
grep '^permissions' "$A"
sed "s|@D@|$D|g" > "$D/turn.json" <<'EOF'
{ "main": [
    { "tool": "subagent", "args": { "agent": "journal-capture", "description": "probe", "prompt": "probe" } },
    { "text": "main done" } ],
  "child": [
    { "tool": "read", "args": { "path": "@D@/tmp/fixed.txt" } },
    { "tool": "write", "args": { "path": "@D@/tmp/child-write.txt", "content": "child\n" } },
    { "text": "child done" } ] }
EOF
echo '$ sh harness/run3.sh <t5> <port> '$SID' turn.json "probe"'
sh "$SP/tn/run3.sh" "$D" 47353 $SID "$D/turn.json" "probe" 2>/dev/null
grep -E 'exit=|killed' "$D/out.txt"
echo '$ node results.mjs requests.jsonl   # child requests'
node "$SP/tn/results.mjs" "$D/requests.jsonl" | grep -A3 '(child)' | grep -E 'child|read:|write:' | cut -c1-200
echo "child-write.txt: $(cat $D/tmp/child-write.txt 2>&1)"
echo '$ pgrep -fl scratchpad/tn'; pgrep -fl 'scratchpad/tn'; echo "exit=$?"
