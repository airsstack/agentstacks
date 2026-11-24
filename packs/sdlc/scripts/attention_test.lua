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

  attention_falls_back_to_the_process_directory_without_a_cwd = function()
    local want = status.attention(path.join(path.absolute("."), status.ROOT), "/sdlc-status")
    local expected = want ~= "" and { context = want } or nil
    for _, event in ipairs({
      { args = { "/sdlc-status" } },
      { cwd = "", args = { "/sdlc-status" } },
      { cwd = 7, args = { "/sdlc-status" } },
    }) do
      local result = attention(event)
      if expected == nil then
        assert(result == nil, "want nil")
      else
        assert(type(result) == "table" and result.context == expected.context, "want the process directory's board")
      end
    end
  end,

  attention_returns_nil_without_a_chain_root = function()
    local cwd = fs.tempdir()
    assert(attention({ cwd = cwd, args = { "/sdlc-status" } }) == nil)
  end,
}
