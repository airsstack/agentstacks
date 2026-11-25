#!/bin/bash
# R11: which hook event names Muse accepts in .muse/hooks.json (echo provider; a warning names a refused event).
# usage: events-run.sh <muse-binary> <scratch-dir>
M=$1; S=$2; R=$S/r11; rm -rf $R; mkdir -p $R/home $R/tmp $R/proj/.muse; cd $R/proj && git init -q .
EV="SessionStart UserPromptSubmit PreToolUse PermissionRequest PostToolUse PostToolUseFailure PostToolBatch PreLLMCall PostLLMCall PreCompact PostCompact SubagentStart SubagentStop Stop StopFailure SessionEnd Notification SessionFork ToolUseStart OnSave"
python3 -I -c 'import json,sys; print(json.dumps({"hooks":{e:[{"hooks":[{"type":"command","command":"true"}]}] for e in sys.argv[1:]}}))' $EV > .muse/hooks.json
echo "\$ events in .muse/hooks.json: $EV"
echo '$ TBH_CREDENTIAL_BACKEND=file muse exec --provider echo --trust-workspace r11'
env -u XDG_CONFIG_HOME HOME=$R/home TMPDIR=$R/tmp TBH_CREDENTIAL_BACKEND=file "$M" exec --provider echo --trust-workspace r11 2>&1 | grep -v bundled | sed "s#$S#W#g"; echo "exit=${PIPESTATUS[0]}"
rm -rf $R
