#!/bin/sh
# usage: run.sh <probe-dir> <port> "<prompt>" [extra opencode run flags...]
# <probe-dir> holds: proj/ (the project), scenario.json. Writes requests.jsonl, out.txt, home/.
# OpenCode runs fully isolated (HOME/XDG under <probe-dir>/home), standalone, no model fetch.
set -u
DIR=$1; PORT=$2; PROMPT=$3; shift 3
H=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$DIR/home"
: > "$DIR/requests.jsonl"
node "$H/mock.mjs" "$PORT" "$DIR/scenario.json" "$DIR/requests.jsonl" &
MOCK=$!
sleep 1
export HOME="$DIR/home" XDG_CONFIG_HOME="$DIR/home/.config" XDG_DATA_HOME="$DIR/home/.local/share" \
  XDG_CACHE_HOME="$DIR/home/.cache" XDG_STATE_HOME="$DIR/home/.local/state" \
  OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_AUTOUPDATE=1 MOCK_PORT="$PORT"
cd "$DIR/proj" || exit 1
mkdir -p "$HOME/.config/opencode"
cat > "$HOME/.config/opencode/opencode.json" <<EOF
{ "\$schema": "https://opencode.ai/config.json",
  "model": "mock/m",
  "providers": { "mock": { "name": "Mock", "package": "@opencode/ai/providers/openai-compatible",
    "settings": { "baseURL": "http://127.0.0.1:$PORT/v1", "apiKey": "x" },
    "models": { "m": { "name": "M" } } } } }
EOF
/Users/hiraq/.opencode/bin/opencode run --standalone --auto --format json "$@" "$PROMPT" > "$DIR/out.txt" 2> "$DIR/err.txt"
echo "exit=$?" >> "$DIR/out.txt"
kill $MOCK 2>/dev/null
