-- The rule-enforcement pipeline.
--
-- Lists the packs installed beside the core pack, loads the `enforcement.json` each one ships at
-- its root, and — for the file being edited — decides which guideline skill to surface.
--
-- Split from the hook module because this is the part with rules in it, and because the doctor
-- has to drive exactly the same resolution the hook drives. A doctor that reimplemented
-- resolution would eventually disagree with the hook, and would then be lying about the one
-- thing it exists to make trustworthy.
--
-- Fail-open is the runtime's job, not this module's: everything here returns values, and nothing
-- decides what the process does about them.

-- The requires start from this module's own name so the module loads under any prefix the
-- caller's runtime gives it.
local base = (...):match("^(.-)[^.]*$")

local globs = require(base .. "globs")
local fs = airsstack.fs
local json = airsstack.json
local path = airsstack.path
local proc = airsstack.proc
local regex = airsstack.regex

local M = {}

M.SENTINEL_PREFIX = "claudestacks-enforce-"

-- Seconds; a sentinel older than this is pruned rather than kept suppressing its rule forever.
M.SENTINEL_MAX_AGE = 24 * 3600

-- Every character outside [A-Za-z0-9._-] becomes '-', matching `tr -c 'A-Za-z0-9._-' '-'`.
function M.sanitize(text)
  return regex.replace_all("[^A-Za-z0-9._-]", text or "", "-")
end

-- Runs git in `dir`; returns trimmed stdout, or nil on any failure.
--
-- `-C` rather than a working directory on the child: `proc.run` takes an argv array and nothing
-- else, so the directory travels as an argument.
--
-- No production path calls this: the repository root is read off `.git` (`toplevel`), so the
-- hook needs no `--allow-exec git` grant. It survives for `enforce_test.lua`, which builds real
-- repositories with `git init` to check the read path against git's own layout, under the grant
-- `cargo make lua` provides.
function M.git(dir, ...)
  local argv = { "git", "-C", dir }
  for _, value in ipairs({ ... }) do
    argv[#argv + 1] = value
  end
  local ok, result = pcall(proc.run, argv)
  if not ok or result.status ~= 0 then
    return nil
  end
  local text = result.stdout:gsub("%s+$", "")
  return text ~= "" and text or nil
end

-- `fs.canonicalize` where the policy and the filesystem allow it, nil otherwise.
function M.realpath(target)
  local ok, resolved = pcall(fs.canonicalize, target)
  return ok and resolved or nil
end

-- Whether `target` exists, answering false where the question itself is refused.
function M.exists(target)
  local ok, found = pcall(fs.exists, target)
  return ok and found
end

-- Whether `target` is a file.
function M.is_file(target)
  local ok, found = pcall(fs.is_file, target)
  return ok and found
end

-- The nearest ancestor holding `.git`, or nil outside a repository.
--
-- Read rather than run: `--allow-exec git` cannot be passed from a worktree-isolated Claude Code
-- session (the core pack's context-handoff skill gives the full account in its
-- `references/protocol.md`), and this runs on every hook dispatch. Ascends because the hook's
-- cwd is rarely the repository root.
local function dot_git(cwd)
  local current = M.realpath(cwd) or cwd
  while true do
    local candidate = path.join(current, ".git")
    -- Guarded: the ascent leaves the granted read root as soon as the caller is below it. A
    -- denial means "cannot see", so the walk continues rather than raising through the hook.
    local seen, present = pcall(fs.exists, candidate)
    if seen and present then
      return current
    end
    local parent = path.dirname(current)
    if parent == current or parent == "" then
      return nil
    end
    current = parent
  end
end

-- The worktree root: what `rev-parse --show-toplevel` answers, without running git.
function M.toplevel(cwd)
  return dot_git(cwd)
end

-- The path `match` globs are tested against.
--
-- Inside a repository: the path relative to the git toplevel, which is what `match` is documented
-- to mean. Outside any repository: the basename, which preserves coverage there rather than
-- silently dropping it.
function M.path_for_matching(file_path, cwd)
  local absolute = path.absolute(file_path)
  local target = M.realpath(absolute) or absolute

  local top = M.toplevel(cwd)
  if top then
    top = M.realpath(top) or top
    if target == top then
      return "."
    end
    if target:sub(1, #top + 1) == top .. "/" then
      return target:sub(#top + 2)
    end
  end
  return path.basename(target)
end

-- The HOME-global SDD artifact root.
function M.sdd_root(home)
  return path.join(home, "cc", "plugins", "sdd")
end

-- Whether `file_path` is an SDD spec or plan under the HOME-global root.
--
-- The directory name must appear as a whole path SEGMENT. A substring check on '/specs/' matched
-- `<key>/a/specs/b/plans/c.md` by accident, which is a phase misclassification rather than a
-- cosmetic slip: it swaps the Definition of Done for the architecture rules.
function M.is_design_doc(file_path, home)
  local target = path.absolute(file_path)
  local root = path.absolute(M.sdd_root(home))
  if target:sub(1, #root + 1) ~= root .. "/" then
    return false
  end

  local relative = target:sub(#root + 2)
  local segments = {}
  for segment in relative:gmatch("[^/]+") do
    segments[#segments + 1] = segment
  end
  -- Directories only, never the filename; the layout is <key>/<specs|plans>/...
  if #segments - 1 < 2 then
    return false
  end
  return segments[2] == "specs" or segments[2] == "plans"
end

-- One pack's enforcement.json, validated, or nil.
function M.load_manifest(install_path)
  local ok, text = pcall(fs.read, path.join(install_path, "enforcement.json"))
  if not ok then
    return nil -- absent or unreadable: skip this pack, keep the rest
  end
  local decoded, data = pcall(json.decode, text)
  if not decoded or type(data) ~= "table" then
    return nil
  end
  if type(data.stack) ~= "string" or data.stack == ""
    or type(data.skill) ~= "string" or data.skill == "" then
    return nil
  end

  local function list(value)
    return type(value) == "table" and value or {}
  end

  local phase = list(data.phase)
  if #phase == 0 then
    phase = { "code", "design" }
  end

  return {
    stack = data.stack,
    skill = data.skill,
    detect = list(data.detect),
    match = list(data.match),
    phase = phase,
  }
end

-- Whether `manifest` declares `phase`.
function M.declares_phase(manifest, phase)
  for _, declared in ipairs(manifest.phase) do
    if declared == phase then
      return true
    end
  end
  return false
end

-- Whether a `detect` marker sits in `directory` or any ancestor.
--
-- Split from `marker_active` because the two phases anchor differently: a code file anchors on
-- itself, while an SDD design doc lives under AIRSSTACK_HOME and has no in-repo location to
-- anchor on, so it anchors on the working directory.
function M.marker_active_in(directory, markers)
  if #markers == 0 then
    return false
  end
  local current = path.absolute(directory ~= "" and directory or ".")
  while true do
    for _, marker in ipairs(markers) do
      if M.is_file(path.join(current, tostring(marker))) then
        return true
      end
    end
    local parent = path.dirname(current)
    if parent == current then
      return false
    end
    current = parent
  end
end

-- Whether a `detect` marker sits at or above the FILE's own directory.
--
-- The file's directory, not the session's: searching upward from `cwd` is wrong for any file
-- outside it, and `cwd` survives only as the fallback when the path has no directory component.
function M.marker_active(file_path, markers, cwd)
  local directory = path.dirname(path.absolute(file_path))
  if directory == "" then
    directory = cwd or "."
  end
  return M.marker_active_in(directory, markers)
end

-- The injected text for one stack and phase.
--
-- The two phases ask for different things. Code phase runs a build, so the Definition of Done
-- applies. A design doc is a spec or plan with nothing to build, so naming the Definition of Done
-- there is a demand the reader cannot satisfy; only the architecture rules can shape a design.
function M.pointer(stack, skill, phase)
  local tail = phase == "design"
    and "apply its architecture rules to this design."
    or "apply its rules (Definition of Done + architecture)."
  return stack .. " work is in play. The " .. skill .. " skill is MANDATORY for "
    .. "this work \u{2014} load it now via Skill before proceeding, and " .. tail
end

-- Where one-shot sentinels live.
function M.sentinel_dir(env)
  local override = env.get("TMPDIR")
  if override and override ~= "" then
    return override
  end
  return "/tmp"
end

-- One sentinel per (session, agent context, stack, phase).
--
-- `agent` is the subagent id when the hook fires inside one, else 'main'. Subagents inherit the
-- parent's session_id, so without that component an explorer reading one .rs file would consume
-- the main thread's only pointer — and the main thread is exactly the context this exists to
-- inform.
function M.sentinel_path(directory, session_id, agent, stack, phase)
  local parts = {
    M.sanitize(session_id ~= "" and session_id or "nosession"),
    M.sanitize(agent ~= "" and agent or "main"),
    M.sanitize(stack),
    M.sanitize(phase),
  }
  return path.join(directory, M.SENTINEL_PREFIX .. table.concat(parts, "-"))
end

-- Read-only probe for the cheap gate; never creates anything.
function M.sentinel_claimed(file)
  return M.exists(file)
end

-- Atomically claims a sentinel. True means this invocation must emit.
--
-- `create_exclusive` is O_CREAT|O_EXCL, atomic by construction, so no locking is needed; a
-- read-then-append claim would be an unguarded read-modify-write that lets concurrent hooks all
-- fire. A failure that is not "already there" returns true — a repeated pointer is better than a
-- silently suppressed one.
function M.claim(file)
  local ok, created = pcall(fs.create_exclusive, file)
  if not ok then
    return true
  end
  return created
end

-- Unlinks sentinels older than `SENTINEL_MAX_AGE`.
function M.prune_sentinels(directory, now)
  local ok, names = pcall(fs.list, directory)
  if not ok then
    return
  end
  for _, name in ipairs(names) do
    if name:sub(1, #M.SENTINEL_PREFIX) == M.SENTINEL_PREFIX then
      local file = path.join(directory, name)
      local read, stamp = pcall(fs.stat, file)
      if read and now - stamp.modified > M.SENTINEL_MAX_AGE then
        pcall(fs.remove, file)
      end
    end
  end
end

-- Unlinks every sentinel for one session; returns the count removed.
function M.clear_session(directory, session_id)
  local prefix = M.SENTINEL_PREFIX
    .. M.sanitize(session_id ~= "" and session_id or "nosession") .. "-"
  local removed = 0
  local ok, names = pcall(fs.list, directory)
  if not ok then
    return 0
  end
  for _, name in ipairs(names) do
    if name:sub(1, #prefix) == prefix and pcall(fs.remove, path.join(directory, name)) then
      removed = removed + 1
    end
  end
  return removed
end

-- The stack:phase keys already claimed for one session and agent context.
function M.held_sentinels(directory, session_id, agent)
  local prefix = M.SENTINEL_PREFIX .. M.sanitize(session_id ~= "" and session_id or "nosession")
    .. "-" .. M.sanitize(agent ~= "" and agent or "main") .. "-"
  local held = {}
  local ok, names = pcall(fs.list, directory)
  if not ok then
    return held
  end
  for _, name in ipairs(names) do
    if name:sub(1, #prefix) == prefix then
      held[#held + 1] = (name:sub(#prefix + 1):gsub("%-", ":", 1))
    end
  end
  table.sort(held)
  return held
end

-- The directories under `packs_root` that hold an `enforcement.json`, as their names in name
-- order. A directory without the file is not listed; one whose file does not load is, and is
-- dropped later by `load_manifest`.
local function listed_packs(packs_root)
  local ok, names = pcall(fs.list, packs_root)
  if not ok then
    return {}
  end
  table.sort(names)
  local listed = {}
  for _, name in ipairs(names) do
    if M.is_file(path.join(packs_root, name, "enforcement.json")) then
      listed[#listed + 1] = name
    end
  end
  return listed
end

-- The ordered resolution pipeline. Returns the pointers to emit.
--
-- `context` carries `file_path`, `cwd`, `session_id`, `agent`, `pack_root`, `home` and
-- `sentinel_dir`. `pack_root` is the core pack's own root; the candidates are the packs beside
-- it, so its parent is the packs root. `trace`, when a table is passed, collects one line per
-- stage so the doctor can explain any of the silent-exit paths without reimplementing this.
function M.resolve(context, trace)
  local function note(line)
    if trace then
      trace[#trace + 1] = line
    end
  end

  local packs_root = path.dirname(context.pack_root)
  local names = listed_packs(packs_root)
  note("packs: " .. #names .. " with enforcement.json under " .. packs_root)

  local candidates = {}
  for _, name in ipairs(names) do
    local manifest = M.load_manifest(path.join(packs_root, name))
    if manifest then
      candidates[#candidates + 1] = { key = name, manifest = manifest }
    else
      note(name .. ": no usable enforcement.json")
    end
  end
  note("manifests: " .. #candidates .. " loaded")
  if #candidates == 0 then
    note("STOP: no pack ships a usable enforcement.json")
    return {}
  end

  local phase = M.is_design_doc(context.file_path, context.home) and "design" or "code"
  note("phase: " .. phase)

  -- CHEAP GATE: if every key this event could produce is already claimed, stop before reading
  -- anything of the project.
  local wanted = {}
  for _, candidate in ipairs(candidates) do
    if M.declares_phase(candidate.manifest, phase) then
      wanted[#wanted + 1] = candidate
    end
  end
  if #wanted == 0 then
    note("STOP: no manifest declares phase " .. phase)
    return {}
  end

  local unclaimed = {}
  for _, candidate in ipairs(wanted) do
    local sentinel = M.sentinel_path(context.sentinel_dir, context.session_id, context.agent,
      candidate.manifest.stack, phase)
    if not M.sentinel_claimed(sentinel) then
      unclaimed[#unclaimed + 1] = candidate
    end
  end
  if #unclaimed == 0 then
    note("STOP: every candidate stack:phase already claimed this context")
    return {}
  end

  local candidate_path = phase == "code"
    and M.path_for_matching(context.file_path, context.cwd)
    or nil
  if candidate_path then
    note("match path: " .. candidate_path)
  end

  local pointers = {}
  for _, candidate in ipairs(unclaimed) do
    local manifest = candidate.manifest

    local active
    if phase == "code" then
      active = M.marker_active(context.file_path, manifest.detect, context.cwd)
    else
      active = M.marker_active_in(context.cwd, manifest.detect)
    end

    if not active then
      note(candidate.key .. ": GATE 2 no detect marker")
    elseif phase == "code" and not globs.matches_any(candidate_path, manifest.match) then
      note(candidate.key .. ": GATE 3 no match glob hit")
    elseif not M.claim(M.sentinel_path(context.sentinel_dir, context.session_id, context.agent,
      manifest.stack, phase)) then
      note(candidate.key .. ": sentinel claimed concurrently")
    else
      note(candidate.key .. ": EMIT " .. manifest.skill)
      pointers[#pointers + 1] = M.pointer(manifest.stack, manifest.skill, phase)
    end
  end
  return pointers
end

-- The home the SDD artifact root hangs off: `AIRSSTACK_HOME`, else `$HOME/.airsstack`.
function M.home(env)
  local home = env.get("AIRSSTACK_HOME")
  if not home or home == "" then
    home = path.join(env.get("HOME") or "", ".airsstack")
  end
  return home
end

-- The doctor: a human-readable trace of the resolution pipeline for one path.
--
-- Drives the same `resolve` the hook drives, with a trace collector attached. The framework has
-- several paths that end in silence and are mutually indistinguishable from outside; this names
-- the one that was taken. `pack_root` is the core pack's own root, as the hook receives it.
--
-- `resolve` claims sentinels as a side effect, so a second doctor run for the same
-- session/agent/stack/phase reports "already claimed" from the doctor's own prior run.
function M.explain(pack_root, file_path, env)
  local home = M.home(env)
  local sentinel_dir = M.sentinel_dir(env)
  local cwd = path.absolute(".")
  local session, agent = "enforce-doctor", "doctor"

  local lines = {
    "runtime:  airsl (Lua)",
    "path:     " .. path.absolute(file_path),
    "cwd:      " .. cwd,
    "packs:    " .. path.dirname(pack_root),
    "sdd root: " .. M.sdd_root(home),
    "sentinel dir: " .. sentinel_dir,
  }

  local held = M.held_sentinels(sentinel_dir, session, agent)
  lines[#lines + 1] = "sentinels held: " .. (#held > 0 and table.concat(held, ", ") or "none")

  local trace = {}
  local pointers = M.resolve({
    file_path = file_path,
    cwd = cwd,
    session_id = session,
    agent = agent,
    pack_root = pack_root,
    home = home,
    sentinel_dir = sentinel_dir,
  }, trace)

  for _, entry in ipairs(trace) do
    lines[#lines + 1] = "  " .. entry
  end
  lines[#lines + 1] = "outcome: " .. #pointers .. " pointer(s)"
  for _, text in ipairs(pointers) do
    lines[#lines + 1] = "  -> " .. text
  end

  return table.concat(lines, "\n") .. "\n"
end

-- The doctor script's whole behaviour: `args` is `{ <pack-root>, <path> }`. Returns `out, err`:
-- the explanation and nil, or nil and the usage text when either argument is missing or empty.
function M.doctor(args, env)
  local pack_root, file_path = args[1], args[2]
  if type(pack_root) ~= "string" or pack_root == "" or type(file_path) ~= "string"
    or file_path == "" then
    return nil, "usage: doctor.lua <pack-root> <path>\n"
  end
  return M.explain(pack_root, file_path, env), nil
end

return M
