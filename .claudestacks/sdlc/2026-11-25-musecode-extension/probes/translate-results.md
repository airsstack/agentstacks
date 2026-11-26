# Translator run — the spec §4.7 text under crates/agentstacks/assets/runtime.lua, airsl 0.1.5

Setup: `<root>` holds `runtime.lua` (copied from `crates/agentstacks/assets/runtime.lua` at
`33e3925`), `translate.lua` (the `lua` block of `spec.md` §4.7, extracted by script) and
`h/probe.lua`:

```lua
return function(event)
  if event.event == "tool.before" and event.tool == "write" then
    return { block = "B " .. tostring(event.path) }
  end
  return { context = "C " .. event.event .. " " .. tostring(event.path or event.source or event.prompt or event.last_message) }
end
```

Payloads are the stdin parts of the raw dumps: `pre-write.json` = `raw/out3/PreToolUse.13894`
(`tool_input.path` `"b.txt"`), `post-read.json` = `raw/out3/PostToolUse.13874`, `start.json` =
`raw/out-root/SessionStart.9907`, `stop.json` = `raw/out-root/SubagentStop.9929`. The SubagentStop
result is a context only, which `M.result` drops (no output).

Re-run after the spec's §4.7 text gained the empty-path and empty-cwd guards (`airsl check`, then the four runs):

```text
$ airsl check translate.lua
1 file(s) compiled, 0 failed
$ airsl run --fail-open --policy confined --allow-read <root> <root>/runtime.lua h.probe 1 neutral < pre-write.json
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"B W/probe/p3/b.txt"}}
exit=0
$ airsl run --fail-open --policy confined --allow-read <root> <root>/runtime.lua h.probe 1 neutral < post-read.json
{"hookSpecificOutput":{"additionalContext":"C tool.after W/probe/p3/a.txt","hookEventName":"PostToolUse"}}
exit=0
$ airsl run --fail-open --policy confined --allow-read <root> <root>/runtime.lua h.probe 1 neutral < start.json
{"hookSpecificOutput":{"additionalContext":"C session.start startup","hookEventName":"SessionStart"}}
exit=0
$ airsl run --fail-open --policy confined --allow-read <root> <root>/runtime.lua h.probe 1 neutral < stop.json
exit=0
```
