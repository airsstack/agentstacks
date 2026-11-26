#!/bin/bash
# tui.sh <project_dir> <script.json> <logdir> <typed-line>: interactive TUI inside a private tmux server
# (tmux answers the terminal queries), types the line + Enter, waits for MOCKDONE, captures the screen, exits.
set -u
P="$1"; S="$2"; L="$3"; LINE="$4"
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1
mkdir -p "$(dirname "$L")"; H="$(mktemp -d "${L%/}.home.XXXX")"; T="$H/tmp"; mkdir -p "$T" "$H/.config/muse" "$L"
printf '{"schema_version":1,"endpoint_transport":{"base_url":"http://127.0.0.1:18782","auth":"bearer"}}' > "$H/.config/muse/settings.json"
printf '{"schema_version":1,"providers":{"meta":{"api_key":"dummy"}}}' > "$H/.config/muse/auth.json"
node "$HERE/mock.mjs" 18782 "$S" "$L" & MP=$!
SOCK="/private/tmp/claude-501/mp18782.sock"
tmux -S "$SOCK" -f /dev/null new-session -d -x 200 -y 50 -c "$P" \
  "env -u XDG_CONFIG_HOME -u TMUX HOME='$H' TMPDIR='$T' AIRSSTACK_HOME=/probe/airs '$BIN' --trust-workspace --yolo 2>'$L/stderr.raw'; sleep 30"
sleep 6
tmux -S "$SOCK" send-keys -l -- "$LINE"; sleep 1.5
tmux -S "$SOCK" send-keys Enter
for i in $(seq 1 40); do sleep 0.5; tmux -S "$SOCK" capture-pane -p > "$L/screen.txt"; grep -q MOCKDONE "$L/screen.txt" && break; done
sleep 3; tmux -S "$SOCK" capture-pane -p -S -200 > "$L/screen.txt"
tmux -S "$SOCK" send-keys C-c; sleep 0.5; tmux -S "$SOCK" send-keys C-c; sleep 1
tmux -S "$SOCK" kill-server 2>/dev/null
kill $MP 2>/dev/null; wait $MP 2>/dev/null
grep -v bundled "$L/stderr.raw" > "$L/stderr" 2>/dev/null; rm -f "$L/stderr.raw"
echo "HOME=$H"
