-- Rule-enforcement hook — runs before a Read, Edit or Write.
--
-- Loads the `enforcement.json` of every pack installed beside the core pack and, for the file
-- being touched, returns the matching guideline skill's pointer as context. args[1] is the core
-- pack's own root; the candidates are the directories beside it.
--
-- Never blocks: it returns `{ context = ... }` or nothing.
--
-- No `--allow-exec git`: the repository root is read off `.git` (`lib/enforce.lua`'s `toplevel`)
-- rather than asked of git, so this runs unchanged in a worktree-isolated agent session, whose
-- guard refuses any command carrying a `git` operand it cannot see through.

-- The requires start from this module's own name so the module loads under any prefix the
-- caller's runtime gives it.
local base = (...):match("^(.-)[^.]*$")

local enforce = require(base .. "lib.enforce")
local path = airsstack.path

return function(event)
  local file_path = event.path
  if type(file_path) ~= "string" or file_path == "" then
    return nil
  end

  local pack_root = event.args and event.args[1]
  if type(pack_root) ~= "string" or pack_root == "" then
    return nil
  end

  local env = airsstack.env
  local sentinel_dir = enforce.sentinel_dir(env)
  local cwd = type(event.cwd) == "string" and event.cwd ~= "" and event.cwd or path.absolute(".")

  enforce.prune_sentinels(sentinel_dir, airsstack.time.now())

  local pointers = enforce.resolve({
    file_path = file_path,
    cwd = cwd,
    session_id = type(event.session_id) == "string" and event.session_id or "",
    agent = type(event.agent_id) == "string" and event.agent_id ~= "" and event.agent_id or "main",
    pack_root = pack_root,
    home = enforce.home(env),
    sentinel_dir = sentinel_dir,
  })

  if #pointers == 0 then
    return nil
  end
  return { context = table.concat(pointers, "\n") }
end
