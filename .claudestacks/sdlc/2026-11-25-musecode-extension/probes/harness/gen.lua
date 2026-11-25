-- Builds the two hook command strings exactly as spec §4.4 says (claude quote_argument and
-- message_command, plus the /bin/sh -c wrapper), and prints them as JSON {lua=, msg=}.
local MARKER = " # agentstacks"
local function quote_argument(arg)
  return '"' .. arg:gsub("[\\\"`]", "\\%0") .. '"'
end
local function lua_command(argv)
  local quoted = {}
  for i, arg in ipairs(argv) do quoted[i] = quote_argument(arg) end
  local inner = table.concat(quoted, " ")
  return "/bin/sh -c '" .. inner:gsub("'", "'\\''") .. "'" .. MARKER
end
local function message_command(event, text)
  local json = airsstack.json.encode({ hookSpecificOutput = { hookEventName = event, additionalContext = text } })
  local escaped = json:gsub("'", "'\"'\"'")
  return "printf '%s\\n' '" .. escaped .. "'" .. MARKER
end
local argv = { "touch", "${TMPDIR:-/tmp}/it's \"q\" `b` \\z" }
local text = "MSG-R8 line1\nline2 it's \"q\" back\\slash"
airsstack.stdio.write(airsstack.json.encode({ lua = lua_command(argv), msg = message_command("SessionStart", text), text = text }) .. "\n")
