#!/bin/sh
# usage: run.sh <probe-dir> <port> <standalone|service> "<prompt>"
# <probe-dir> holds proj/ and scenario.json. Writes requests.jsonl, out.txt, err.txt, home/.
# OpenCode v2.4.1 (pinned) runs isolated: HOME and XDG under <probe-dir>/home.
set -u
DIR=$1; PORT=$2; MODE=$3; PROMPT=$4
OC=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/target/opencode-check/2.4.1/bin/opencode
H=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness
mkdir -p "$DIR/home/.config/opencode" "$DIR/bin"
ln -sf "$OC" "$DIR/bin/opencode"
: > "$DIR/requests.jsonl"
node "$H/mock.mjs" "$PORT" "$DIR/scenario.json" "$DIR/requests.jsonl" &
MOCK=$!
sleep 1
export HOME="$DIR/home" XDG_CONFIG_HOME="$DIR/home/.config" XDG_DATA_HOME="$DIR/home/.local/share" \
  XDG_CACHE_HOME="$DIR/home/.cache" XDG_STATE_HOME="$DIR/home/.local/state" \
  OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_AUTOUPDATE=1 PATH="/usr/bin:/bin:/usr/sbin:/sbin:$(dirname "$(command -v node)"):$(dirname "$(command -v airsl)")"
cat > "$HOME/.config/opencode/opencode.json" <<EOF
{ "\$schema": "https://opencode.ai/config.json",
  "model": "mock/m",
  "providers": { "mock": { "name": "Mock", "package": "@opencode/ai/providers/openai-compatible",
    "settings": { "baseURL": "http://127.0.0.1:$PORT/v1", "apiKey": "x" },
    "models": { "m": { "name": "M" } } } } }
EOF
cd "$DIR/proj" || exit 1
if [ "$MODE" = standalone ]; then F=--standalone; else F=; fi
"$OC" run $F --auto --format json "$PROMPT" > "$DIR/out.txt" 2> "$DIR/err.txt"
echo "exit=$?" >> "$DIR/out.txt"
sleep 3
if [ "$MODE" = service ]; then "$OC" service stop >> "$DIR/err.txt" 2>&1; fi
kill $MOCK 2>/dev/null
