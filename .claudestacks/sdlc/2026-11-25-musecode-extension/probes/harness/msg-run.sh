#!/bin/bash
# R3: a message hook command in the claude extension's printf form reaches the model.
# usage: msg-run.sh <muse-binary> <scratch-dir> <port>
M=$1; S=$2; P=$3; HERE=$(cd "$(dirname "$0")" && pwd)
R=$S/r3; rm -rf $R; mkdir -p $R/home/.config/muse $R/tmp $R/proj/.muse; cd $R/proj && git init -q .
printf '{"schema_version":1,"endpoint_transport":{"base_url":"http://127.0.0.1:%s","auth":"bearer"}}' "$P" > $R/home/.config/muse/settings.json
printf '{"schema_version":1,"providers":{"meta":{"api_key":"dummy"}}}' > $R/home/.config/muse/auth.json
JSON='{"hookSpecificOutput":{"additionalContext":"MSG-R3 it'"'"'s \"q\"","hookEventName":"SessionStart"}}'
ESC=$(printf '%s' "$JSON" | sed "s/'/'\"'\"'/g")
CMD="printf '%s\\n' '$ESC' # agentstacks"
python3 -I -c 'import json,sys; print(json.dumps({"hooks":{"SessionStart":[{"matcher":"startup","hooks":[{"type":"command","command":sys.argv[1]}]}]}},indent=2))' "$CMD" > .muse/hooks.json
echo '$ cat .muse/hooks.json'; cat .muse/hooks.json
echo '[]' > $R/script.json
node "$HERE/mock.mjs" "$P" $R/script.json $R/log & MP=$!
for i in 1 2 3 4 5 6 7 8 9 10; do curl -s -o /dev/null "http://127.0.0.1:$P/muse-code/models" && [ -d $R/log ] && break; sleep 0.5; done
[ -d $R/log ] || { echo "mock not listening on $P"; kill $MP; exit 1; }
echo "\$ SHELL=/bin/zsh TBH_CREDENTIAL_BACKEND=file muse exec --trust-workspace --yolo r3"
env -u XDG_CONFIG_HOME HOME=$R/home TMPDIR=$R/tmp SHELL=/bin/zsh TBH_CREDENTIAL_BACKEND=file "$M" exec --trust-workspace --yolo "r3" 2>&1 | grep -v bundled | sed "s#$S#W#g"
kill $MP
echo '$ first request: context around MSG-R3'
python3 -I -c 'import sys; s=open(sys.argv[1]).read(); i=s.find("MSG-R3"); print(s[max(0,i-60):i+40] if i>=0 else "ABSENT")' $R/log/req-001.json
rm -rf $R
