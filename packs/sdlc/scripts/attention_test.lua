-- Tests for attention — the session-start hook module that reports the board rows needing
-- attention.
--
--   cargo make lua

local attention = require("attention")
local status = require("lib.status")

local fs = airsstack.fs
local path = airsstack.path

-- Writes a chain holding one approved intent under `<cwd>/<chain root>` and returns `cwd`.
local function project_with_open_chain()
  local cwd = fs.tempdir()
  local chain = path.join(cwd, status.ROOT, "2026-10-13-mfa")
  fs.mkdir(path.join(cwd, ".claudestacks"))
  fs.mkdir(path.join(cwd, ".claudestacks", "sdlc"))
  fs.mkdir(chain)
  fs.write(
    path.join(chain, "intent.md"),
    "---\nstatus: approved\ncreated: 2026-10-13\n---\n# intent\n"
  )
  return cwd
end

return {
  attention_returns_context_for_open_chains = function()
    local cwd = project_with_open_chain()
    local result = attention({ cwd = cwd, args = { "/sdlc-status" } })
    assert(type(result) == "table" and type(result.context) == "string", "want a context result")
    local first = result.context:match("^[^\n]*")
    local want = "claudestacks-sdlc: 1 open, 0 need attention — full board: /sdlc-status"
    assert(first == want, first)
  end,

  attention_returns_nil_without_a_chain_root = function()
    local cwd = fs.tempdir()
    assert(attention({ cwd = cwd, args = { "/sdlc-status" } }) == nil)
  end,
}
