#!/bin/bash
# items-run.sh <project_dir> <script.json> <logdir> <prompt> [extra muse args...]
# Fresh HOME per run, mock on port 18782, muse exec from project root. stdout -> <logdir>/stdout, stderr (minus 'bundled') -> <logdir>/stderr
set -u
P="$1"; S="$2"; L="$3"; PROMPT="$4"; shift 4
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1
mkdir -p "$(dirname "$L")"; H="$(mktemp -d "${L%/}.home.XXXX")"; T="$H/tmp"; mkdir -p "$T" "$H/.config/muse" "$L"
printf '{"schema_version":1,"endpoint_transport":{"base_url":"http://127.0.0.1:18782","auth":"bearer"}}' > "$H/.config/muse/settings.json"
printf '{"schema_version":1,"providers":{"meta":{"api_key":"dummy"}}}' > "$H/.config/muse/auth.json"
node "$HERE/mock.mjs" 18782 "$S" "$L" & MP=$!
sleep 0.7
( cd "$P" && env -u XDG_CONFIG_HOME HOME="$H" TMPDIR="$T" AIRSSTACK_HOME=/probe/airs "$BIN" exec --trust-workspace --yolo "$@" "$PROMPT" ) > "$L/stdout" 2> "$L/stderr.raw"
echo "exit=$?" >> "$L/stdout"
kill $MP 2>/dev/null; wait $MP 2>/dev/null
grep -v bundled "$L/stderr.raw" > "$L/stderr"; rm -f "$L/stderr.raw"
echo "HOME=$H"
