# airsstack.hook.payload() called twice (airsl 0.1.4, 2026-11-22)

`twice.lua`:
```lua
local a = airsstack.hook.payload()
local b = airsstack.hook.payload()
airsstack.stdio.write(tostring(a and a.hook_event_name) .. " " .. tostring(b and b.hook_event_name) .. "\n")
```

```
$ echo '{"hook_event_name":"PreToolUse"}' | airsl run --fail-open --policy confined twice.lua
PreToolUse nil
exit=0
```

Finding: the second call returns nil; the payload is readable once.
