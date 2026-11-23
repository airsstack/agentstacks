# airsl patched to pass the module name to a required chunk

Run 2026-11-23. Scratch copy of /Users/hiraq/Projects/airsstack/airsl (no .git, no target) at /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/airsl-patched.
The one change is in crates/airsl/src/require_loader.rs. Output pasted verbatim.

~~~~text
$ diff -u require_loader.rs.orig require_loader.rs
--- require_loader.rs.orig	2026-11-23 17:04:34
+++ require_loader.rs	2026-11-23 17:04:34
@@ -199,7 +199,7 @@
 
     lua.load(&source)
         .set_name(format!("@{}", to_script_string(path)))
-        .eval::<Value>()
+        .call::<Value>(target.as_str())
         .map_err(|source| Error::lua(target.as_str(), source))
 }
 
$ cargo build -q -p airsl-cli
$ /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/airsl-target/debug/airsl --version
airsl 0.1.4
--- R/main.lua
local f = require("core.hooks.enforce")
airsstack.stdio.write(f() .. "\n")
--- R/core/hooks/enforce.lua
local base = (...):match("^(.-)[^.]*$")
local lib = require(base .. "lib.x")
return function() return "lib=" .. lib.v .. " base=[" .. base .. "]" end
--- R/core/hooks/lib/x.lua
local base = (...):match("^(.-)[^.]*$")
return { v = "ok via " .. base .. "x" }
--- R/core/hooks/enforce_test.lua
local f = require("enforce")
return { loads_sibling_lib = function() assert(f() == "lib=ok via lib.x base=[]", f()) end }
$ /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/airsl-target/debug/airsl run --policy confined R/main.lua
lib=ok via core.hooks.lib.x base=[core.hooks.]
exit=0
$ /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/airsl-target/debug/airsl test R/core
R/core/hooks/enforce_test.lua
  ok    loads_sibling_lib

1 passed, 0 failed (1 files)
exit=0
$ cargo test -q -p airsl --lib   (patched tree)
...............................
test result: ok. 553 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

~~~~
