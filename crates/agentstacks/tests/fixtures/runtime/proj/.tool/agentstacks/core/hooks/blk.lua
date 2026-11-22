return function(ev)
  return { block = "edits to " .. tostring(ev.event) .. " are not allowed" }
end
