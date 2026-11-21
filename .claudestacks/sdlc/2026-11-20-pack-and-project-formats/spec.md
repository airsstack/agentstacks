---
status: approved
created: 2026-11-21
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
| A `.DS_Store` file sits in the plugins directory. | `find . -name '.*'` → `./.DS_Store` |

### 1.1 Parser facts (probed this session)

Probes: throwaway crates built with `cargo build --offline` against serde-saphyr `2.1.1`, toml
`1.2.0+spec-1.1.0` and semver `1.0.28` (versions from the probe's `Cargo.lock`). `SY/`, `T/` and `S/`
abbreviate `~/.cargo/registry/src/*/serde-saphyr-2.1.1/src/`, `toml-1.2.0/src/` and
`semver-1.0.28/src/`. Raw output:
`/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/f6313af9-24e7-452d-b5aa-2740e0bbaa85/scratchpad/probe-{yaml,toml}-results.md`.

| Fact | Proof |
|---|---|
| serde-saphyr refuses an unquoted value or list item that starts with `{{`, with a line and column; a quoted one is a plain string | `description: {{skill status}} runs it` → `line 1 column 15: expected string scalar`; `tools:\n  - {{skill x}}` → `line 2 column 6: expected string scalar`; `description: "{{skill status}} runs it"` → `Str("{{skill status}} runs it")` |
| A token later in a plain value, or at the start of a folded block value, parses; one inside a flow sequence is refused | `description: run {{skill status}} now` → `"run {{skill status}} now"`; `description: >\n  {{skill status}} runs it` → `"{{skill status}} runs it\n"`; `tools: [a, {{skill x}}]` → `expected string scalar` on line 1 (probe `probe-mid`, raw output in `…/scratchpad/probe-msg-results.md`) |
| The bare message comes from `DefaultMessageFormatter.format_message(&err)` (no location, no snippet); `location().line()` is stable, columns are not (they differ between `Display` and `location()` and with the target type); line numbers count from the YAML text, not the file | `description: {{skill status}} runs it` → `loc=(1, 14) msg="expected string scalar"`, while `Display` says column 15; `a: 1\na: 2` → `loc=(2, 1) msg="duplicate mapping key: a, set DuplicateKeyPolicy in Options if acceptable"` (probe `probe-msg`, `…/scratchpad/probe-msg-results.md`; exports `SY/lib.rs:32,44`; trait `SY/de/error.rs:89-114`) |
| Errors expose an optional location | `Error::location() -> Option<Location>` (`SY/de/error.rs:1801`), `.line()`/`.column()` 1-based (`SY/location.rs:176,183`); `a: [1, 2` → `unclosed bracket '['` at (1,4) |
| No generic value type is exported; `deserialize_any` fills an own enum | own enum `{Null, Bool, Int, Float, Str, Seq, Map(Vec<(String, Value)>)}` filled from a nested mapping (`SY/de/deserializer.rs:917`) |
| Duplicate mapping keys are an error by default | `a: 1\na: 2` → `line 2 column 1: duplicate mapping key: a` (`SY/de/message_formatters.rs:257`) |
| Plain scalars follow YAML 1.1 booleans: `yes`, `on`, `y`, `True` → true; `no`, `off` → false; `~`, `null` → null | probe Q8 (`SY/parse_scalars.rs:34-47`) |
| Empty, whitespace-only and comment-only YAML is `Null`, not an error | probe Q9 |
| Anchors and aliases are accepted under a default budget | `a: &x 1\nb: *x` → `b = Int(1)`; `Budget` (`SY/budget.rs:103`) |
| toml syntax errors carry a byte span; a 1-based line is the count of `\n` before `span().start` plus 1 | `a = [1,` → message ``unclosed array, expected `]` `` (backtick-quoted, byte for byte as printed: `…/scratchpad/probe-toml-results.md:19-20`), span `7..7`, `Display` "line 1, column 8" (`T/de/error.rs:64,69,116`) |
| Duplicate keys and duplicate tables are toml syntax errors; an integer beyond i64 is a syntax error | `a = 1\na = 2` and `[x]\n[x]` → `duplicate key`; `a = 9223372036854775808` → `u64 value was too large` |
| A `deny_unknown_fields` struct parse stops at the first error | two unknown keys plus a wrong-typed field → exactly one error, `unknown field 'bar'` |
| A raw `toml::Table` holds `Datetime` values and is sorted by key (`preserve_order` off by default) | `1979-05-27` → `Datetime`; input order z,b,a,m → iteration a,b,m,z (`T/Cargo.toml:113`, `T/map.rs:11-43`) |
| Empty, whitespace-only and comment-only toml is an empty table | probe Q6 → `Ok({})` |
| semver's `Op` is `#[non_exhaustive]` with 8 variants: Exact, Greater, GreaterEq, Less, LessEq, Tilde, Caret, Wildcard | `S/lib.rs:246-257` (attribute on `:247`) |
| globset `Glob::new` refuses malformed globs but accepts absolute ones, so "relative" is checked by agentstacks itself (leading `/`) | globset `0.4.21`: `"["` → `error parsing glob '[': unclosed character class; missing ']'`; `"/tmp/**"` → Ok; `"**/*.md"` matches `skills/discuss/SKILL.md` (probe `probe-glob`, `…/scratchpad/probe-glob-results.md`) |
| `*` parses to no comparators (matches everything); `""` is an error; `0.1` is Caret; `>= 0.1.4` (space) parses; `Version::parse("1.2")` is an error | probe Q8, Q9 |

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
| `pack.toml` exists at the pack root; without it the run ends with this refusal and the walk's own refusals only | `<dir>: not a pack (no pack.toml)` |
| `commands/` and `rules/` are reserved: refused once per directory when it holds a file (an empty one is ignored, like any empty directory) | `<pack>/commands: reserved, not supported in api 0` |
| a skill directory contains `SKILL.md` | `<pack>/skills/<name>: missing SKILL.md` |
| `skills/` holds only directories, each named per §4.1 | `<pack>/skills/README.md: only skill directories belong in skills/` / `<pack>/skills/My_Skill: "My_Skill" is not a valid name` |
| `agents/` holds only `<name>.md` files (no subdirectories), stem per §4.1; a subdirectory is refused once, at its own path | `<pack>/agents/notes.txt: agents/ holds only <name>.md files` / `<pack>/agents/x: agents/ holds only <name>.md files` / `<pack>/agents/Coder.md: "Coder" is not a valid name` |
| a skill's `name` equals its directory name; an agent's `name` its file stem | `<file>: name "x" does not match "y"` |
| at least one skill, agent, hook or native hook (`native/<tool>/hooks.toml` entry), counting only files not excluded; not reported when another refusal may have hidden an item (§10) | `<pack>: pack declares nothing to install` |
| no symlinks anywhere | `<path>: symlinks are not allowed in a pack` (in an extension: `<path>: symlinks are not allowed in an extension`) |
| only regular files and directories (no FIFOs, sockets, devices) | `<path>: only regular files and directories are allowed` |
| empty directories are ignored: not installed, not hashed, never a refusal (an empty `skills/a/` is not a skill) | — |
| the dot-entry rule below is applied first: a dot entry is ignored even when it is a symlink, a special file or a non-UTF-8 name | — |
| paths are valid UTF-8 | `<path>: path is not valid UTF-8` |
| a text that is read but never rendered (`pack.toml`, `SKILL.md`, an agent `.md`, `native/<tool>/hooks.toml`, and in an extension `extension.toml`) is valid UTF-8; a rendered file keeps its own message (§3.4) | `<file>: not valid UTF-8` |
| a file that cannot be read; a pack load and an extension load each read every walked file once, excluded and never-rendered files included | `<path>: <the std::io::Error Display>` (§10) |
| entries whose name starts with `.` (e.g. `.DS_Store`, `.git/`) are ignored: not hashed, not installed; `plan` lists them once | (not an error) |
| a skill and an agent may share a name; two skills (or two agents) in one pack cannot, by construction | — |

### 2.2 `native/<tool>/`

- `native/` holds only directories (`<pack>/native/README.md: only tool directories belong in
  native/`), each named per §4.1 (`<pack>/native/Claude: "Claude" is not a valid name`).
- `<tool>` must match the name rule (§4.1); it is not checked against a list of known tools,
  because extensions come from git. `plan` lists each unused `native/<tool>/` once
  (`native/cluade: not for claude`), so a typo is visible.
- Only the extension named `<tool>` receives that directory's files. They are rendered by the same
  rules as neutral files (§3.4) and appear in `plan`. Native `.md` files are not held to §5,
  because their frontmatter is tool-specific.
- `native/<tool>/hooks.toml` declares tool-native hooks with the `[[hooks]]` schema of §6, except
  that `event` and `match` are opaque tool strings and `agents` is not accepted (generic
  `unknown key`). Grants, `policy`, `may_block`, forced
  fail-open, `exec ⊆ requires.binaries` and the `run` rules apply unchanged. The file itself is
  never installed (like `pack.toml`); its hooks reach the tool through the extension.
- Neutral content (every file outside `<pack>/native/`) must not reach into `native/`: a `root`
  token (this pack or another) followed by one or more `/` and then the segment `native`, which
  ends the text or is followed by `/` or whitespace, is refused:
  `<file>:<line>: neutral content must not reference native/` (key-path location for frontmatter
  and hook fields). So `{{root}}/native/claude/x`, `{{root core}}/native/x`, `{{root}}//native/x`
  and `{{root}}/native` at the end of a value are refused; `{{root}}/natives/x`,
  `{{root}}/nativex`, `{{root}}/hooks/native/x`, `{{root}}/./native/x` and
  `{{skill_dir a}}/native/x` are not. Content under `native/` may reference `native/`. The scanner
  decides this while it has the text and records it on the token, so the rule runs over every
  token, wherever it came from. It still runs when `pack.toml` is refused (§10).

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

Var placement: `arguments` only in a skill's own `skills/<name>/SKILL.md` (all real uses are
there, §1), in its frontmatter or body; a file elsewhere that happens to be named `SKILL.md`
(`docs/SKILL.md`) is refused. The others are allowed anywhere tokens render. The var list is fixed; a new var is added when real content needs one.

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
| `{{root core extra}}` (any kind given two or more target words) | `<file>:<line>: token "root" takes one target` (with that kind's name) |
| `{{root Core}}` | `<file>:<line>: "Core" is not a valid name` |
| `{{skill a/b/c}}` | `<file>:<line>: "a/b/c" is not a valid target (expected [pack/]name)` |
| `{{skill /x}}`, `{{skill core/}}` | same message, with that target |
| `{{skill Core/x}}` | `<file>:<line>: "Core" is not a valid name` |
| `{{root core/x}}` | `<file>:<line>: token "root" takes a pack name, not "core/x"` |
| `{{var core/tmpdir}}`, `{{var session}}` | `<file>:<line>: unknown var "…" (known: session_id, arguments, tmpdir, home, airsstack_home, project_dir)` |
| `{{var arguments}}` outside a `skills/<name>/SKILL.md` (e.g. in `agents/b.md` or `docs/SKILL.md`) | `<file>:<line>: var "arguments" is only valid in SKILL.md` |
| `{{option a-b}}` | `<file>:<line>: "a-b" is not a valid option name` |
| `{{option core/archive_keep}}` | `<file>:<line>: option tokens name this pack's options only` |
| `{{option archive_kep}}` where the pack declares no such option | `<file>:<line>: pack "core" has no option "archive_kep"` |
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
- **Frontmatter is parsed first, then its string values are scanned.** A value or list item that
  starts with a token (including a flow-sequence item) must be quoted, as YAML requires (§1.1).
  When serde-saphyr refuses the frontmatter and the line it reports has `{{` right after `:`, `-`,
  `[` or `,` (spaces between allowed), the diagnostic gets the hint
  `(a value starting with {{ must be quoted)`. `name` may not contain a token:
  `<file>: name must be a literal`.
- `render` may not list `.lua` or `.sh` files: `render[2]: "x.lua" is code; code is installed
  verbatim` (at the entry's index, §4.2). A rendered file whose content is not valid UTF-8 is
  refused: `<file>: not valid UTF-8, cannot render`. A text that is read but never rendered gets
  `<file>: not valid UTF-8` instead (§2.1).
- Everything else is installed byte-for-byte and never scanned. Code receives names and paths at
  runtime through declared hook `args`.

### 3.5 Who does what

The core scans, validates every token and splices values in. The extension never parses text: it
returns the tool's name, invocation form or path for the items the core asks about, and the tool's
runtime expression for each var. If it cannot supply a value, the item holding that token becomes a
skip with a reason, shown in `plan`.

Each text is scanned exactly once. The frontmatter parser scans frontmatter string values; the
Markdown body scan starts after the closing `---`. The manifest parsers scan hook fields. The
pack loader scans bodies and `render` files. Every token found is kept on the loaded `Pack` with
its location. The pack-local rules (§3.2 placement and option-name rows, §2.2 no `native/` from
neutral content, own-pack targets) and the set-level target rules (§3.3) run over all of them,
wherever they came from.

## 4. `pack.toml`

```toml
render  = ["enforcement.json"]                # top-level keys come before the first [table]
exclude = ["**/*_test.lua"]

[pack]
name        = "sdlc"
version     = "0.4.2"
api         = 0
description = "intent → spec → plan → execute chain"

[requires]
packs    = ["core"]
airsl    = ">=0.1.4"
binaries = ["git"]

[options.archive_keep]
type        = "integer"
default     = 20
description = "Discussions kept per project besides the current one"

# [[hooks]] tables follow here (§6)
```

Proof: every TOML and YAML example in this spec was parsed with toml `1.2.0+spec-1.1.0` and
serde-saphyr `2.1.1` (probe `probe-examples`, raw output for this text in
`…/scratchpad/probe-examples-results-v2.md`: `exclude = ["**/*_test.lua"]`,
`render = ["enforcement.json"]` at the top level); each key lands at the path its section describes.
In the earlier text of this example `render`/`exclude` followed `[requires]`, where TOML reads them
as `requires.render`/`requires.exclude` (`…/scratchpad/probe-examples-results.md`).

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
| `pack.api` | required, `0` | `sdlc: api 1 not supported (this agentstacks speaks 0)`; when `pack.name` is missing or invalid: `pack.api: 1 not supported (this agentstacks speaks 0)` |
| `pack.description` | required, not empty or whitespace-only | `pack.description: empty` |
| `requires.packs` | names; no duplicates; not itself (each refusal at the entry's index) | `requires.packs[0]: "Core" is not a valid name` / `requires.packs[1]: "core" listed twice` / `requires.packs[0]: a pack cannot require itself` |
| `requires.airsl` | semver range (§4.3); required when the pack installs any `.lua` file (root, skill or `native/<tool>/`; excluded ones do not count; checked at pack load) | `requires.airsl: "nope" is not a version range` / `requires.airsl: required (the pack contains .lua files)` |
| `requires.binaries` | names; no duplicates (each refusal at the entry's index) | `requires.binaries[0]: "Ripgrep" is not a valid name` / `requires.binaries[1]: "git" listed twice` |
| `render` | each entry a pack path (below), not code (§3.4): refused at `render[<i>]`. Checked at pack load, in this order, one refusal per entry: walked (exists), else not excluded, else installed (`pack.toml` and `native/<tool>/hooks.toml` are walked but never installed); every refusal is at `render[<i>]`. An entry the walk refused (§2.1) is not reported as missing | `render[0]: "/x" must be relative` / `render[0]: "../x" must stay inside the pack` / `render[2]: "x.lua" is code; code is installed verbatim` / `render[0]: "enforcment.json" does not exist` / `render[0]: "x.json" is also excluded` / `render[0]: "pack.toml" is never installed` / `render[0]: "native/claude/hooks.toml" is never installed` |
| pack path (each `render` entry; the path after a hook `run` form, §6.2) | `/`-separated, relative, names a file inside the pack, kept as written; checked in this order | `"" must not be empty` / `"." must name a file inside the pack` / `"/x" must be relative` / `"a\b" must use / as the separator` / `"../x" must stay inside the pack` (any `..` segment) / `"./x" must not have a . segment` (also `"a/./b"`) / `"dir/" must not end with /` / `"a//b" must not have an empty segment` |
| `exclude` | valid globs; relative (no leading `/`); refused at `exclude[<i>]`. `*` and `?` stay inside one path segment and never match `/`; only `**` crosses segments (`*.md` matches `README.md`, not `skills/a/SKILL.md`; `**/*.md` matches both). May not match `pack.toml`, a `SKILL.md`, an agent file or an own-pack hook `run` target: refused at `exclude[<i>]`, `<i>` being the index of the pattern, once per pattern and file. A glob that matches a `render` file reports only the `render` refusal above | `exclude[0]: "[" is not a valid glob` / `exclude[0]: "/tmp/**" must be relative` / `exclude[0]: "**/*.md" excludes skills/discuss/SKILL.md` |
| `options.<n>` | `<n>` follows the option rule (§4.1) | `options.archive-keep: "archive-keep" is not a valid option name` |
| `options.<n>.type` | required; `boolean`, `integer` (i64), `string` | `options.keep.type: "number" (expected boolean, integer, string)` |
| `options.<n>.default` | optional; matches `type` | `options.archive_keep.default: expects integer, got string` |
| `options.<n>.description` | required, not empty | `options.archive_keep.description: empty` |

Excluded files are not scanned and not installed; the tree hash still covers them (§8.1).

### 4.3 `airsl` range

The core intersects every pack's `requires.airsl` (Cargo-flavour ranges, `semver` crate). Each
comparator becomes a version interval; the 8 operators of §1.1 are supported, and `*` (no
comparators) is unbounded. `Op` is `#[non_exhaustive]` (§1.1), so an operator added by a later
`semver` release is refused at `pack.toml` parse, naming the range:
`requires.airsl: ">=0.1" uses an operator this agentstacks does not support`. No
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
| file present; an `extension.toml` the walk refused is not reported as absent | `<dir>: not an extension (no extension.toml)` |
| `extension.toml` is valid UTF-8 | `<dir>/extension.toml: not valid UTF-8` |
| `name` §4.1; `version` semver; `description` not empty | same shapes as §4.2, prefixed `extension.` |
| `api` = 0; refused here, at load, only: a refused extension fails to load and the set check never sees it (§9) | `extension.api: 1 not supported (this agentstacks speaks 0)` |
| `entry` relative, no `..` (checked first), ends in `.lua`, not empty, an existing file. Existence is checked on the entry with its leading `./` segments removed (`./adapter.lua` loads as `adapter.lua`), even when another field of the manifest is refused; the message names the entry as written. An entry the walk refused is not reported as missing; an entry under a dot directory is not walked, so it does not exist | `extension.entry: "../a.lua" must stay inside the extension` (same message for an absolute `entry`: `extension.entry: "/a.lua" must stay inside the extension`) / `extension.entry: "adapter.js" is not a .lua file` / `extension.entry: empty` / `extension.entry: "adapter.lua" does not exist` / `extension.entry: "./lib/adapter.lua" does not exist` |
| unknown key, at either level | `extension.toml: extension.foo: unknown key` / `extension.toml: foo: unknown key` |
| no symlinks; non-UTF-8 paths refused; dot entries ignored | as §2.1 |

What the adapter exports and how it is run belong to `extension-host-and-hooks`.

## 5. Skill and agent frontmatter

YAML between `---` lines at the top of `SKILL.md` or `agents/<name>.md`; strict keys; parsed
before token scanning (§3.4). The YAML is deserialised with serde-saphyr into an own value enum
(`deserialize_any`, §1.1), never straight into a struct; keys and types are then checked by hand so
every field error in the file is reported, not only the first. YAML 1.1 booleans apply (§1.1): an
unquoted `argument_hint: yes` is a boolean and refused as `argument_hint: expected a string, got
boolean`.

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
| the file is valid UTF-8 | `<file>: not valid UTF-8` |
| frontmatter missing, not first, or unterminated | `<file>: missing frontmatter` |
| YAML syntax error, including a duplicate key (§1.1) | `<file>:<line>: <parser message>`, where `<parser message>` is `DefaultMessageFormatter.format_message` and `<line>` is `location().line()` plus 1 (the opening `---` line), so it counts from the file; `<file>: <parser message>` when the parser gives no location. No column is reported (§1.1). Stops this file only, and its body is not scanned |
| an integer above `i64::MAX` anywhere in the block (agentstacks' own wording, a syntax refusal like the row above) | `<file>:<line>: integer beyond i64` |
| frontmatter is a mapping (empty frontmatter parses as null, §1.1) | `<file>: frontmatter: expected a mapping, got null` |
| `name`, `description` required, not empty; `name` literal | `<file>: name: required` / `<file>: description: required` / `<file>: name: empty` / `<file>: description: empty` / `<file>: name must be a literal` |
| `name` = directory (skill) or file stem (agent) | `<file>: name "x" does not match "y"` |
| unknown key | `<file>: unknown key "disable-model-invocation"` |
| `invocation` ∈ {user, any}, default `any` | `<file>: invocation: "manual" (expected user, any)` |
| `argument_hint` string | `<file>: argument_hint: expected a string, got <type>` |
| `model` ∈ {fast, balanced, deep}, optional | `<file>: model: "sonnet" (expected fast, balanced, deep)` |
| `effort` ∈ {low, medium, high}, optional | `<file>: effort: "max" (expected low, medium, high)` |
| `tools` an array ⊆ {read, write, edit, grep, glob, shell, skill}, no duplicates, optional. An unknown tool, a repeated tool (each later occurrence) and an item of the wrong type are refused at the item's index `tools[<i>]` | `<file>: tools[1]: unknown "bash"` / `<file>: tools[2]: "read" listed twice` / `<file>: tools: expected array, got string` / `<file>: tools[0]: expected string, got integer` |
| `native` keys follow §4.1, an invalid key refused at `native.<key>`; contents opaque (never scanned for tokens), passed to that extension only | `<file>: native.Claude: "Claude" is not a valid tool name` |

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
| `match` only where §6.1 allows; values from that set; not empty; no duplicates; omitted = all. Whole-list refusals at `match`, one value's refusal at `match[<i>]`. With `event` missing or unknown, `match` and `agents` are not checked | `hooks[1].match: not allowed on session.end` / `hooks[1].match[0]: "bash" not valid for tool.before` / `hooks[1].match: empty list never fires` / `hooks[1].match[1]: "read" listed twice` / `hooks[0].match[0]: expected string, got integer` |
| `agents` only on `agent.stop`; not empty; each a valid name (at `agents[<i>]`); this pack's agents (checked at pack load, at `agents[<i>]`; an agent file the walk refused is not reported); omitted = any agent | `hooks[0].agents: only allowed on agent.stop` / `hooks[0].agents: empty list never fires` / `hooks[0].agents[0]: "Coder" is not a valid name` / `hooks[3].agents[1]: pack "core" has no agent "task-briefer"` |
| `message`: `text` required, not empty; `run`, `args`, `grants`, `may_block` refused | `hooks[0].text: required` / `hooks[0].text: empty` / `hooks[0].run: not allowed on a message hook` (same shape for `args`, `grants`) / `hooks[0].may_block: message hooks cannot block` |
| `lua`: `text` refused | `hooks[1].text: not allowed on a lua hook` |
| `lua`: `run` required, ends in `.lua`, written pack-relative (`hooks/x.lua`), `{{root}}/…` or `{{root <pack>}}/…`, where the path after the form is a pack path (§4.2: no leading `/`, no `..`, `.` or empty segment); nothing else (absolute paths, `{{skill_dir …}}`, `{{var …}}`, a token anywhere but the start). A token refusal in `run` is reported without a form refusal on top | `hooks[1].run: required` / `hooks[1].run: "x.sh" is not a .lua file` / `hooks[1].run: must be pack-relative, {{root}}/… or {{root <pack>}}/…` / `hooks[1].run: "Core" is not a valid name` |
| own-pack `run` (pack-relative, `{{root}}/…`, or `{{root <this pack>}}/…`), in `pack.toml` or a `native/<tool>/hooks.toml`: an existing, non-excluded file among the `{{root}}` files (§2: not `pack.toml`, not under `skills/`, `agents/` or `native/`), checked at pack load. A `run` file the walk refused is not reported as missing | `hooks[1].run: "hooks/missing.lua" does not exist` / `<pack>/native/claude/hooks.toml: hooks[0].run: "hooks/missing.lua" does not exist` |
| other-pack `run` (`{{root <pack>}}/…`): `<pack>` is required (§3.3) and the file exists and is not excluded there, checked by the set check (§9) | `hooks[1].run: "{{root core}}/hooks/enforc.lua" does not exist in core` |
| `grants.policy` ∈ {confined, pure}, default confined; `trusted` gets its own message | `hooks[1].grants.policy: "trusted" is not allowed for pack hooks` / `hooks[1].grants.policy: "foo" (expected confined, pure)` |
| grant paths (`read`, `write`) absolute or starting with `{{root…}}`, `{{skill_dir …}}` or `{{var …}}`; refused at the entry's index; a path with a token refusal gets no path refusal on top | `hooks[1].grants.read[0]: "data" must be absolute or start with a token` / `hooks[1].grants.write[0]: "data" must be absolute or start with a token` |
| `grants.env` names `^[A-Z_][A-Z0-9_]*$`, no duplicates; at the entry's index | `hooks[1].grants.env[0]: "home" is not an environment variable name` / `hooks[1].grants.env[1]: "HOME" listed twice` |
| `grants.exec` ⊆ `requires.binaries` ∪ {airsl}, no duplicates; at the entry's index | `hooks[1].grants.exec[0]: "git" is not in requires.binaries` / `hooks[1].grants.exec[1]: "airsl" listed twice` |
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
| at least one `[tools.*]` and one `[packs.*]`. A section that is missing or empty counts as missing. A section of the wrong type gets only its wrong-type refusal, which does not suppress `nothing to install` for the other section when that one is missing or empty (`tools = 1` with no `[packs.*]` gives both refusals; `tools = 1` with `packs = "x"` gives the two wrong-type refusals only) | `agentstacks.toml: nothing to install` / `agentstacks.toml: tools: expected table, got integer` |
| table keys follow §4.1 | `packs.Core: not a valid name` |
| exactly one of `git` / `path` | `packs.core: needs exactly one of git, path` |
| `git` needs exactly one of `tag` / `rev`; `rev` 40 or 64 lowercase hex; no `branch` | `packs.core: needs exactly one of tag, rev` / `packs.core.rev: not a commit id` |
| a `git` URL starting with `-` | `packs.core.git: URL must not start with "-"` |
| `subdir` is a repository-relative path (`SubPath`): relative, no `..` segment at all, no leading `/`; `.` and empty segments are allowed (`./packs//core` is `packs/core`) | `packs.core.subdir: must stay inside the repository` |
| `tag`, `rev`, `subdir` only on `git` sources; an entry with both `git` and `path` gets only the `git, path` refusal (plus any malformed field values), not this one | `packs.team-lint: tag, rev and subdir apply only to git sources` |
| "resolving inside" for `path` is decided lexically on the declared string; symlinks are resolved by `core-plan-and-apply` | — |
| `path` is a project-relative path (`LocalPath`): relative to the directory holding `agentstacks.toml`, its `..` segments never climbing above it (`a/../b` is accepted). The project directory itself is accepted (`.`, `./`, `packs/..`) | `packs.team-lint.path: absolute paths are not portable` / `packs.team-lint.path: must stay inside the project` |
| `options` only under `[packs.*]`; checked per §4.4 | `tools.claude.options: unknown key` |
| any string empty or whitespace-only | `packs.core.tag: empty` |
| unknown key | `packs.core.branch: unknown key` |
| key = fetched `pack.name` / `extension.name` | `packs.core: source contains pack "kore"` / `tools.claude: source contains extension "claud"` |
| every `requires.packs` entry is listed | `sdlc requires pack "core", which agentstacks.toml does not list (add it: --pack core)` |
| `requires.packs` has no cycle | `requires cycle: a → b → a` |

Two path kinds, two types, never interchanged: a **repository-relative** path (`SubPath`:
`subdir` here and in the lock, and the lock's `[[file]].path`, which names a written file below
the project root the same way) only descends and never holds a `..` segment; a
**project-relative** path (`LocalPath`: a pack or tool `path`, here and in the lock) may climb with
`..` as long as it never leaves the project. Both are decided lexically on the declared string;
the file system is not read.

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
path = ".claude/skills/discuss/SKILL.md"
hash = "sha256:<64 hex>"
by   = "core"

[[merge]]
file = ".claude/settings.json"
key  = "<locator>"
hash = "sha256:<64 hex>"
by   = "core"
```

| Rule | Refusal |
|---|---|
| `api` required, `0` | `agentstacks.lock: api 1 not supported (this agentstacks speaks 0)` |
| `[airsl]` `version` exact semver | `agentstacks.lock: airsl.version: "0.1" is not a version` |
| `[airsl]` present iff a locked pack declares `requires.airsl`. Deciding it needs the loaded packs, so this chain provides `check_airsl_presence` and `core-plan-and-apply` calls it (§13) | `agentstacks.lock: [airsl] missing, but locked packs require airsl` / `agentstacks.lock: [airsl] present, but no locked pack requires airsl` |
| `[[tool]]`/`[[pack]]`: `name`, `version`, `tree` required; exactly one of `git`/`path`; git entries need `rev` and carry `tag` only when `agentstacks.toml` pins a tag (a missing or extra `tag` makes the lock stale, not malformed); `tag`, `rev`, `subdir` refused on path entries; the other §7 source rules apply | `agentstacks.lock: pack[1].rev: required` / `agentstacks.lock: pack[1].tree: required` / `agentstacks.lock: pack[1]: needs exactly one of git, path` / `agentstacks.lock: pack[2]: tag, rev and subdir apply only to git sources` |
| `[[file]]`: `path`, `hash`, `by` required; `path` a repository-relative path (§7: relative, `/`-separated, no `..` segment anywhere); unique, compared with empty and `.` segments removed, so `./a` and `a` are one path, as are `a//b` and `a/b` (each later matching entry is refused at its own index `file[<i>]`, and the message names the path as that entry writes it) | `agentstacks.lock: file[0].path: "/x" must be relative` / `agentstacks.lock: file[0].path: "../x" must stay inside the project` / `agentstacks.lock: file[0].path: "a/../b" must stay inside the project` / `agentstacks.lock: file[0].path: empty` / `agentstacks.lock: file[1]: file ".claude/x" listed twice` / `agentstacks.lock: file[1]: file "a" listed twice` |
| `[[merge]]`: `file`, `key` (non-empty), `hash`, `by` required; unique by (`file`, `key`), each later repeat refused at its own index | `agentstacks.lock: merge[0].key: empty` / `agentstacks.lock: merge[1]: merge ".claude/settings.json" "<key>" listed twice` |
| hashes `^sha256:[0-9a-f]{64}$` | `agentstacks.lock: file[0].hash: bad hash (expected sha256:<64 lowercase hex>)` |
| `by` names a locked pack or tool | `agentstacks.lock: file[0].by: "kore" is not a locked pack or tool` (same shape for `merge[i].by`) |
| tool names unique; pack names unique; each later repeat refused at its own index | `agentstacks.lock: pack[1]: pack "core" listed twice` / `agentstacks.lock: tool[1]: tool "claude" listed twice` |
| entry `name` §4.1, `version` semver; `[[merge]].file` a non-empty string | §4.2 shapes with the entry key path: `agentstacks.lock: pack[1].name: "Core" is not a valid name` / `agentstacks.lock: pack[1].version: "1.2" is not semver` |
| syntax error | `agentstacks.lock:<line>: <parser message>` (recovery: delete the lock and re-plan) |
| unknown key | `agentstacks.lock: pack[1].branch: unknown key` |
| written sorted: tools and packs by name, files by path, merges by (file, key); fields in the order shown above, the comment line after `api = 0` | (writer rule) The writer formats the text itself: a serialised `toml::Table` would sort fields alphabetically (§1.1) |

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

After each file parses, `set::check_pack_set` validates the project as a whole and returns a
`ValidSet`, the only input later stages accept. Packs or extensions that failed to load are
skipped, so their failures produce no follow-on errors.

```text
check(project_file: &Path, project: ProjectFile,
      load_pack: impl FnMut(&PackName, &PackEntry) -> Result<Pack, Diagnostics>,
      load_tool: impl FnMut(&ToolName, &Source) -> Result<Extension, Diagnostics>) -> Result<ValidSet, Diagnostics>
```

The error is the `Diagnostics` collection of §10; `into_sorted()` gives every refusal in output
order. The check calls `load_pack` once for every `[packs.*]` key and `load_tool` once for every
`[tools.*]` key, with that key's entry, so no listed key can lack a load result and no unlisted
key can have one. A failed load keeps its own refusals and the other loads still run.

1. every `requires.packs` entry is listed in `agentstacks.toml`; no cycles (§7);
2. every `skill`/`agent`/`skill_dir`/`root` token that names **another** pack targets a required
   pack, and the item exists there (§3.3); own-pack targets are checked at pack load and not
   re-checked here;
3. every `option` token names a declared option of its pack (`pack "core" has no option
   "archive_kep"`, at the token's location); option values resolve (§4.4);
4. every other-pack hook `run` (`{{root <pack>}}/…`) resolves in that required pack (§6.2;
   own-pack `run` and `agents` are checked at pack load);
5. the `airsl` ranges intersect (§4.3);
6. no two packs ship a skill with the same name, and no two packs ship an agent with the same name:
   `skill "status" is shipped by both sdlc and team-lint`. A skill and an agent may share a name.
   Agents use the same shape (`agent "coder" is shipped by both core and team-lint`). Three or
   more packs give one diagnostic per name, packs in name order: `skill "status" is shipped by
   sdlc, team-lint and x`.

Message shapes for several packs: the disjoint-range refusal lists every pack that declares
`requires.airsl`, in name order, each with its range as written (`airsl: core needs >=0.2, sdlc
needs <0.2`); the installed-version refusal joins the ranges with `, ` in the same order
(`airsl 0.1.3 installed, packs need >=0.1.4, <0.3`).

## 10. Errors

- Every refusal carries a location: `<file>:<line>: <what>` for syntax errors and for tokens in
  Markdown and `render` files; `<file>: <key.path>: <what>` for TOML and frontmatter field errors
  and for tokens in TOML fields and frontmatter values (key paths: `hooks[2].grants.read[0]`,
  `description`,
  `options.archive_keep.default`, `pack[1].rev`). Messages name the offending value and, where a
  set is fixed, the allowed set.
- Key paths of list elements: a refusal about one element of a TOML list carries the element's
  index: `render[<i>]` (pack-path, code, missing, excluded and never-installed refusals),
  `exclude[<i>]` (glob and excludes-protected-file refusals), `requires.packs[<i>]`, `requires.binaries[<i>]`, `hooks[<n>].match[<i>]`,
  `hooks[<n>].agents[<i>]` (name and "has no agent" refusals), `hooks[<n>].args[<i>]`,
  `hooks[<n>].grants.read|write|env|exec[<i>]`, and a wrong-typed item of any string list
  (`hooks[0].match[0]: expected string, got integer`). A refusal about the whole list stays on the
  list key (`hooks[1].match: empty list never fires`, `hooks[1].match: not allowed on
  session.end`), e.g. `hooks[2].agents[1]: pack "core" has no agent "codr"` is indexed. In
  frontmatter, every `tools` refusal about one item (unknown, repeated, wrong type) is at
  `tools[<i>]`, and an invalid `native` key is at `native.<key>`.
- Set-level refusals (§9) about **one token or one hook field** keep that token's or field's own
  location: `<file>:<line>` for a token in Markdown or a `render` file, `<pack>/pack.toml: <key.path>`
  for a token or `run` in a hook (the §3.3 and §6.2 messages). Every other set-level refusal is
  located at `agentstacks.toml`, with the key path of the entry it concerns when there is one:
  `agentstacks.toml: packs.sdlc: sdlc requires pack "core", …`; option resolution (§4.4) at
  `agentstacks.toml: packs.core.options.archive_keep: …`, or `agentstacks.toml: packs.core: …` for a
  required option that is not set. A refusal spanning several packs (cycle, airsl range, name
  clash) carries the file only.
- Generic shapes, used by every field not given a message elsewhere: missing required key
  `<key.path>: required`; wrong type `<key.path>: expected <type>, got <type>`; empty or
  whitespace-only string `<key.path>: empty`; unknown key `<key.path>: unknown key`.
- "Exact message" in tests means the `Diagnostic`'s `Display`: location plus message.
- Type words in `expected <type>, got <type>`: `boolean`, `integer`, `float`, `string`,
  `datetime`, `array`, `table`, and `null` (YAML only). YAML uses the same words: a sequence is
  `array`, a mapping is `table`.
- An I/O failure is `<path>: <the std::io::Error Display>`.
- A text file that is read but never rendered (`pack.toml`, `SKILL.md`, an agent `.md`,
  `native/<tool>/hooks.toml`, `extension.toml`) and is not UTF-8: `<file>: not valid UTF-8`. A
  rendered file keeps `<file>: not valid UTF-8, cannot render` (§3.4).
- Sort order: by file path as a `/`-separated string, byte order; within a file, key-path and
  no-line diagnostics first in the order they were found (stable sort), then by line.
- **All errors per run**, sorted by file; within a file, key-path errors (no line) first, then by
  line. A TOML or YAML syntax error stops only that file.
- **A refusal never stops loading** (settled rule). A walk refusal (symlink, special file,
  non-UTF-8 path, I/O failure) does not stop a pack or extension load: the walk returns every
  walkable file with every refusal, and the loader checks the walkable files. A refused
  `pack.toml` or `extension.toml` does not stop it either: the loader keeps every check that does
  not need the refused part. For a pack with a refused `pack.toml` that is the layout rules
  (§2.1), the frontmatter and Markdown bodies of every `SKILL.md` and agent file (which `exclude`
  can never remove) and the `native/` reach rule (§2.2); everything that needs the manifest
  (native hooks, `exclude`, `render`, hook targets, the airsl rule, nothing to install, other
  rendered files, the pack-local token rules) is skipped, because without `exclude` it would
  report on files the manifest may exclude. For an extension, `entry` existence is still checked
  when `entry` itself parsed. A refused frontmatter skips its own body. Only a directory with no
  `pack.toml` at all ends a pack load early: `not a pack` plus the walk's refusals.
- **No knock-on refusals** (settled rule). A refusal never causes a second one about the same
  thing; these are suppressed:

  | Suppressed refusal | When |
  |---|---|
  | `not a pack (no pack.toml)` / `not an extension (no extension.toml)` | the walk refused the manifest (e.g. it is a symlink) or a directory above it, including a root that could not be listed |
  | `missing SKILL.md` | the walk refused that `SKILL.md` |
  | `"…" does not exist` (a `render` entry, an own-pack hook `run`, `extension.entry`) | the walk refused that file or a directory above it |
  | `pack "<p>" has no skill "…"` / `… has no agent "…"` (tokens, hook `agents`) | the walk refused that item |
  | `pack declares nothing to install` | the walk refused anything under `skills/`, `agents/` or `native/` (a non-UTF-8 name included) or the root itself, a skill, agent or tool directory name was refused (§2.1, §2.2), or a `native/<tool>/hooks.toml` failed to read or parse |
  | the body of a skill or agent file | its frontmatter was refused |
- Every TOML file is parsed into a raw `toml::Table`, then validated by hand; never a
  `deny_unknown_fields` struct parse, which stops at the first error (§1.1). Syntax errors (which
  include duplicate keys, duplicate tables and integers beyond i64, §1.1) take their line from the
  error span. Empty TOML is an empty table, so it is refused field by field (`api: required`). A
  value of a TOML type no field accepts (datetime, float) gets the generic wrong-type message.
- One `thiserror` enum per module, no string errors, so `cli-commands` can map them to exit codes.
  A validation run returns a list of `Diagnostic { location, error }`.

## 11. Architecture

Settled with the author on 2026-11-21, applying the Rust guidelines' architecture rules:
`modularity.md` Rule 1 (a module mixing concept clusters is split into folder modules; cohesion,
not item count, decides a file), `mod-rs-export-only.md` (each module states why it exists without
an "and"; `mod.rs` re-exports flat), and `static-dispatch.md` (behaviour with a production and a
test implementation is injected through a generic trait bound, not `Box<dyn Trait>`).

**`types/` holds the domain types and all the logic, and touches nothing outside the process.** The
only way logic reaches the disk is the `FileTree` port, so every rule is testable with an in-memory
fake. **`fs/` is the adapter**: the one place that calls `std::fs`.

```text
crates/agentstacks/src/
├─ types/                types + logic + ports; no std::fs, no network, no process
│  ├─ api.rs             API: the one supported api number                                    (§4, §4.5, §7, §8)
│  ├─ report/            Location, TypeWord (word), FieldError (field), FormatError (format),
│  │                     Diagnostic, Diagnostics                                               (§10)
│  ├─ name/              PackName, ToolName, ItemName, OptionName, BinaryName, EnvName, the
│  │                     kebab rule; invalid.rs: InvalidName, InvalidOptionName, InvalidEnvName (§4.1)
│  ├─ tokens/            Token, TokenKind, Var, ItemRef (reference), TokenTarget, TokenError,
│  │                     TextOrigin (origin), names_own_pack (own), LocatedToken, TokenValues,
│  │                     scan, render                                                          (§3)
│  ├─ format/            YamlValue (yaml, public); document, fields (crate-private: they hand
│  │                     out toml types)                                                       (§1.1, §10)
│  ├─ frontmatter/       Frontmatter (block), BodyStart (body), FrontmatterError, SkillMeta
│  │                     (skill), agent/{meta,tool}: AgentMeta, AgentTool; Invocation,
│  │                     ModelTier (model), Effort, field readers, wording, syntax              (§5)
│  ├─ manifest/          pack/{file,path,version}: PackManifest, PackPath, PackVersion;
│  │                     option/{decl,kind,value}: OptionDecl, OptionType, OptionValue;
│  │                     Requires, AirslReq (airsl), ExcludeGlob (exclude), ManifestError, invalid (§4)
│  ├─ hooks/             Hook, HookEvent, HookAction, HookRun, LuaHook, MessageHook, MatchValue
│  │                     (matcher), Grants, Policy, native/{hook,file}: NativeHook, NativeHooks,
│  │                     invalid                                                               (§6, §2.2)
│  ├─ tree/              walk/{run,outcome,error}: walk, Walked, WalkError; hash/{tree,content,
│  │                     error}: tree_hash, ContentHash, ContentHashError; TreePath (path)      (§2.1, §8.1)
│  ├─ port/              FileTree (tree), EntryKind (kind); FakeFileTree (fake, #[cfg(test)])   (this section)
│  ├─ project/           ProjectFile (file), PackEntry (entry), source/{kind,fields,error}: Source,
│  │                     SourceFields, SourceError; pin/{kind,rule}: Pin, PinRule; git/{url,tag,
│  │                     commit}: GitUrl, GitTag, CommitId; path/{local,sub}: LocalPath, SubPath;
│  │                     ProjectError, invalid                                                 (§7)
│  ├─ pack/              Pack (load), PackLayout (layout), install/{checks,set}: InstallSet and the
│  │                     install checks; HookTargets (targets), item/{loaded,kind}: PackSkill,
│  │                     PackAgent, ItemKind; NativeDir (native), PackError, the body scan (scan)
│  │                     and pack-local token rules (local)                                    (§2, §3)
│  ├─ extension/         Extension (loaded), ExtensionManifest (manifest), EntryPath (entry),
│  │                     ExtensionError                                                        (§4.5)
│  ├─ lock/              Lockfile (file), LockContents, entry/{locked,written}: LockedEntry,
│  │                     LockedSource, FileEntry, MergeEntry; Owner, MergeFile/MergeKey (merge),
│  │                     LockError, write_lock (write), check_airsl_presence (airsl)            (§8)
│  └─ set/               check_pack_set (check) → ValidSet (valid); the cross-pack checks: graph,
│                        targets, runs, clash, options, airsl, interval, fetched; SetError      (§9)
├─ fs/                   StdFileTree (disk.rs): FileTree over std::fs (the only std::fs caller)
└─ validation.rs plan.rs executor.rs host.rs   unchanged seams for later chains
```

Rules:

- `types/mod.rs` declares each group `pub mod` and re-exports every public item flat
  (`agentstacks::types::PackName`); each group's `mod.rs` is export-only with the four-question doc.
- Within a group, items keep one file each as built; new code puts a concept that is always read
  together in one file (`modularity.md`: "do not fragment a concept into five one-line files").
- **Every file and folder name is one word, never with `_`, and never repeats its folder's word**
  (author rule). Files that share a word go into a subfolder named by that word, with the word
  dropped (`manifest/option/{decl,kind,value}.rs`); small related items may share one file
  (`name/invalid.rs`). Type names stay descriptive (`OptionDecl`); only file paths are short.
- Error enums live in the group that produces them (`tokens/error.rs`, `manifest/error.rs`,
  `pack/error.rs`, …); `report::FormatError` joins them with `#[from]` and also carries
  `std::io::Error` (`FormatError::Io`) and `WalkError`.
- Readers that take text (`&str`) and return types stay in `types/`: they are pure logic.
- **The public API exposes no `toml`, `semver`, `globset` or `serde_saphyr` type.** Own types
  stand in: `PackVersion` (a parsed semver version, also used for an extension's and the lock's
  airsl version), `OptionValue` (boolean, `i64` or string), `AirslReq` (its `semver::VersionReq`
  accessor is crate-private), `ExcludeGlob` (its matcher is private) and `YamlValue`; the TOML
  readers in `format/` are `pub(crate)`.
- `FileTree` (in `types/port/`), rooted at one directory, paths relative to that root, an empty
  path naming the root: `root(&self) -> &Path` (for messages, which keep their `<root>/<rel>`
  form); `kind(&self, rel) -> io::Result<EntryKind>` without following a symlink,
  `EntryKind::{File, Dir, Symlink, Other}`; `read_dir(&self, rel) -> io::Result<Vec<OsString>>`
  (entry names, any order); `read_file(&self, rel) -> io::Result<Vec<u8>>`; and a provided
  `read_text(&self, rel) -> io::Result<String>` over `read_file`, failing with
  `io::ErrorKind::InvalidData` displayed as `not valid UTF-8`, so every reader of a text words it
  the same way (§10). Read-only: writing the lock or any file belongs to `executor`
  (`core-plan-and-apply`). Logic takes it as a generic bound.
- `walk<T: FileTree>(tree: &T) -> (Walked, Diagnostics)` returns every walkable file (`Walked`
  also lists the ignored dot entries and the refused entries) together with every refusal: a
  refusal never stops the walk, so a caller keeps checking what was walkable (§10).
  `tree_hash<T: FileTree>(tree: &T) -> Result<ContentHash, Diagnostics>` reads every walkable file
  and reports every walk refusal and unreadable file of the run.
- `fs::StdFileTree::new(root)` implements it with `symlink_metadata`, `read_dir` and `read`. It
  reads only below its root: a path that is absolute or holds a `..`, root or prefix component is
  refused with `io::ErrorKind::InvalidInput` before any disk access.
- **The no-I/O rule is guarded by a test**: `tests/isolation.rs` reads every `.rs` file
  under `src/types` (recursively) and fails when code (not a comment) names `std::fs`, `std::process`,
  `std::net` or `File::open`.
- The former top-level `model/` and `lock/` are dissolved into `types/` as listed. `lib.rs`
  declares `executor`, `host`, `fs`, `plan`, `types` and `validation` (opened this
  session: `crates/agentstacks/src/lib.rs`).

```text
agentstacks.toml ─► ProjectFile ─────────┐
pack dir ─► FileTree ─► Pack::load ──────┤
ext dir ──► FileTree ─► Extension::load ─┼─► set::check_pack_set ─► ValidSet ─► (extension host, plan)
agentstacks.lock ─► Lockfile ────────────┘
              fs::StdFileTree in production · FakeFileTree in tests
```

Dependencies (latest stable per `cargo search <name> --limit 1` this session): `serde` 1 (derive),
`toml` 1 (`1.2.1+spec-1.1.0`; probed at `1.2.0`, §1.1), `serde-saphyr` 2 (probed at `2.1.1`, §1.1; claudestacks uses `{ version = "2.0",
default-features = false, features = ["deserialize"] }`, chosen there because it parses YAML in
safe Rust — Proof: `../claudestacks/Cargo.toml` `[workspace.dependencies]`; `3.0.0-rc.1` is a
pre-release, not used), `sha2` 0.11 (`0.11.1`), `semver` 1 (`1.0.28`), `globset` 0.4 (`0.4.21`),
`thiserror` 2 (`2.0.21`); dev-only `tempfile` 3 (`3.27.0`) for the `fs/` adapter's real-disk
tests. Licences are checked by `cargo make deny` when added.

## 12. Testing

- **Unit tests** colocated in every logic-bearing file, covering each rule in §2–§10 with at least
  one breaking case, including every checked example in §3.2, §4.1 and §8.1.
- **Logic tests use the in-memory fake.** `FakeFileTree` in `types/port/fake.rs`
  (`#[cfg(test)]`, crate-visible) builds trees with files, directories, symlinks, special files,
  non-UTF-8 names, unreadable files, unlistable directories and uninspectable entries; `walk`,
  `tree_hash`, the pack layout, pack load and extension load are tested against it, with no temp
  directories.
- **The adapter is tested on the real disk**: `fs/` tests (with `tempfile`) prove `StdFileTree`
  reports a symlink, a special file (unix socket), a non-UTF-8 name (Linux) and an unreadable file
  as the fake does, refuses a path that could leave its root, and gives the same tree hash as the
  §8.1 example.
- **Fixtures** under `crates/agentstacks/tests/fixtures/`, loaded through the public API over the
  real disk. Valid packs, one per feature (`valid-hooks`, `valid-native-hooks`, `valid-options`,
  `valid-native-files`, `valid-render-exclude`, `valid-tokens`, `valid-shared-name`), and valid
  extensions (`minimal`, `entry-current-dir`). **One invalid input per pack refusal**, each valid
  except for the one refusal its name says: the tables in `tests/pack.rs`
  (`load_refuses_each_fixture`, `SPEC_ROW_CASES`) assert each one's exact location and message,
  and `many-errors` asserts that refusals from several files come back in one run. The invalid
  extensions are tabled the same way in `tests/extension.rs`.
- **Doctests on public items**: each public type and function carries an example that runs
  (accepting and refusing where it refuses).
- **The no-I/O guard** `tests/isolation.rs` (§11).
- **Coverage rule:** every refusal in this spec has a test; the plan maps each one.
- `cargo make dod` green. Converting the real plugins is not tested here (`plugins-to-packs`).

## 13. Handed to other chains

| Chain | Receives |
|---|---|
| `core-plan-and-apply` | source resolution with `git` (URLs after `--`; tree listed with `ls-tree`, blobs read with `cat-file`, never a working-tree checkout); a **delete** operation for files of a removed pack; the meaning of `[[merge]].key`; stale-lock reconciliation (including a lock `tag` that disagrees with `agentstacks.toml`); tree mismatches (§8.2); `rev` over a moved tag; calling `check_airsl_presence` with the locked packs (§8) |
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
