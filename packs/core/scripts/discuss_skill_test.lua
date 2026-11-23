-- Pins the parts of skills/discuss/SKILL.md that a careless edit would break silently: who may
-- invoke it, and how the command lines carry substituted values.
--
--   cargo make lua   (paths are relative to the repository root)

local fs = airsstack.fs

local SKILL = "packs/core/skills/discuss/SKILL.md"

local function text()
  return fs.read(SKILL)
end

return {
  only_the_author_can_invoke_it = function()
    local frontmatter = text():match("^%-%-%-\n(.-)\n%-%-%-\n")
    assert(frontmatter, "no frontmatter")
    -- Wrapped in newlines so the key matches on any line, first and last included.
    assert(("\n" .. frontmatter .. "\n"):find("\ninvocation: user\n"),
      "model invocation not disabled")
  end,

  every_user_config_placeholder_is_single_quoted = function()
    local body = text()
    local count = 0
    for before, after in body:gmatch("(.){{option archive_keep}}(.)") do
      count = count + 1
      assert(before == "'" and after == "'", "unquoted {{option archive_keep}}")
    end
    assert(count >= 1, "the start line does not pass --keep")
  end,

  command_lines_carry_the_session_and_the_plugin_root = function()
    local body = text()
    assert(body:find("--session {{var session_id}}", 1, true))
    assert(body:find('"{{var project_dir}}/{{root}}/scripts/discuss.lua"', 1, true))
    assert(not body:find("CLAUDE_PLUGIN_ROOT", 1, true), "an unsubstituted plugin variable")
  end,

  every_command_is_named = function()
    local body = text()
    -- Each needle is the backticked command as the table rows and bullets write it, so prose
    -- cannot satisfy it. `list`, `show` and `done` are anchored on their table cell because the
    -- bare backticked form recurs elsewhere.
    for _, command in ipairs({ "| ` start --keep", "` add <", "| ` list` |",
      "| ` list --archive` |", "| ` show <", "| ` done` |", "` report-path <" }) do
      assert(body:find(command, 1, true), "missing command:" .. command)
    end
  end,
}
