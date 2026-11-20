---
status: superseded
created: 2026-11-20
built-on: e3b5c8c86089
---

# Spec: Pack and project formats

This spec defines the formats every later `foundation/01` chain reads or writes: the **pack** (a
directory with `pack.toml`, the content the agent gets), the **extension manifest**
(`extension.toml`, how a tool extension identifies itself), **`agentstacks.toml`** (what the human
asked for) and **`agentstacks.lock`** (what was pinned and written). It defines the tool-neutral
**token** syntax that replaces Claude-only constructs in pack content, and the Rust types in
`crates/agentstacks` that parse and validate all of it. A malformed input is refused with its
location. Every format is `api = 0`: unstable, free to change across `0.x`.

Proof notation: `Proof:` followed by the command run in this design session and its real output,
or a `file:line` opened in this session. `P/` abbreviates `../claudestacks/plugins/` (absolute:
`/Users/hiraq/Projects/airsstack/claudestacks/plugins/`).

## 1. Design premises (what the six plugins contain)

| Fact | Proof |
|---|---|
| Plugin-level content: `skills/` (all six), `agents/` (core, sdlc, journal), `hooks/` (core, sdlc, journal), `scripts/` (core, sdlc, journal), `references/` (sdlc, journal), `enforcement.json` (both guidelines), `.claude-plugin/` (all six), `README.md` (6), `LICENSE` (5). No `commands/`, no `rules/`. | `find "$p" -type f ! -name '*_test.lua'` per plugin, grouped by first path segment → e.g. `claudestacks/: 3 agents/ 11 hooks/ 15 scripts/ 13 skills/`, `claudestacks-cmux/: 23 skills/`; `ls P/*/README.md \| wc -l` → `6`; `ls P/*/LICENSE \| wc -l` → `5` |
| Skill directories carry their own `references/` and `scripts/`. | `find . -path '*/skills/*' -type f` → `claudestacks-cmux/cmux-browser :: scripts/cmux-snap.lua`, `claudestacks-guideline-rust/rust-guidelines :: references/strong-types.md` |
| 60 namespaced cross-references: 42 in `.md`, 11 in `.lua`, 6 in `.json`, 1 in `.sh`. | `grep -rhoE --include="*.$e" 'claudestacks(-[a-z]+(-[a-z]+)?)?:[a-z][a-z0-9-]+' . \| wc -l` → `md 42`, `json 6`, `lua 11`, `sh 1` |
| Cross-pack refs: sdlc→core 10, journal→core 1, sdlc→guideline-rust 2, core→guideline-rust 1, core→journal 1. The guideline-rust refs are examples, not dependencies. | per-plugin grep grouped by target → `claudestacks-sdlc -> claudestacks (10)` …; `P/claudestacks-sdlc/skills/design/SKILL.md:128` "invoke its skill (e.g. `claudestacks-guideline-rust:rust-guidelines`)"; `P/claudestacks/skills/orchestrate/SKILL.md:61` |
| `${CLAUDE_PLUGIN_ROOT}` appears in 38 `.md` and 3 `.json` non-test files (plus 1 test file, `P/claudestacks/scripts/discuss_skill_test.lua`); no non-test `.sh` or `.lua` file uses it. | `grep -rl 'CLAUDE_PLUGIN_ROOT' . \| grep -v _test \| sed 's/.*\.//' \| sort \| uniq -c` → `3 json`, `38 md` |
| 19 of those references point into a skill's own directory (`…/skills/<name>/…`); one hook passes such a path as an argument. | `grep -rnE 'CLAUDE_PLUGIN_ROOT\}?/skills/' P/ \| grep -v _test \| wc -l` → `19`; `P/claudestacks/hooks/style.sh:32` `"$PLUGIN/skills/discuss/references/protocol.md"` |
| Claude runtime substitutions in skill text: `${CLAUDE_SESSION_ID}` 6 times, `$ARGUMENTS` 5 times, all in `SKILL.md` files. | `grep -rho 'CLAUDE_SESSION_ID' --include=SKILL.md P/ \| wc -l` → `6`; `grep -rn '\$ARGUMENTS' --include='*.md' P/` → `enforce-doctor/SKILL.md:6,15,18`, `discuss/SKILL.md:10,41` |
| Core declares two user options, `style_reinject` (boolean, default true) and `archive_keep` (number, default 20), read as `${user_config.archive_keep}` in skill text and as env `CLAUDE_PLUGIN_OPTION_STYLE_REINJECT` in a hook. | `P/claudestacks/.claude-plugin/plugin.json` `userConfig`; `P/claudestacks/skills/discuss/SKILL.md:43`; `P/claudestacks/hooks/style.sh:30` |
| The enforce hook discovers guideline data by walking the installed-plugins registry, filtered to the claudestacks marketplace. | `P/claudestacks/hooks/enforce.lua:3`; `P/claudestacks/hooks/lib/enforce.lua:24` `M.MARKETPLACE_SUFFIX = "@claudestacks"`, `:266-291` `select_record` |
| Core's `SubagentStop` matcher hard-codes sdlc and journal agent names. | `P/claudestacks/hooks/hooks.json:92` |
| Hook events in use: `SessionStart` (matchers `startup\|resume\|clear`, `compact`, none), `SessionEnd`, `UserPromptSubmit`, `PreToolUse` (`Read\|Edit\|Write`, `^SubagentHandback$`), `PostToolUse` (`Write`), `SubagentStop` (agent regex, none). | `cat` of the three `hooks.json` files |
| Every hook wrapper runs `airsl run --fail-open --policy confined` with explicit grants (env `HOME TMPDIR AIRSSTACK_HOME AIRSSTACK_ENFORCE_REGISTRY CLAUDE_PLUGIN_OPTION_STYLE_REINJECT`; read `/`, plugin root, `$TMPDIR`, `$AIRSSTACK_HOME` root, `.`; write `$TMPDIR`, `$AIRSSTACK_HOME` root). | bodies of `P/claudestacks/hooks/{comm-check,enforce,handoff-check,rearm,style}.sh`, `P/claudestacks-sdlc/scripts/attention.sh`, `P/claudestacks-journal/scripts/session-start.sh` |
| Two hooks block on purpose (text on stderr + `exit 2`; JSON on stdout + `exit 0`). | `P/claudestacks/hooks/comm-check.sh`, `handoff-check.sh`: `case "$VIOLATIONS" in '{'*) … exit 0 ;; esac … >&2; exit 2` |
| One hook echoes static text only (SessionEnd). `preflight.sh` echoes a reminder **and** looks for airsl, printing a STATUS/FIX warning when it is missing. | `hooks.json:36`; `P/claudestacks/hooks/preflight.sh:18` (reminder), `:24-25` `AIRSL_BIN` lookup, `:40` `STATUS: the airsl runtime was not found …`, `:43` `FIX: install airsl …` |
| Binaries scripts execute: `git` 26, `cmux` 18, `airsl` 7, `tar` 2 grants. | `grep -rhoE -- '--allow-exec [a-z]+' P/ \| sort \| uniq -c` |
| Agents' frontmatter: `name description model tools` (10/10), `effort` (6/10); `model` ∈ {haiku, sonnet, opus}, `effort` ∈ {low, high}, `tools` ⊆ {Read, Write, Edit, Bash, Glob, Grep, Skill}. | per-agent `awk` over frontmatter → e.g. `coder.md tools: [Read, Edit, Write, Grep, Glob, Bash, Skill] model: sonnet effort: high` |
| Skills' frontmatter: `name`, `description` (32/32), `disable-model-invocation: true` (4), `argument-hint` (1). | frontmatter key count → `32 description:`, `32 name:`, `4 disable-model-invocation:`, `1 argument-hint:` |
| `journal-capture`, `journal-note`, `journal-recall` are each both a skill and an agent. No two plugins ship the same skill name. | `comm -12 <(ls */skills) <(ls */agents \| sed 's/\.md$//')` → those three; reviewer probe `ls */skills \| sort \| uniq -d` → empty |
| `{{` does not occur in any plugin `.md` or `.json`; `[[` does (28 hits). | candidate-syntax grep → `\{\{ files=0 hits=0`, `\[\[ files=7 hits=28` |
| All 48 plugin, skill and agent names match `^[a-z][a-z0-9-]{0,63}$`. | `ls P/ P/*/skills P/*/agents \| … \| grep -vE '^[a-z][a-z0-9-]{0,63}$'` → no output, exit 1; total `48` |
| A YAML value that starts with `{{` is a YAML syntax error unless quoted; a token later in the value parses. | `ruby -ryaml`: `description: {{skill status}} runs it` → `Psych::SyntaxError … did not find expected key while parsing a block mapping`; `description: "{{skill status}} runs it"` → `{"description"=>"{{skill status}} runs it"}`; `description: run {{skill status}} now` → parses |
| A `.DS_Store` file sits in the plugins directory. | `find . -name '.*'` → `./.DS_Store` |

## 2. Pack layout

```text
<pack>/
├─ pack.toml                 required; never installed
├─ skills/<name>/SKILL.md    one directory per skill, installed as a unit at the tool's skill location
├─ agents/<name>.md          one file per agent, installed at the tool's agent location
├─ native/<tool>/            content for one tool only (§2.2)
└─ <anything else>           installed under the pack's install root (`{{root}}`)
```

`{{root}}` holds only the "anything else" files; skills and agents are never under it. A path into
a skill's directory is written with `{{skill_dir …}}` (§3).

### 2.1 Rules

| Rule | Refusal |
|---|---|
| `pack.toml` exists at the pack root | `<dir>: not a pack (no pack.toml)` |
| `commands/` and `rules/` are reserved | `<pack>/commands: reserved, not supported in api 0` |
| a skill directory contains `SKILL.md` | `<pack>/skills/<name>: missing SKILL.md` |
| a skill's `name` equals its directory name; an agent's `name` its file stem | `<file>: name "x" does not match "y"` |
| at least one skill, agent or hook, counting only files not excluded | `<pack>: pack declares nothing to install` |
| no symlinks anywhere | `<path>: symlinks are not allowed in a pack` |
| paths are valid UTF-8 | `<path>: path is not valid UTF-8` |
| entries whose name starts with `.` (e.g. `.DS_Store`, `.git/`) are ignored: not hashed, not installed; `plan` lists them once | (not an error) |
| a skill and an agent may share a name; two skills (or two agents) in one pack cannot, by construction | — |

### 2.2 `native/<tool>/`

- `<tool>` must match the name rule (§4.1); it is not checked against a list of known tools,
  because extensions come from git. `plan` lists each unused `native/<tool>/` once
  (`native/cluade: not for claude`), so a typo is visible.
- Only the extension named `<tool>` receives that directory's files. They are rendered by the same
  rules as neutral files (§3.4) and appear in `plan`. Native `.md` files are not held to §5,
  because their frontmatter is tool-specific.
- `native/<tool>/hooks.toml` declares tool-native hooks with the `[[hooks]]` schema of §6, except
  that `event` and `match` are opaque tool strings. Grants, `policy`, `may_block`, forced
  fail-open and `exec ⊆ requires.binaries` apply unchanged.
- Neutral content must not reach into `native/`: a `root` token followed by `/native/` in neutral
  content is refused: `<file>:<line>: neutral content must not reference native/`.

## 3. Tokens

A token is a placeholder that the core replaces with a tool-specific value at render time. There
is no template engine and no logic: no conditionals, loops or filters.

### 3.1 Grammar

```text
token     = "{{" ws* kind ( ws+ target )? ws* "}}"         ; one line only
kind      = "skill" | "agent" | "skill_dir" | "root" | "var" | "option"
ws        = " " | "\t"
target    = item-ref | pack | var-name | option-name
item-ref  = ( pack "/" )? item        ; skill, agent, skill_dir
pack      = name                      ; root (optional)
item      = name
var-name  = "session_id" | "arguments" | "tmpdir" | "home" | "airsstack_home" | "project_dir"
option-name = §4.1 option rule        ; option
name      = §4.1 name rule
escape    = "\{{"                     → the literal "{{"; the backslash is removed
```

| Kind | Target | Renders to (value supplied by the extension) |
|---|---|---|
| `skill` | item-ref | the skill's **invocation form** (Claude: `/<name>`); authors never write `/` |
| `agent` | item-ref | the agent's name as the tool addresses it |
| `skill_dir` | item-ref (a skill) | the path of that skill's installed directory |
| `root` | pack or empty (this pack) | the path of that pack's install root |
| `var` | var-name | the tool's runtime expression for that value |
| `option` | option-name of **this** pack | the option's resolved value (§4.4) |

Var placement: `arguments` only in `SKILL.md` files (all real uses are there, §1); the others
anywhere tokens render. The var list is fixed; a new var is added when real content needs one.

Limitation (accepted): a literal backslash immediately before a token cannot be written, because
`\{{` is always the escape. A `\\` escape is added when real content needs one.

### 3.2 Checked examples

| Input | Result |
|---|---|
| `run {{skill sdlc/status}} now` | token `skill` → `sdlc/status` |
| `{{ root }}/scripts/x.lua` | token `root`, this pack; inner spaces trimmed |
| `{{skill_dir discuss}}/references/protocol.md` | token `skill_dir` → this pack's `discuss` |
| `$\{{ secrets.X }}` | literal `${{ secrets.X }}` |
| `}}` with no `{{` | literal text |
| `{{skil core/x}}` | `<file>:<line>: unknown token kind "skil"` |
| `{{skill core/x` / a newline before `}}` | `<file>:<line>: unterminated token` |
| `{{}}`, `{{ }}` | `<file>:<line>: empty token` |
| `{{skill}}` (also `agent`, `skill_dir`, `var`, `option`) | `<file>:<line>: token "skill" needs a target` |
| `{{root core extra}}` | `<file>:<line>: token "root" takes one target` |
| `{{skill a/b/c}}` | `<file>:<line>: "a/b/c" is not a valid target (expected [pack/]name)` |
| `{{skill /x}}`, `{{skill core/}}` | same message, with that target |
| `{{skill Core/x}}` | `<file>:<line>: "Core" is not a valid name` |
| `{{root core/x}}` | `<file>:<line>: token "root" takes a pack name, not "core/x"` |
| `{{var core/tmpdir}}`, `{{var session}}` | `<file>:<line>: unknown var "…" (known: session_id, arguments, tmpdir, home, airsstack_home, project_dir)` |
| `{{var arguments}}` outside a `SKILL.md` | `<file>:<line>: var "arguments" is only valid in SKILL.md` |
| `{{option a-b}}` | `<file>:<line>: "a-b" is not a valid option name` |
| `{{option core/archive_keep}}` | `<file>:<line>: option tokens name this pack's options only` |
| `{{skill_dir core/coder}}` where `coder` is only an agent | `<file>:<line>: pack "core" has no skill "coder"` |

### 3.3 Target rules (cross-pack)

- `skill`, `agent`, `skill_dir` and `root` targets name **this pack or a pack in
  `requires.packs`**; else
  `packs/sdlc/skills/plan/SKILL.md:68: {{skill guideline-rust/rust-guidelines}} targets pack "guideline-rust", which sdlc does not require. Add it to requires.packs or remove the ref.`
- The named item exists in that pack: `… pack "core" has no agent "codr"`.
- Option values are inserted as-is and never re-scanned: a string option containing `{{` renders
  literally.

### 3.4 Where tokens are rendered

- Every `.md` file that is not excluded, and every file listed in `render`.
- In `pack.toml` (and `native/<tool>/hooks.toml`): hook `text`, `args`, `run` and grant paths.
- **Frontmatter is parsed first, then its string values are scanned.** A value that starts with a
  token must be quoted, as YAML requires (§1); the YAML error for an unquoted one gets the hint
  `(a value starting with {{ must be quoted)`. `name` may not contain a token:
  `<file>: name must be a literal`.
- `render` may not list `.lua` or `.sh` files: `render: "x.lua" is code; code is installed
  verbatim`. A rendered file whose content is not valid UTF-8 is refused:
  `<file>: not valid UTF-8, cannot render`.
- Everything else is installed byte-for-byte and never scanned. Code receives names and paths at
  runtime through declared hook `args`.

### 3.5 Who does what

The core scans, validates every token and splices values in. The extension never parses text: it
returns the tool's name, invocation form or path for the items the core asks about, and the tool's
runtime expression for each var. If it cannot supply a value, the item holding that token becomes a
skip with a reason, shown in `plan`.

## 4. `pack.toml`

```toml
[pack]
name        = "sdlc"
version     = "0.4.2"
api         = 0
description = "intent → spec → plan → execute chain"

[requires]
packs    = ["core"]
airsl    = ">=0.1.4"
binaries = ["git"]

render  = ["enforcement.json"]
exclude = ["**/*_test.lua"]

[options.archive_keep]
type        = "integer"
default     = 20
description = "Discussions kept per project besides the current one"

[[hooks]]                                     # §6
```

### 4.1 Names

- Pack, tool, extension, skill and agent names: `^[a-z][a-z0-9-]{0,63}$` (48/48 real names fit, §1).
- Option names: `^[a-z][a-z0-9_]{0,63}$` (the real options are `style_reinject`, `archive_keep`;
  Proof: `P/claudestacks/.claude-plugin/plugin.json` `userConfig` keys).
- Checked: `core` ✓, `guideline-rust` ✓, `Core` ✗, `9lives` ✗, `-x` ✗, 65×`a` ✗;
  `archive_keep` ✓ as option, ✗ as pack name.

### 4.2 Field rules

| Field | Rule | Refusal |
|---|---|---|
| any unknown key | refused with its key path | `packs/core/pack.toml: hooks[2].grant: unknown key` |
| `pack.name` | required, §4.1 | `pack.name: "Core" is not a valid name` |
| `pack.version` | required, semver | `pack.version: "1.2" is not semver` |
| `pack.api` | required, `0` | `sdlc: api 1 not supported (this agentstacks speaks 0)` |
| `pack.description` | required, not empty or whitespace-only | `pack.description: empty` |
| `requires.packs` | names; no duplicates; not itself | `requires.packs: "core" listed twice` / `requires.packs: a pack cannot require itself` |
| `requires.airsl` | semver range; required when the pack has any non-excluded `.lua` file | `sdlc: contains .lua files but declares no requires.airsl` |
| `requires.binaries` | names; no duplicates | `requires.binaries: "git" listed twice` |
| `render` | each path exists, relative, no `..`; not code (§3.4); not excluded | `render: "enforcment.json" does not exist` / `render: "x.json" is also excluded` |
| `exclude` | valid globs, relative; may not match `pack.toml`, a `SKILL.md`, an agent file, a hook `run` target or a `render` file | `exclude: "[" is not a valid glob` / `exclude: "**/*.md" excludes skills/discuss/SKILL.md` |
| `options.<n>.type` | `boolean`, `integer` (i64), `string` | `options.keep.type: "number" (expected boolean, integer, string)` |
| `options.<n>.default` | optional; matches `type` | `options.archive_keep.default: expects integer, got string` |
| `options.<n>.description` | required, not empty | `options.archive_keep.description: empty` |

Excluded files are not scanned and not installed; the tree hash still covers them (§8.1).

### 4.3 `airsl` range

The core intersects every pack's `requires.airsl` (Cargo-flavour ranges, `semver` crate). No
common version → `airsl: core needs >=0.2, sdlc needs <0.2`. Installed airsl outside it →
`airsl 0.1.3 installed, packs need >=0.1.4`. Probing the binary and pinning it belong to
`core-plan-and-apply` and `extension-host-and-hooks`.

### 4.4 Option resolution

| Case | Result |
|---|---|
| set in `agentstacks.toml`, right type | that value |
| not set, `default` present | `default` |
| not set, no `default` | `core: option "x" is required` |
| wrong type (`archive_keep = "many"`) | `core: option archive_keep expects integer, got string` |
| not declared by the pack | `core: no option "archive_kep"` |
| rendering | boolean → `true`/`false`; integer → decimal; string → as-is |

### 4.5 `extension.toml`

An extension directory has `extension.toml` at its root:

```toml
[extension]
name        = "claude"
version     = "0.1.0"
api         = 0
description = "Renders packs into Claude Code project config"
entry       = "adapter.lua"
```

| Rule | Refusal |
|---|---|
| file present | `<dir>: not an extension (no extension.toml)` |
| `name` §4.1; `version` semver; `api` = 0; `description` not empty | same shapes as §4.2, prefixed `extension.` |
| `entry` relative, no `..`, an existing `.lua` file | `extension.entry: "adapter.lua" does not exist` |
| unknown key | `extension.toml: extension.foo: unknown key` |
| no symlinks; non-UTF-8 paths refused; dot entries ignored | as §2.1 |

What the adapter exports and how it is run belong to `extension-host-and-hooks`.

## 5. Skill and agent frontmatter

YAML between `---` lines at the top of `SKILL.md` or `agents/<name>.md`; strict keys; parsed
before token scanning (§3.4).

```yaml
# skill                                   # agent
name: discuss                             name: coder
description: …                            description: …
invocation: user      # user | any        model: balanced      # fast | balanced | deep
argument_hint: "[list | <id> | done]"     effort: high         # low | medium | high
native:                                   tools: [read, edit, write, grep, glob, shell, skill]
  claude: { … }                           native:
                                            claude: { model: claude-opus-5-5 }
```

| Rule | Refusal |
|---|---|
| frontmatter missing, not first, or unterminated | `<file>: missing frontmatter` |
| YAML syntax error | `<file>:<line>: <parser message>` (stops this file only) |
| `name`, `description` required, not empty; `name` literal | `<file>: description: empty` / `<file>: name must be a literal` |
| `name` = directory (skill) or file stem (agent) | `<file>: name "x" does not match "y"` |
| unknown key | `<file>: unknown key "disable-model-invocation"` |
| `invocation` ∈ {user, any}, default `any` | `<file>: invocation: "manual" (expected user, any)` |
| `argument_hint` string | `<file>: argument_hint: expected a string` |
| `model` ∈ {fast, balanced, deep}, optional | `<file>: model: "sonnet" (expected fast, balanced, deep)` |
| `effort` ∈ {low, medium, high}, optional | `<file>: effort: "max" (expected low, medium, high)` |
| `tools` ⊆ {read, write, edit, grep, glob, shell, skill}, no duplicates, optional | `<file>: tools: unknown "bash"` / `<file>: tools: "read" listed twice` |
| `native` keys follow §4.1; contents opaque, passed to that extension only | `<file>: native: "Claude" is not a valid tool name` |

The vocabulary covers what the plugins use (§1) plus `effort: medium`, which completes the scale.
A field the tool cannot express is dropped with a reason in `plan`. A `native.<tool>` value wins
over the neutral field for that tool.

## 6. Hooks

```toml
[[hooks]]
kind  = "message"
event = "session.end"
text  = "Reminder: run {{skill snapshot-save}} to flush durable session learnings."

[[hooks]]
kind      = "lua"
event     = "tool.before"
match     = ["read", "edit", "write"]
run       = "{{root core}}/hooks/enforce.lua"
args      = ["--manifest", "{{root}}/enforcement.json"]
may_block = false
[hooks.grants]
policy = "confined"
read   = ["/"]
write  = ["{{var tmpdir}}"]
env    = ["HOME", "TMPDIR"]
exec   = []
```

### 6.1 Event vocabulary

| Event | `match` values | Other selectors |
|---|---|---|
| `session.start` | `startup`, `resume`, `clear`, `compact` | — |
| `session.end` | — | — |
| `prompt.submit` | — | — |
| `tool.before` | `read`, `edit`, `write` | — |
| `tool.after` | `read`, `edit`, `write` | — |
| `agent.stop` | — | `agents = [<this pack's agent names>]` |

This covers what the plugins use (§1); `tool.after` also accepts `read` and `edit` so the two tool
events share one set. A Claude event or tool with no neutral form (today: `PreToolUse` on
`SubagentHandback`) goes in `native/claude/hooks.toml` (§2.2).

### 6.2 Field rules

| Rule | Refusal |
|---|---|
| `kind` ∈ {message, lua} | `hooks[0].kind: "sh" (expected message, lua)` |
| `event` in §6.1 | `hooks[0].event: "tool.pre" unknown` |
| `match` only where §6.1 allows; values from that set; not empty; no duplicates; omitted = all | `hooks[1].match: "bash" not valid for tool.before` / `hooks[1].match: empty list never fires` / `hooks[1].match: "read" listed twice` |
| `agents` only on `agent.stop`; this pack's agents; omitted = any agent | `hooks[3].agents: pack "core" has no agent "task-briefer"` |
| `message`: `text` required, not empty; `run`, `args`, `grants`, `may_block` refused | `hooks[0].text: empty` / `hooks[0].may_block: message hooks cannot block` |
| `lua`: `run` required, an existing, non-excluded `.lua` file in this pack (pack-relative or `{{root}}/…`) or in a required pack (`{{root <pack>}}/…`) | `hooks[1].run: "{{root core}}/hooks/enforc.lua" does not exist in core` |
| `grants.policy` ∈ {confined, pure}, default confined | `hooks[1].grants.policy: "trusted" is not allowed for pack hooks` |
| grant paths absolute or starting with `{{root…}}`, `{{skill_dir …}}` or `{{var …}}` | `hooks[1].grants.read: "data" must be absolute or start with a token` |
| `grants.env` names `^[A-Z_][A-Z0-9_]*$` | `hooks[1].grants.env: "home" is not an environment variable name` |
| `grants.exec` ⊆ `requires.binaries` ∪ {airsl} | `hooks[1].grants.exec: "git" is not in requires.binaries` |
| `may_block` boolean, default false | `hooks[1].may_block: expected true or false` |

### 6.3 Fixed behaviour

- Every `lua` hook runs with airsl's `--fail-open`; no field changes it. Proof: `airsl run --help`
  (airsl 0.1.4) → `--fail-open  Discard errors and always exit 0.`
- Grants map one-to-one onto `airsl run` flags. Proof: same help → `--policy <POLICY>` (`trusted`,
  `confined`, `pure`), `--allow-read <DIR>`, `--allow-write <DIR>`, `--allow-env <NAME>`,
  `--allow-exec <PROGRAM>`.
- Only a hook with `may_block = true` may block; `plan` shows "this hook can block tool calls".
  The neutral payload and result protocol belong to `extension-host-and-hooks`.
- A Lua script that a skill tells the model to run is not a hook: its command line is Markdown
  prose and is not checked against `requires.binaries`.

## 7. `agentstacks.toml`

```toml
api = 0

[tools.claude]
git    = "https://github.com/airsstack/agentstacks"
tag    = "packs-v0.1.0"
subdir = "extensions/claude"

[packs.core]
git     = "https://github.com/airsstack/agentstacks"
tag     = "packs-v0.1.0"
subdir  = "packs/core"
options = { archive_keep = 50 }

[packs.team-lint]
path = "packs/team-lint"
```

| Rule | Refusal |
|---|---|
| `api` required, `0` | `agentstacks.toml: api 1 not supported (this agentstacks speaks 0)` |
| at least one `[tools.*]` and one `[packs.*]` | `agentstacks.toml: nothing to install` |
| table keys follow §4.1 | `packs.Core: not a valid name` |
| exactly one of `git` / `path` | `packs.core: needs exactly one of git, path` |
| `git` needs exactly one of `tag` / `rev`; `rev` 40 or 64 lowercase hex; no `branch` | `packs.core: needs exactly one of tag, rev` / `packs.core.rev: not a commit id` |
| a `git` URL starting with `-` | `packs.core.git: URL must not start with "-"` |
| `subdir` relative, no `..` segment, no leading `/` | `packs.core.subdir: must stay inside the repository` |
| `path` relative to the directory holding `agentstacks.toml`, resolving inside it | `packs.team-lint.path: absolute paths are not portable` / `packs.team-lint.path: must stay inside the project` |
| `options` only under `[packs.*]`; checked per §4.4 | `tools.claude.options: unknown key` |
| any string empty or whitespace-only | `packs.core.tag: empty` |
| unknown key | `packs.core.branch: unknown key` |
| key = fetched `pack.name` / `extension.name` | `packs.core: source contains pack "kore"` / `tools.claude: source contains extension "claud"` |
| fetched `extension.toml` api = 0 | `tools.claude: api 1 not supported (this agentstacks speaks 0)` |
| every `requires.packs` entry is listed | `sdlc requires pack "core", which agentstacks.toml does not list (add it: --pack core)` |
| `requires.packs` has no cycle | `requires cycle: a → b → a` |

Proof for the `-` rule: `git ls-remote -h` (git 2.39.3) prints usage, i.e. git takes a
dash-leading argument in the repository slot as its own flag; the same usage lists
`--upload-pack <exec>  path of git-upload-pack on the remote host`, an option that names a program.

Shorthand: `init --pack core` / `--tool claude` expands to the official repository, latest
`packs-v*` tag, `subdir = "packs/<name>"` / `"extensions/<name>"`; the full form is what gets
written. A bare name that is not official is refused: `"x" is not an official pack; pass a git URL
or a path`. Expansion and tag lookup are `cli-commands` / `core-plan-and-apply` code.

## 8. `agentstacks.lock`

```toml
api = 0
# Generated by agentstacks. Do not edit.

[airsl]
version = "0.1.4"

[[tool]]
name    = "claude"
version = "0.1.0"
git     = "https://github.com/airsstack/agentstacks"
tag     = "packs-v0.1.0"
subdir  = "extensions/claude"
rev     = "<40 hex>"
tree    = "sha256:<64 hex>"

[[pack]]
name    = "core"
version = "0.3.0"
git     = "https://github.com/airsstack/agentstacks"
tag     = "packs-v0.1.0"
subdir  = "packs/core"
rev     = "<40 hex>"
tree    = "sha256:<64 hex>"

[[pack]]
name    = "team-lint"
version = "0.1.0"
path    = "packs/team-lint"
tree    = "sha256:<64 hex>"

[[file]]
path = ".claude/skills/status/SKILL.md"
hash = "sha256:<64 hex>"
by   = "sdlc"

[[merge]]
file = ".claude/settings.json"
key  = "<locator>"
hash = "sha256:<64 hex>"
by   = "core"
```

| Rule | Refusal |
|---|---|
| `api` required, `0` | `agentstacks.lock: api 1 not supported …` |
| `[airsl]` present iff a locked pack declares `requires.airsl`; `version` exact semver | `agentstacks.lock: airsl.version: "0.1" is not a version` |
| `[[tool]]`/`[[pack]]`: `name`, `version`, `tree` required; exactly one of `git`/`path`; git entries need `rev`, plus `tag` when `agentstacks.toml` pinned one; §7 source rules apply | `agentstacks.lock: pack core: missing rev` / `… pack core: needs exactly one of git, path` |
| `[[file]]`: `path`, `hash`, `by` required; `path` relative, `/`-separated, no `..`; unique | `agentstacks.lock: file ".claude/x" listed twice` |
| `[[merge]]`: `file`, `key` (non-empty), `hash`, `by` required; unique by (`file`, `key`) | `agentstacks.lock: merge ".claude/settings.json" "<key>" listed twice` |
| hashes `^sha256:[0-9a-f]{64}$` | `agentstacks.lock:<line>: bad hash` |
| `by` names a locked pack or tool | `agentstacks.lock: file …: by "kore" is not a locked pack or tool` |
| tool names unique; pack names unique | `agentstacks.lock: pack "core" listed twice` |
| syntax error / unknown key | `agentstacks.lock:<line>: …` (recovery: delete the lock and re-plan) |
| written sorted: tools and packs by name, files by path, merges by (file, key) | (writer rule) |

A missing lock is not an error. A lock that disagrees with `agentstacks.toml` is stale, not
malformed. `rev` wins over a tag that has since moved. Reconciliation, the meaning of
`[[merge]].key`, and tree mismatches (§8.2) belong to `core-plan-and-apply`.

### 8.1 Tree hash

Walk every regular file of a pack or extension directory, skipping dot entries (§2.1) and refusing
symlinks; excluded files are included. For each file form the byte string
`<relative path, "/"-separated>` + `0x00` + `<lowercase hex SHA-256 of the raw bytes>` + `0x0A`
(no other separators). Sort these by path bytes, concatenate, and hash: tree hash = `sha256:` +
hex SHA-256 of the concatenation. Raw bytes, no line-ending normalisation, modes ignored,
directories not hashed.

Checked example (`shasum -a 256`, this session): `a.md` = `"a\n"` →
`87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7`; `b/c.lua` = `"x\n"` →
`73cb3858a687a8494ca3323053016282f3dad39d42cf62ca4e79dda2aac7d9ac`;
`printf 'a.md\0%s\nb/c.lua\0%s\n' $A $C | shasum -a 256` →
`b753fa890a9101e377ba1a71365c2257312468aa5d939b5e084de025486dc99b`, so the tree hash is
`sha256:b753fa89…dc99b`. A walk visiting `b/c.lua` first gives the same result, because lines are
sorted first (`printf 'b/c.lua\na.md\n' | LC_ALL=C sort` → `a.md`, `b/c.lua`).

Byte stability requires that git content is read from the object store, never from a working-tree
checkout. A checkout applies line-ending conversion through `.gitattributes` and `core.eol`
(Proof: `man git-config`, section `core.eol`: "Sets the line ending type to use in the working
directory for files that are marked as text … Alternatives are lf, crlf and native"; section
`core.autocrlf`: "Setting this variable to "true" is the same as setting the text attribute to
"auto" on all files and core.eol to "crlf""). `git cat-file` returns object content unconverted
unless `--filters` is given (Proof: `man git-cat-file`, `--filters`: "Show the content as converted
by the filters configured in the current working tree … (i.e. smudge filters, end-of-line
conversion, etc)"). The resolver therefore lists the tree with `git ls-tree` and reads blobs with
`git cat-file` (handed to `core-plan-and-apply`, §13).

### 8.2 Tree mismatch (handed off)

A git pack whose fetched tree differs from the lock at the same `rev` is refused (corruption or
tampering). A path pack whose tree changed is normal: `plan` shows the change and `apply` updates
the lock.

## 9. Whole-set checks

After each file parses, `pack_set::check` validates the project as a whole and returns a
`ValidSet`, the only input later stages accept. Packs or extensions that failed to load are
skipped, so their failures produce no follow-on errors.

1. every `requires.packs` entry is listed in `agentstacks.toml`; no cycles (§7);
2. every `skill`/`agent`/`skill_dir`/`root` token targets this pack or a required pack, and the
   item exists (§3.3);
3. every `option` token names a declared option; option values resolve (§4.4);
4. every hook `run` and `agents` entry resolves (§6.2);
5. the `airsl` ranges intersect (§4.3);
6. no two packs ship a skill with the same name, and no two packs ship an agent with the same name:
   `skill "status" is shipped by both sdlc and team-lint`. A skill and an agent may share a name.

## 10. Errors

- Every refusal carries a location: `<file>:<line>: <what>`, or `<file>: <key.path>: <what>` when
  there is no line. Messages name the offending value and, where a set is fixed, the allowed set.
- **All errors per run**, sorted by file; within a file, key-path errors (no line) first, then by
  line. A TOML or YAML syntax error stops only that file.
- One `thiserror` enum per module, no string errors, so `cli-commands` can map them to exit codes.
  A validation run returns a list of `Diagnostic { location, error }`.

## 11. Architecture

Modules in `crates/agentstacks` (export-only `mod.rs`, one item per file, newtypes for domain
values per the Rust guidelines):

```text
model/
├─ name.rs         PackName, ToolName, ItemName, OptionName       (§4.1)
├─ source.rs       Source::{Git{url, pin: Pin::{Tag, Rev}, subdir}, Path}
├─ token.rs        Token{kind, target}, scan(&str) → tokens or errors   (§3)
├─ frontmatter.rs  SkillMeta, AgentMeta                           (§5)
├─ pack.rs         PackManifest + Pack::load(dir)                 (§2, §4, §6)
├─ extension.rs    ExtensionManifest + Extension::load(dir)       (§4.5)
├─ project.rs      ProjectFile (agentstacks.toml)                 (§7)
└─ pack_set.rs     ValidSet + check()                             (§9)
lock/
├─ lockfile.rs     Lockfile: parse + sorted serialise             (§8)
└─ tree_hash.rs    tree_hash(dir)                                 (§8.1)
```

`validation/` stays for checks on proposed **operations** (`core-plan-and-apply`). Proof of the
existing seams: `crates/agentstacks/src/lib.rs:10-16` (module docs: `model`, `validation`, `plan`,
`executor`, `lock`, `extension_host`); `model.rs`, `lock.rs`: "This module currently holds no
items."

```text
agentstacks.toml ─► ProjectFile ─┐
pack dirs ───────► Pack::load ───┤
extension dirs ──► Extension::load ┼─► pack_set::check ─► ValidSet ─► (extension host, plan)
agentstacks.lock ► Lockfile ─────┘
```

Dependencies (latest stable per `cargo search <name> --limit 1` this session): `serde` 1 (derive),
`toml` 1 (`1.2.1+spec-1.1.0`), `serde-saphyr` 2 (claudestacks uses `{ version = "2.0",
default-features = false, features = ["deserialize"] }`, chosen there because it parses YAML in
safe Rust — Proof: `../claudestacks/Cargo.toml` `[workspace.dependencies]`; `3.0.0-rc.1` is a
pre-release, not used), `sha2` 0.11 (`0.11.1`), `semver` 1 (`1.0.28`), `globset` 0.4 (`0.4.21`),
`thiserror` 2 (`2.0.21`). Licences are checked by `cargo make deny` when added.

## 12. Testing

- **Unit tests** colocated in every logic-bearing file, covering each rule in §2–§10 with at least
  one breaking case, including every checked example in §3.2, §4.1 and §8.1.
- **Fixtures** under `crates/agentstacks/tests/fixtures/`: small valid packs and extensions, one per
  feature (both hook kinds, native hooks, options, `native/`, `render`/`exclude`, cross-pack `root`,
  `skill_dir` and refs), and one invalid input per refusal, each asserting the exact message and
  location.
- **Coverage rule:** every refusal in this spec has a test; the plan maps each one.
- `cargo make dod` green. Converting the real plugins is not tested here (`plugins-to-packs`).

## 13. Handed to other chains

| Chain | Receives |
|---|---|
| `core-plan-and-apply` | source resolution with `git` (URLs after `--`; tree listed with `ls-tree`, blobs read with `cat-file`, never a working-tree checkout); a **delete** operation for files of a removed pack; the meaning of `[[merge]].key`; stale-lock reconciliation; tree mismatches (§8.2); `rev` over a moved tag |
| `extension-host-and-hooks` | the adapter contract behind `extension.entry`; the neutral hook payload and result protocol; enforcing `may_block`; airsl as a require-binary operation and its pin; a runtime warning when airsl disappears after install (today `preflight.sh:40`) |
| `claude-extension` | mapping the neutral vocabulary (events, `match`, tiers, tools, `invocation`, vars) to Claude Code; invocation forms, names and paths for tokens; `native/claude/hooks.toml` for `SubagentHandback` |
| `plugins-to-packs` | the three example guideline refs rewritten as prose; the `SubagentStop` matcher split per pack; the registry gate removed from `enforce.lua`; hooks ported to the neutral protocol; options via `args`; the user-facing name in `P/claudestacks-sdlc/scripts/lib/status.lua:825` passed via `args` or called out as a change |
| `cli-commands` | the shorthand expansion and its refusal; mapping error enums to exit codes; `doctor` reporting a missing airsl |

## 14. Non-goals

- A registry, hosted index or source aliases; `branch` pins.
- Embedding packs or extensions in the binary.
- Data shared between packs (each pack's hooks act on that pack's own data).
- Commands and rules (reserved, §2.1).
- Version ranges on `requires.packs`; version checks on binaries other than airsl.
- A `\\` escape before tokens (§3.1).
- Fetching, caching, rendering to any tool, writing files, or running hooks: this chain defines
  and validates formats only.
- OpenCode-specific needs beyond keeping `native/<tool>/` tool-neutral in shape.
- A stable format: everything here is `api = 0`.
