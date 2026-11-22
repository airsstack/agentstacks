#!/bin/sh
# Probe: the agentstacks hook runtime design on the airsl binary (cwd = project dir, relative paths).
cd /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-runtime/proj || exit 1
printf 'hello\n' > .tool/agentstacks/core/data.txt
R=.tool/agentstacks/runtime.lua
P='{"hook_event_name":"PreToolUse","session_id":"s1","cwd":"/p","tool_name":"Edit"}'
run() {
  echo "\$ echo '$P' | airsl run $*"
  echo "$P" | airsl run "$@" 2>/tmp/probe-runtime-stderr.txt
  echo "exit=$?"
  echo "stderr: $(cat /tmp/probe-runtime-stderr.txt)"
}
airsl --version
run --fail-open --policy confined --allow-read .tool/agentstacks/core $R core.hooks.ctx 0 neutral --manifest x.json
run --fail-open --policy confined $R core.hooks.ctx 0 neutral
run --fail-open --policy confined $R core.hooks.blk 0 neutral
run --fail-open --policy confined $R core.hooks.blk 1 neutral
run --fail-open --policy confined $R core.hooks.raw 0 native
run --fail-open --policy confined $R core.hooks.missing 0 neutral
run --policy confined $R core.hooks.missing 0 neutral
run --fail-open --policy confined $R core.hooks.both 0 neutral
P='{"hook_event_name":"Notification","session_id":"s1","cwd":"/p"}'
run --policy confined $R core.hooks.ctx 0 neutral
rm -f /tmp/probe-runtime-stderr.txt
