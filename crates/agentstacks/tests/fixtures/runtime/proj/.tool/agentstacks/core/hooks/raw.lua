return function(ev)
  return { context = "native payload tool_name=" .. tostring(ev.payload.tool_name) .. " event=" .. tostring(ev.event) }
end
