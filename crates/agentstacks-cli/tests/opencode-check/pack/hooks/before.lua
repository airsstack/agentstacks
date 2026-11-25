-- Blocks a write whose path ends in blocked.txt; any other write gets a context line.
-- An event without a string path is left alone.
return function(event)
  local path = event.path
  if type(path) ~= "string" then
    return nil
  end
  if path == "blocked.txt" or path:match("/blocked%.txt$") then
    return { block = "CHECK-BLOCK " .. path }
  end
  return { context = "CHECK-BEFORE " .. path }
end
