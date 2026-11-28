# The Pack Format

The contract for a pack at `api = 1`: the layout of the directory, every field of `pack.toml`, the
`{{...}}` tokens, the frontmatter of skills and agents, hooks and their events, options, the
`native/<tool>/` directories, and `requires.agentstacks`. A pack is tool-neutral content; the tool
extension decides how each item lands in a tool's own files.

Every refusal quoted below was produced by the agentstacks 1.0 binary or is pinned by a test in
`crates/agentstacks/src`. Each is shown after the file's path, or with the path where the location is the point. What
`api = 1` promises, and what it does not (the exact wording of a refusal is not covered), is on
[the stability page](ch06-06-stability.md). To write a pack step by step, follow
[Writing a Pack](ch06-04-pack-authoring.md). Hook mechanics are in [Hooks](ch06-02-hooks.md), and
what an extension does with a pack is in [The Extension API](ch06-01-extensions.md).

## 1. Layout

```text
<pack>/
  pack.toml                 required: the manifest
  skills/<name>/SKILL.md    one directory per skill; any other files in it belong to the skill
  agents/<name>.md          one file per agent
  native/<tool>/            files and hooks for one tool only (section 8)
  <anything else>           root files: scripts, data, README.md, hooks/*.lua
```

| Rule | Refusal |
|---|---|
| the directory holds a `pack.toml` | `./packs/x: not a pack (no pack.toml)` |
| a skill directory holds a `SKILL.md` | `./packs/x/skills/lint: missing SKILL.md` |
| `skills/` holds only skill directories | `./packs/x/skills/loose.md: only skill directories belong in skills/` |
| `agents/` holds only `<name>.md` files | `./packs/x/agents/sub.txt: agents/ holds only <name>.md files` |
| `native/` holds only tool directories | `./packs/x/native/loose.md: only tool directories belong in native/` |
| `commands/` and `rules/` are reserved | `./packs/x/commands: reserved, not supported in api 1` |
| no symlink anywhere | `./packs/x/link.md: symlinks are not allowed in a pack` |
| a rendered file is UTF-8 | `./packs/x/data.json: not valid UTF-8, cannot render` |
| the pack installs something: a skill, an agent or a hook | `./packs/x: pack declares nothing to install` |

- **Names.** A skill, agent, pack or tool name is 1 to 64 bytes of `a-z`, `0-9` and `-`, starting
  with a letter: `"Team-Lint" is not a valid name`. A skill and an agent may share a name. An
  option name uses `_` where the others use `-` (section 6).
- **Dot entries** (a file or directory whose name starts with `.`) are ignored, never entered and
  never refused. An empty directory contributes nothing.
- **What is installed.** Every file except `pack.toml` and each `native/<tool>/hooks.toml`, less
  what `exclude` removes. A skill's files go to the tool's skill directory, an agent file becomes
  the tool's agent, and every root file goes under the pack's install root, a directory the
  extension names (see `{{root}}` in section 3).
- **What is rendered.** Every installed `.md` file, and each file `render` lists. Rendering replaces
  the tokens of section 3. Any other file is installed byte for byte, so a script or a JSON file
  never changes.

## 2. `pack.toml`

```toml
exclude = ["README.md", "**/*_test.lua"]
render  = ["data/settings.json"]

[pack]
name        = "team-lint"
version     = "1.0.0"
api         = 1
description = "Team lint rules"

[requires]
packs       = ["core"]
agentstacks = ">=1.0"
airsl       = ">=0.1.5"
binaries    = ["git"]

[options.max_warnings]
type        = "integer"
default     = 0
description = "Warnings the lint skill tolerates"

[[hooks]]
kind  = "message"
event = "session.start"
text  = "Run {{skill lint}} before you commit."
```

`render` and `exclude` are keys of the top-level table, so write them before the first `[table]`
header: a key written after a header belongs to that table. The top level holds only `pack`,
`requires`, `render`, `exclude`, `options` and `hooks`; any other key is `foo: unknown key`, and so
is an unknown key in `[pack]` (`pack.foo: unknown key`). A file is read to the end: every refusal
of it is reported at once, except that a TOML syntax error is the file's only refusal.

### `[pack]`

| Key | Required | Rule | Refusal |
|---|---|---|---|
| `name` | yes | a pack name | `pack.name: "Team-Lint" is not a valid name` |
| `version` | yes | `major.minor.patch` semver; tracks this pack's own releases — bump it if and only if the pack's shipped content changed, with a changelog entry (see [Versions and Release Streams](ch06-08-versioning.md)) | `pack.version: "1.2" is not semver` |
| `api` | yes | the integer `1` | `team-lint: api 2 not supported (this agentstacks speaks 1)` |
| `description` | yes | text that is not blank | `pack.description: empty` |

A missing key is `pack.name: required`, and a value of the wrong type is
`pack.api: expected integer, got float`. A pack written for agentstacks 0.x says `api = 0`; the
refusal is `team-lint: api 0 is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)`.
When the pack's name is itself invalid, the same two refusals are located at `pack.api` instead of
using the name.

### `render` and `exclude`

`render` lists files to render besides the Markdown. `exclude` lists globs of files to leave out.
Both hold paths relative to the pack, written with `/`.

| Rule | Refusal |
|---|---|
| a `render` path is relative | `render[0]: "/x" must be relative` |
| it stays inside the pack | `render[1]: "../x" must stay inside the pack` |
| it is not code: `.lua` and `.sh` are installed verbatim | `render[2]: "x.lua" is code; code is installed verbatim` |
| it names a file of the pack | `render[0]: "data.json" does not exist` |
| it is a file the pack installs (not `pack.toml`, not a `hooks.toml`) | `render[0]: "pack.toml" is never installed` |
| no `exclude` glob matches it | `render[0]: "README.md" is also excluded` |
| an `exclude` entry is a glob | `exclude[0]: "[" is not a valid glob` |
| an `exclude` entry is relative | `exclude[1]: "/tmp/**" must be relative` |
| `exclude` leaves `pack.toml`, every `SKILL.md`, every agent file and every own-pack hook script in place | `exclude[0]: "**/SKILL.md" excludes skills/lint/SKILL.md` |

A glob is matched against each file's path from the pack root. `*` and `?` stay inside one path
segment; only `**` crosses `/`. An excluded file is not installed and not read for tokens, so a
test file (`**/*_test.lua`) can sit beside the script it tests and never ship. A path with `.`, `//`
or a trailing `/` in it is refused with the rule it breaks (`"a//b" must not have an empty segment`).

### `[requires]`

| Key | Meaning | Refusal |
|---|---|---|
| `packs` | packs that must also be in the project | `requires.packs[0]: a pack cannot require itself`; `requires.packs[2]: "core" listed twice` |
| `agentstacks` | the agentstacks versions the pack works with (section 9) | `requires.agentstacks: "nope" is not a version range` |
| `airsl` | the airsl versions the pack's Lua needs | `requires.airsl: required (the pack contains .lua files)` when the pack installs any `.lua` file and sets none |
| `binaries` | programs the pack needs on `PATH` | `requires.binaries[1]: "git" listed twice` |

A range uses one of `=`, `>`, `>=`, `<`, `<=`, `~`, `^` or `*`, and may hold several comparators
(`>=0.1.5, <0.3`). `airsl` is needed because hooks and scripts run under airsl. An unknown key in
the table is `requires.foo: unknown key`.

What a project's set of packs must satisfy together is checked after every pack loads, and is
refused at `agentstacks.toml`:

| Rule | Refusal |
|---|---|
| each required pack is listed in the project | `agentstacks.toml: packs.team-lint: team-lint requires pack "core", which agentstacks.toml does not list (add it: --pack core)` |
| packs do not require each other in a loop | `agentstacks.toml: requires cycle: a → b → a` |
| the `airsl` ranges of all packs overlap | `agentstacks.toml: airsl: a needs >=0.2, b needs <0.2` |
| the installed airsl is inside them | `agentstacks.toml: airsl 0.1.5 installed, packs need >=9.0` |
| each `binaries` entry is on `PATH` | `agentstacks.toml: packs.team-lint: binary "git" not found on PATH` |
| the key under `[packs.<key>]` is the name inside the source | `agentstacks.toml: packs.zzz: source contains pack "a"` |

## 3. Tokens and vars

A token is `{{kind target}}` in the text of a pack. Rendering replaces it with a value for the tool
being set up. The kind is the first word; the target is the rest, separated by spaces or tabs.

| Token | Replaced with | Target |
|---|---|---|
| `{{skill name}}` | how the tool invokes the skill, such as `/team-lint-lint` for Claude Code | a skill, `[pack/]name` |
| `{{agent name}}` | the tool's name for the agent | an agent, `[pack/]name` |
| `{{skill_dir name}}` | the directory the skill is installed in | a skill, `[pack/]name` |
| `{{root}}`, `{{root pack}}` | the install root of this pack, or of the named pack | none, or a pack name |
| `{{var name}}` | a value the extension supplies | one of the vars below |
| `{{option name}}` | the value of one of this pack's options | an option of this pack |

The vars are `session_id`, `arguments`, `tmpdir`, `home`, `airsstack_home` and `project_dir`. What
each becomes is the extension's choice (`$ARGUMENTS` for Claude Code's `arguments`). A var an
extension does not supply is a visible skip, never a silent drop. `{{var arguments}}` is valid only
in a `SKILL.md`. An option renders as `true` or `false`, a decimal integer, or the string as written.

`\{{` is a literal `{{` that is not a token. A `}}` with no opener is plain text.

**Where tokens are read:** installed `.md` files (a `SKILL.md` and an agent file from the line after
the closing `---`), files listed in `render`, the `description` and `argument_hint` values of
frontmatter, and the fields of hooks: `text`, `args`, `run` and the paths of `grants`. Tokens are
not read in `name`, in the values under `native`, in a file that is not rendered, or in an excluded
file.

| Rule | Refusal |
|---|---|
| the kind is one of the six | `agents/linter.md:9: unknown token kind "skil"` |
| the token closes on its line | `unterminated token` |
| the token is not empty | `empty token` |
| a kind that needs a target has one | `token "skill" needs a target` |
| a kind takes one target | `token "root" takes one target` |
| a target is `[pack/]name` | `"a/b/c" is not a valid target (expected [pack/]name)` |
| `root` names a pack, not an item | `token "root" takes a pack name, not "core/x"` |
| the var exists | `unknown var "cwd" (known: session_id, arguments, tmpdir, home, airsstack_home, project_dir)` |
| `arguments` is used in a `SKILL.md` | `agents/linter.md:9: var "arguments" is only valid in SKILL.md` |
| an option token names this pack's option | `option tokens name this pack's options only` |
| the option is declared | `skills/lint/SKILL.md:8: pack "team-lint" has no option "nope"` |
| a skill or agent of this pack exists | `agents/linter.md:9: pack "team-lint" has no skill "nope"` |
| neutral content does not reach into `native/` | `skills/lint/SKILL.md:10: neutral content must not reference native/` |
| a token names another pack only if the pack requires it | `agents/linter.md:9: {{skill core/review}} targets pack "core", which team-lint does not require. Add it to requires.packs or remove the ref.` |

A token for another pack is also refused when that pack is required and ships no such item. The
`native/` row applies to a `{{root}}` token followed by `/native`: a pack's native files are for
one tool, so content that every tool installs may not point at them.

## 4. Frontmatter

A `SKILL.md` and an agent file start with a YAML block between two `---` lines, the first on line 1.
A file with no block is `skills/lint/SKILL.md: missing frontmatter`. Each key is checked: every
error of the file is reported together, and an unknown key is `unknown key "colour"`.

**Skill** (`skills/<name>/SKILL.md`):

| Key | Required | Value |
|---|---|---|
| `name` | yes | the directory name, written literally: `name "other" does not match "lint"`, `name must be a literal` |
| `description` | yes | text; tokens are allowed |
| `invocation` | no | `user` (only the user starts it) or `any` (the default): `invocation: "anyone" (expected user, any)` |
| `argument_hint` | no | text; tokens are allowed |
| `native` | no | a mapping of tool name to tool-specific values, kept opaque |

**Agent** (`agents/<name>.md`):

| Key | Required | Value |
|---|---|---|
| `name` | yes | the file name without `.md`, written literally |
| `description` | yes | text; tokens are allowed |
| `model` | no | `fast`, `balanced` or `deep`: `model: "sonnet" (expected fast, balanced, deep)` |
| `effort` | no | `low`, `medium` or `high`: `effort: "max" (expected low, medium, high)` |
| `tools` | no | a list from `read`, `write`, `edit`, `grep`, `glob`, `shell`, `skill`: `tools[1]: unknown "bash"`; a repeat is `"read" listed twice` |
| `native` | no | as for a skill |

The words are tool-neutral. The extension maps a `model` word and each `tools` entry to the tool's
own names, and skips with a reason what the tool cannot express. A value starting with `{` must be
quoted in YAML; the refusal for an unquoted token says so.

The `native` mapping is for values that only one tool understands. Its keys must be tool names
(`native.Claude: "Claude" is not a valid tool name`), and the extension for that tool reads its
entry. The core never looks inside.

## 5. Hooks

A hook is one `[[hooks]]` table in `pack.toml`: an event, an optional selector and an action.
[Hooks](ch06-02-hooks.md) says how the core runs one; this is what a pack may write.

| Key | Applies to | Rule |
|---|---|---|
| `kind` | all | `message` or `lua`: `hooks[0].kind: "sh" (expected message, lua)` |
| `event` | all | one of the six neutral events below: `hooks[1].event: "tool.pre" unknown` |
| `match` | events with selectors | a list of the values the event accepts; absent means every value |
| `agents` | `agent.stop` | a list of agents of this pack; absent means any agent |
| `text` | `message` | required, not empty; tokens are allowed |
| `run` | `lua` | required, a `.lua` file; tokens are allowed |
| `args` | `lua` | a list of strings; tokens are allowed |
| `may_block` | `lua` | `true` or `false` (the default); a hook that is not `may_block` cannot block |
| `grants` | `lua` | what the script may touch; nothing is granted by default |

Every Lua hook runs fail-open, so there is no `fail_open` key (`hooks[2].fail_open: unknown key`).

### Events

| Event | When | `match` values | `agents` |
|---|---|---|---|
| `session.start` | a session starts | `startup`, `resume`, `clear`, `compact` | no |
| `session.end` | a session ends | none | no |
| `prompt.submit` | the user submits a prompt | none | no |
| `tool.before` | before a tool call | `read`, `edit`, `write` | no |
| `tool.after` | after a tool call | `read`, `edit`, `write` | no |
| `agent.stop` | an agent stops | none | yes |

The vocabulary is frozen for all of 1.x: a new event or `match` word needs `api = 2` (see
[the stability page](ch06-06-stability.md)). The extension maps each event to the tool's own, and
skips a hook with a reason when the tool has no such event.

| Rule | Refusal |
|---|---|
| `match` only on `session.start`, `tool.before`, `tool.after` | `hooks[0].match: not allowed on session.end` |
| each `match` value is one the event accepts | `hooks[1].match[0]: "bash" not valid for tool.after` |
| a `match` or `agents` list is not empty | `hooks[1].match: empty list never fires` |
| no value is listed twice | `hooks[1].match[1]: "edit" listed twice` |
| `agents` only on `agent.stop` | `hooks[1].agents: only allowed on agent.stop` |
| each `agents` entry is an agent of the pack | `hooks[1].agents[0]: pack "team-lint" has no agent "nobody"` |
| a message hook has no `run`, `args`, `grants` | `hooks[0].run: not allowed on a message hook` |
| a message hook does not block | `hooks[0].may_block: message hooks cannot block` |
| a lua hook has no `text` | `hooks[1].text: not allowed on a lua hook` |

### `run`

`run` is a Lua file, written in one of three forms:

| Form | Names |
|---|---|
| `hooks/x.lua` | a root file of this pack |
| `{{root}}/hooks/x.lua` | the same file |
| `{{root core}}/hooks/x.lua` | a root file of pack `core`, which this pack must require |

The path must name an installed root file: not `pack.toml`, and nothing under `skills/`, `agents/`
or `native/`. Each segment, with `.lua` taken off the last, uses only letters, digits, `_` and `-`,
because the runtime requires the script as a dotted module name.

| Rule | Refusal |
|---|---|
| it is a `.lua` file | `hooks[1].run: "remind.sh" is not a .lua file` |
| it is written in a form above | `hooks[1].run: must be pack-relative, {{root}}/… or {{root <pack>}}/…` |
| each segment is a module name | `hooks[1].run: "hooks/my.remind.lua" segment "my.remind" may use only letters, digits, _ and -` |
| the file exists in this pack | `hooks[1].run: "hooks/remind.lua" does not exist` |
| in another pack, it is an installed root file there | `"hooks/x.lua" does not exist in core` |

### `grants`

```toml
[hooks.grants]
read  = ["{{root}}"]
write = ["{{var tmpdir}}"]
env   = ["TMPDIR"]
exec  = ["git"]
```

| Key | Value | Refusal |
|---|---|---|
| `policy` | `confined`, the default and the only one allowed | `"trusted" is not allowed for pack hooks`; `"pure" is not supported in api 1 (airsl has no require under pure, airsstack/airsl#12)`; `"x" (expected confined)` |
| `read`, `write` | paths: absolute, or starting with `{{root}}`, `{{skill_dir ...}}` or `{{var ...}}` | `hooks[1].grants.read[0]: "data" must be absolute or start with a token` |
| `env` | environment variable names (`^[A-Z_][A-Z0-9_]*$`) | `"home" is not an environment variable name` |
| `exec` | `airsl`, or a binary in `requires.binaries` | `hooks[1].grants.exec[0]: "git" is not in requires.binaries` |

A repeat in `env` or `exec` is `"x" listed twice`.

## 6. Options

An option is a setting a project can change. A pack declares it under `[options.<name>]`; a skill
or a hook reads it with `{{option name}}`.

```toml
[options.max_warnings]
type        = "integer"
default     = 0
description = "Warnings the lint skill tolerates"
```

| Key | Required | Rule | Refusal |
|---|---|---|---|
| table name | | `snake_case`: letters, digits and `_`, starting with a letter | `options.archive-keep: "archive-keep" is not a valid option name` |
| `type` | yes | `boolean`, `integer` (64-bit) or `string` | `options.x.type: required`; `options.x.type: "number" (expected boolean, integer, string)` |
| `default` | no | a value of the declared type | `options.max_warnings.default: expects integer, got string` |
| `description` | yes | text | `options.x.description: required` |

A project sets a value in `agentstacks.toml`:

```toml
[packs.team-lint]
git    = "https://github.com/acme/lint-packs"
tag    = "v1.0.0"
subdir = "packs/team-lint"

[packs.team-lint.options]
max_warnings = 5
```

The value must have the declared type, an option with no default must be set, and a key the pack
does not declare is refused:

| Rule | Refusal |
|---|---|
| a required option is set | `agentstacks.toml: packs.team-lint: option "max_warnings" is required` |
| the value has the declared type | `agentstacks.toml: packs.team-lint.options.max_warnings: option max_warnings expects integer, got string` |
| the key is declared | `agentstacks.toml: packs.team-lint.options.extra: no option "extra"` |

## 7. `native/<tool>/`

Content for one tool only. `<tool>` is a tool name (`claude`, `musecode`, `opencode`); the extension of that
tool reads the directory. A directory no installed tool claims is shown as a note in the plan,
`note     native/other: not for claude`.

- Every file in it is installed, relative to the tool's own directory, and rendered when it is
  Markdown. `hooks.toml` is not installed as a file.
- `native/<tool>/hooks.toml` declares hooks in the tool's own event vocabulary:

```toml
[[hooks]]
kind  = "message"
event = "UserPromptSubmit"
text  = "Lint is part of done."
```

A native hook has the keys of a neutral one, with two differences: `event` and `match` are opaque
text that the core never checks (the extension for the tool does), and `agents` is not accepted
(`native/claude/hooks.toml: hooks[0].agents: unknown key`). Kinds, `run`, `args`, `may_block` and `grants` follow section 5,
and `exec` is checked against the pack's own `requires.binaries`. The file's only top-level key is
`hooks`; an empty file declares none. The file itself is parsed, never installed.

Neutral content may not point into `native/` with a `{{root}}` token (section 3). Whether the tool
knows an event is the extension's to say: a native hook it cannot express becomes a skip with a
reason, as `team-lint/native-hook/0: claude treats hook output at agent stop as feedback that keeps the agent running`
shows.

## 8. Official packs as examples

| Pack | Shows |
|---|---|
| [`guideline-rust`](https://github.com/airsstack/agentstacks/tree/main/packs/guideline-rust) | a skill with references, a `render` file and an `exclude`; no hooks, no Lua |
| [`core`](https://github.com/airsstack/agentstacks/tree/main/packs/core) | agents, options, message and Lua hooks with grants, `native/claude/hooks.toml` |
| [conformance](https://github.com/airsstack/agentstacks/tree/main/crates/agentstacks/assets/conformance) | one of every item kind, token kind and hook event, with `requires.agentstacks` |

## 9. `requires.agentstacks`

```toml
[requires]
agentstacks = ">=1.0"
```

Declare it when the pack uses something a 1.N release added. A pack that omits it works with every
agentstacks that speaks `api = 1`. The range is checked first, before anything else in the pack is
read: an agentstacks the range excludes refuses the pack with one line and goes no further, because
a pack written for a later release may use things this one does not know.

| Rule | Refusal |
|---|---|
| the value is a range | `requires.agentstacks: "nope" is not a version range` |
| the running agentstacks is in it | `requires.agentstacks: ">=9.0" does not match agentstacks 1.0.0 (install a newer agentstacks)` |

The second line is the only refusal reported for a pack whose range excludes the running version.
Once the range is fixed or agentstacks is updated, the rest of the pack is checked.
