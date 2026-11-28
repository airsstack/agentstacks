-- Bump suggestion from Conventional Commit subjects.
--
-- Pure logic with no IO, so `suggest_test.lua` covers it directly. The
-- release wrapper feeds subjects through argv (see below); `require`
-- callers get only the table.

local suggest = {}

--- The kind of one subject line: "feat", "fix", or nil when neither.
---@param subject string one `git log --format=%s` line
---@return string|nil
function suggest.parse_subject(subject)
  local kind = subject:match("^([A-Za-z]+)")
  if kind ~= "feat" and kind ~= "fix" then
    return nil
  end
  local rest = subject:sub(#kind + 1)
  if rest:match("^%b()!?%s*:") or rest:match("^!%s*:") or rest:match("^%s*:") then
    return kind
  end
  return nil
end

--- Suggests a bump for a list of subjects: "minor" when any subject is a
--- feature, "patch" when any (and only) fixes are present, nil otherwise.
--- A suggestion is never "major": sizing up stays a human decision.
---@param subjects string[] subject lines in any order
---@return string|nil
function suggest.from_subjects(subjects)
  local saw_fix = false
  for _, subject in ipairs(subjects) do
    local kind = suggest.parse_subject(subject)
    if kind == "feat" then
      return "minor"
    end
    if kind == "fix" then
      saw_fix = true
    end
  end
  if saw_fix then
    return "patch"
  end
  return nil
end

local function main(args)
  local suggestion = suggest.from_subjects(args)
  if suggestion == nil then
    print("suggestion: none")
  else
    print("suggestion: " .. suggestion)
  end
end

if arg ~= nil and arg[0] ~= nil and arg[0]:sub(-#"suggest.lua") == "suggest.lua" then
  local rest = {}
  for index = 1, #arg do
    rest[#rest + 1] = arg[index]
  end
  main(rest)
end

return suggest
