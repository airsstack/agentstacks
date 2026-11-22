local translate = require("translate")

local function last_nonblank_line(s)
  local last
  for line in s:gmatch("[^\n]+") do
    if line:match("%S") then
      last = line
    end
  end
  return last
end

return {
  text_is_a_non_empty_string = function()
    local text = translate.text()
    assert(type(text) == "string", "text() must return a string")
    assert(#text > 0, "text() must not be empty")
  end,

  text_defines_event_and_result = function()
    local text = translate.text()
    assert(text:find("function M.event(p)", 1, true), "missing function M.event(p)")
    assert(text:find("function M.result(r, p)", 1, true), "missing function M.result(r, p)")
  end,

  text_ends_by_returning_the_module = function()
    assert(last_nonblank_line(translate.text()) == "return M")
  end,
}
