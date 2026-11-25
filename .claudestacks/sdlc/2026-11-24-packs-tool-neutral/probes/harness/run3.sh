#!/bin/sh
# usage: run3.sh <probe-dir> <port> <session-id> <scenario.json> "<prompt>"
# OpenCode v2.4.1 (pinned), isolated (HOME, XDG, TMPDIR under <probe-dir>), --standalone like
# check.mjs (RUNMODE=" " for the background service), WITHOUT --auto, `opencode` on the shell's PATH through <probe-dir>/bin. The mock is
# mock2.mjs (separate steps for the main agent and subagents). A run is killed after 120 s.
set -u
DIR=$1; PORT=$2; SID=$3; SCEN=$4; PROMPT=$5
OC=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/target/opencode-check/2.4.1/bin/opencode
H=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$DIR/home/.config/opencode" "$DIR/bin" "$DIR/tmp"
ln -sf "$OC" "$DIR/bin/opencode"
node "$H/mock2.mjs" "$PORT" "$SCEN" "$DIR/requests.jsonl" &
MOCK=$!
sleep 1
export HOME="$DIR/home" XDG_CONFIG_HOME="$DIR/home/.config" XDG_DATA_HOME="$DIR/home/.local/share" \
  XDG_CACHE_HOME="$DIR/home/.cache" XDG_STATE_HOME="$DIR/home/.local/state" TMPDIR="$DIR/tmp" \
  OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_AUTOUPDATE=1 PATH="$DIR/bin:$PATH"
cat > "$HOME/.config/opencode/opencode.json" <<EOF
{ "\$schema": "https://opencode.ai/config.json",
  "model": "mock/m",
  "providers": { "mock": { "name": "Mock", "package": "@opencode/ai/providers/openai-compatible",
    "settings": { "baseURL": "http://127.0.0.1:$PORT/v1", "apiKey": "x" },
    "models": { "m": { "name": "M" } } } } }
EOF
cd "$DIR/proj" || exit 1
"$OC" run ${RUNMODE:---standalone} --session "$SID" --format json "$PROMPT" >> "$DIR/out.txt" 2>> "$DIR/err.txt" < /dev/null &
RUN=$!
( sleep 120; kill $RUN 2>/dev/null && echo "killed after 120 s" >> "$DIR/out.txt" ) &
WATCH=$!
wait $RUN; echo "exit=$?" >> "$DIR/out.txt"
kill $WATCH 2>/dev/null
sleep 5
"$OC" service stop >> "$DIR/err.txt" 2>&1
kill $MOCK 2>/dev/null
