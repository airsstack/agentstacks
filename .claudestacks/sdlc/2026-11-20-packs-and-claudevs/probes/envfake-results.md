# A test can swap airsstack.env for a fake

Run 2026-11-23 in /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/envfake with the installed airsl. Output pasted verbatim.

~~~~text
$ airsl --version
airsl 0.1.4
--- t/lib/vaultish.lua
local M = {}
function M.home() return airsstack.env.get("AIRSSTACK_HOME") end
return M
--- t/env_test.lua
local v = require("lib.vaultish")
return {
  real_env_get_without_grant = function()
    local ok, err = pcall(airsstack.env.get, "AIRSSTACK_HOME")
    airsstack.stdio.write("real: ok=" .. tostring(ok) .. " err=" .. tostring(err) .. "\n")
  end,
  fake_env_is_seen_by_a_module = function()
    local real = airsstack.env
    airsstack.env = { get = function(n) if n == "AIRSSTACK_HOME" then return "/fake/home" end end }
    local got = v.home()
    airsstack.env = real
    airsstack.stdio.write("fake: " .. tostring(got) .. "\n")
    assert(got == "/fake/home", tostring(got))
  end,
}
$ airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" --allow-exec git t
t/env_test.lua
fake: /fake/home
  ok    fake_env_is_seen_by_a_module
real: ok=false err=env.get denied: `AIRSSTACK_HOME` is not granted — no environment variables are
stack traceback:
	[C]: in ?
	[C]: in function 'pcall'
	t/env_test.lua:4: in function <t/env_test.lua:3>

  ok    real_env_get_without_grant

2 passed, 0 failed (1 files)
exit=0
~~~~
