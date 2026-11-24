-- Tests for session-start — the session-start hook module that provisions the vault, refreshes
-- its index and returns the orientation card.
--
--   cargo make lua

local session_start = require("session-start")

local fs = airsstack.fs
local path = airsstack.path

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
  session_start_provisions_an_empty_vault = function()
    local home = fs.tempdir()

    with_env({ AIRSSTACK_HOME = home }, function()
      session_start({ event = "session.start", args = {} })
    end)

    local vault_root = path.join(home, "journal")
    for _, name in ipairs({ "daily", "sessions", "notes", "mocs", ".index" }) do
      assert(fs.is_dir(path.join(vault_root, name)), name .. " is created under the vault root")
    end
  end,

  session_start_on_an_empty_vault_returns_nil = function()
    local home = fs.tempdir()

    with_env({ AIRSSTACK_HOME = home }, function()
      local result = session_start({ event = "session.start", args = {} })
      assert(result == nil, "no session or note matches, so there is no card")
    end)
  end,
}
