-- Tests for rearm — the session-start hook module that re-arms the enforcement pointers.
--
--   cargo make lua

local hook = require("rearm")
local enforce = require("lib.enforce")
local fs = airsstack.fs

-- Runs `body` with `airsstack.env` swapped for a table answering from `values`, and restores the
-- real one afterwards even when `body` raises.
local function with_env(values, body)
  local real = airsstack.env
  airsstack.env = {
    get = function(name)
      return values[name]
    end,
  }
  local ok, err = pcall(body)
  airsstack.env = real
  if not ok then
    error(err, 0)
  end
end

return {
  rearm_clears_this_session_only = function()
    local tmp = fs.tempdir()
    enforce.claim(enforce.sentinel_path(tmp, "s1", "main", "rust", "code"))
    enforce.claim(enforce.sentinel_path(tmp, "s2", "main", "rust", "code"))

    with_env({ TMPDIR = tmp }, function()
      local result = hook({ event = "session.start", session_id = "s1", args = {} })
      assert(result == nil, "a rearm hook returns nothing")
    end)

    assert(#enforce.held_sentinels(tmp, "s1", "main") == 0, "s1 is cleared")
    assert(#enforce.held_sentinels(tmp, "s2", "main") == 1, "s2 is kept")
  end,

  rearm_without_a_session_id_leaves_other_sessions_alone = function()
    local tmp = fs.tempdir()
    enforce.claim(enforce.sentinel_path(tmp, "s2", "main", "rust", "code"))

    with_env({ TMPDIR = tmp }, function()
      assert(hook({ event = "session.start", args = {} }) == nil)
    end)

    assert(#enforce.held_sentinels(tmp, "s2", "main") == 1)
  end,
}
