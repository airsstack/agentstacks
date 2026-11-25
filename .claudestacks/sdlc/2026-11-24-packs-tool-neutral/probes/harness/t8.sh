# t8: as t7 (service started first), but each turn runs --standalone.
# t8: as t7 (service started first), but each turn runs --standalone.
# t8: as t7 (service started first), but each turn runs --standalone.
SP=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad
OC=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/target/opencode-check/2.4.1/bin/opencode
D=$SP/tn/t8; SID=ses_zzzzprobe0000000000000008; PORT1=47391; PORT2=47392
rm -rf "$D"; mkdir -p "$D/home/.config/opencode" "$D/bin" "$D/tmp"; cp -R "$SP/ptn/proj" "$D/proj"
ln -sf "$OC" "$D/bin/opencode"
export HOME="$D/home" XDG_CONFIG_HOME="$D/home/.config" XDG_DATA_HOME="$D/home/.local/share" \
  XDG_CACHE_HOME="$D/home/.cache" XDG_STATE_HOME="$D/home/.local/state" TMPDIR="$D/tmp" \
  OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_AUTOUPDATE=1 PATH="$D/bin:$PATH"
# One mock serves both turns, so the model URL in the service's config stays valid.
cat > "$D/scen.json" <<'EOF'
{ "main": [ { "text": "first answer" },
    { "tool": "shell", "args": { "command": "opencode service status > status.txt 2>&1; opencode session export \"$OPENCODE_SESSION_ID\" > e.json 2> e.err; echo $? > rc.txt" } },
    { "text": "main done" } ], "child": [] }
EOF
node "$SP/tn/mock2.mjs" $PORT1 "$D/scen.json" "$D/requests.jsonl" &
MOCK=$!
sleep 1
cat > "$HOME/.config/opencode/opencode.json" <<EOF
{ "\$schema": "https://opencode.ai/config.json",
  "model": "mock/m",
  "providers": { "mock": { "name": "Mock", "package": "@opencode/ai/providers/openai-compatible",
    "settings": { "baseURL": "http://127.0.0.1:$PORT1/v1", "apiKey": "x" },
    "models": { "m": { "name": "M" } } } } }
EOF
cd "$D/proj"
echo '$ opencode service start'; "$OC" service start 2>&1 | tail -2; echo "exit=$?"
echo '$ opencode service status'; "$OC" service status 2>&1; echo "exit=$?"
echo '$ opencode run --standalone --session '$SID' "first prompt"'; "$OC" run --standalone --session $SID --format json "first prompt" > out1.txt 2>&1 < /dev/null; echo "exit=$?"
echo '$ opencode run --standalone --session '$SID' "second prompt"'; "$OC" run --standalone --session $SID --format json "second prompt" > out2.txt 2>&1 < /dev/null; echo "exit=$?"
sleep 3
echo "inside the session: service status = $(cat status.txt); export rc = $(cat rc.txt); e.err = $(cat e.err)"
echo '$ opencode session export '$SID'   # through the running service, after both turns'
"$OC" session export "$SID" > after.json 2>/dev/null; echo "exit=$?"
node -e 'const j=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));console.log("messages "+j.messages.length+", server-restarted messages "+j.messages.filter(m=>m.type==="synthetic"&&/server restarted/.test(m.text||"")).length);j.messages.forEach(m=>console.log("  "+m.type+" "+(m.content||[]).map(c=>c.type+(c.name?":"+c.name:"")).join(",")))' after.json
node -e 'const j=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));console.log("in-session export: messages "+j.messages.length+" ("+j.messages.map(m=>m.type).join(" ")+")")' e.json
echo '$ opencode service stop'; "$OC" service stop 2>&1; echo "exit=$?"
kill $MOCK 2>/dev/null
echo '$ pgrep -fl scratchpad/tn/t8'; pgrep -fl 'scratchpad/tn/t8'; echo "exit=$?"
