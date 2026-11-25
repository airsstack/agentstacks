#!/bin/bash
# R1 + R2 probes. usage: final-run.sh <muse-binary> <scratch-dir>
M=$1; S=$2
echo "### R1 — hook command wrapped in /bin/sh -c, under three login shells"
R=$S/r1; rm -rf $R; mkdir -p $R/home $R/tmp $R/proj/.muse; cd $R/proj && git init -q .
INNER='"printf" "%s|%s|%s\n" "${TMPDIR:-/tmp}/x" "it'"'"'s" "$(pwd)"'
ESC=$(printf '%s' "$INNER" | sed "s/'/'\\\\''/g")
CMD="/bin/sh -c '$ESC > r1-out.txt' # agentstacks"
python3 -I -c 'import json,sys; print(json.dumps({"hooks":{"SessionStart":[{"matcher":"startup","hooks":[{"type":"command","command":sys.argv[1]}]}]}},indent=2))' "$CMD" > .muse/hooks.json
echo '$ cat .muse/hooks.json'; cat .muse/hooks.json
for sh in /bin/zsh /bin/sh /bin/bash; do rm -f r1-out.txt
  echo "\$ SHELL=$sh TBH_CREDENTIAL_BACKEND=file muse exec --provider echo --trust-workspace r1"
  env -u XDG_CONFIG_HOME HOME=$R/home TMPDIR=$R/tmp SHELL=$sh TBH_CREDENTIAL_BACKEND=file $M exec --provider echo --trust-workspace "r1" 2>&1 | grep -v bundled | sed "s#$S#W#g"
  echo "\$ cat r1-out.txt"; sed "s#$S#W#g" r1-out.txt; done
echo; echo "### R2 — SubagentStop {decision: block} (first stop only)"
R=$S/r2; rm -rf $R; mkdir -p $R/home $R/tmp $R/proj/.muse $R/out; cd $R/proj && git init -q .
printf '%s\n' '#!/bin/sh' 'n=$(ls "$1" | wc -l | tr -d " "); cat > "$1/stop-$n.json"' 'if [ "$n" = 0 ]; then printf '"'"'{"decision":"block","reason":"KEEPGOING-R2"}\n'"'"'; fi' > $R/stop.sh; chmod +x $R/stop.sh
echo '$ cat stop.sh'; cat $R/stop.sh
python3 -I -c 'import json,sys; print(json.dumps({"hooks":{"SubagentStop":[{"hooks":[{"type":"command","command":sys.argv[1]+" "+sys.argv[2]}]}]}}))' $R/stop.sh $R/out > .muse/hooks.json
echo '$ TBH_CREDENTIAL_BACKEND=file muse exec --provider echo --trust-workspace r2'
env -u XDG_CONFIG_HOME HOME=$R/home TMPDIR=$R/tmp TBH_CREDENTIAL_BACKEND=file $M exec --provider echo --trust-workspace "r2" 2>&1 | grep -v bundled | sed "s#$S#W#g"; echo "exit=${PIPESTATUS[0]}"
echo '$ per stop payload: file subagent_id stop_hook_active'
for f in $(ls $R/out | sort -t- -k2 -n); do python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); print(sys.argv[1].split("/")[-1], d.get("subagent_id"), d.get("stop_hook_active"))' $R/out/$f; done
rm -rf $S/r1 $S/r2
