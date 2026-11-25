# Probe results: agentstacks and airsl side

Topic: AGENTSTACKS AND AIRSL SIDE (chain `2026-11-24-opencode-extension`).

Versions: `agentstacks` built from this worktree at commit `8a96f78` with `cargo build -p agentstacks-cli`
(one run, `Finished dev profile … in 21.61s`), binary `target/debug/agentstacks`; `airsl 0.1.5`
(`airsl --version` → `airsl 0.1.5`) on PATH.
`A=<repo>/target/debug/agentstacks` below. Scratchpad `S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad`.

Agentstacks sources cited are copied to `probes/src/agentstacks/<repo path>` (prefixed `agentstacks/`
to keep them apart from the OpenCode tree).

## P1: Two tools in one project (claude + a stub opencode)

**Setup** (`$S/as-01/proj/`):

- `ext/claude/` = `cp -R <repo>/extensions/claude` (unchanged).
- `packs/core/`, `packs/guideline-rust/` = `cp -R <repo>/packs/{core,guideline-rust}` (unchanged).
  They were copied in because `init` refuses a path outside the project (control below).
- The stub is the authoring starter (`book/src/ch06-00-authoring.md` step 3) with `.opencode` paths:

```toml $S/as-01/proj/ext/opencode/extension.toml
[extension]
name        = "opencode"
version     = "0.1.0"
api         = 0
description = "Probe-only stub: renders packs into .opencode/"
entry       = "adapter.lua"
```

```lua $S/as-01/proj/ext/opencode/lib/paths.lua
-- lib/paths.lua: pure rules; no agentstacks.* here.
local M = {}

M.ROOT = ".opencode/agentstacks"

function M.skill_dir(name)
  return ".opencode/skills/" .. name
end

function M.agent_file(name)
  return ".opencode/agents/" .. name .. ".md"
end

return M
```

```lua $S/as-01/proj/ext/opencode/adapter.lua
-- adapter.lua: the only file that calls agentstacks.*; the rules live in lib/.
local paths = agentstacks.require("lib.paths")

local NO_HOOKS = "opencode has no hooks"

agentstacks.on("values", function(q)
  local out = { root = paths.ROOT, skills = {}, agents = {}, vars = {} }
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills or {}) do
      out.skills[p.name .. "/" .. s] = {
        invoke = "/" .. p.name .. "-" .. s,
        dir = paths.skill_dir(p.name .. "-" .. s),
      }
    end
    for _, a in ipairs(p.agents or {}) do
      out.agents[p.name .. "/" .. a] = p.name .. "-" .. a
    end
  end
  return out
end)

agentstacks.on("plan", function(q)
  for _, p in ipairs(q.packs) do
    for _, skill in ipairs(p.skills or {}) do
      for _, f in ipairs(skill.files) do
        agentstacks.copy({ path = skill.dir .. "/" .. f.rel, file = f.id })
      end
    end
    for _, a in ipairs(p.agents or {}) do
      local file = paths.agent_file(p.name .. "-" .. a.name)
      agentstacks.write({ path = file, content = a.text, covers = { a.id } })
    end
    for _, h in ipairs(p.hooks or {}) do
      agentstacks.skip({ item = h.id, reason = NO_HOOKS })
    end
    local native = p.native or {}
    for _, f in ipairs(native.files or {}) do
      agentstacks.copy({ path = ".opencode/" .. f.rel, file = f.id })
    end
    for _, h in ipairs(native.hooks or {}) do
      agentstacks.skip({ item = h.id, reason = NO_HOOKS })
    end
  end
end)
```

**Command** (`init --help` says `--tool <TOOL>  A tool: an official name or a directory in the project. Repeat for more`):

```sh
cd $S/as-01/proj
$A init --tool ext/claude --tool ext/opencode --pack packs/core --pack packs/guideline-rust < /dev/null > ../init.out 2> ../init.err; echo "exit=$?" >> ../init.out
$A apply --yes > ../apply.out 2>&1; echo "exit=$?" >> ../apply.out
$A plan        > ../plan2.out 2>&1; echo "exit=$?" >> ../plan2.out
$A doctor      > ../doctor.out 2>&1; echo "exit=$?" >> ../doctor.out
```

**Output** `init.out` (full) and `init.err`:

```text
agentstacks plan

  create   .claude/agents/core-coder.md                                                     core
  create   .claude/agents/core-explorer.md                                                  core
  create   .claude/agents/core-reviewer.md                                                  core
  create   .claude/agentstacks/core/hooks/doctor.lua                                        core
  create   .claude/agentstacks/core/hooks/enforce.lua                                       core
  create   .claude/agentstacks/core/hooks/lib/enforce.lua                                   core
  create   .claude/agentstacks/core/hooks/lib/globs.lua                                     core
  create   .claude/agentstacks/core/hooks/rearm.lua                                         core
  create   .claude/agentstacks/core/scripts/comm_check_hook.lua                             core
  create   .claude/agentstacks/core/scripts/discuss.lua                                     core
  create   .claude/agentstacks/core/scripts/handoff.lua                                     core
  create   .claude/agentstacks/core/scripts/handoff_check_hook.lua                          core
  create   .claude/agentstacks/core/scripts/handoff_report.lua                              core
  create   .claude/agentstacks/core/scripts/install-airsl.sh                                core
  create   .claude/agentstacks/core/scripts/lib/comm_report.lua                             core
  create   .claude/agentstacks/core/scripts/lib/discuss.lua                                 core
  create   .claude/agentstacks/core/scripts/lib/handoff.lua                                 core
  create   .claude/agentstacks/core/scripts/lib/handoff_report.lua                          core
  create   .claude/agentstacks/core/scripts/lib/statusline.lua                              core
  create   .claude/agentstacks/core/scripts/lib/style.lua                                   core
  create   .claude/agentstacks/core/scripts/statusline-install.lua                          core
  create   .claude/agentstacks/core/scripts/statusline.lua                                  core
  create   .claude/agentstacks/core/scripts/style.lua                                       core
  create   .claude/agentstacks/guideline-rust/enforcement.json                              guideline-rust
  create   .claude/agentstacks/runtime.lua                                                  claude
  create   .claude/agentstacks/translate.lua                                                claude
  create   .claude/settings.json                                                            shared
  create   .claude/skills/core-context-handoff/SKILL.md                                     core
  create   .claude/skills/core-context-handoff/references/protocol.md                       core
  create   .claude/skills/core-discuss/SKILL.md                                             core
  create   .claude/skills/core-discuss/references/protocol.md                               core
  create   .claude/skills/core-enforce-doctor/SKILL.md                                      core
  create   .claude/skills/core-orchestrate/SKILL.md                                         core
  create   .claude/skills/core-process-guidelines/SKILL.md                                  core
  create   .claude/skills/core-process-guidelines/references/agent-orchestration.md         core
  create   .claude/skills/core-process-guidelines/references/conventional-commits.md        core
  create   .claude/skills/core-process-guidelines/references/model-routing.md               core
  create   .claude/skills/core-snapshot-load/SKILL.md                                       core
  create   .claude/skills/core-snapshot-save/SKILL.md                                       core
  create   .claude/skills/core-statusline/SKILL.md                                          core
  create   .claude/skills/guideline-rust-guidelines/SKILL.md                                guideline-rust
  create   .claude/skills/guideline-rust-guidelines/references/doc-comment-discipline.md    guideline-rust
  create   .claude/skills/guideline-rust-guidelines/references/microsoft-guidelines.md      guideline-rust
  create   .claude/skills/guideline-rust-guidelines/references/mod-rs-export-only.md        guideline-rust
  create   .claude/skills/guideline-rust-guidelines/references/modularity.md                guideline-rust
  create   .claude/skills/guideline-rust-guidelines/references/static-dispatch.md           guideline-rust
  create   .claude/skills/guideline-rust-guidelines/references/strict-quality.md            guideline-rust
  create   .claude/skills/guideline-rust-guidelines/references/strong-types.md              guideline-rust
  create   .claude/skills/guideline-rust-guidelines/references/unit-test-mandate.md         guideline-rust
  create   .claude/skills/guideline-rust-guidelines/references/workspace.md                 guideline-rust
  create   .opencode/agents/core-coder.md                                                   core
  create   .opencode/agents/core-explorer.md                                                core
  create   .opencode/agents/core-reviewer.md                                                core
  create   .opencode/agentstacks/core/hooks/doctor.lua                                      core
  create   .opencode/agentstacks/core/hooks/enforce.lua                                     core
  create   .opencode/agentstacks/core/hooks/lib/enforce.lua                                 core
  create   .opencode/agentstacks/core/hooks/lib/globs.lua                                   core
  create   .opencode/agentstacks/core/hooks/rearm.lua                                       core
  create   .opencode/agentstacks/core/scripts/comm_check_hook.lua                           core
  create   .opencode/agentstacks/core/scripts/discuss.lua                                   core
  create   .opencode/agentstacks/core/scripts/handoff.lua                                   core
  create   .opencode/agentstacks/core/scripts/handoff_check_hook.lua                        core
  create   .opencode/agentstacks/core/scripts/handoff_report.lua                            core
  create   .opencode/agentstacks/core/scripts/install-airsl.sh                              core
  create   .opencode/agentstacks/core/scripts/lib/comm_report.lua                           core
  create   .opencode/agentstacks/core/scripts/lib/discuss.lua                               core
  create   .opencode/agentstacks/core/scripts/lib/handoff.lua                               core
  create   .opencode/agentstacks/core/scripts/lib/handoff_report.lua                        core
  create   .opencode/agentstacks/core/scripts/lib/statusline.lua                            core
  create   .opencode/agentstacks/core/scripts/lib/style.lua                                 core
  create   .opencode/agentstacks/core/scripts/statusline-install.lua                        core
  create   .opencode/agentstacks/core/scripts/statusline.lua                                core
  create   .opencode/agentstacks/core/scripts/style.lua                                     core
  create   .opencode/agentstacks/guideline-rust/enforcement.json                            guideline-rust
  create   .opencode/agentstacks/runtime.lua                                                opencode
  create   .opencode/skills/core-orchestrate/SKILL.md                                       core
  create   .opencode/skills/core-process-guidelines/SKILL.md                                core
  create   .opencode/skills/core-process-guidelines/references/agent-orchestration.md       core
  create   .opencode/skills/core-process-guidelines/references/conventional-commits.md      core
  create   .opencode/skills/core-process-guidelines/references/model-routing.md             core
  create   .opencode/skills/core-snapshot-load/SKILL.md                                     core
  create   .opencode/skills/core-snapshot-save/SKILL.md                                     core
  create   .opencode/skills/guideline-rust-guidelines/SKILL.md                              guideline-rust
  create   .opencode/skills/guideline-rust-guidelines/references/doc-comment-discipline.md  guideline-rust
  create   .opencode/skills/guideline-rust-guidelines/references/microsoft-guidelines.md    guideline-rust
  create   .opencode/skills/guideline-rust-guidelines/references/mod-rs-export-only.md      guideline-rust
  create   .opencode/skills/guideline-rust-guidelines/references/modularity.md              guideline-rust
  create   .opencode/skills/guideline-rust-guidelines/references/static-dispatch.md         guideline-rust
  create   .opencode/skills/guideline-rust-guidelines/references/strict-quality.md          guideline-rust
  create   .opencode/skills/guideline-rust-guidelines/references/strong-types.md            guideline-rust
  create   .opencode/skills/guideline-rust-guidelines/references/unit-test-mandate.md       guideline-rust
  create   .opencode/skills/guideline-rust-guidelines/references/workspace.md               guideline-rust
  skip     core/hook/3: claude shows no hook output to the model at session end
  skip     core/skill/context-handoff: opencode gave no value for {{var project_dir}}
  skip     core/skill/discuss: opencode gave no value for {{var arguments}}
  skip     core/skill/enforce-doctor: opencode gave no value for {{var arguments}}
  skip     core/hook/1: opencode gave no value for {{var tmpdir}}
  skip     core/hook/5: opencode gave no value for {{var tmpdir}}
  skip     core/hook/0: opencode has no hooks
  skip     core/hook/2: opencode has no hooks
  skip     core/hook/3: opencode has no hooks
  skip     core/hook/4: opencode has no hooks
  skip     core/hook/6: opencode has no hooks
  skip     core/hook/7: opencode has no hooks
  skip     core/hook/8: opencode has no hooks
  skip     core/hook/9: opencode has no hooks

  airsl    0.1.5 (new pin)

92 to write, 0 to delete, 0 unchanged, 14 skipped
exit=1
```

```text
stdin is not a terminal; pass --yes to apply
agentstacks.toml written; nothing applied (run apply)
```

```toml $S/as-01/proj/agentstacks.toml (written by init)
api = 0

[tools.claude]
path = "ext/claude"

[tools.opencode]
path = "ext/opencode"

[packs.core]
path = "packs/core"

[packs.guideline-rust]
path = "packs/guideline-rust"
```

`apply.out` is `init.out` plus one line (command: `diff <(sed '$d' ../init.out) <(sed '$d' ../apply.out)`, then `tail -8 ../apply.out`):

```text
112a113
> applied: 92 written, 0 deleted
  skip     core/hook/8: opencode has no hooks
  skip     core/hook/9: opencode has no hooks

  airsl    0.1.5 (new pin)

92 to write, 0 to delete, 0 unchanged, 14 skipped
applied: 92 written, 0 deleted
exit=0
```

`plan2.out` and `doctor.out`:

```text
agentstacks plan

  skip     core/hook/3: claude shows no hook output to the model at session end
  skip     core/skill/context-handoff: opencode gave no value for {{var project_dir}}
  skip     core/skill/discuss: opencode gave no value for {{var arguments}}
  skip     core/skill/enforce-doctor: opencode gave no value for {{var arguments}}
  skip     core/hook/1: opencode gave no value for {{var tmpdir}}
  skip     core/hook/5: opencode gave no value for {{var tmpdir}}
  skip     core/hook/0: opencode has no hooks
  skip     core/hook/2: opencode has no hooks
  skip     core/hook/3: opencode has no hooks
  skip     core/hook/4: opencode has no hooks
  skip     core/hook/6: opencode has no hooks
  skip     core/hook/7: opencode has no hooks
  skip     core/hook/8: opencode has no hooks
  skip     core/hook/9: opencode has no hooks

  airsl    0.1.5 (pinned)

0 to write, 0 to delete, 92 unchanged, 14 skipped
exit=0
```

```text
healthy
exit=0
```

Root files and runtime per tool:

```sh
cmp .claude/agentstacks/runtime.lua .opencode/agentstacks/runtime.lua && echo "runtime.lua identical"
shasum -a 256 .claude/agentstacks/runtime.lua .opencode/agentstacks/runtime.lua .claude/agentstacks/core/hooks/enforce.lua .opencode/agentstacks/core/hooks/enforce.lua
ls .opencode/agentstacks
grep -n -B1 -A4 'runtime.lua"' agentstacks.lock
grep -c '^\[\[file\]\]' agentstacks.lock
```

```text
runtime.lua identical
05ba59c1af4002b5a212f9eacd1cb74253f69cc79c9bf3a95d4dda312f89a282  .claude/agentstacks/runtime.lua
05ba59c1af4002b5a212f9eacd1cb74253f69cc79c9bf3a95d4dda312f89a282  .opencode/agentstacks/runtime.lua
aa47979da5908fc2ea4e0eda39394dfe9816c7160c962a752e7631f23a8304cf  .claude/agentstacks/core/hooks/enforce.lua
aa47979da5908fc2ea4e0eda39394dfe9816c7160c962a752e7631f23a8304cf  .opencode/agentstacks/core/hooks/enforce.lua
core
guideline-rust
runtime.lua
151-[[file]]
152:path = ".claude/agentstacks/runtime.lua"
153-hash = "sha256:05ba59c1af4002b5a212f9eacd1cb74253f69cc79c9bf3a95d4dda312f89a282"
154-by   = "claude"
155-
156-[[file]]
--
396-[[file]]
397:path = ".opencode/agentstacks/runtime.lua"
398-hash = "sha256:05ba59c1af4002b5a212f9eacd1cb74253f69cc79c9bf3a95d4dda312f89a282"
399-by   = "opencode"
400-
401-[[file]]
91
```

(91 locked files = 92 written minus `.claude/settings.json`, which is `shared` and never locked.)

Control, packs and tools by absolute path (`$S/as-01b/proj`, empty dir):

```sh
$A init --tool <repo>/extensions/claude --pack <repo>/packs/core < /dev/null; echo "exit=$?"; ls -A
```

```text
--pack "/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/packs/core": absolute paths are not portable
--tool "/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/extensions/claude": absolute paths are not portable
exit=1
```

**Answer**: Yes. Each tool gets its own `<root>/runtime.lua` (identical bytes, locked `by = "claude"`
/ `by = "opencode"`) and its own copy of every root file under its root (`.claude/agentstacks/core/…`,
`.opencode/agentstacks/core/…`, same hashes). No path clash: one `apply` wrote 92 files, then `plan`
showed `0 to write … 92 unchanged` and `doctor` said `healthy`. The core skipped for opencode (empty
`vars`): `core/skill/context-handoff` (`{{var project_dir}}`), `core/skill/discuss` and
`core/skill/enforce-doctor` (`{{var arguments}}`), `core/hook/1` and `core/hook/5` (`{{var tmpdir}}`).
`runtime.lua` is written for opencode even though the stub skips every hook. Skip lines do not name
the tool: `core/hook/3` appears twice, once per tool, and only the reason text tells them apart.
`native/claude` (`core-statusline` skill) went to claude only, with no opencode skip line in this run
(the as-05 run shows `note     native/claude: not for opencode`).

## P2: Does `agentstacks init --tool opencode` refuse today?

**Setup**: empty directory `$S/as-02/proj`.

**Command**:

```sh
cd $S/as-02/proj && $A init --tool opencode --pack core < /dev/null; echo "exit=$?"; ls -A
```

**Output**:

```text
--tool "opencode": not an official tool; pass a path, or add a git source to agentstacks.toml
exit=1
```

Source: `probes/src/agentstacks/crates/agentstacks/src/types/shorthand.rs:48`:

```rust
pub const OFFICIAL_TOOLS: [&str; 1] = ["claude"];
```

**Answer**: Yes, it refuses with exit 1 and writes nothing (`ls -A` empty):
`--tool "opencode": not an official tool; pass a path, or add a git source to agentstacks.toml`.

## P3: The hook command line, run by hand

**Setup** (`$S/as-03/proj/`): `packs/core` and `packs/guideline-rust` copied from the repo, an empty
`Cargo.toml` and `src/main.rs` (`touch`), a probe pack, and a stub `opencode` extension that logs each
lua hook from the `plan` payload into `<root>/argv.json` (one `write` covering every lua hook) and
writes a hand-written `<root>/translate.lua`. The var values it supplies copy the claude extension's
shell-expression style (`tmpdir = "${TMPDIR:-/tmp}"`).

```toml $S/as-03/proj/ext/opencode/extension.toml
[extension]
name        = "opencode"
version     = "0.1.0"
api         = 0
description = "Probe-only stub: logs every lua hook argv and writes a minimal translator"
entry       = "adapter.lua"
```

```lua $S/as-03/proj/ext/opencode/adapter.lua
-- Probe-only stub. Installs skills and agents like the authoring starter, logs every lua hook
-- exactly as the plan payload gives it into .opencode/agentstacks/argv.json (that write covers the
-- lua hooks), skips message hooks, and writes a hand-written translate.lua.
local ROOT = ".opencode/agentstacks"

-- Values offered for {{var}} names; only names in q.vars are returned.
local VARS = { tmpdir = "${TMPDIR:-/tmp}", project_dir = ".", session_id = "$SESSION", arguments = "$ARGUMENTS" }

local TRANSLATE = [==[
-- probe translator: payload {"event":"tool.before","tool":"read","path":"x"} -> neutral event
local M = {}
local EVENTS = { ["tool.before"] = true, ["tool.after"] = true }
function M.event(p)
  if type(p) ~= "table" or not EVENTS[p.event] then
    return nil
  end
  return { event = p.event, session_id = p.session_id or "probe-session", cwd = p.cwd, tool = p.tool, path = p.path }
end
function M.result(r, p)
  if r == nil then
    return nil
  end
  return airsstack.json.encode({ context = r.context, block = r.block, from = p.event }) .. "\n"
end
return M
]==]

agentstacks.on("values", function(q)
  local out = { root = ROOT, skills = {}, agents = {}, vars = {} }
  for _, v in ipairs(q.vars) do
    out.vars[v] = VARS[v]
  end
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills or {}) do
      out.skills[p.name .. "/" .. s] = { invoke = "/" .. p.name .. "-" .. s, dir = ".opencode/skills/" .. p.name .. "-" .. s }
    end
    for _, a in ipairs(p.agents or {}) do
      out.agents[p.name .. "/" .. a] = p.name .. "-" .. a
    end
  end
  return out
end)

agentstacks.on("plan", function(q)
  local log, covers = {}, {}
  for _, p in ipairs(q.packs) do
    for _, skill in ipairs(p.skills or {}) do
      for _, f in ipairs(skill.files) do
        agentstacks.copy({ path = skill.dir .. "/" .. f.rel, file = f.id })
      end
    end
    for _, a in ipairs(p.agents or {}) do
      agentstacks.write({ path = ".opencode/agents/" .. p.name .. "-" .. a.name .. ".md", content = a.text, covers = { a.id } })
    end
    for _, h in ipairs(p.hooks or {}) do
      if h.kind == "lua" then
        log[#log + 1] = { id = h.id, event = h.event, match = h.match, agents = h.agents, may_block = h.may_block, command = h.command }
        covers[#covers + 1] = h.id
      else
        agentstacks.skip({ item = h.id, reason = "probe: message hooks not rendered" })
      end
    end
    for _, h in ipairs((p.native and p.native.hooks) or {}) do
      agentstacks.skip({ item = h.id, reason = "probe: native hooks not rendered" })
    end
  end
  agentstacks.write({ path = ROOT .. "/argv.json", content = airsstack.json.encode(log), covers = covers })
  agentstacks.write({ path = ROOT .. "/translate.lua", content = TRANSLATE })
end)
```

```toml $S/as-03/proj/packs/probe/pack.toml
[pack]
name = "probe"
version = "0.1.0"
api = 0
description = "Probe-only pack: one context hook, one blocking hook"

[requires]
airsl = ">=0.1.5"

[[hooks]]
kind = "lua"
event = "tool.before"
run = "hooks/echo.lua"
args = ["a1", "{{var tmpdir}}"]

[[hooks]]
kind = "lua"
event = "tool.before"
run = "hooks/deny.lua"
may_block = true
```

```lua $S/as-03/proj/packs/probe/hooks/echo.lua
return function(e)
  return { context = "event=" .. tostring(e.event) .. " tool=" .. tostring(e.tool) .. " path=" .. tostring(e.path)
    .. " session=" .. tostring(e.session_id) .. " args=" .. table.concat(e.args, ",") }
end
```

```lua $S/as-03/proj/packs/probe/hooks/deny.lua
return function(e)
  if e.path == "secret.txt" then
    return { block = "probe: secret.txt is off limits" }
  end
  return nil
end
```

```zsh $S/as-03/run-hook.zsh (execs the argv array with no shell expansion of any element)
#!/bin/zsh
# usage: run-hook.zsh <hook id> <stdin file>
# Runs the hook argv from .opencode/agentstacks/argv.json exactly as given (no shell expansion of
# any element: zsh array exec), from the project dir, with <stdin file> on stdin.
input="${2:A}"
cd "${0:A:h}/proj" || exit 99
args=("${(@f)$(jq -r --arg id "$1" '.[] | select(.id == $id) | .command[]' .opencode/agentstacks/argv.json)}")
print -r -- "argv: ${(qq)args[@]}"
print -r -- "stdin: $(cat $input)"
TIMEFMT='time: %E real'
time ( "${args[@]}" < "$input" > ../stdout.txt 2> ../stderr.txt; echo $? > ../code.txt )
print -r -- "exit=$(cat ../code.txt)"
print -r -- "stdout:"; cat ../stdout.txt
print -r -- "stderr:"; cat ../stderr.txt
```

```zsh $S/as-03/run-nofailopen.zsh (control: same argv minus --fail-open)
#!/bin/zsh
# usage: run-nofailopen.zsh <hook id> <stdin file>
# Control: the same argv as run-hook.zsh with the "--fail-open" element removed, to show the
# error that --fail-open hides.
input="${2:A}"
cd "${0:A:h}/proj" || exit 99
args=("${(@f)$(jq -r --arg id "$1" '.[] | select(.id == $id) | .command[] | select(. != "--fail-open")' .opencode/agentstacks/argv.json)}")
print -r -- "argv: ${(qq)args[@]}"
"${args[@]}" < "$input" > ../stdout.txt 2> ../stderr.txt
print -r -- "exit=$?"
print -r -- "stdout:"; cat ../stdout.txt
print -r -- "stderr:"; cat ../stderr.txt
```

Payload files (`printf '%s' … >`, no trailing newline; `p-empty.json` via `: >`):

```text
p-read-x.json    {"event":"tool.before","tool":"read","path":"x"}
p-secret.json    {"event":"tool.before","tool":"edit","path":"secret.txt"}
p-rs.json        {"event":"tool.before","tool":"read","path":"src/main.rs","session_id":"s1"}
p-unmapped.json  {"event":"session.idle"}
p-empty.json     (0 bytes)
p-notjson.json   not json
p-array.json     [1,2]
```

**Command**:

```sh
cd $S/as-03/proj && $A init --tool ext/opencode --pack packs/core --pack packs/guideline-rust --pack packs/probe --yes > ../init.out 2>&1; echo "exit=$?" >> ../init.out
grep -v "create   .opencode/skills\|create   .opencode/agentstacks/core/\|create   .opencode/agents" ../init.out
cat .opencode/agentstacks/argv.json
```

**Output** (init; the grep pattern `.opencode/agents` also matches `.opencode/agentstacks`, so all
51 `create` lines are cut here):

```text
agentstacks plan

  skip     core/hook/0: probe: message hooks not rendered
  skip     core/hook/3: probe: message hooks not rendered

  airsl    0.1.5 (new pin)
  note     native/claude: not for opencode

51 to write, 0 to delete, 0 unchanged, 2 skipped
applied: 51 written, 0 deleted
exit=0
```

The `create` lines outside skills, agents and core root files (`grep "^  create" ../init.out | grep -v -e "create   .opencode/skills/" -e "create   .opencode/agents/" -e "create   .opencode/agentstacks/core/"`):

```text
  create   .opencode/agentstacks/argv.json                                                  opencode
  create   .opencode/agentstacks/guideline-rust/enforcement.json                            guideline-rust
  create   .opencode/agentstacks/probe/hooks/deny.lua                                       probe
  create   .opencode/agentstacks/probe/hooks/echo.lua                                       probe
  create   .opencode/agentstacks/runtime.lua                                                opencode
  create   .opencode/agentstacks/translate.lua                                              opencode
```

`argv.json`, raw, as the `plan` payload gave each lua hook:

```json
[{"command":["airsl","run","--fail-open","--policy","confined","--allow-read","${TMPDIR:-/tmp}","--allow-write","${TMPDIR:-/tmp}","--allow-env","TMPDIR",".opencode/agentstacks/runtime.lua","core.hooks.rearm","0","neutral"],"event":"session.start","id":"core/hook/1","match":["compact"],"may_block":false},{"command":["airsl","run","--fail-open","--policy","confined","--allow-read",".opencode/skills/core-discuss",".opencode/agentstacks/runtime.lua","core.scripts.style","0","neutral",".opencode/skills/core-discuss/references/protocol.md"],"event":"session.start","id":"core/hook/2","may_block":false},{"command":["airsl","run","--fail-open","--policy","confined","--allow-read",".opencode/skills/core-discuss",".opencode/agentstacks/runtime.lua","core.scripts.style","0","neutral",".opencode/skills/core-discuss/references/protocol.md","true"],"event":"prompt.submit","id":"core/hook/4","may_block":false},{"command":["airsl","run","--fail-open","--policy","confined","--allow-read","/","--allow-write","${TMPDIR:-/tmp}","--allow-env","HOME","--allow-env","TMPDIR","--allow-env","AIRSSTACK_HOME",".opencode/agentstacks/runtime.lua","core.hooks.enforce","0","neutral",".opencode/agentstacks/core"],"event":"tool.before","id":"core/hook/5","match":["read","edit","write"],"may_block":false},{"command":["airsl","run","--fail-open","--policy","confined","--allow-read","/","--allow-env","TMPDIR",".opencode/agentstacks/runtime.lua","core.scripts.handoff_check_hook","1","neutral"],"event":"tool.after","id":"core/hook/6","match":["write"],"may_block":true},{"command":["airsl","run","--fail-open","--policy","confined","--allow-read","/","--allow-env","TMPDIR",".opencode/agentstacks/runtime.lua","core.scripts.comm_check_hook","1","neutral"],"event":"tool.after","id":"core/hook/7","match":["write"],"may_block":true},{"agents":["coder","explorer","reviewer"],"command":["airsl","run","--fail-open","--policy","confined","--allow-read","/","--allow-env","TMPDIR",".opencode/agentstacks/runtime.lua","core.scripts.handoff_check_hook","1","neutral"],"event":"agent.stop","id":"core/hook/8","may_block":true},{"command":["airsl","run","--fail-open","--policy","confined","--allow-read","/","--allow-env","TMPDIR",".opencode/agentstacks/runtime.lua","core.scripts.comm_check_hook","1","neutral"],"event":"agent.stop","id":"core/hook/9","may_block":true},{"command":["airsl","run","--fail-open","--policy","confined",".opencode/agentstacks/runtime.lua","probe.hooks.echo","0","neutral","a1","${TMPDIR:-/tmp}"],"event":"tool.before","id":"probe/hook/0","may_block":false},{"command":["airsl","run","--fail-open","--policy","confined",".opencode/agentstacks/runtime.lua","probe.hooks.deny","1","neutral"],"event":"tool.before","id":"probe/hook/1","may_block":true}]
```

`<root>/translate.lua` as written by the stub (`cat proj/.opencode/agentstacks/translate.lua`):

```lua
-- probe translator: payload {"event":"tool.before","tool":"read","path":"x"} -> neutral event
local M = {}
local EVENTS = { ["tool.before"] = true, ["tool.after"] = true }
function M.event(p)
  if type(p) ~= "table" or not EVENTS[p.event] then
    return nil
  end
  return { event = p.event, session_id = p.session_id or "probe-session", cwd = p.cwd, tool = p.tool, path = p.path }
end
function M.result(r, p)
  if r == nil then
    return nil
  end
  return airsstack.json.encode({ context = r.context, block = r.block, from = p.event }) .. "\n"
end
return M
```

Runs (command: `for c in "probe/hook/0 p-read-x.json" "probe/hook/1 p-secret.json" "probe/hook/1 p-read-x.json" "core/hook/5 p-rs.json" "probe/hook/0 p-unmapped.json" "probe/hook/0 p-empty.json" "probe/hook/0 p-notjson.json" "probe/hook/0 p-array.json"; do set -- ${=c}; echo "=== $1 < $2"; zsh run-hook.zsh $1 $2 2>&1; done > q3-runs.out`; zsh `time` prints in 10 ms steps):

```text
=== probe/hook/0 < p-read-x.json
argv: 'airsl' 'run' '--fail-open' '--policy' 'confined' '.opencode/agentstacks/runtime.lua' 'probe.hooks.echo' '0' 'neutral' 'a1' '${TMPDIR:-/tmp}'
stdin: {"event":"tool.before","tool":"read","path":"x"}
time: 0.01s real
exit=0
stdout:
{"context":"event=tool.before tool=read path=x session=probe-session args=a1,${TMPDIR:-/tmp}","from":"tool.before"}
stderr:
=== probe/hook/1 < p-secret.json
argv: 'airsl' 'run' '--fail-open' '--policy' 'confined' '.opencode/agentstacks/runtime.lua' 'probe.hooks.deny' '1' 'neutral'
stdin: {"event":"tool.before","tool":"edit","path":"secret.txt"}
time: 0.00s real
exit=0
stdout:
{"block":"probe: secret.txt is off limits","from":"tool.before"}
stderr:
=== probe/hook/1 < p-read-x.json
argv: 'airsl' 'run' '--fail-open' '--policy' 'confined' '.opencode/agentstacks/runtime.lua' 'probe.hooks.deny' '1' 'neutral'
stdin: {"event":"tool.before","tool":"read","path":"x"}
time: 0.00s real
exit=0
stdout:
stderr:
=== core/hook/5 < p-rs.json
argv: 'airsl' 'run' '--fail-open' '--policy' 'confined' '--allow-read' '/' '--allow-write' '${TMPDIR:-/tmp}' '--allow-env' 'HOME' '--allow-env' 'TMPDIR' '--allow-env' 'AIRSSTACK_HOME' '.opencode/agentstacks/runtime.lua' 'core.hooks.enforce' '0' 'neutral' '.opencode/agentstacks/core'
stdin: {"event":"tool.before","tool":"read","path":"src/main.rs","session_id":"s1"}
time: 0.08s real
exit=0
stdout:
{"context":"rust work is in play. The /guideline-rust-guidelines skill is MANDATORY for this work — load it now via Skill before proceeding, and apply its rules (Definition of Done + architecture).","from":"tool.before"}
stderr:
=== probe/hook/0 < p-unmapped.json
argv: 'airsl' 'run' '--fail-open' '--policy' 'confined' '.opencode/agentstacks/runtime.lua' 'probe.hooks.echo' '0' 'neutral' 'a1' '${TMPDIR:-/tmp}'
stdin: {"event":"session.idle"}
time: 0.00s real
exit=0
stdout:
stderr:
=== probe/hook/0 < p-empty.json
argv: 'airsl' 'run' '--fail-open' '--policy' 'confined' '.opencode/agentstacks/runtime.lua' 'probe.hooks.echo' '0' 'neutral' 'a1' '${TMPDIR:-/tmp}'
stdin: 
time: 0.00s real
exit=0
stdout:
stderr:
=== probe/hook/0 < p-notjson.json
argv: 'airsl' 'run' '--fail-open' '--policy' 'confined' '.opencode/agentstacks/runtime.lua' 'probe.hooks.echo' '0' 'neutral' 'a1' '${TMPDIR:-/tmp}'
stdin: not json
time: 0.00s real
exit=0
stdout:
stderr:
=== probe/hook/0 < p-array.json
argv: 'airsl' 'run' '--fail-open' '--policy' 'confined' '.opencode/agentstacks/runtime.lua' 'probe.hooks.echo' '0' 'neutral' 'a1' '${TMPDIR:-/tmp}'
stdin: [1,2]
time: 0.00s real
exit=0
stdout:
stderr:
```

Control without `--fail-open` (`for c in "probe/hook/0 p-empty.json" "probe/hook/0 p-notjson.json" "probe/hook/0 p-array.json"; … zsh run-nofailopen.zsh $1 $2 2>&1; done > q3-control.out`):

```text
=== probe/hook/0 < p-empty.json (no --fail-open)
argv: 'airsl' 'run' '--policy' 'confined' '.opencode/agentstacks/runtime.lua' 'probe.hooks.echo' '0' 'neutral' 'a1' '${TMPDIR:-/tmp}'
exit=0
stdout:
stderr:
=== probe/hook/0 < p-notjson.json (no --fail-open)
argv: 'airsl' 'run' '--policy' 'confined' '.opencode/agentstacks/runtime.lua' 'probe.hooks.echo' '0' 'neutral' 'a1' '${TMPDIR:-/tmp}'
exit=1
stdout:
stderr:
airsl: lua error in .opencode/agentstacks/runtime.lua: lua error in <json>: expected ident at line 1 column 2
stack traceback:
	[C]: in field 'payload'
	.opencode/agentstacks/runtime.lua:9: in main chunk

=== probe/hook/0 < p-array.json (no --fail-open)
argv: 'airsl' 'run' '--policy' 'confined' '.opencode/agentstacks/runtime.lua' 'probe.hooks.echo' '0' 'neutral' 'a1' '${TMPDIR:-/tmp}'
exit=0
stdout:
stderr:
```

Timing with `/usr/bin/time -p`, and the same echo argv through `sh -c` (as a shell-quoted command
string, the way the claude extension renders it):

```sh
cd $S/as-03/proj
/usr/bin/time -p airsl run --fail-open --policy confined .opencode/agentstacks/runtime.lua probe.hooks.echo 0 neutral a1 '${TMPDIR:-/tmp}' < ../p-read-x.json
/usr/bin/time -p airsl run --fail-open --policy confined --allow-read / --allow-write '${TMPDIR:-/tmp}' --allow-env HOME --allow-env TMPDIR --allow-env AIRSSTACK_HOME .opencode/agentstacks/runtime.lua core.hooks.enforce 0 neutral .opencode/agentstacks/core < ../p-rs.json
sh -c '"airsl" "run" "--fail-open" "--policy" "confined" ".opencode/agentstacks/runtime.lua" "probe.hooks.echo" "0" "neutral" "a1" "${TMPDIR:-/tmp}"' < ../p-read-x.json; echo "exit=$?"
```

```text
--- /usr/bin/time -p probe.hooks.echo
{"context":"event=tool.before tool=read path=x session=probe-session args=a1,${TMPDIR:-/tmp}","from":"tool.before"}
real 0.00
user 0.00
sys 0.00
--- /usr/bin/time -p core.hooks.enforce
{"context":"rust work is in play. The /guideline-rust-guidelines skill is MANDATORY for this work — load it now via Skill before proceeding, and apply its rules (Definition of Done + architecture).","from":"tool.before"}
real 0.02
user 0.01
sys 0.01
--- through sh -c (shell expands the var element)
{"context":"event=tool.before tool=read path=x session=probe-session args=a1,/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/","from":"tool.before"}
exit=0
```

Did `core.hooks.enforce` record its once-per-session sentinel for session `s1` when its write grant
was the literal `${TMPDIR:-/tmp}`?

```sh
echo "s1 sentinels: $(ls "$TMPDIR" | grep -c 'claudestacks-enforce-s1-')"
```

```text
s1 sentinels: 0
```

Source: the runtime reads the payload with `airsstack.hook.payload()`
(`probes/src/agentstacks/book/src/ch06-02-hooks.md:84`) and prints only `translate.result`'s string
(`:101-104`).

**Answer**:

- The argv is `airsl run --fail-open --policy confined [--allow-*]… <root>/runtime.lua <module> <0|1> neutral [args…]`,
  all project-relative, and it works when exec'd from the project dir: the payload is read from stdin
  and stdout is exactly the translator's string (`{"context":…}` / `{"block":…}`), exit 0 in every case,
  including a `may_block` block (exit code does not signal a block; the text does).
- Empty stdin, a non-object JSON (`[1,2]`) and an unmapped event: exit 0, no stdout, no stderr.
  Non-JSON stdin: exit 0 and silent under `--fail-open`; without it, exit 1 and stderr
  `airsl: lua error in .opencode/agentstacks/runtime.lua: lua error in <json>: expected ident at line 1 column 2`
  (traceback at `runtime.lua:9`, the `payload()` line). Empty stdin is silent even without `--fail-open`.
- One run takes about 10 ms or less (`real 0.00` echo; `real 0.02` enforce; 0.08 s on enforce's first, cold run).
- `{{var}}` values land verbatim in the argv. Exec'd without a shell, `${TMPDIR:-/tmp}` stays literal
  (in args and in `--allow-write`), and enforce then wrote no `s1` sentinel; through `sh -c` it expands.
  So an OpenCode shim that spawns the argv directly needs var values that are not shell expressions,
  or it must run the command through a shell.
- The plan payload gives `agents` by pack-local name (`["coder","explorer","reviewer"]` on `core/hook/8`).

## P4: `reads` for a file that does not exist yet

**Setup / command**: book only, plus the P1 run (the claude extension `read` `.claude/settings.json`,
which did not exist in `$S/as-01/proj`).

**Output** (citations, copied to `probes/src/agentstacks/book/src/`):

- `probes/src/agentstacks/book/src/ch06-01-extensions.md:30`: "Its `reads` key is optional: a list of project paths the extension may read with `agentstacks.read`."
- `ch06-01-extensions.md:43-55`: each entry must be relative, inside the project, `/`-separated, non-empty, not repeated; the refusals table lists no rule about the file existing.
- `ch06-01-extensions.md:57-58`: naming `agentstacks.toml` or `agentstacks.lock` in `reads` is allowed.
- `ch06-01-extensions.md:104`: `agentstacks.read(path)` returns "the file's bytes as a Lua string, or `nil` when no regular file is there".
- `ch06-01-extensions.md:112-114`: the path must be listed in `extension.reads` and resolve inside the project.
- `ch06-01-extensions.md:118`: "The core records what each read returned (the hash, or "absent")."
- `probes/src/agentstacks/book/src/ch05-00-plan-and-apply.md:87`: shared means the path is in `reads` and the extension read it in this run; `:94`: shared + absent + `read` returned absent → `create`; `:122`: "`.claude/settings.json` absent; the extension read `nil` and proposes it | `create` (shared), new-file mode".
- P1 output: `create   .claude/settings.json   shared`, and it is not in the lock (91 locked of 92 written).

**Answer**: Any `reads` entry that is a valid relative project path is allowed whether or not the
file exists; `read` returns `nil` for a missing file, the core records "absent", and a write to it is
a shared `create` that is never locked (seen in P1 for `.claude/settings.json`).

## P5: `{{var …}}` names in the five packs

**Command** (repo `packs/`):

```sh
grep -n "exclude" */pack.toml
grep -rhoE '\{\{var [^}]*\}\}' . | sort | uniq -c | sort -rn
grep -roE '\{\{var [a-z_]+\}\}' . | sort | uniq -c | awk '{print $1"\t"$2" "$3}'
```

**Output**:

```text
journal/pack.toml:1:exclude = ["README.md", "**/*_test.lua"]
core/pack.toml:1:exclude = ["README.md", "**/*_test.lua"]
guideline-rust/pack.toml:2:exclude = ["README.md"]
guideline-python/pack.toml:2:exclude = ["README.md"]
sdlc/pack.toml:1:exclude = ["README.md", "**/*_test.lua"]

  74 {{var project_dir}}
   7 {{var session_id}}
   5 {{var arguments}}
   4 {{var tmpdir}}
   2 {{var airsstack_home}}
```

```text
15	./core/native/claude/skills/core-statusline/SKILL.md:{{var project_dir}}
3	./core/pack.toml:{{var tmpdir}}
1	./core/scripts/discuss_skill_test.lua:{{var project_dir}}
1	./core/scripts/discuss_skill_test.lua:{{var session_id}}
2	./core/skills/context-handoff/SKILL.md:{{var project_dir}}
2	./core/skills/discuss/SKILL.md:{{var arguments}}
4	./core/skills/discuss/SKILL.md:{{var project_dir}}
1	./core/skills/discuss/SKILL.md:{{var session_id}}
3	./core/skills/enforce-doctor/SKILL.md:{{var arguments}}
2	./core/skills/enforce-doctor/SKILL.md:{{var project_dir}}
1	./core/skills/enforce-doctor/SKILL.md:{{var tmpdir}}
2	./journal/pack.toml:{{var airsstack_home}}
1	./journal/pack.toml:{{var project_dir}}
3	./journal/skills/capture/SKILL.md:{{var project_dir}}
3	./journal/skills/capture/SKILL.md:{{var session_id}}
1	./journal/skills/helped/SKILL.md:{{var project_dir}}
1	./journal/skills/link/SKILL.md:{{var project_dir}}
3	./journal/skills/note/SKILL.md:{{var project_dir}}
2	./journal/skills/note/SKILL.md:{{var session_id}}
1	./journal/skills/recall/SKILL.md:{{var project_dir}}
4	./journal/skills/review/SKILL.md:{{var project_dir}}
2	./journal/skills/setup/SKILL.md:{{var project_dir}}
5	./sdlc/skills/adr/SKILL.md:{{var project_dir}}
5	./sdlc/skills/design/SKILL.md:{{var project_dir}}
1	./sdlc/skills/distill/SKILL.md:{{var project_dir}}
3	./sdlc/skills/execute/SKILL.md:{{var project_dir}}
3	./sdlc/skills/intent/SKILL.md:{{var project_dir}}
3	./sdlc/skills/milestone/SKILL.md:{{var project_dir}}
5	./sdlc/skills/plan/SKILL.md:{{var project_dir}}
3	./sdlc/skills/roadmap/SKILL.md:{{var project_dir}}
1	./sdlc/skills/setup/SKILL.md:{{var project_dir}}
3	./sdlc/skills/status/SKILL.md:{{var project_dir}}
2	./sdlc/skills/triage/SKILL.md:{{var project_dir}}
```

The engine's own list, from a stub that gives no vars (`$S/as-05/proj`: all five packs copied from
the repo, stub records the `values` payload's `vars`):

```lua $S/as-05/proj/ext/opencode/adapter.lua
-- Probe-only stub: gives every skill and agent a location but no vars, so the core skips each item
-- that holds a {{var}} token; writes the values payload's `vars` list to <root>/vars.json.
local ROOT = ".opencode/agentstacks"
local seen_vars

agentstacks.on("values", function(q)
  seen_vars = q.vars
  local out = { root = ROOT, skills = {}, agents = {}, vars = {} }
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills or {}) do
      out.skills[p.name .. "/" .. s] = { invoke = "/" .. p.name .. "-" .. s, dir = ".opencode/skills/" .. p.name .. "-" .. s }
    end
    for _, a in ipairs(p.agents or {}) do
      out.agents[p.name .. "/" .. a] = p.name .. "-" .. a
    end
  end
  return out
end)

agentstacks.on("plan", function(q)
  for _, p in ipairs(q.packs) do
    for _, skill in ipairs(p.skills or {}) do
      for _, f in ipairs(skill.files) do
        agentstacks.copy({ path = skill.dir .. "/" .. f.rel, file = f.id })
      end
    end
    for _, a in ipairs(p.agents or {}) do
      agentstacks.write({ path = ".opencode/agents/" .. p.name .. "-" .. a.name .. ".md", content = a.text, covers = { a.id } })
    end
    for _, h in ipairs(p.hooks or {}) do
      agentstacks.skip({ item = h.id, reason = "probe: hooks not rendered" })
    end
    for _, h in ipairs((p.native and p.native.hooks) or {}) do
      agentstacks.skip({ item = h.id, reason = "probe: hooks not rendered" })
    end
  end
  agentstacks.write({ path = ROOT .. "/vars.json", content = airsstack.json.encode(seen_vars) })
end)
```

```toml $S/as-05/proj/ext/opencode/extension.toml
[extension]
name        = "opencode"
version     = "0.1.0"
api         = 0
description = "Probe-only stub: gives no vars and records the values payload's vars list"
entry       = "adapter.lua"
```

```sh
cd $S/as-05/proj && $A init --tool ext/opencode --pack packs/core --pack packs/sdlc --pack packs/journal --pack packs/guideline-rust --pack packs/guideline-python --yes > ../init.out 2>&1; echo "exit=$?" >> ../init.out
grep -v '^  create' ../init.out; cat .opencode/agentstacks/vars.json
```

```text
agentstacks plan

  skip     core/skill/context-handoff: opencode gave no value for {{var project_dir}}
  skip     core/skill/discuss: opencode gave no value for {{var arguments}}
  skip     core/skill/enforce-doctor: opencode gave no value for {{var arguments}}
  skip     core/hook/1: opencode gave no value for {{var tmpdir}}
  skip     core/hook/5: opencode gave no value for {{var tmpdir}}
  skip     journal/skill/capture: opencode gave no value for {{var session_id}}
  skip     journal/skill/helped: opencode gave no value for {{var project_dir}}
  skip     journal/skill/link: opencode gave no value for {{var project_dir}}
  skip     journal/skill/note: opencode gave no value for {{var session_id}}
  skip     journal/skill/recall: opencode gave no value for {{var project_dir}}
  skip     journal/skill/review: opencode gave no value for {{var project_dir}}
  skip     journal/skill/setup: opencode gave no value for {{var project_dir}}
  skip     journal/hook/0: opencode gave no value for {{var airsstack_home}}
  skip     sdlc/skill/adr: opencode gave no value for {{var project_dir}}
  skip     sdlc/skill/design: opencode gave no value for {{var project_dir}}
  skip     sdlc/skill/distill: opencode gave no value for {{var project_dir}}
  skip     sdlc/skill/execute: opencode gave no value for {{var project_dir}}
  skip     sdlc/skill/intent: opencode gave no value for {{var project_dir}}
  skip     sdlc/skill/milestone: opencode gave no value for {{var project_dir}}
  skip     sdlc/skill/plan: opencode gave no value for {{var project_dir}}
  skip     sdlc/skill/roadmap: opencode gave no value for {{var project_dir}}
  skip     sdlc/skill/setup: opencode gave no value for {{var project_dir}}
  skip     sdlc/skill/status: opencode gave no value for {{var project_dir}}
  skip     sdlc/skill/triage: opencode gave no value for {{var project_dir}}
  skip     core/hook/0: probe: hooks not rendered
  skip     core/hook/2: probe: hooks not rendered
  skip     core/hook/3: probe: hooks not rendered
  skip     core/hook/4: probe: hooks not rendered
  skip     core/hook/6: probe: hooks not rendered
  skip     core/hook/7: probe: hooks not rendered
  skip     core/hook/8: probe: hooks not rendered
  skip     core/hook/9: probe: hooks not rendered
  skip     journal/hook/1: probe: hooks not rendered
  skip     sdlc/hook/0: probe: hooks not rendered
  skip     sdlc/hook/1: probe: hooks not rendered

  airsl    0.1.5 (new pin)
  note     native/claude: not for opencode

95 to write, 0 to delete, 0 unchanged, 35 skipped
applied: 95 written, 0 deleted
exit=0

["airsstack_home","arguments","project_dir","session_id","tmpdir"]
```

(95 `create` lines cut.)

**Answer**: Five names: `airsstack_home`, `arguments`, `project_dir`, `session_id`, `tmpdir` (the
`values` payload lists exactly these, sorted). By item kind, excluding `core/scripts/discuss_skill_test.lua`
(excluded by `**/*_test.lua`) and `core/native/claude/…` (claude only):

| var | skills (uses / items) | agents | hooks (items) |
|---|---|---|---|
| `project_dir` | 57 / 21 (core 3, journal 7, sdlc 11) | 0 | 1 (`journal/hook/0` read grant) |
| `session_id` | 6 / 3 (core discuss, journal capture, journal note) | 0 | 0 |
| `arguments` | 5 / 2 (core discuss, core enforce-doctor) | 0 | 0 |
| `tmpdir` | 1 / 1 (core enforce-doctor) | 0 | 2 (`core/hook/1`, `core/hook/5`; 3 uses in grants) |
| `airsstack_home` | 0 | 0 | 1 (`journal/hook/0`, 2 uses in grants) |

With no vars, 21 skills and 3 hooks are core-skipped for opencode (as-05 plan above); no agent uses a var.
`core/native/claude/skills/core-statusline/SKILL.md` holds 15 more `project_dir` uses, claude only.
