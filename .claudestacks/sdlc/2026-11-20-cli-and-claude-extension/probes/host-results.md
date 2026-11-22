# Host probe: the payloads an extension receives, plan and apply (real agentstacks)

- Library: `agentstacks` v0.0.0 at worktree commit 8107626 (path dependency, `crates/agentstacks`), `airsl v0.1.4` (from `cargo tree -i airsl`), rustc 1.99.0. airsl binary on PATH: `airsl 0.1.4`.
- Probe crate: `/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/host-probe` (Cargo.lock copied from the worktree). main.rs saved as `probes/host-main.rs`.
- Temp project (left in place, now applied): `/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/host-project`
- Real adapters: `StdWorkspace::open`, `GitCli::new("git", HOME)`, `SystemBinary::from_env()`, `AirslAdapter::new()`; flows `agentstacks::app::plan::plan` and `agentstacks::app::apply::apply`.

## Fixture files

### `agentstacks.toml`

```
api = 0

[tools.probe]
path = "ext"

[packs.core]
path = "packs/core"
```

### `packs/core/pack.toml`

```
[pack]
name = "core"
version = "0.1.0"
api = 0
description = "Probe pack"

[requires]
airsl = ">=0.1.4"

[[hooks]]
kind = "message"
event = "session.end"
text = "Reminder: run {{skill status}} before you stop."

[[hooks]]
kind = "lua"
event = "tool.before"
match = ["read", "edit", "write"]
run = "{{root}}/hooks/enforce.lua"
args = ["--x", "{{root}}/data"]

[hooks.grants]
read = ["{{root}}"]
write = ["{{var tmpdir}}"]
env = ["HOME"]
```

### `packs/core/skills/status/SKILL.md`

```
---
name: status
description: Show the status board
invocation: user
argument_hint: "[chain]"
---

Args: {{var arguments}}
Session: {{var session_id}}
See {{skill_dir status}}/references/x.md
```

### `packs/core/skills/status/references/x.md`

```
# Reference x
```

### `packs/core/agents/coder.md`

```
---
name: coder
description: Writes code
model: balanced
effort: high
tools: [read, shell]
native:
  claude:
    model: x
  probe:
    color: blue
---

You write code.
```

### `packs/core/hooks/enforce.lua`

```
return function(e) return nil end
```

### `packs/core/native/probe/hooks.toml`

```
[[hooks]]
kind = "lua"
event = "SubagentStop"
match = ["x"]
run = "hooks/enforce.lua"
```

### `ext/extension.toml`

```
[extension]
name = "probe"
version = "0.1.0"
api = 0
description = "Probe extension that dumps its payloads"
entry = "adapter.lua"
reads = [".claude/settings.json"]
```

### `ext/adapter.lua`

```
local values_q = nil

local function sorted_keys(t)
  local ks = {}
  for k, _ in pairs(t) do ks[#ks + 1] = tostring(k) end
  table.sort(ks)
  return table.concat(ks, ",")
end

agentstacks.on("values", function(q)
  values_q = q
  local out = {
    root = ".claude/agentstacks",
    skills = {},
    agents = {},
    vars = { session_id = "${CLAUDE_SESSION_ID}", arguments = "$ARGUMENTS", tmpdir = "$TMPDIR" },
  }
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills) do
      out.skills[p.name .. "/" .. s] = { invoke = "/" .. s, dir = ".claude/skills/" .. s }
    end
    for _, a in ipairs(p.agents) do
      out.agents[p.name .. "/" .. a] = a
    end
  end
  return out
end)

agentstacks.on("plan", function(q)
  local ids = {}
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills or {}) do
      for _, f in ipairs(s.files or {}) do ids[#ids + 1] = f.id end
    end
    for _, a in ipairs(p.agents or {}) do ids[#ids + 1] = a.id end
    for _, h in ipairs(p.hooks or {}) do ids[#ids + 1] = h.id end
    local n = p.native or {}
    for _, f in ipairs(n.files or {}) do ids[#ids + 1] = f.id end
    for _, h in ipairs(n.hooks or {}) do ids[#ids + 1] = h.id end
  end
  local dump = {
    values_q = values_q,
    plan_q = q,
    covered = ids,
    json_keys = sorted_keys(airsstack.json),
    agentstacks_keys = sorted_keys(agentstacks),
    agentstacks_json_keys = sorted_keys(agentstacks.json),
    airsstack_keys = sorted_keys(airsstack),
    enc = airsstack.json.encode('a "q": b\nc\\d é'),
  }
  agentstacks.write { path = "payload.json", content = airsstack.json.encode(dump), covers = ids }
  agentstacks.write { path = q.root .. "/translate.lua", content = "local M = {}\nfunction M.event(p) return nil end\nfunction M.result(r) return nil end\nreturn M\n" }
  local old = agentstacks.read(".claude/settings.json")
  local new = agentstacks.json.edit(old or "", { "hooks", "SessionEnd" }, { { hooks = { { type = "command", command = "echo hi" } } } })
  agentstacks.write { path = ".claude/settings.json", content = new }
end)
```

Attempt 1 had `match = "x"` in `native/probe/hooks.toml` (as briefed); attempt 2 used `match = ["x"]` (the file above). No other fixture change.

## Probe main.rs (`probes/host-main.rs`)

```rust
//! Probe: run the real agentstacks plan (and optionally apply) against a fixture project.
use agentstacks::fs::StdWorkspace;
use agentstacks::host::AirslAdapter;
use agentstacks::process::{GitCli, SystemBinary};
use agentstacks::types::lock::write_lock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut argv = std::env::args().skip(1);
    let root = argv.next().ok_or("usage: host-probe <project> [apply]")?;
    let do_apply = argv.next().as_deref() == Some("apply");

    let workspace = StdWorkspace::open(&root)?;
    let git = GitCli::new("git", std::env::var_os("HOME"));
    let binary = SystemBinary::from_env();
    let adapter = AirslAdapter::new();

    let plan = match agentstacks::app::plan::plan(&workspace, &git, &binary, &adapter) {
        Ok(plan) => plan,
        Err(found) => {
            println!("PLAN: Err");
            for d in found.into_sorted() {
                println!("  {d}");
            }
            return Ok(());
        }
    };
    println!("PLAN: Ok");
    println!("== steps");
    for step in plan.steps() {
        let owner = step
            .owner
            .as_ref()
            .map_or_else(|| "-".to_owned(), ToString::to_string);
        println!("- {} | {:?} | owner={} | origin={:?}", step.path, step.action, owner, step.origin);
    }
    println!("== step contents");
    for step in plan.steps() {
        let Some(bytes) = step.new.as_ref() else { continue };
        println!("--- {} ({} bytes)", step.path, bytes.len());
        if step.path.to_string() == "payload.json" {
            println!("raw: {}", String::from_utf8_lossy(bytes));
            let value: serde_json::Value = serde_json::from_slice(bytes)?;
            println!("pretty:\n{}", serde_json::to_string_pretty(&value)?);
        } else {
            println!("{}", String::from_utf8_lossy(bytes));
        }
    }
    println!("== skips");
    for s in plan.skips() {
        println!("- {s:?}");
    }
    println!("== notes");
    for n in plan.notes() {
        println!("- {n}");
    }
    println!("== binaries");
    for b in plan.binaries() {
        println!("- {b:?}");
    }
    println!("== airsl pin");
    println!("{:?}", plan.airsl());
    println!("== new lock");
    print!("{}", write_lock(plan.lock()));

    if do_apply {
        println!("== apply");
        match agentstacks::app::apply::apply(&plan, &workspace) {
            Ok(()) => println!("APPLY: Ok"),
            Err(found) => {
                println!("APPLY: Err");
                for d in found.into_sorted() {
                    println!("  {d}");
                }
            }
        }
    }
    Ok(())
}
```

## Commands

```
cd /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/host-probe && cargo build -q
./target/debug/host-probe /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/host-project            # attempt 1, then attempt 2 (P1, P2)
./target/debug/host-probe /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/host-project apply      # P3: plan again, then apply
cd /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/host-project && find .claude agentstacks.lock payload.json -type f | sort; cat .claude/settings.json; cat agentstacks.lock
```

## Attempt 1 (refusal by the loader), raw output

```
PLAN: Err
  /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/host-project/packs/core/native/probe/hooks.toml: hooks[0].match: expected array, got string
```

Note: the location is the absolute canonical path of the pack file for a `path` source, not project-relative.

## P1 + P2: plan, raw output (attempt 2)

```
PLAN: Ok
== steps
- .claude/agentstacks/core/hooks/enforce.lua | Create | owner=core | origin=Some(Core { pack: PackName("core") })
- .claude/agentstacks/runtime.lua | Create | owner=probe | origin=Some(Tool { tool: ToolName("probe") })
- .claude/agentstacks/translate.lua | Create | owner=probe | origin=Some(Extension { tool: ToolName("probe"), file: TreePath("adapter.lua"), line: 52 })
- .claude/settings.json | Create | owner=- | origin=Some(Extension { tool: ToolName("probe"), file: TreePath("adapter.lua"), line: 55 })
- payload.json | Create | owner=core | origin=Some(Extension { tool: ToolName("probe"), file: TreePath("adapter.lua"), line: 51 })
== step contents
--- .claude/agentstacks/core/hooks/enforce.lua (34 bytes)
return function(e) return nil end

--- .claude/agentstacks/runtime.lua (905 bytes)
-- agentstacks hook runtime, api 0.
-- usage: airsl run ... runtime.lua <hook-module> <may_block: 0|1> <kind: neutral|native> [args...]
local translate = require("translate")
local name, may_block, kind = arg[1], arg[2] == "1", arg[3]
local args = {}
for i = 4, #arg do
  args[#args + 1] = arg[i]
end
local payload = airsstack.hook.payload()
local event
if kind == "native" then
  event = { payload = payload, args = args }
else
  event = translate.event(payload)
  if event == nil then
    return
  end
  event.args = args
end
local result = require(name)(event)
if type(result) == "table" and result.block ~= nil and not may_block then
  airsstack.stdio.error("agentstacks: hook " .. name .. " tried to block but is not declared may_block; reason: "
    .. tostring(result.block) .. "\n")
  result.block = nil
end
local out = translate.result(result)
if out ~= nil then
  airsstack.stdio.write(out)
end

--- .claude/agentstacks/translate.lua (93 bytes)
local M = {}
function M.event(p) return nil end
function M.result(r) return nil end
return M

--- .claude/settings.json (179 bytes)
{
  "hooks": {
    "SessionEnd": [
      {
        "hooks": [
          {
            "command": "echo hi",
            "type": "command"
          }
        ]
      }
    ]
  }
}
--- payload.json (2020 bytes)
raw: {"agentstacks_json_keys":"edit","agentstacks_keys":"copy,json,on,read,require,require_binary,skip,write","airsstack_keys":"agentstacks,ext,json","covered":["core/skill/status/SKILL.md","core/skill/status/references/x.md","core/agent/coder","core/hook/0","core/hook/1","core/native-hook/0"],"enc":"\"a \\\"q\\\": b\\nc\\\\d é\"","json_keys":"decode,encode,encode_pretty","plan_q":{"api":0,"packs":[{"agents":[{"id":"core/agent/coder","meta":{"description":"Writes code","effort":"high","model":"balanced","native":{"color":"blue"},"tools":["read","shell"]},"name":"coder","text":"\nYou write code.\n"}],"hooks":[{"event":"session.end","id":"core/hook/0","kind":"message","text":"Reminder: run /status before you stop."},{"command":["airsl","run","--fail-open","--policy","confined","--allow-read",".claude/agentstacks/core","--allow-write","$TMPDIR","--allow-env","HOME",".claude/agentstacks/runtime.lua","core.hooks.enforce","0","neutral","--x",".claude/agentstacks/core/data"],"event":"tool.before","id":"core/hook/1","kind":"lua","match":["read","edit","write"],"may_block":false}],"name":"core","native":{"files":[],"hooks":[{"command":["airsl","run","--fail-open","--policy","confined",".claude/agentstacks/runtime.lua","core.hooks.enforce","0","native"],"event":"SubagentStop","id":"core/native-hook/0","kind":"lua","match":["x"],"may_block":false}]},"skills":[{"dir":".claude/skills/status","files":[{"id":"core/skill/status/SKILL.md","rel":"SKILL.md","text":"\nArgs: $ARGUMENTS\nSession: ${CLAUDE_SESSION_ID}\nSee .claude/skills/status/references/x.md\n"},{"id":"core/skill/status/references/x.md","rel":"references/x.md","text":"# Reference x\n"}],"id":"core/skill/status","meta":{"argument_hint":"[chain]","description":"Show the status board","invocation":"user"},"name":"status"}],"version":"0.1.0"}],"root":".claude/agentstacks","tool":"probe"},"values_q":{"api":0,"packs":[{"agents":["coder"],"name":"core","skills":["status"],"version":"0.1.0"}],"tool":"probe","vars":["arguments","session_id","tmpdir"]}}
pretty:
{
  "agentstacks_json_keys": "edit",
  "agentstacks_keys": "copy,json,on,read,require,require_binary,skip,write",
  "airsstack_keys": "agentstacks,ext,json",
  "covered": [
    "core/skill/status/SKILL.md",
    "core/skill/status/references/x.md",
    "core/agent/coder",
    "core/hook/0",
    "core/hook/1",
    "core/native-hook/0"
  ],
  "enc": "\"a \\\"q\\\": b\\nc\\\\d é\"",
  "json_keys": "decode,encode,encode_pretty",
  "plan_q": {
    "api": 0,
    "packs": [
      {
        "agents": [
          {
            "id": "core/agent/coder",
            "meta": {
              "description": "Writes code",
              "effort": "high",
              "model": "balanced",
              "native": {
                "color": "blue"
              },
              "tools": [
                "read",
                "shell"
              ]
            },
            "name": "coder",
            "text": "\nYou write code.\n"
          }
        ],
        "hooks": [
          {
            "event": "session.end",
            "id": "core/hook/0",
            "kind": "message",
            "text": "Reminder: run /status before you stop."
          },
          {
            "command": [
              "airsl",
              "run",
              "--fail-open",
              "--policy",
              "confined",
              "--allow-read",
              ".claude/agentstacks/core",
              "--allow-write",
              "$TMPDIR",
              "--allow-env",
              "HOME",
              ".claude/agentstacks/runtime.lua",
              "core.hooks.enforce",
              "0",
              "neutral",
              "--x",
              ".claude/agentstacks/core/data"
            ],
            "event": "tool.before",
            "id": "core/hook/1",
            "kind": "lua",
            "match": [
              "read",
              "edit",
              "write"
            ],
            "may_block": false
          }
        ],
        "name": "core",
        "native": {
          "files": [],
          "hooks": [
            {
              "command": [
                "airsl",
                "run",
                "--fail-open",
                "--policy",
                "confined",
                ".claude/agentstacks/runtime.lua",
                "core.hooks.enforce",
                "0",
                "native"
              ],
              "event": "SubagentStop",
              "id": "core/native-hook/0",
              "kind": "lua",
              "match": [
                "x"
              ],
              "may_block": false
            }
          ]
        },
        "skills": [
          {
            "dir": ".claude/skills/status",
            "files": [
              {
                "id": "core/skill/status/SKILL.md",
                "rel": "SKILL.md",
                "text": "\nArgs: $ARGUMENTS\nSession: ${CLAUDE_SESSION_ID}\nSee .claude/skills/status/references/x.md\n"
              },
              {
                "id": "core/skill/status/references/x.md",
                "rel": "references/x.md",
                "text": "# Reference x\n"
              }
            ],
            "id": "core/skill/status",
            "meta": {
              "argument_hint": "[chain]",
              "description": "Show the status board",
              "invocation": "user"
            },
            "name": "status"
          }
        ],
        "version": "0.1.0"
      }
    ],
    "root": ".claude/agentstacks",
    "tool": "probe"
  },
  "values_q": {
    "api": 0,
    "packs": [
      {
        "agents": [
          "coder"
        ],
        "name": "core",
        "skills": [
          "status"
        ],
        "version": "0.1.0"
      }
    ],
    "tool": "probe",
    "vars": [
      "arguments",
      "session_id",
      "tmpdir"
    ]
  }
}
== skips
== notes
== binaries
== airsl pin
Some(AirslPin { locked: None, installed: PackVersion(Version { major: 0, minor: 1, patch: 4 }) })
== new lock
api = 0
# Generated by agentstacks. Do not edit.

[airsl]
version = "0.1.4"

[[tool]]
name    = "probe"
version = "0.1.0"
path    = "ext"
tree    = "sha256:18c57ff4919d1710b7637cf1df74142f8abe47a1f916b88859d3d4751daef3ab"

[[pack]]
name    = "core"
version = "0.1.0"
path    = "packs/core"
tree    = "sha256:07d7b84fbb2a1798704be5a7c7c4163a6b1d02a3c78f2d3624eabff12e0a0840"

[[file]]
path = ".claude/agentstacks/core/hooks/enforce.lua"
hash = "sha256:ed96dd22fe3daba5995ed15029e83d8e76105e594e83b583ad425318197716e6"
by   = "core"

[[file]]
path = ".claude/agentstacks/runtime.lua"
hash = "sha256:4ce3f46b94f471cd15beac1d987207647b082d0399275e0485b64f1f1b2e72f4"
by   = "probe"

[[file]]
path = ".claude/agentstacks/translate.lua"
hash = "sha256:d4007ee83cb0fdbe18bbb1fac98a2565e38bab50c7707c875ff88ff89781c408"
by   = "probe"

[[file]]
path = "payload.json"
hash = "sha256:bc97216603bf2e641ca45af0f6b437f37050e813f2e92471e8b47c0acd137962"
by   = "core"
```

## P3: apply, raw output

The plan section of the apply run was byte-identical to P1 (`diff` printed nothing, then `IDENTICAL`). The apply tail:

```
== apply
APPLY: Ok
```

Files on disk after apply:

```
== files
.claude/agentstacks/core/hooks/enforce.lua
.claude/agentstacks/runtime.lua
.claude/agentstacks/translate.lua
.claude/settings.json
agentstacks.lock
payload.json
== .claude/settings.json
{
  "hooks": {
    "SessionEnd": [
      {
        "hooks": [
          {
            "command": "echo hi",
            "type": "command"
          }
        ]
      }
    ]
  }
}
== agentstacks.lock
api = 0
# Generated by agentstacks. Do not edit.

[airsl]
version = "0.1.4"

[[tool]]
name    = "probe"
version = "0.1.0"
path    = "ext"
tree    = "sha256:18c57ff4919d1710b7637cf1df74142f8abe47a1f916b88859d3d4751daef3ab"

[[pack]]
name    = "core"
version = "0.1.0"
path    = "packs/core"
tree    = "sha256:07d7b84fbb2a1798704be5a7c7c4163a6b1d02a3c78f2d3624eabff12e0a0840"

[[file]]
path = ".claude/agentstacks/core/hooks/enforce.lua"
hash = "sha256:ed96dd22fe3daba5995ed15029e83d8e76105e594e83b583ad425318197716e6"
by   = "core"

[[file]]
path = ".claude/agentstacks/runtime.lua"
hash = "sha256:4ce3f46b94f471cd15beac1d987207647b082d0399275e0485b64f1f1b2e72f4"
by   = "probe"

[[file]]
path = ".claude/agentstacks/translate.lua"
hash = "sha256:d4007ee83cb0fdbe18bbb1fac98a2565e38bab50c7707c875ff88ff89781c408"
by   = "probe"

[[file]]
path = "payload.json"
hash = "sha256:bc97216603bf2e641ca45af0f6b437f37050e813f2e92471e8b47c0acd137962"
by   = "core"
```

## Findings

1. **values payload** keys: `api`, `tool`, `packs[]` (`name`, `version`, `skills` = list of names, `agents` = list of names), `vars` (sorted var names used by this tool's packs: `arguments`, `session_id`, `tmpdir`). Matches docs/extensions.md section 6.
2. **plan payload** keys: `api`, `tool`, `root`, `packs[]` with `name`, `version`, `skills[]` (`id`, `name`, `dir`, `meta`, `files[]` of `id`/`rel`/`text`), `agents[]` (`id`, `name`, `meta`, `text`), `hooks[]`, `native` (`files[]`, `hooks[]`). Matches the docs example.
3. `meta` for the skill: `argument_hint`, `description`, `invocation` (no `name`). For the agent: `description`, `effort`, `model`, `tools`, `native`. `meta.native` is **this tool's entry value unwrapped** (`{"color":"blue"}`), and the `claude:` entry is dropped for tool `probe`.
4. `text` of SKILL.md and of the agent starts with `\n`: the blank line after the closing `---` is kept in the body.
5. Lua hook `command` argv as delivered: `["airsl","run","--fail-open","--policy","confined","--allow-read",".claude/agentstacks/core","--allow-write","$TMPDIR","--allow-env","HOME",".claude/agentstacks/runtime.lua","core.hooks.enforce","0","neutral","--x",".claude/agentstacks/core/data"]`. `{{root}}` is `<root>/core`; `{{var tmpdir}}` is the literal `$TMPDIR` (so the extension must not single-quote it). Project-relative paths throughout.
6. Native hook entry: `{"command":[..., ".claude/agentstacks/runtime.lua","core.hooks.enforce","0","native"],"event":"SubagentStop","id":"core/native-hook/0","kind":"lua","match":["x"],"may_block":false}`, no `agents`. Matches docs. A string `match` in `native/<tool>/hooks.toml` is refused: `…/native/probe/hooks.toml: hooks[0].match: expected array, got string`.
7. Hook ids follow `[[hooks]]` order: message hook (declared first) is `core/hook/0`, the lua hook `core/hook/1`. The message hook text arrived rendered: `Reminder: run /status before you stop.` A hook has no `match` key when it has none (the message hook).
8. `native.files` was `[]`: the native hooks file `native/probe/hooks.toml` is not a native file item.
9. Sandbox keys: `airsstack` = `agentstacks,ext,json`; `airsstack.json` = `decode,encode,encode_pretty`; `agentstacks` = `copy,json,on,read,require,require_binary,skip,write`; `agentstacks.json` = `edit`.
10. `airsstack.json.encode('a "q": b\nc\\d é')` = `"a \"q\": b\nc\\d é"`: escapes `"`, newline, backslash; leaves `é` as raw UTF-8. airsl's `encode` emitted an empty Lua table (`native.files`) as `[]`. Object keys came out sorted.
11. Owners: root file → `core`; `runtime.lua` → tool `probe`; `translate.lua` (write without covers) → `probe`; `payload.json` (covers all from one pack) → `core`; `.claude/settings.json` (in `reads`, read returned absent) → owner none, `Create`, and **not in the lock** (shared, as docs/plan-and-apply.md line 87/122 says).
12. `plan.binaries()` was empty although `requires.airsl` is set; the airsl requirement surfaced only as the pin `AirslPin { locked: None, installed: 0.1.4 }` and `[airsl] version = "0.1.4"` in the lock. No skips, no notes.
13. apply wrote exactly the five planned files plus `agentstacks.lock`; the lock on disk is byte-identical to `write_lock(plan.lock())`. No stray temp files left.
