-- Tests for lib/enforce — the resolution pipeline and its gates.
--
--   cargo make lua
--
-- The read grant is `/`, not `/tmp`: fixtures live under `fs.tempdir()`, but the sweep below
-- also reads real source files anywhere under the repository's `packs/` tree. The exec grant is
-- for `enforce.git`, which builds real repositories for the fixtures.

local hook = require("enforce")
local enforce = require("lib.enforce")
local fs = airsstack.fs
local json = airsstack.json
local path = airsstack.path

local MANIFEST = {
  stack = "rust",
  detect = { "Cargo.toml" },
  match = { "**/*.rs", "**/Cargo.toml" },
  skill = "/guideline-rust-guidelines",
  phase = { "code", "design" },
}

local PYTHON = {
  stack = "python",
  detect = { "pyproject.toml" },
  match = { "**/*.py" },
  skill = "/guideline-python-guidelines",
  phase = { "code", "design" },
}

-- A packs root holding `core` beside one directory per entry of `siblings`. An entry's value is
-- the manifest table to write as `enforcement.json`, a string to write verbatim, or `false` for
-- a directory with no such file. Returns the packs root and the pack root `<packs root>/core`.
local function packs(siblings)
  local root = fs.tempdir()
  fs.mkdir(path.join(root, "core"))
  for name, manifest in pairs(siblings) do
    local dir = path.join(root, name)
    fs.mkdir(dir)
    if type(manifest) == "table" then
      fs.write(path.join(dir, "enforcement.json"), json.encode(manifest))
    elseif type(manifest) == "string" then
      fs.write(path.join(dir, "enforcement.json"), manifest)
    end
  end
  return root, path.join(root, "core")
end

-- A git repository with a Cargo.toml and a Rust file.
local function repo()
  local dir = fs.tempdir()
  enforce.git(dir, "init", "-q")
  enforce.git(dir, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q",
    "--allow-empty", "-m", "init")
  fs.mkdir(path.join(dir, "src"))
  fs.write(path.join(dir, "Cargo.toml"), "[package]\n")
  fs.write(path.join(dir, "src", "main.rs"), "fn main() {}\n")
  fs.write(path.join(dir, "README.md"), "# notes\n")
  return dir
end

local function resolve(overrides, trace)
  local context = {
    session_id = "s",
    agent = "main",
    home = fs.tempdir(),
    sentinel_dir = fs.tempdir(),
  }
  for key, value in pairs(overrides) do
    context[key] = value
  end
  return enforce.resolve(context, trace)
end

-- Runs `body` with `airsstack.env` swapped for a table answering from `values`, and restores the
-- real one afterwards even when `body` raises, so a failing assertion cannot leak the fake into
-- the tests that follow.
local function with_env(values, body)
  local real = airsstack.env
  airsstack.env = {
    get = function(name)
      return values[name]
    end,
  }
  local ok, err = pcall(body)
  airsstack.env = real
  if not ok then
    error(err, 0)
  end
end

-- A `tool.before` event for `src/main.rs` in `project`, as the hook module receives it.
local function read_event(project, pack_root, overrides)
  local event = {
    event = "tool.before",
    tool = "read",
    path = path.join(project, "src", "main.rs"),
    cwd = project,
    session_id = "s",
    args = { pack_root },
  }
  for key, value in pairs(overrides or {}) do
    event[key] = value
  end
  return event
end

-- The index of the first trace line equal to `line`, or nil.
local function position(trace, line)
  for index, entry in ipairs(trace) do
    if entry == line then
      return index
    end
  end
  return nil
end

-- Splits `text` into lines. Every hook under `packs/` is a module returning a function, and the
-- sweep below reads source rather than calling them: the property it guards ("no hook may emit
-- `permissionDecision`") is a textual one, and must hold whatever event or fixture would reach
-- the emitting line.
local function lines_of(text)
  local result = {}
  for line in (text .. "\n"):gmatch("(.-)\n") do
    result[#result + 1] = line
  end
  return result
end

-- The `packs` directory this test file sits under, found from `arg[0]`, the path the harness
-- read this file from, so it needs neither `git` nor any assumption about the caller's cwd.
-- Anchored on a marker file, not on the directory's name: `packs` is far too common a name to
-- match on, and the first ancestor called that could be a directory this repository does not
-- own. Requiring `core/pack.toml` underneath keeps the walk inside a checkout of this repository.
local function packs_root()
  local dir = path.dirname(path.absolute(arg[0]))
  for _ = 1, 10 do
    if fs.is_file(path.join(dir, "core", "pack.toml")) then
      return dir
    end
    local parent = path.dirname(dir)
    if parent == dir then
      return nil
    end
    dir = parent
  end
  return nil
end

-- Every `.lua` file under `root`, relative-to-root paths, sorted.
local function all_lua_files(root)
  local ok, entries = pcall(fs.walk, root)
  assert(ok, "could not walk " .. root .. ": " .. tostring(entries))
  local found = {}
  for _, rel in ipairs(entries) do
    if rel:match("%.lua$") and fs.is_file(path.join(root, rel)) then
      found[#found + 1] = rel
    end
  end
  table.sort(found)
  return found
end

-- The code lines of `source`, one entry per source line, with every comment removed: both a `--`
-- line comment and a `--[[ ... ]]` block comment (which may open and close on the same line, or
-- span several). Aware of `'` and `"` string literals so a `--` embedded in one — for example
-- the `"--show-toplevel"` argument in `scripts/lib/handoff.lua` — is not mistaken for a comment
-- opener, which would hide code written after such a string on the same line. Long bracket
-- strings are
-- tracked for the same reason, and they are not
-- hypothetical: `scripts/lib/comm_report.lua` builds its topic pattern with one
-- (`regex.compile([[^([0-9]+)\. (\S.*)$]])`), so a `--` inside one would truncate a real line.
--
-- Long brackets are tracked at their own level (`[[`, `[=[`, `[==[` … each closed only by a `]`
-- with the same run of `=`), in both forms Lua gives them: `--[==[ … ]==]` is a block comment and
-- drops out, while a bare `[==[ … ]==]` is a string literal whose text is KEPT. Keeping it is
-- deliberate and matches how `'`/`"` strings are treated here — a field name reaching the CLI
-- inside a string literal is still a field reaching the CLI, so a payload like
-- `json.decode([[{"permissionDecision":"defer"}]])` must not be able to hide in one.
--
-- One construct still slips through, accepted rather than chased: a key built by concatenation,
-- e.g. `s["permission" .. "Decision"] = "defer"`, never puts the substring "permissionDecision"
-- in a single token for a text scan to find.
local function strip_comments(source)
  local out = {}
  -- While inside a multi-line long bracket: its `=` count, and whether it is a comment.
  local long_level, long_is_comment = nil, false
  for _, line in ipairs(lines_of(source)) do
    local code = {}
    local i, n = 1, #line
    local quote = nil

    if long_level then
      local closer = "]" .. string.rep("=", long_level) .. "]"
      local close = line:find(closer, 1, true)
      if close then
        if not long_is_comment then
          code[#code + 1] = line:sub(1, close - 1)
        end
        i = close + #closer
        long_level, long_is_comment = nil, false
      else
        if not long_is_comment then
          code[#code + 1] = line
        end
        i = n + 1
      end
    end

    while i <= n do
      local ch = line:sub(i, i)
      local comment_eq = line:match("^%-%-%[(=*)%[", i)
      local string_eq = line:match("^%[(=*)%[", i)
      if quote then
        code[#code + 1] = ch
        if ch == "\\" then
          code[#code + 1] = line:sub(i + 1, i + 1)
          i = i + 2
        else
          if ch == quote then
            quote = nil
          end
          i = i + 1
        end
      elseif ch == '"' or ch == "'" then
        quote = ch
        code[#code + 1] = ch
        i = i + 1
      elseif comment_eq then
        local closer = "]" .. comment_eq .. "]"
        local close = line:find(closer, i + 4 + #comment_eq, true)
        if close then
          i = close + #closer
        else
          long_level, long_is_comment = #comment_eq, true
          i = n + 1
        end
      elseif line:sub(i, i + 1) == "--" then
        i = n + 1
      elseif string_eq then
        local closer = "]" .. string_eq .. "]"
        local close = line:find(closer, i + 2 + #string_eq, true)
        if close then
          code[#code + 1] = line:sub(i, close + #closer - 1)
          i = close + #closer
        else
          long_level, long_is_comment = #string_eq, false
          code[#code + 1] = line:sub(i)
          i = n + 1
        end
      else
        code[#code + 1] = ch
        i = i + 1
      end
    end
    out[#out + 1] = table.concat(code)
  end
  return out
end

return {
  a_matching_rust_file_emits_one_pointer = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local pointers = resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    })
    assert(#pointers == 1, "expected one pointer, got " .. #pointers)
    assert(pointers[1]:find("rust-guidelines", 1, true), pointers[1])
    assert(pointers[1]:find("Definition of Done", 1, true), "code phase names the DoD")
  end,

  a_root_cargo_toml_matches_the_zero_segment_glob = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local pointers = resolve({
      file_path = path.join(project, "Cargo.toml"),
      cwd = project,
      pack_root = pack_root,
    })
    assert(#pointers == 1, "a root Cargo.toml must match **/Cargo.toml")
  end,

  a_file_matching_no_glob_emits_nothing = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local pointers = resolve({
      file_path = path.join(project, "README.md"),
      cwd = project,
      pack_root = pack_root,
    })
    assert(#pointers == 0, "gate 3 should have stopped this")
  end,

  a_repository_without_the_detect_marker_emits_nothing = function()
    local project = fs.tempdir()
    fs.mkdir(path.join(project, "src"))
    fs.write(path.join(project, "src", "main.rs"), "fn main() {}\n")
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local pointers = resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    })
    assert(#pointers == 0, "gate 2 should have stopped this")
  end,

  two_sibling_packs_both_apply = function()
    local project = repo()
    fs.write(path.join(project, "pyproject.toml"), "[project]\n")
    fs.write(path.join(project, "src", "app.py"), "print(1)\n")
    local _, pack_root = packs({
      ["guideline-rust"] = MANIFEST,
      ["guideline-python"] = PYTHON,
    })

    local trace = {}
    local pointers = resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    assert(#pointers == 1, "only the rust pack matches a .rs file")
    local python = position(trace, "guideline-python: GATE 3 no match glob hit")
    local rust = position(trace, "guideline-rust: EMIT /guideline-rust-guidelines")
    assert(python and rust and python < rust, "candidates run in name order:\n"
      .. table.concat(trace, "\n"))

    local other = resolve({
      file_path = path.join(project, "src", "app.py"),
      cwd = project,
      pack_root = pack_root,
    })
    assert(#other == 1 and other[1]:find("guideline-python-guidelines", 1, true),
      "the python pack applies too")
  end,

  directory_without_manifest_is_not_listed = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST, ["no-manifest"] = false })
    local trace = {}
    resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    local text = table.concat(trace, "\n")
    assert(not text:find("no-manifest", 1, true), text)
    assert(not text:find("core:", 1, true), "core ships no enforcement.json and is not listed\n" .. text)
  end,

  unloadable_manifest_is_skipped_and_others_apply = function()
    local project = repo()
    local _, pack_root = packs({
      ["guideline-rust"] = MANIFEST,
      ["guideline-broken"] = "{ not json",
    })
    local trace = {}
    local pointers = resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    assert(position(trace, "guideline-broken: no usable enforcement.json"), table.concat(trace, "\n"))
    assert(#pointers == 1, "the other pack still yields its pointer")
  end,

  a_manifest_missing_required_fields_skips_that_pack = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = { stack = "rust" } })
    local pointers = resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    })
    assert(#pointers == 0, "an unusable manifest must not emit")
  end,

  no_manifest_anywhere_stops = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = "{ not json", ["notes"] = false })
    local trace = {}
    local pointers = resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    assert(#pointers == 0)
    assert(trace[#trace] == "STOP: no pack ships a usable enforcement.json",
      table.concat(trace, "\n"))
  end,

  trace_counts_packs_under_root = function()
    local project = repo()
    local root, pack_root = packs({
      ["guideline-rust"] = MANIFEST,
      ["guideline-python"] = PYTHON,
      ["notes"] = false,
    })
    local trace = {}
    resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    assert(trace[1] == "packs: 2 with enforcement.json under " .. root, tostring(trace[1]))
  end,

  trace_has_no_registry_or_gate_one_lines = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local trace = {}
    resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    local text = table.concat(trace, "\n")
    for _, gone in ipairs({ "registry:", "project key:", "GATE 1", "using " }) do
      assert(not text:find(gone, 1, true), gone .. " must not appear:\n" .. text)
    end
  end,

  a_missing_packs_root_resolves_to_nothing = function()
    local project = repo()
    local trace = {}
    assert(#resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = "/nonexistent/agentstacks/core",
    }, trace) == 0)
    assert(trace[#trace] == "STOP: no pack ships a usable enforcement.json",
      table.concat(trace, "\n"))
  end,

  the_sentinel_makes_the_pointer_one_shot_per_context = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local context = {
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
      session_id = "s1",
      agent = "main",
      home = fs.tempdir(),
      sentinel_dir = fs.tempdir(),
    }
    assert(#enforce.resolve(context) == 1, "the first call emits")
    assert(#enforce.resolve(context) == 0, "the second call is suppressed")
  end,

  a_subagent_does_not_consume_the_main_threads_pointer = function()
    -- Subagents inherit the parent's session_id, so without the agent component an explorer
    -- reading one .rs file would spend the pointer the main thread exists to receive.
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local shared = {
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
      session_id = "s1",
      home = fs.tempdir(),
      sentinel_dir = fs.tempdir(),
    }

    local function with(agent)
      local context = { agent = agent }
      for key, value in pairs(shared) do
        context[key] = value
      end
      return enforce.resolve(context)
    end

    assert(#with("explorer-1") == 1, "the subagent gets its own pointer")
    assert(#with("main") == 1, "the main thread still gets one")
  end,

  a_design_doc_gets_the_architecture_pointer_rather_than_the_dod = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local home = fs.tempdir()
    local spec = path.join(enforce.sdd_root(home), "key-abc", "specs", "a.md")
    fs.mkdir(path.dirname(spec))
    fs.write(spec, "# spec\n")

    local pointers = resolve({
      file_path = spec,
      cwd = project,
      pack_root = pack_root,
      home = home,
    })
    assert(#pointers == 1, "a design doc under the SDD root should fire")
    assert(pointers[1]:find("architecture rules to this design", 1, true), pointers[1])
    assert(not pointers[1]:find("Definition of Done", 1, true),
      "a design has nothing to build, so the DoD is a demand the reader cannot satisfy")
  end,

  a_specs_segment_deeper_in_the_tree_is_not_a_design_doc = function()
    -- A substring test on '/specs/' matched `<key>/a/specs/b/plans/c.md` by accident, which swaps
    -- the Definition of Done for the architecture rules.
    local home = fs.tempdir()
    local root = enforce.sdd_root(home)
    assert(enforce.is_design_doc(path.join(root, "key", "specs", "a.md"), home) == true)
    assert(enforce.is_design_doc(path.join(root, "key", "plans", "a.md"), home) == true)
    assert(enforce.is_design_doc(path.join(root, "key", "a", "specs", "b", "c.md"), home) == false)
    assert(enforce.is_design_doc(path.join(root, "key", "a.md"), home) == false)
    assert(enforce.is_design_doc("/elsewhere/specs/a.md", home) == false)
  end,

  a_manifest_that_does_not_declare_the_phase_is_skipped = function()
    local project = repo()
    local _, pack_root = packs({
      ["guideline-rust"] = {
        stack = "rust", skill = "s", detect = { "Cargo.toml" },
        match = { "**/*.rs" }, phase = { "design" },
      },
    })
    assert(#resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }) == 0, "a design-only manifest must not fire on a code file")
  end,

  the_match_path_is_relative_to_the_repository_root = function()
    local project = repo()
    assert(enforce.path_for_matching(path.join(project, "src", "main.rs"), project)
      == "src/main.rs")
  end,

  outside_a_repository_the_match_path_is_the_basename = function()
    local loose = path.join(fs.tempdir(), "loose.rs")
    fs.write(loose, "fn main() {}\n")
    assert(enforce.path_for_matching(loose, path.dirname(loose)) == "loose.rs")
  end,

  -- Resolution without `--allow-exec git`. The production path must not need the grant: a
  -- worktree-isolated agent session cannot pass it, its guard refusing the whole command over
  -- the `git` operand. These fixtures are built by hand rather than by `git init`, so they
  -- exercise the read-only path whether or not the grant is present.

  the_toplevel_of_a_hand_built_checkout_is_found_without_git = function()
    local project = fs.tempdir()
    fs.mkdir(path.join(project, ".git"))
    fs.mkdir(path.join(project, "src"))
    fs.write(path.join(project, "src", "main.rs"), "fn main() {}\n")

    assert(enforce.path_for_matching(path.join(project, "src", "main.rs"), project)
      == "src/main.rs")
  end,

  a_linked_worktree_whose_dot_git_is_a_file_is_its_own_toplevel = function()
    -- A linked worktree holds `.git` as a file (`gitdir: ...`), not a directory.
    local project = fs.tempdir()
    fs.write(path.join(project, ".git"), "gitdir: /elsewhere/.git/worktrees/w\n")
    fs.mkdir(path.join(project, "src"))
    fs.write(path.join(project, "src", "main.rs"), "fn main() {}\n")

    assert(enforce.toplevel(path.join(project, "src")) == enforce.realpath(project),
      tostring(enforce.toplevel(path.join(project, "src"))))
    assert(enforce.path_for_matching(path.join(project, "src", "main.rs"),
      path.join(project, "src")) == "src/main.rs")
    assert(enforce.path_for_matching(project, project) == ".")
  end,

  a_sentinel_is_claimed_exactly_once = function()
    local directory = fs.tempdir()
    local file = enforce.sentinel_path(directory, "s", "main", "rust", "code")
    assert(enforce.claim(file) == true, "the first claim wins")
    assert(enforce.claim(file) == false, "the second finds it taken")
  end,

  clearing_a_session_leaves_other_sessions_alone = function()
    local directory = fs.tempdir()
    enforce.claim(enforce.sentinel_path(directory, "mine", "main", "rust", "code"))
    enforce.claim(enforce.sentinel_path(directory, "yours", "main", "rust", "code"))

    assert(enforce.clear_session(directory, "mine") == 1)
    assert(#enforce.held_sentinels(directory, "mine", "main") == 0)
    assert(#enforce.held_sentinels(directory, "yours", "main") == 1)
  end,

  a_stale_sentinel_is_pruned_and_a_fresh_one_is_kept = function()
    local directory = fs.tempdir()
    local file = enforce.sentinel_path(directory, "s", "main", "rust", "code")
    enforce.claim(file)

    local now = airsstack.time.now()
    enforce.prune_sentinels(directory, now)
    assert(fs.exists(file), "a fresh sentinel must survive")

    enforce.prune_sentinels(directory, now + enforce.SENTINEL_MAX_AGE + 60)
    assert(not fs.exists(file), "a sentinel past its age must be pruned")
  end,

  a_file_that_is_not_a_sentinel_is_never_pruned = function()
    local directory = fs.tempdir()
    local other = path.join(directory, "unrelated.txt")
    fs.write(other, "x")
    enforce.prune_sentinels(directory, airsstack.time.now() + 10 * enforce.SENTINEL_MAX_AGE)
    assert(fs.exists(other), "only this hook's own sentinels are prunable")
  end,

  a_held_sentinel_is_reported_as_stack_colon_phase = function()
    local directory = fs.tempdir()
    enforce.claim(enforce.sentinel_path(directory, "s", "main", "rust", "code"))
    local held = enforce.held_sentinels(directory, "s", "main")
    assert(held[1] == "rust:code", tostring(held[1]))
  end,

  hook_returns_pointer_for_a_rust_file = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    with_env({ TMPDIR = fs.tempdir() }, function()
      local result = hook(read_event(project, pack_root))
      assert(result and result.context, "expected a context result")
      assert(result.context:find("/guideline-rust-guidelines", 1, true), result.context)
    end)
  end,

  hook_returns_nil_when_no_pack_matches = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-python"] = PYTHON })
    with_env({ TMPDIR = fs.tempdir() }, function()
      assert(hook(read_event(project, pack_root)) == nil)
    end)
  end,

  hook_ignores_an_event_without_a_path = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    with_env({ TMPDIR = fs.tempdir() }, function()
      assert(hook(read_event(project, pack_root, { path = false })) == nil)
    end)
  end,

  hook_keys_sentinel_by_agent_id = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    with_env({ TMPDIR = fs.tempdir() }, function()
      local function fire(agent)
        return hook(read_event(project, pack_root, { agent_id = agent }))
      end
      assert(fire("a1") and fire("a2"), "each agent gets its own pointer")
      assert(fire("a1") == nil, "a1 already has its pointer")
      assert(fire("a2") == nil, "a2 already has its pointer")
    end)
  end,

  nil_agent_id_uses_main = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local tmp = fs.tempdir()
    with_env({ TMPDIR = tmp }, function()
      assert(hook(read_event(project, pack_root)), "the first event emits")
      assert(enforce.held_sentinels(tmp, "s", "main")[1] == "rust:code",
        "the sentinel is keyed by the agent name `main`")
      assert(hook(read_event(project, pack_root)) == nil, "the second event is suppressed")
    end)
  end,

  the_trace_names_a_missing_detect_marker = function()
    local project = fs.tempdir()
    fs.mkdir(path.join(project, "src"))
    fs.write(path.join(project, "src", "main.rs"), "fn main() {}\n")
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local trace = {}
    resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    assert(position(trace, "guideline-rust: GATE 2 no detect marker"), table.concat(trace, "\n"))
  end,

  the_trace_names_the_match_path = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local trace = {}
    resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    assert(position(trace, "match path: src/main.rs"), table.concat(trace, "\n"))
  end,

  the_trace_names_a_phase_no_manifest_declares = function()
    local project = repo()
    local _, pack_root = packs({
      ["guideline-rust"] = {
        stack = "rust", skill = "s", detect = { "Cargo.toml" },
        match = { "**/*.rs" }, phase = { "design" },
      },
    })
    local trace = {}
    resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    assert(trace[#trace] == "STOP: no manifest declares phase code", table.concat(trace, "\n"))
  end,

  the_trace_names_an_already_claimed_context = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local context = {
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
      session_id = "s1",
      agent = "main",
      home = fs.tempdir(),
      sentinel_dir = fs.tempdir(),
    }
    enforce.resolve(context)
    local trace = {}
    assert(#enforce.resolve(context, trace) == 0)
    assert(trace[#trace] == "STOP: every candidate stack:phase already claimed this context",
      table.concat(trace, "\n"))
  end,

  the_trace_names_a_sentinel_claimed_concurrently = function()
    -- A competing hook winning the claim between the cheap gate and the claim itself is not
    -- reproducible with real files, so the claim is made to lose.
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local real = enforce.claim
    enforce.claim = function()
      return false
    end
    local trace = {}
    local ok, pointers = pcall(resolve, {
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    enforce.claim = real
    assert(ok, tostring(pointers))
    assert(#pointers == 0, "a lost claim must not emit")
    assert(position(trace, "guideline-rust: sentinel claimed concurrently"),
      table.concat(trace, "\n"))
  end,

  packs_are_visited_in_name_order = function()
    -- Creation order and table iteration order are both arbitrary; only the listing's own order
    -- can put these in name order.
    local project = repo()
    local _, pack_root = packs({
      ["pack-d"] = "{", ["pack-b"] = "{", ["pack-e"] = "{", ["pack-a"] = "{", ["pack-c"] = "{",
    })
    local trace = {}
    resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      pack_root = pack_root,
    }, trace)
    local last = 0
    for _, name in ipairs({ "pack-a", "pack-b", "pack-c", "pack-d", "pack-e" }) do
      local at = position(trace, name .. ": no usable enforcement.json")
      assert(at and at > last, name .. " out of order:\n" .. table.concat(trace, "\n"))
      last = at
    end
  end,

  the_home_prefers_airsstack_home = function()
    local env = {
      get = function(name)
        return ({ AIRSSTACK_HOME = "/custom/home", HOME = "/users/me" })[name]
      end,
    }
    assert(enforce.home(env) == "/custom/home", enforce.home(env))
  end,

  the_home_falls_back_to_dot_airsstack_under_home = function()
    for _, airsstack_home in ipairs({ "", "unset" }) do
      local env = {
        get = function(name)
          if name == "AIRSSTACK_HOME" then
            return airsstack_home ~= "unset" and airsstack_home or nil
          end
          return ({ HOME = "/users/me" })[name]
        end,
      }
      assert(enforce.home(env) == "/users/me/.airsstack", enforce.home(env))
    end
  end,

  the_home_without_either_variable_is_a_relative_dot_airsstack = function()
    local env = {
      get = function()
        return nil
      end,
    }
    assert(enforce.home(env) == ".airsstack", enforce.home(env))
  end,

  hook_ignores_an_event_without_a_pack_root = function()
    local project = repo()
    local _, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    with_env({ TMPDIR = fs.tempdir() }, function()
      assert(hook(read_event(project, pack_root, { args = false })) == nil, "no arguments")
      assert(hook(read_event(project, pack_root, { args = {} })) == nil, "no pack root argument")
      assert(hook(read_event(project, pack_root, { args = { "" } })) == nil, "an empty pack root")
    end)
  end,

  doctor_prints_packs_header_and_trace = function()
    local project = repo()
    local root, pack_root = packs({ ["guideline-rust"] = MANIFEST })
    local tmp = fs.tempdir()
    local env = {
      get = function(name)
        return ({ TMPDIR = tmp, AIRSSTACK_HOME = fs.tempdir() })[name]
      end,
    }

    local out, err = enforce.doctor({ pack_root, path.join(project, "src", "main.rs") }, env)
    assert(err == nil, tostring(err))
    assert(out, "expected the doctor text")
    assert(out:find("packs:    " .. root .. "\n", 1, true), out)
    assert(out:find("packs: 1 with enforcement.json under " .. root, 1, true), out)
    assert(out:find("guideline-rust: EMIT /guideline-rust-guidelines", 1, true), out)
    assert(out:find("outcome: 1 pointer(s)", 1, true), out)
    assert(not out:find("parity", 1, true), "the parity section is gone:\n" .. out)
    assert(not out:find("registry", 1, true), "the registry header is gone:\n" .. out)
  end,

  doctor_trace_names_the_stop_when_no_pack_ships_a_manifest = function()
    local project = repo()
    local _, pack_root = packs({ ["notes"] = false })
    local env = {
      get = function(name)
        return ({ TMPDIR = fs.tempdir() })[name]
      end,
    }

    local out = enforce.doctor({ pack_root, path.join(project, "src", "main.rs") }, env)
    assert(out:find("STOP: no pack ships a usable enforcement.json", 1, true), out)
    assert(out:find("outcome: 0 pointer(s)", 1, true), out)
  end,

  doctor_usage_without_arguments = function()
    local env = {
      get = function()
        return nil
      end,
    }
    local usage = "usage: doctor.lua <pack-root> <path>\n"
    for _, args in ipairs({ {}, { "/r", "" }, { "", "/f" }, { "/r" } }) do
      local out, err = enforce.doctor(args, env)
      assert(out == nil and err == usage, tostring(out) .. " / " .. tostring(err))
    end
  end,

  no_hook_ever_emits_a_permission_decision = function()
    -- Any hook that returns `permissionDecision` on PreToolUse can have the CLI swallow the tool
    -- call it fired on outright — no `tool_result` at all — when the session is non-interactive,
    -- the tool batch is solo, and the abort signal is not already set: a subagent's run ends
    -- reported `completed` having done nothing. Hooks return `{ context = ... }` or
    -- `{ block = ... }` instead, and this sweeps every Lua file under `packs/`, not one named
    -- hook.
    local root = packs_root()
    assert(root, "could not resolve the packs root from this test file's own path "
      .. "(arg[0] = " .. tostring(arg[0]) .. "): no ancestor directory holds "
      .. "`core/pack.toml`. This guard sweeps a checkout of this repository, and refuses "
      .. "rather than sweeping whatever `packs` directory it happens to sit under")

    -- A named file, not a count: a threshold drifts with the suite's size, failing the day a
    -- pack is removed rather than the day the root is wrong.
    assert(fs.is_file(path.join(root, "core", "hooks", "enforce.lua")),
      "resolved the packs root to " .. root .. ", which holds no core/hooks/enforce.lua")
    local files = all_lua_files(root)

    -- This test file itself is excluded: it names `permissionDecision` in code, in its own
    -- assertion messages and in the string literal every other file's source is searched for,
    -- and that is talking *about* the field, not emitting it. Guarding this file's own vocabulary
    -- is not the property under test.
    local self_path = path.absolute(arg[0])

    for _, rel in ipairs(files) do
      local file = path.join(root, rel)
      if file ~= self_path then
        local source = fs.read(file)
        for lineno, code in ipairs(strip_comments(source)) do
          assert(not code:find("permissionDecision", 1, true), file .. ":" .. lineno
            .. ": emits `permissionDecision` — a hook that returns this field can have the CLI "
            .. "swallow the tool call outright (no tool_result at all) in a non-interactive "
            .. "subagent session, stranding whatever the tool call was meant to do with the run "
            .. "still reported `completed`. Return `{ context = ... }` or `{ block = ... }` "
            .. "from the hook module instead.")
        end
      end
    end
  end,
}
