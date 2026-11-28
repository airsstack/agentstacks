-- Renders the agentstacks-sdlc status board for the current repository.
-- When the canonical root is absent but the legacy root exists, points at
-- the setup skill instead of printing an empty board.
--
--   airsl run --fail-open --policy confined --allow-read . scripts/status.lua

local status = require("lib.status")
local path = airsstack.path

local cwd = path.absolute(".")
local hint = status.migration_hint(cwd)
if hint then
  airsstack.stdio.write(hint .. "\n")
else
  airsstack.stdio.write(
    status.render(path.join(cwd, status.ROOT)) .. "\n")
end
