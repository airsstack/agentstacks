# Settings probe: JSON decode/edit facts for the claude extension's settings.json merge (real agentstacks)

- Library: `agentstacks` v0.0.0 at worktree commit 8107626 (path dependency `crates/agentstacks`), `airsl v0.1.4` (`cargo tree -i airsl`), toolchain from the worktree's `rust-toolchain.toml`: `rustc 1.94.1 (e408947bf 2026-03-25)`. airsl binary on PATH: `airsl 0.1.4`.
- Probe crate: `/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/settings-probe` (copy of host-probe, renamed, rust-toolchain.toml copied). main.rs saved as `probes/settings-main.rs`.
- Fixture project: `/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/settings-project` = host-project fixture (tool `probe`, pack `core`) with `packs/core/agents/coder.md` native block changed to `probe: {empty: {}, list: [], s: "x"}`. Adapter saved as `probes/settings-adapter.lua`.
- S7 extension dir: `/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/s7-ext` (extension.toml exactly as below, adapter.lua = `agentstacks.on("plan", function(q) end)`).
- Real adapters: `StdWorkspace::open`, `GitCli`, `SystemBinary::from_env`, `AirslAdapter::new`; flow `agentstacks::app::plan::plan`; `Extension::load(&StdFileTree::new(dir))`; `ProjectFile::parse(Path::new("agentstacks.toml"), text)`.

## Reading note for `has_metatable`

The adapter computes `has_metatable = okm and (mt ~= nil) or ("getmetatable err: " .. tostring(mt))`. Because of the Lua and/or idiom, the value `"getmetatable err: nil"` means **pcall succeeded and getmetatable returned nil (no metatable)**, not an error. `true` means a metatable is present. `metatable_keys` never appears, so for every value that has one, getmetatable returned a non-table (protected) value.

## Fixture: `packs/core/agents/coder.md`

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
    empty: {}
    list: []
    s: "x"
---

You write code.
```

## S7 extension.toml

```
[extension]
name        = "claude"
version     = "0.1.0"
api         = 0
description = "Renders packs into Claude Code project config"
entry       = "adapter.lua"
reads       = [".claude/settings.json"]
```

## Commands

```
cd /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/settings-probe && cargo build -q
./target/debug/settings-probe /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/settings-project /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/s7-ext
```

Attempt 1 refused with `probe/adapter.lua: lua hooks need .claude/agentstacks/translate.lua (write it)` (the adapter did not write translate.lua; the fixture has lua hooks). The translate.lua write from the host probe was added; attempt 2 is below. S7 output was identical in both attempts.

## Raw output (attempt 2), byte for byte

```
===== S1-S6 (plan run)
PLAN: Ok
--- out.json (pretty, keys sorted)
{
  "S1": {
    "01_null": {
      "a": {
        "has_metatable": "getmetatable err: nil",
        "tostring": "userdata: 0x0",
        "type": "userdata"
      },
      "input": "{\"a\":null}",
      "ok": true,
      "top": {
        "has_metatable": "getmetatable err: nil",
        "len": 0,
        "next_is_nil": false,
        "tostring": "table: 0xad3075600",
        "type": "table"
      }
    },
    "02_empty_arr": {
      "a": {
        "has_metatable": true,
        "len": 0,
        "next_is_nil": true,
        "tostring": "table: 0xad30758c0",
        "type": "table"
      },
      "input": "{\"a\":[]}",
      "ok": true,
      "top": {
        "has_metatable": "getmetatable err: nil",
        "len": 0,
        "next_is_nil": false,
        "tostring": "table: 0xad3075880",
        "type": "table"
      }
    },
    "03_empty_obj": {
      "a": {
        "has_metatable": "getmetatable err: nil",
        "len": 0,
        "next_is_nil": true,
        "tostring": "table: 0xad3075b40",
        "type": "table"
      },
      "input": "{\"a\":{}}",
      "ok": true,
      "top": {
        "has_metatable": "getmetatable err: nil",
        "len": 0,
        "next_is_nil": false,
        "tostring": "table: 0xad3075b00",
        "type": "table"
      }
    },
    "04_top_arr": {
      "a": {
        "has_metatable": "getmetatable err: nil",
        "tostring": "nil",
        "type": "nil"
      },
      "input": "[]",
      "ok": true,
      "top": {
        "has_metatable": true,
        "len": 0,
        "next_is_nil": true,
        "tostring": "table: 0xad3075d40",
        "type": "table"
      }
    },
    "05_json_empty_string": {
      "input": "\"\"",
      "ok": true,
      "top": {
        "has_metatable": true,
        "tostring": "",
        "type": "string"
      }
    },
    "06_lua_empty": {
      "err": "lua error in <json>: EOF while parsing a value at line 1 column 0\nstack traceback:\n\t[C]: in ?\n\t[C]: in function 'pcall'\n\tadapter.lua:72: in function <adapter.lua:44>\n",
      "input": "",
      "ok": false
    },
    "07_whitespace": {
      "err": "lua error in <json>: EOF while parsing a value at line 2 column 0\nstack traceback:\n\t[C]: in ?\n\t[C]: in function 'pcall'\n\tadapter.lua:72: in function <adapter.lua:44>\n",
      "input": "  \n",
      "ok": false
    }
  },
  "S2": {
    "01_null/a": {
      "ok": false,
      "result": "probe/adapter.lua:40: json.edit: value: userdata cannot be JSON\nstack traceback:\n\t[C]: in ?\n\t[C]: in function 'pcall'\n\tadapter.lua:40: in upvalue 'try_edit'\n\tadapter.lua:83: in function <adapter.lua:44>\n"
    },
    "01_null/top": {
      "ok": false,
      "result": "probe/adapter.lua:40: json.edit: value: userdata cannot be JSON\nstack traceback:\n\t[C]: in ?\n\t[C]: in function 'pcall'\n\tadapter.lua:40: in upvalue 'try_edit'\n\tadapter.lua:81: in function <adapter.lua:44>\n"
    },
    "02_empty_arr/a": {
      "ok": true,
      "result": "{\n  \"x\": {}\n}"
    },
    "02_empty_arr/top": {
      "ok": true,
      "result": "{\n  \"x\": {\n    \"a\": {}\n  }\n}"
    },
    "03_empty_obj/a": {
      "ok": true,
      "result": "{\n  \"x\": {}\n}"
    },
    "03_empty_obj/top": {
      "ok": true,
      "result": "{\n  \"x\": {\n    \"a\": {}\n  }\n}"
    },
    "04_top_arr/top": {
      "ok": true,
      "result": "{\n  \"x\": {}\n}"
    },
    "05_json_empty_string/top": {
      "ok": true,
      "result": "{\n  \"x\": \"\"\n}"
    }
  },
  "S3": {
    "start_brace_nl": {
      "ok": true,
      "result": "{\n  \"hooks\": {\n    \"SessionEnd\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"printf '%s\\\\n' 'x' # agentstacks\",\n            \"type\": \"command\"\n          }\n        ]\n      }\n    ]\n  }\n}\n"
    },
    "start_empty": {
      "ok": true,
      "result": "{\n  \"hooks\": {\n    \"SessionEnd\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"printf '%s\\\\n' 'x' # agentstacks\",\n            \"type\": \"command\"\n          }\n        ]\n      }\n    ]\n  }\n}"
    }
  },
  "S4": {
    "decode_ok": true,
    "old": "{\n  \"permissions\": {\n    \"allow\": [\"Bash(ls:*)\"]\n  },\n  \"hooks\": {\n    \"SessionEnd\": [\n      { \"hooks\": [ { \"type\": \"command\", \"command\": \"echo user-hook\", \"timeout\": 5 } ] },\n      { \"hooks\": [ { \"type\": \"command\", \"command\": \"echo old # agentstacks\" } ] }\n    ]\n  }\n}\n",
    "step1": {
      "ok": true,
      "result": "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(ls:*)\"\n    ]\n  },\n  \"hooks\": {\n    \"SessionEnd\": [\n      {\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"echo user-hook\",\n            \"timeout\": 5\n          }\n        ]\n      },\n      {\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"echo old # agentstacks\"\n          }\n        ]\n      }\n    ],\n    \"PreToolUse\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"cd \\\"$CLAUDE_PROJECT_DIR\\\" && \\\"airsl\\\" \\\"run\\\" # agentstacks\",\n            \"type\": \"command\"\n          }\n        ],\n        \"matcher\": \"Read|Edit|Write\"\n      }\n    ]\n  }\n}\n"
    },
    "step2": {
      "ok": true,
      "result": "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(ls:*)\"\n    ]\n  },\n  \"hooks\": {\n    \"SessionEnd\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"echo user-hook\",\n            \"timeout\": 5,\n            \"type\": \"command\"\n          }\n        ]\n      },\n      {\n        \"hooks\": [\n          {\n            \"command\": \"printf '%s\\\\n' 'new' # agentstacks\",\n            \"type\": \"command\"\n          }\n        ]\n      }\n    ],\n    \"PreToolUse\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"cd \\\"$CLAUDE_PROJECT_DIR\\\" && \\\"airsl\\\" \\\"run\\\" # agentstacks\",\n            \"type\": \"command\"\n          }\n        ],\n        \"matcher\": \"Read|Edit|Write\"\n      }\n    ]\n  }\n}\n"
    }
  },
  "S5": {
    "hooks_5": {
      "ok": false,
      "result": "probe/adapter.lua:40: json.edit: \"hooks\" is not an object\nstack traceback:\n\t[C]: in ?\n\t[C]: in function 'pcall'\n\tadapter.lua:40: in upvalue 'try_edit'\n\tadapter.lua:148: in function <adapter.lua:44>\n"
    },
    "hooks_arr": {
      "ok": false,
      "result": "probe/adapter.lua:40: json.edit: \"hooks\" is not an object\nstack traceback:\n\t[C]: in ?\n\t[C]: in function 'pcall'\n\tadapter.lua:40: in upvalue 'try_edit'\n\tadapter.lua:149: in function <adapter.lua:44>\n"
    }
  },
  "S6": {
    "empty": {
      "describe": {
        "has_metatable": "getmetatable err: nil",
        "len": 0,
        "next_is_nil": true,
        "tostring": "table: 0xad3074ec0",
        "type": "table"
      },
      "encode": {
        "ok": true,
        "result": "{}"
      }
    },
    "list": {
      "describe": {
        "has_metatable": true,
        "len": 0,
        "next_is_nil": true,
        "tostring": "table: 0xad3074f00",
        "type": "table"
      },
      "encode": {
        "ok": true,
        "result": "[]"
      }
    },
    "native_encode": {
      "ok": true,
      "result": "{\"empty\":{},\"list\":[],\"s\":\"x\"}"
    },
    "native_keys": "empty,list,s",
    "s": {
      "describe": {
        "has_metatable": true,
        "tostring": "x",
        "type": "string"
      },
      "encode": {
        "ok": true,
        "result": "\"x\""
      }
    }
  }
}
--- s3a.json (205 bytes) debug-escaped:
"{\n  \"hooks\": {\n    \"SessionEnd\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"printf '%s\\\\n' 'x' # agentstacks\",\n            \"type\": \"command\"\n          }\n        ]\n      }\n    ]\n  }\n}\n"
--- s3a.json raw:
{
  "hooks": {
    "SessionEnd": [
      {
        "hooks": [
          {
            "command": "printf '%s\\n' 'x' # agentstacks",
            "type": "command"
          }
        ]
      }
    ]
  }
}
<EOF>
--- s3b.json (204 bytes) debug-escaped:
"{\n  \"hooks\": {\n    \"SessionEnd\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"printf '%s\\\\n' 'x' # agentstacks\",\n            \"type\": \"command\"\n          }\n        ]\n      }\n    ]\n  }\n}"
--- s3b.json raw:
{
  "hooks": {
    "SessionEnd": [
      {
        "hooks": [
          {
            "command": "printf '%s\\n' 'x' # agentstacks",
            "type": "command"
          }
        ]
      }
    ]
  }
}<EOF>
--- s4.json (690 bytes) debug-escaped:
"{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(ls:*)\"\n    ]\n  },\n  \"hooks\": {\n    \"SessionEnd\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"echo user-hook\",\n            \"timeout\": 5,\n            \"type\": \"command\"\n          }\n        ]\n      },\n      {\n        \"hooks\": [\n          {\n            \"command\": \"printf '%s\\\\n' 'new' # agentstacks\",\n            \"type\": \"command\"\n          }\n        ]\n      }\n    ],\n    \"PreToolUse\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"cd \\\"$CLAUDE_PROJECT_DIR\\\" && \\\"airsl\\\" \\\"run\\\" # agentstacks\",\n            \"type\": \"command\"\n          }\n        ],\n        \"matcher\": \"Read|Edit|Write\"\n      }\n    ]\n  }\n}\n"
--- s4.json raw:
{
  "permissions": {
    "allow": [
      "Bash(ls:*)"
    ]
  },
  "hooks": {
    "SessionEnd": [
      {
        "hooks": [
          {
            "command": "echo user-hook",
            "timeout": 5,
            "type": "command"
          }
        ]
      },
      {
        "hooks": [
          {
            "command": "printf '%s\\n' 'new' # agentstacks",
            "type": "command"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "Read|Edit|Write"
      }
    ]
  }
}
<EOF>
===== S7a Extension::load(/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/s7-ext)
Ok: manifest=ExtensionManifest { name: ToolName("claude"), version: PackVersion(Version { major: 0, minor: 1, patch: 0 }), description: "Renders packs into Claude Code project config", entry: EntryPath("adapter.lua"), reads: [SubPath(".claude/settings.json")] } reads=[SubPath(".claude/settings.json")]
===== S7b ProjectFile::parse
input: "api = 0\n\n[tools.claude]\npath = \"ext\"\n\n[packs.team]\npath = \"packs/team\"\n"
Ok: ProjectFile { tools: {ToolName("claude"): Path(LocalPath("ext"))}, packs: {PackName("team"): PackEntry { source: Path(LocalPath("packs/team")), options: {} }} }
```

## Findings

- S1: `{"a":null}` → v.a is `userdata` (tostring `userdata: 0x0`), no metatable. `{"a":[]}` → table, `next==nil`, metatable present. `{"a":{}}` → table, `next==nil`, no metatable. `[]` → table, empty, metatable present. `'""'` → Lua string `""`. Empty text and `"  \n"` → error `lua error in <json>: EOF while parsing a value at line 1 column 0` / `line 2 column 0` (+ traceback).
- S2: null → `json.edit: value: userdata cannot be JSON`. A decoded empty array (`[]`, metatable-marked) goes through `json.edit` as `{}` (an object): the array marker is lost. Empty object → `{}`. Errors are prefixed `probe/adapter.lua:40: ` (tool/entry:line) and carry a stack traceback.
- S3: start `"{}\n"` → 205 bytes ending in `}\n`; start `""` → 204 bytes, no trailing newline. Two-space indent; keys of the inserted value are sorted (`command` before `type`).
- S4: whole document re-pretty-printed (inline `["Bash(ls:*)"]` expanded); existing key order preserved (`permissions` then `hooks`); new key `PreToolUse` appended after `SessionEnd`; keys inside each replaced value sorted (`matcher` after `hooks`; user handler re-emitted as `command, timeout, type`; the untouched SessionEnd after step 1 kept `type, command, timeout`); integer `5` stays `5`; trailing newline of the old text kept.
- S5: `{"hooks":5}` and `{"hooks":[]}` both → `json.edit: "hooks" is not an object`.
- S6: native `empty: {}` → table, no metatable, encode `{}`; `list: []` → table with metatable, encode `[]`; whole native encodes `{"empty":{},"list":[],"s":"x"}`.
- S7a: `Extension::load` → Ok (name claude, version 0.1.0, entry adapter.lua, reads [.claude/settings.json]). S7b: `ProjectFile::parse` → Ok (tools claude=path ext, packs team=path packs/team).
