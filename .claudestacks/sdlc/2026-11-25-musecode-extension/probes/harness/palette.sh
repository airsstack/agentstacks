#!/bin/bash
# palette.sh <project_dir> <logdir> <typed-text>: start the TUI in tmux, type text WITHOUT Enter, capture the screen (the / palette).
set -u
P="$1"; L="$2"; TXT="$3"
BIN=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1
mkdir -p "$(dirname "$L")"; H="$(mktemp -d "${L%/}.home.XXXX")"; T="$H/tmp"; mkdir -p "$T" "$H/.config/muse" "$L"
printf '{"schema_version":1,"endpoint_transport":{"base_url":"http://127.0.0.1:18782","auth":"bearer"}}' > "$H/.config/muse/settings.json"
printf '{"schema_version":1,"providers":{"meta":{"api_key":"dummy"}}}' > "$H/.config/muse/auth.json"
SOCK="/private/tmp/claude-501/mp18782.sock"
tmux -S "$SOCK" -f /dev/null new-session -d -x 200 -y 50 -c "$P" "env -u XDG_CONFIG_HOME -u TMUX HOME='$H' TMPDIR='$T' '$BIN' --trust-workspace --yolo 2>/dev/null; sleep 30"
sleep 6; tmux -S "$SOCK" send-keys -l -- "$TXT"; sleep 2
tmux -S "$SOCK" capture-pane -p > "$L/screen.txt"
tmux -S "$SOCK" kill-server 2>/dev/null
