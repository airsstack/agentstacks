# Writing a Pack

This page is written for an AI coding agent to follow, step by step. A human can hand it over with
one line:

> Build an agentstacks pack for `<what the pack gives an agent>`. Follow
> https://raw.githubusercontent.com/airsstack/agentstacks/main/book/src/ch06-04-pack-authoring.md

Everything here is `api = 1`: the pack format is frozen for all of 1.x. What that promises is on
[the stability page](ch06-06-stability.md). A pack is the other half of an extension: a pack says
*what* the agent gets, in tool-neutral terms, and a tool extension says *how* a tool gets it.
Writing an extension is [Writing a Tool Extension](ch06-00-authoring.md).

## Read first

Fetch these before writing files. They are the contract; this page only tells you the order of work.

| Document | What it gives you |
|---|---|
| [ch06-05-pack-format.md](https://raw.githubusercontent.com/airsstack/agentstacks/main/book/src/ch06-05-pack-format.md) | the pack format: layout, every `pack.toml` field, tokens, frontmatter, hooks and events, options, `native/<tool>/`, every refusal |
| [ch06-02-hooks.md](https://raw.githubusercontent.com/airsstack/agentstacks/main/book/src/ch06-02-hooks.md) | how a hook runs, and the neutral event and result a Lua hook gets and returns |
| [packs/guideline-rust](https://github.com/airsstack/agentstacks/tree/main/packs/guideline-rust) | a worked example: one skill with references, a `render` file and an `exclude` |
| [packs/core](https://github.com/airsstack/agentstacks/tree/main/packs/core) | a larger example: agents, options, message and Lua hooks, `native/claude/hooks.toml` |

## What you build

```text
<pack>/
  pack.toml              the manifest: name, version, api = 1, requires, options, hooks
  skills/<name>/SKILL.md one directory per skill, frontmatter then a Markdown body
  agents/<name>.md       one file per agent, frontmatter then a Markdown body
  hooks/<hook>.lua       a hook script, run under airsl
  hooks/<hook>_test.lua  its test, run with `airsl test`; excluded from what is installed
  native/<tool>/         optional: files and hooks for one tool only
```

## Rules you must not break

- **Tool-neutral first.** Write skills, agents and hooks once, with tokens (`{{skill lint}}`)
  where a tool's own name or path would go. Put what only one tool understands under
  `native/<tool>/` or a skill's `native:` key, never in the neutral text.
- **Tokens, not paths.** Point at another skill with `{{skill name}}`, at an installed file with
  `{{root}}/...`, at a skill's directory with `{{skill_dir name}}`. Never write `.claude/` or any
  other tool's directory in neutral content.
- **Declare what you use.** A pack that installs a `.lua` file declares `requires.airsl`. A hook
  that runs a program lists it in `requires.binaries` and in `grants.exec`. A pack that points at
  another pack lists it in `requires.packs`.
- **Hooks get nothing by default.** A Lua hook runs confined. Grant each path and environment
  variable it needs, no more. `trusted` is not allowed in a pack.
- **Lua only, under airsl.** A hook is a Lua script. A tool that wants another language is the
  extension's problem, not the pack's.
- **Unsupported is visible.** A pack never needs to hide what a tool cannot do. The extension turns
  it into a skip with a reason.

## Steps

### 1. Lay out the pack

Pick a name (lowercase letters, digits and `-`, starting with a letter) and make the directories.
This page builds `team-lint`: one skill, one agent, one message hook, one Lua hook, one option.

```sh
mkdir -p packs/team-lint/{skills/lint,agents,hooks,native/claude}
```

### 2. Write `pack.toml`

```toml
exclude = ["**/*_test.lua"]

[pack]
name        = "team-lint"
version     = "1.0.0"
api         = 1
description = "Team lint rules"

[requires]
agentstacks = ">=1.0"
airsl       = ">=0.1.5"
```

`exclude` is written before the first `[table]` header so it stays a top-level key. `api = 1` is the
only value this agentstacks accepts. `airsl` is required because the pack will install a `.lua`
file. `requires.agentstacks` is covered in step 8.

### 3. Add a skill

`skills/lint/SKILL.md`:

```markdown
---
name: lint
description: Lint the files you changed and fix what it reports
invocation: user
argument_hint: "[path]"
---

Lint {{var arguments}} (or the files you changed). Allow at most {{option max_warnings}} warnings.
Hand anything you cannot fix to {{agent linter}}.
```

`name` is the directory name, written literally. `invocation: user` means only the user starts the
skill; leave it out to let the agent start it too. `{{var arguments}}` is what the user typed after
the skill's name, and it is valid in a `SKILL.md` only. Any other file in the skill's directory
(`references/*.md`, scripts) is installed with the skill, and `{{skill_dir lint}}` names the
directory.

### 4. Add an agent

`agents/linter.md`:

```markdown
---
name: linter
description: Reads lint output and proposes the smallest fix
model: balanced
effort: medium
tools: [read, grep, shell]
---

You are {{agent linter}}. Follow {{skill lint}}.
```

`model` is `fast`, `balanced` or `deep`; `effort` is `low`, `medium` or `high`; `tools` is a list
from `read`, `write`, `edit`, `grep`, `glob`, `shell` and `skill`. The words are neutral: each tool
extension maps them to the tool's own.

### 5. Add an option

An option lets a project change a value without editing the pack. Add to `pack.toml` after
`[requires]`:

```toml
[options.max_warnings]
type        = "integer"
default     = 0
description = "Warnings the lint skill tolerates"
```

`{{option max_warnings}}` in the skill becomes `0`, or the value the project sets under
`[packs.team-lint.options]` in `agentstacks.toml`.

### 6. Add hooks

A message hook shows the agent text at an event. A Lua hook runs a script. Append to `pack.toml`:

```toml
[[hooks]]
kind  = "message"
event = "session.start"
match = ["startup", "resume"]
text  = "Run {{skill lint}} before you commit."

[[hooks]]
kind  = "lua"
event = "tool.after"
match = ["edit", "write"]
run   = "hooks/remind.lua"
args  = ["{{option max_warnings}}"]
```

`hooks/remind.lua` returns a function. It gets the neutral event and returns `nil`, or a table with
`context`, `block`, or both ([Hooks](ch06-02-hooks.md), section 5):

```lua
return function(event)
  return { context = "Edited " .. tostring(event.path) .. ": run the lint skill." }
end
```

`hooks/remind_test.lua` tests it, beside it. A test file requires the module by its bare name and
returns a table of test functions; `exclude` keeps it out of the install:

```lua
local hook = require("remind")

return {
  names_the_edited_file = function()
    local result = hook({ event = "tool.after", path = "src/a.rs" })
    assert(result.context == "Edited src/a.rs: run the lint skill.", result.context)
  end,
}
```

The six events, which `match` values each takes, and the rest of the hook keys (`may_block`,
`grants`) are in [section 5 of the pack format](ch06-05-pack-format.md#5-hooks). A hook that needs
to read a file or run a program says so with `grants`; it gets nothing otherwise.

Run the tests where the hook lives: `airsl test packs/team-lint` should report `0 failed`, and
`airsl check packs/team-lint` should compile every file.

### 7. Add `native/<tool>/`

Put what one tool alone understands under `native/<tool>/`. A hook in the tool's own event words
goes in `native/claude/hooks.toml`:

```toml
[[hooks]]
kind  = "message"
event = "UserPromptSubmit"
text  = "Lint is part of done."
```

The core does not check a native `event` or `match`; the extension for that tool does. A native hook
the tool cannot express becomes a skip with a reason in the plan, so check the plan (step 9) for a
`skip` line. Files in `native/<tool>/` are installed for that tool only, and neutral content must
not point at them with `{{root}}`.

### 8. Say which agentstacks you need

```toml
[requires]
agentstacks = ">=1.0"
```

A pack written for `api = 1` works with every agentstacks 1.x, so `>=1.0` is a floor you can always
write. When a later 1.N release adds something you use (a new optional key, a new var), write
`>=1.N` instead: an older agentstacks then refuses the pack with `requires.agentstacks: ">=1.N"
does not match agentstacks <version> (install a newer agentstacks)` rather than failing on a key it
does not know. See [the stability page](ch06-06-stability.md).

### 9. Try it

In a scratch project that holds a copy of your pack, install it with an official tool:

```sh
agentstacks init --tool claude --pack ./packs/team-lint --yes
agentstacks plan      # 0 to write
agentstacks doctor    # healthy
```

`init` writes `agentstacks.toml` with `[packs.team-lint] path = "./packs/team-lint"`, checks the
pack and applies it. It was run on this pack with a copy of the official claude extension
(`--tool ./ext/claude`) and wrote six files, `.claude/skills/team-lint-lint/SKILL.md`,
`.claude/agents/team-lint-linter.md`, the hook script under `.claude/agentstacks/team-lint/hooks/`
and the runtime files and `.claude/settings.json` that run the hooks; `plan` then showed `0 to
write` and `doctor` said `healthy`. Open the files and read them: the skill's `{{var arguments}}`
became `$ARGUMENTS`, `{{agent linter}}` became `team-lint-linter`, and `{{option max_warnings}}`
became `0`.

Read every `skip` line of the plan. Each one is something the tool cannot express, with its reason;
decide whether it matters for the pack. Change a file of the pack and run `agentstacks plan` again;
it shows the update.

To run the pack through an extension and the built-in conformance pack together, use
`agentstacks ext test <extension-dir> --pack ./packs/team-lint`. It installs both in a temporary
project, never in yours, and ends in `passed` or `failed` ([The Command Line](ch04-00-cli.md)).

### 10. Publish it and add it to a project

A pack is a directory in a git repository. Commit it, tag a release and push the tag:

```sh
git tag v1.0.0
git push origin v1.0.0
```

`init` and `ext add` have no shorthand for a pack in your repository. `ext add` adds tool
extensions only, and `init --pack` takes an official name or a directory. To use the pack from git,
write a `[packs.<name>]` table into the project's `agentstacks.toml` by hand, with the repository,
a tag and the directory the pack is in:

```toml
api = 1

[tools.claude]
git    = "https://github.com/airsstack/agentstacks"
tag    = "packs-v1.0.0"
subdir = "extensions/claude"

[packs.team-lint]
git    = "https://github.com/acme/lint-packs"
tag    = "v1.0.0"
subdir = "packs/team-lint"
```

The key after `packs.` must be the name in `pack.toml`; a key that differs is refused with
`source contains pack "<name>"`. Use `rev` (a commit) in place of `tag`, never both. To set an
option, add a `[packs.team-lint.options]` table with the values
([section 6 of the pack format](ch06-05-pack-format.md#6-options)). Then run `agentstacks plan` and
`agentstacks apply`. The tag, the commit and the tree hash of the source are pinned in
`agentstacks.lock`. `agentstacks update` moves a third-party source to the newest tag with the same
prefix (`v1.0.0` to `v1.0.1`; a tag with another prefix such as `other-2.0.0` is not chosen). The
official repository's own tags are limited to `packs-v1.*`
(see [The Command Line](ch04-00-cli.md)).

This was run end to end on `team-lint` with a local git repository (`git = "file:///..."`): the
plan showed six files, and the `[packs.team-lint.options]` value `max_warnings = 5` appeared in the
installed skill.

### 11. Before you call it done

- [ ] `pack.toml` has `api = 1`, a valid `name` and `version`, and a `description`.
- [ ] `agentstacks plan` on the pack has no refusal; each `skip` has a reason you accept.
- [ ] `agentstacks apply --yes`, then `agentstacks plan` shows `0 to write`; `doctor` says `healthy`.
- [ ] `airsl test <pack>` reports `0 failed`, and `airsl check <pack>` compiles every file.
- [ ] Tests and anything else that is not for the agent are in `exclude`.
- [ ] No path or name of one tool in neutral content: tokens everywhere, tool-specific files under
      `native/<tool>/`.
- [ ] Every hook grant is the least it needs, and every program it runs is in `requires.binaries`.
- [ ] `requires.packs` lists every pack a token or a `run` points into, and `requires.airsl` is set.
- [ ] A `README.md` is in `exclude` unless the agent should read it.

## Common refusals

Wording is as the CLI prints it. A row that starts at `pack.toml` or at a file inside the pack is
shown without the pack's path, which the CLI prints in front of it (`./packs/team-lint/pack.toml:
...`). The wording is not covered by the stability promise, the location is.

| You see | Fix |
|---|---|
| `./packs/team-lint: not a pack (no pack.toml)` | the directory has no `pack.toml` at its root |
| `pack.toml: team-lint: api 0 is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)` | set `api = 1` |
| `pack.toml: requires.airsl: required (the pack contains .lua files)` | add `airsl = ">=0.1.5"` to `[requires]` |
| `skills/lint/SKILL.md: name "other" does not match "lint"` | `name` is the directory name |
| `agents/linter.md:9: pack "team-lint" has no skill "nope"` | fix the token, or add the skill |
| `agents/linter.md:9: var "arguments" is only valid in SKILL.md` | use `{{var arguments}}` in a `SKILL.md` only |
| `pack.toml: hooks[0].match: not allowed on session.end` | that event takes no `match` |
| `pack.toml: hooks[1].run: "hooks/my.remind.lua" segment "my.remind" may use only letters, digits, _ and -` | rename the file; a segment is letters, digits, `_` and `-` |
| `pack.toml: hooks[1].grants.exec[0]: "git" is not in requires.binaries` | add `binaries = ["git"]` to `[requires]` |
| `pack.toml: exclude[0]: "**/SKILL.md" excludes skills/lint/SKILL.md` | narrow the glob; a `SKILL.md` cannot be excluded |
| `agentstacks.toml: packs.team-lint: option "max_warnings" is required` | give the option a `default`, or set it in the project |

The full list, with the rule behind each, is in [The Pack Format](ch06-05-pack-format.md).
