# Relative require from a hook module under airsl (plain lib.x)

Run 2026-11-23 in /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/rq. Output pasted verbatim.

~~~~text
$ airsl --version
airsl 0.1.4
--- R/main.lua
local f = require("core.hooks.enforce")
airsstack.stdio.write(f() .. "\n")
--- R/core/hooks/enforce.lua
local lib = require("lib.x")
return function() return "lib=" .. lib.v .. " base=[]" end
--- R/core/hooks/lib/x.lua
return { v = "ok" }
--- R/core/hooks/enforce_test.lua
local f = require("enforce")
return { loads_sibling_lib = function() assert(f() == "lib=ok base=[]", f()) end }
$ airsl run --policy confined R/main.lua
airsl: lua error in R/main.lua: lua error in core.hooks.enforce: module `lib.x` not found under `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/rq/R`
stack traceback:
	[C]: in function 'require'
	...96af-1faf5ebe44a5/scratchpad/rq/R/core/hooks/enforce.lua:1: in main chunk
	[C]: in function 'require'
	R/main.lua:1: in main chunk

stack traceback:
	[C]: in function 'require'
	R/main.lua:1: in main chunk

exit=1
$ airsl test R/core
R/core/hooks/enforce_test.lua
  ok    loads_sibling_lib

1 passed, 0 failed (1 files)
exit=0
~~~~
