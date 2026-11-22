# airsl test probe (airsl 0.1.4, 2026-11-22)

## ext/lib/quote.lua
```lua
local M = {}
function M.yaml(s) return airsstack.json.encode(s) end
return M
```
## ext/lib/quote_test.lua
```lua
local quote = require("quote")
return { encodes_quotes = function() assert(quote.yaml('a "q"') == '"a \\"q\\""') end }
```
## ext/root_test.lua
```lua
local quote = require("lib.quote")
return { from_root = function() assert(quote.yaml("x") == '"x"') end,
         fails = function() assert(false, "expected failure") end }
```
## output
```
$ airsl test lib/quote_test.lua
lib/quote_test.lua
  ok    encodes_quotes

1 passed, 0 failed (1 files)
exit=0
$ airsl test root_test.lua
root_test.lua
  FAIL  fails
  ok    from_root

1 passed, 1 failed (1 files)

root_test.lua: fails
    runtime error: root_test.lua:3: expected failure
stack traceback:
	[C]: in ?
	[C]: in function 'assert'
	root_test.lua:3: in function <root_test.lua:3>
exit=1
$ airsl test .
./lib/quote_test.lua
  ok    encodes_quotes
./root_test.lua
  FAIL  fails
  ok    from_root

2 passed, 1 failed (2 files)

./root_test.lua: fails
    runtime error: ./root_test.lua:3: expected failure
stack traceback:
	[C]: in ?
	[C]: in function 'assert'
	./root_test.lua:3: in function <./root_test.lua:3>
exit=1
```
