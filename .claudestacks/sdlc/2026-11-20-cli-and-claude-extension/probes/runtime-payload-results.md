# runtime.lua with translate.result(result, payload) (airsl 0.1.4, 2026-11-22)

Change against crates/agentstacks/assets/runtime.lua (commit 8107626):
```diff
26c26
< local out = translate.result(result)
---
> local out = translate.result(result, payload)
```
`translate.lua`:
```lua
local M = {}
function M.event(p) return { event = "tool.before", session_id = p.session_id } end
function M.result(r, p)
  if r == nil then return nil end
  return airsstack.json.encode({ event = p.hook_event_name, context = r.context }) .. "\n"
end
return M
```
`core/hooks/h.lua`: `return function(e) return { context = "ctx" } end`

```
$ echo '{"hook_event_name":"PreToolUse","session_id":"s1"}' | airsl run --fail-open --policy confined runtime.lua core.hooks.h 0 neutral
{"context":"ctx","event":"PreToolUse"}
exit=0
$ echo '{"hook_event_name":"PreToolUse","session_id":"s1"}' | airsl run --fail-open --policy confined runtime.lua core.hooks.h 0 native
{"context":"ctx","event":"PreToolUse"}
exit=0
```
