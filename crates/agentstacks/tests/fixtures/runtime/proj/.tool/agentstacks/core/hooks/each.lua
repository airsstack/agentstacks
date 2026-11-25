return function(ev)
  if ev.path == "quiet" then
    return nil
  end
  local result = { context = "each " .. ev.path }
  if ev.path == "stop" then
    result.block = "stop " .. ev.path
  end
  return result
end
