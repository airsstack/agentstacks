# t9: --standalone session, no background service: does `opencode service status` print `stopped`
# from the session's shell, start nothing, and leave the session without a "server restarted" message?
SP=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad
OC=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/target/opencode-check/2.4.1/bin/opencode
D=$SP/tn/t9; SID=ses_zzzzprobe0000000000000009
rm -rf "$D"; mkdir -p "$D"; cp -R "$SP/ptn/proj" "$D/proj"
printf '{ "main": [ { "text": "first answer" } ], "child": [] }\n' > "$D/turn1.json"
cat > "$D/turn2.json" <<'EOF'
{ "main": [ { "tool": "shell", "args": { "command": "opencode service status > status.txt 2>&1; echo $? > rc.txt; opencode service status > status2.txt 2>&1" } },
    { "text": "main done" } ], "child": [] }
EOF
echo '$ sh harness/run3.sh <t9> <port> '$SID' turn1.json "first prompt"; then turn2.json "second prompt"   # --standalone'
sh "$SP/tn/run3.sh" "$D" 47401 $SID "$D/turn1.json" "first prompt" 2>/dev/null
sh "$SP/tn/run3.sh" "$D" 47402 $SID "$D/turn2.json" "second prompt" 2>/dev/null
grep -E 'exit=|killed' "$D/out.txt"
echo "inside the session: status = $(cat $D/proj/status.txt), rc = $(cat $D/proj/rc.txt), status again = $(cat $D/proj/status2.txt)"
HOME="$D/home" XDG_CONFIG_HOME="$D/home/.config" XDG_DATA_HOME="$D/home/.local/share" XDG_CACHE_HOME="$D/home/.cache" XDG_STATE_HOME="$D/home/.local/state" \
  "$OC" session export --standalone "$SID" > "$D/after.json" 2>/dev/null; echo "after-run export exit=$?"
node -e 'const j=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));console.log("messages "+j.messages.length+", server-restarted messages "+j.messages.filter(m=>m.type==="synthetic"&&/server restarted/.test(m.text||"")).length)' "$D/after.json"
echo '$ pgrep -fl scratchpad/tn/t9'; pgrep -fl 'scratchpad/tn/t9'; echo "exit=$?"
