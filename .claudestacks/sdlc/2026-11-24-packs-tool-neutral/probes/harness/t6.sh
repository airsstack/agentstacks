# Which turn-2 shell command puts "The server restarted" into the session? Background service
# mode (no --standalone), two turns each, one fresh probe dir per command.
SP=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad
OC=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/target/opencode-check/2.4.1/bin/opencode
N=0
probe() {
  N=$((N+1)); LABEL=$1; CMD=$2
  D=$SP/tn/t6-$N; SID=ses_zzzzprobe000000000000006$N
  rm -rf "$D"; mkdir -p "$D"; cp -R "$SP/ptn/proj" "$D/proj"
  printf '{ "main": [ { "text": "first answer" } ], "child": [] }\n' > "$D/turn1.json"
  node -e 'process.stdout.write(JSON.stringify({main:[{tool:"shell",args:{command:process.argv[1]}},{text:"main done"}],child:[]}))' "$CMD" > "$D/turn2.json"
  RUNMODE=" " sh "$SP/tn/run3.sh" "$D" 4736$N $SID "$D/turn1.json" "first prompt" 2>/dev/null
  RUNMODE=" " sh "$SP/tn/run3.sh" "$D" 4737$N $SID "$D/turn2.json" "second prompt" 2>/dev/null
  HOME="$D/home" XDG_CONFIG_HOME="$D/home/.config" XDG_DATA_HOME="$D/home/.local/share" XDG_CACHE_HOME="$D/home/.cache" XDG_STATE_HOME="$D/home/.local/state" \
    "$OC" session export --standalone "$SID" > "$D/after.json" 2>/dev/null
  R=$(node -e 'const j=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));const n=j.messages.filter(m=>m.type==="synthetic"&&/server restarted/.test(m.text||"")).length;console.log("messages "+j.messages.length+", server-restarted messages "+n)' "$D/after.json")
  echo "[$LABEL] turn2 shell: $CMD"
  echo "    runs: $(grep -cE '^exit=0' "$D/out.txt") of 2 exit=0; $R"
  for f in out1.txt rc.txt; do [ -f "$D/proj/$f" ] && echo "    $f: $(head -c 200 "$D/proj/$f" | tr '\n' ' ')"; done
}
probe control 'echo hi > out1.txt; echo $? > rc.txt'
probe status 'opencode service status > out1.txt 2>&1; echo $? > rc.txt'
probe export 'opencode session export "$OPENCODE_SESSION_ID" > e.json 2> out1.txt; echo $? > rc.txt'
probe export-server 'U=$(opencode service status); opencode session export --server "$U" "$OPENCODE_SESSION_ID" > e.json 2> out1.txt; echo "$? $U" > rc.txt'
echo '$ pgrep -fl scratchpad/tn/t6'; pgrep -fl 'scratchpad/tn/t6'; echo "exit=$?"
