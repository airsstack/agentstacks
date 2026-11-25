---
status: approved
created: 2026-11-25
built-on: 932d121a5442
---

# Spec: The official Muse Code extension

This spec adds the official `musecode` tool extension, which renders the same packs into native Muse
Code project config through the public `api = 0` surface the `claude` and `opencode` extensions
use. Muse Code reads hooks in Claude Code's format, so the extension is the claude extension's
shape with Muse Code's paths, tool names and limits. The core change is one constant. A
`musecode-check` task and CI job run the extension inside the pinned real Muse Code against a local
mock model.

Proof convention:

- `probes/<file>` is a file in this chain directory holding raw probe output. Short forms:
  `I§n` = `probes/initial-results.md` item n; `H-Pn` = `probes/hooks-results.md` section Pn;
  `B-Bn` = `probes/items-results.md` section Bn; `F-Rn` = `probes/final-results.md` section Rn;
  `C1.n`, `C2` = `probes/release-results.md`; `C3.n` = `probes/inventory-results.md`;
  `An` = `probes/agentstacks-results.md` section An; `T` = `probes/translate-results.md`. Raw hook
  dumps of the first runs are in `probes/raw/`.
- The scripts behind `F-R1`-`F-R11` (`probes/harness/final-run.sh`, `final2-run.sh`, `msg-run.sh`,
  `events-run.sh`, `gen.lua`) re-run as saved. Of the `H-P` and `B-B` runs only the raw output and
  some helpers are kept (`hooks-results.md:4` names a `run.sh` that was overwritten); each claim
  this spec rests on from them is either re-proven by an `F-R` run or quoted from their raw output.
- Every Muse Code run used the binary `muse-bin-1.4.25-R9361.1` (`Muse Code 1.4.25
  (1.4.25-R9361.1)`), called directly, with `HOME` and `TMPDIR` in a scratch directory, launched
  from the project root, and with the mock model `probes/harness/mock.mjs` or `--provider echo`.
- `probes/docs/<file>:<line>` is Muse Code documentation fetched on 2026-11-25: `user-*.txt` from
  `dev.meta.ai/docs/muse-code`, `guides_*.txt` from `meta-models.github.io/muse-code-sdk/1.4`
  (written against 1.3.0; a probe of 1.4.25 overrides it), `changelog.txt`, `install.sh`,
  `launcher.sh`. Every behaviour the spec relies on is proven by a 1.4.25 run; a doc line is cited
  only beside such a run or for a fact no run can reach (named where used).
- `src/…` is `crates/agentstacks/src/…` at commit `33e3925`. `claude/lib/<m>` is
  `extensions/claude/lib/<m>.lua` at the same commit.
- `I§8` counted `core-coder` with `tools: [Read, Edit]` as working because it was listed; `B-B3`
  shows such an agent is listed but cannot be spawned. `B-B3` is the proof used here.

## 1. Settled decisions

| # | Decision | Why / proof |
|---|---|---|
| D1 | Target Muse Code **1.4.25-R9361.1**, pinned for checks | intent; current stable on 2026-11-25 (`C1.1`) |
| D2 | `.muse/hooks.json` is **shared**: declared in `reads`, our handlers marked ` # agentstacks`, the user's own groups kept, rewritten whole with `agentstacks.json.edit` | author, 2026-11-25. Muse Code reads project hooks from that one file in Claude Code's format (`I§1`, `I§5`, `probes/docs/guides_extend_hooks_.txt`) |
| D3 | Skills in `.agents/skills/<p>-<s>/`, agents in `.agents/agents/<p>-<a>.md`, pack root `.muse/agentstacks`, native files copied to `.agents/<rel>` | Muse Code reads project skills and agents there (`I§7`, `I§8`, `B-B3`); `.muse/` holds the hooks file (`I§1`) |
| D4 | Lua hook commands are wrapped: `/bin/sh -c '<quoted argv>' # agentstacks`, with no `cd` | hooks run as `$SHELL -c` with the user's `SHELL` (`H-P1`), so `/bin/sh` fixes the shell; the form built by the §4.4 rule (`probes/harness/gen.lua`) works under zsh, sh and bash (`F-R7`; also `F-R1`); hooks run in the launch directory, which is the workspace root (`I§2`, `H-P10`) |
| D5 | Agent `tools` are written as Muse Code tool names; the model tier is left out; `effort` is passed through | Claude names (`Read`, `Edit`, …) are listed but every spawn fails with `` `<id>` tools: unknown_tool ``; native names spawn (`B-B3`); `effort` `low`/`medium`/`high` are accepted (`B-B3`); a model id that does not exist runs on the lead's model with a notice (`B-B3`), and the neutral tier is not a Muse Code model id |
| D6 | `agent.stop` hooks with an `agents` list are skipped | Muse Code gives hooks no agent name: `subagent_id` is the spawn UUID and a `core-coder` matcher never fires (`B-B4`) |
| D7 | `{{var project_dir}}` is `.` | Muse Code sets no project-directory variable (`I§2`, `B-B5`); the `bash` tool starts in the workspace root (`B-B5`), hooks run there (`I§2`), and `read_file` resolves a relative path against it (`I§4`, `raw/out3`) |
| D8 | The translator makes a relative tool path absolute against the payload `cwd` | Muse Code passes the model's path as given (`raw/out3/PreToolUse.13868`: `"path": "a.txt"`); a relative `event.path` is resolved differently by the two pack hooks that read it, so absolute is the safe contract (`C3.3`) |
| D9 | Every Muse Code run in `musecode-check` sets `TBH_CREDENTIAL_BACKEND=file`, and the check confirms the mock is listening before it starts Muse Code | without it, each run under an isolated `HOME` on macOS shows "Keychain Not Found"; with it, none (`F-R4`). A run whose mock did not bind reached an unknown listener (`F-R4`) |
| D10 | `musecode-check` downloads the raw binary by versioned URL and checks its SHA256 on every run (a file of the wrong size cannot match it); Linux x86_64 and macOS arm64 only | no login and no archive (`C1.2`-`C1.4`); the macOS download is byte-identical to the binary every probe here called directly, without the launcher (`C1.4`, `H-P`, `B-B`, `F-R` runs) |
| D11 | No `musecode`-specific command files: a skill is invoked as `/<p>-<s>` | a command "lives" "Only inside a plugin" (`probes/docs/guides_extend_skills_.txt:404-406`); the TUI expands `/<id> args` itself (`B-B1`) |

## 2. Architecture

Approved by the author on 2026-11-25.

```text
extensions/musecode/                 Lua under airsl · zero grants · reads = [".muse/hooks.json"]
  extension.toml      manifest
  adapter.lua         the only caller of agentstacks.*: values + plan → operations
  lib/values.lua      root, skill/agent names, var forms                    ┐
  lib/frontmatter.lua SKILL.md + agent text (Muse tool names, name rule)    │ pure: data in,
  lib/hooks.lua       pack hook → Muse hook group, or a skip reason          │ text/tables out;
  lib/hookfile.lua    merge our groups into the user's .muse/hooks.json      │ each has its own
  lib/translate.lua   translate.lua text: Muse payload ↔ neutral event       ┘ <name>_test.lua
                          │ proposes
core (Rust)               ▼
  src/types/shorthand.rs   OFFICIAL_TOOLS gains "musecode" (+ colocated test)
  tests/musecode.rs        the shipped extension on the real host (tests/fixtures/musecode/)
crates/agentstacks-cli/tests/musecode-check/   mock.mjs (Responses API) · check.mjs · pack/
Makefile.toml musecode-check task · .github/workflows/ci.yml musecode-check job
book: ch06-04-musecode.md (new), SUMMARY.md, tool lists · README.md · CLAUDE.md · CHANGELOGs
```

**Why each module exists.**

| Module | Its one job | Why it is separate |
|---|---|---|
| `adapter.lua` | turn the `values` and `plan` payloads into operations | the only file allowed to call `agentstacks.*` (`book/src/ch06-00-authoring.md` "What you build") |
| `lib/values.lua` | where Muse Code puts things and what each `{{var}}` stands for | answers the `values` event alone |
| `lib/frontmatter.lua` | the skill and agent file texts | one concept: Muse Code markdown files with frontmatter |
| `lib/hooks.lua` | decide, per pack hook, its Muse Code hook group or its skip reason | the hook rules; no file text |
| `lib/hookfile.lua` | merge our groups into the user's `.muse/hooks.json` | the shared-file merge rules (the claude extension keeps the same rules apart in `claude/lib/settings`) |
| `lib/translate.lua` | the text of `<root>/translate.lua` | one generated file with its own contract (`book/src/ch06-02-hooks.md` §6) |

**Dependency direction.** `adapter → lib/*`. A `lib/` module never calls `agentstacks.*` and never
calls `require`; `adapter.lua` hands it data, as in `extensions/claude/`. The claude extension's
rules are copied and changed, not shared: `agentstacks.require` loads only the extension's own files
(`book/src/ch06-01-extensions.md` §10). The core stays tool-agnostic: the Rust change is one
constant in `types/`. Each `lib/` file is named after one concept with one word.

**How logic is tested.** Every `lib/` rule is a unit test on plain values (`airsl test extensions`,
part of `dod` through `cargo make lua`). The adapter is covered on the real host by
`tests/musecode.rs` (plan fixtures, translator runs) and inside real Muse Code by `musecode-check`
(§8).

**Rust guideline rules applied** (`claudestacks-guideline-rust:rust-guidelines`):

- `modularity.md` Rule 1: `shorthand.rs` keeps its one job, "the one place the official source is
  named" (`src/types/shorthand.rs:10-11`). Rule 2: no new type.
- `mod-rs-export-only.md`: no `mod.rs` or `lib.rs` change; no new Rust module.
- `static-dispatch.md`: no trait objects; the change adds no code path that dispatches.
- `unit-test-mandate.md`: `shorthand.rs` gains a colocated test for `musecode`; `tests/musecode.rs`
  complements it and does not replace it.
- `doc-comment-discipline.md`: the `OFFICIAL_TOOLS` doc stays "The tool names the official
  repository holds under `extensions/`." with no count (`src/types/shorthand.rs:47`).
- `strong-types.md`, `workspace.md`, `strict-quality.md`, `microsoft-guidelines.md`: no new public
  parameter, crate, dependency or lint change; the gate is `cargo make dod`.
- Module layout (global rules): one-word file names, no `<concept>_<qualifier>` files; the Lua tests
  follow the `<module>_test.lua` convention `airsl test` requires (`extensions/claude/lib/`).

## 3. Core changes

### 3.1 `OFFICIAL_TOOLS`

`src/types/shorthand.rs:48` becomes
`pub const OFFICIAL_TOOLS: [&str; 3] = ["claude", "musecode", "opencode"];` (name order, as the
array is today). `init --tool musecode` then expands to the official repository with subdir
`extensions/musecode`, as `opencode` does (`src/types/shorthand.rs:215-220`). Today it refuses:
`--tool "musecode": not an official tool; pass a path, or add a git source to agentstacks.toml`
(`A1`). New test in the same file: `official_tool_musecode_expands_to_extensions_subdir`, the shape
of `official_tool_opencode_expands_to_extensions_subdir`.

The extension reaches users with the next `packs-v*` tag, which holds `extensions/` and `packs/`
together (`src/types/shorthand.rs:35`, `TAG_PREFIX = "packs-v"`).

### 3.2 No other core change

The runtime, the host and the protocol are unchanged: every event the extension needs maps to one
neutral event (§4.7), so `runtime.lua` needs no change, and the operations are the existing `write`,
`copy` and `skip` (`book/src/ch06-01-extensions.md` §7).

## 4. The musecode extension

### 4.1 `extension.toml` and `values`

```toml
[extension]
name        = "musecode"
version     = "0.1.0"
api         = 0
description = "Renders packs into Muse Code project config"
entry       = "adapter.lua"
reads       = [".muse/hooks.json"]
```

Same keys and shape as `extensions/claude/extension.toml`. This text, extracted from this spec,
loads, and `reads` is enforced: the claude adapter copied under it was refused reading
`.claude/settings.json` (`A2`).

**`values` result** (`lib/values`):

- `root = ".muse/agentstacks"`.
- `skills["<p>/<s>"] = { invoke = "/<p>-<s>", dir = ".agents/skills/<p>-<s>" }`. The skill id is the
  frontmatter `name`, which the extension writes as `<p>-<s>` (`B-B2`: `name` wins over the
  directory); the TUI invokes it as `/<id>` (`B-B1`).
- `agents["<p>/<a>"] = "<p>-<a>"`. The agent id is its frontmatter `name` (`B-B3`).
- `vars` holds only the names the payload lists (`book/src/ch06-01-extensions.md` §6):

| Var | Value | Proof |
|---|---|---|
| `project_dir` | `.` | D7 |
| `session_id` | `${MUSE_SESSION_ID}` | set in the `bash` tool's environment (`B-B5`) |
| `arguments` | `<arguments>` | `$ARGUMENTS` is never substituted; the TUI appends the arguments after the skill text (`B-B1`). Same value as the opencode extension |
| `tmpdir` | `${TMPDIR:-/tmp}` | `TMPDIR` reaches hooks (`I§2`) and the `bash` tool (`B-B5`); expanded by `/bin/sh` (`F-R1`) |
| `home` | `$HOME` | `HOME` reaches hooks (`I§2`) and the `bash` tool (`B-B5`) |
| `airsstack_home` | `${AIRSSTACK_HOME:-$HOME/.airsstack}` | the `bash` tool inherits `AIRSSTACK_HOME` (`B-B5`); hooks do not (`I§2`), so in a hook it is always the default (§9) |

The packs use five of these names (`C3.1`: `project_dir` 73, `tmpdir` 5, `arguments` 5,
`session_id` 3, `airsstack_home` 2, `home` 0); `home` is in the table for packs that use it, as in
the other two extensions.

### 4.2 Frontmatter (`lib/frontmatter`)

Rendering is `claude/lib/frontmatter`'s: every value written with `airsstack.json.encode`, generated
keys first in the order below, a `meta.native` key replacing a generated key of the same name, the
other native keys after them sorted by name, then `---` and the body text. JSON-encoded values are
read by Muse Code with the quotes removed (`B-B2`).

**Skill** `SKILL.md`, generated keys:

| Key | Value | When |
|---|---|---|
| `name` | `<p>-<s>` | always |
| `description` | `meta.description` | always |
| `disable-model-invocation` | `true` | `invocation = "user"`: hidden from the model's list, still run by the user as `/<id>` (`B-B1`, `I§7`) |
| `argument-hint` | `meta.argument_hint` | when set: shown as hint text after the command in the TUI (`B-B2`) |

**Agent** `.agents/agents/<p>-<a>.md`, generated keys:

| Key | Value | When |
|---|---|---|
| `name` | `<p>-<a>` | always |
| `description` | `meta.description` | always |
| `tools` | each tool of `meta.tools`, in the order the list gives (the payload keeps the pack's order, `src/types/frontmatter/agent/meta.rs:251`), mapped `read`→`read_file`, `edit`→`edit_file`, `write`→`write_file`, `grep`→`search`, `glob`→`search`, `shell`→`bash`, `skill`→`read_skill`; a name already written is not repeated | when `meta.tools` is set. An empty list is written as the literal `tools: []`, because `airsstack.json.encode` writes an empty table as `{}` (`extensions/opencode/lib/frontmatter.lua:4-6`, which writes `[]` for the same reason) (`B-B3`: every mapped name and `[]` spawn) |
| `effort` | `meta.effort` | when set (`low`, `medium`, `high`, `src/types/frontmatter/effort.rs:83-85`; all accepted, `B-B3`) |

`model` is never generated (D5). Example, the core `coder` agent (`tools: [read, edit, write, grep,
glob, shell, skill]`, `effort: high`, `packs/core/agents/coder.md:9-11`):

```text
---
name: "core-coder"
description: "<description>"
tools: ["read_file","edit_file","write_file","search","bash","read_skill"]
effort: "high"
---
<body>
```

**Skip rules** (checked in this order; the first that applies is the reason). F2-F5 check the
values as written, after a native key has replaced a generated one:

| # | Case | Item | Reason |
|---|---|---|---|
| F1 | a `meta.native` key with a character other than `A-Z a-z 0-9 _ -` | skill (group) or agent | `native.musecode key "<key>" cannot be written as frontmatter` |
| F2 | the skill `name` longer than 128 characters (UTF-8 characters, `utf8.len`) | skill (group) | `musecode skill names are at most 128 characters (this one has <n>)` (`F-R9`: 128 `valid`, 129 refused; pack and item names are at most 64 bytes each, `src/types/name/kebab.rs:20-22`, so `<p>-<s>` can reach 129) |
| F3 | the agent `name` not matching `^[a-z]+(-[a-z]+)*$` | agent | `musecode agent names are lowercase letter words joined by - (got "<name>")` (`B-B3`: a digit, an underscore or a capital letter is dropped from the catalog and refused at spawn) |
| F4 | the agent `description` longer than 4096 bytes | agent | `musecode drops an agent whose description is over 4096 bytes (this one has <n>)` (`B-B3`: 4096 listed, 4097 not) |
| F5 | the whole `SKILL.md` text longer than 262144 bytes | skill (group) | `musecode refuses a SKILL.md over 262144 bytes (this one has <n>)` (`F-R9`: 262144 `valid`, 262145 `SKILL.md exceeds 262144 byte limit`) |

A native `name` or `description` that is not a string is written as given and not checked by F2-F4.
None of the official packs hits F1-F5 (`C3.1`: every agent name letters and `-`; `A4`: the longest
agent description is 598 bytes).

### 4.3 Files

| Item | What is written |
|---|---|
| skill `SKILL.md` | `.agents/skills/<p>-<s>/SKILL.md`, frontmatter and text (§4.2) |
| other skill file | copied next to `SKILL.md` |
| agent | `.agents/agents/<p>-<a>.md` (§4.2) |
| native file | `native/musecode/<rel>` is copied to `.agents/<rel>` |
| hooks | `.muse/hooks.json`, shared (§4.5) |
| `.muse/agentstacks/translate.lua` | written when at least one Lua hook is covered by the hooks file write (§4.7) |
| `.muse/agentstacks/runtime.lua`, root files | written by the core (`book/src/ch06-01-extensions.md` §7) |

A path joins the pack and item names with `-`, so `core/status` and `sdlc/status` never clash. Two
items whose joined names are equal, or a native file on a generated path, propose the same path
twice, and the core refuses it (`book/src/ch06-01-extensions.md` §9).

### 4.4 Hooks (`lib/hooks`)

`M.render(hook, vars, agents)` returns `{ event = <Muse event>, group = { matcher = <m or nil>,
hooks = { { type = "command", command = <c> } } } }`, or `nil, reason`. It is
`claude/lib/hooks`'s `render` with the differences below. `M.MARKER = " # agentstacks"`.

**Events and matchers.**

| Neutral event | Muse event | Matcher |
|---|---|---|
| `session.start` | `SessionStart` | `match` joined with `\|`, none when absent; Muse matches it on `source` (`H-P2`, `H-P3`) |
| `session.end` | `SessionEnd` | none |
| `prompt.submit` | `UserPromptSubmit` | none (a matcher there is refused by Muse, `H-P3`) |
| `tool.before`, `tool.after` | `PreToolUse`, `PostToolUse` | `match` mapped `read`→`Read`, `edit`→`Edit`, `write`→`Write`, joined with `\|`; `Read\|Edit\|Write` when absent. Muse matches these on `read_file`, `edit_file`, `write_file` (`I§4`, `raw/out3-hooks.json`) |
| `agent.stop` | `SubagentStop` | none; a hook with `agents` is skipped (D6, rule H3) |

A native hook (`native/musecode/hooks.toml`) keeps its `event` as written and its `match` joined
with `|` (none when absent or empty).

**Commands.**

- Lua hook: each argv element quoted as `claude/lib/hooks` `quote_argument` does (`"…"` with `\`,
  `"` and `` ` `` escaped by `\`), joined with spaces; then every `'` replaced by `'\''`; the result
  is `/bin/sh -c '<quoted argv>' # agentstacks`. `probes/harness/gen.lua` implements exactly this
  rule. For the argv `["touch", "${TMPDIR:-/tmp}/it's \"q\" `b` \\z"]` it gives (`F-R7`):

  ```text
  /bin/sh -c '"touch" "${TMPDIR:-/tmp}/it'\''s \"q\" \`b\` \\z"' # agentstacks
  ```

  and under `SHELL` zsh, sh and bash Muse Code ran it in the project root and created the file
  `it's "q" `b` \z` in `$TMPDIR` (`F-R7`).

- Message hook: `claude/lib/hooks` `message_command`, unchanged:
  `printf '%s\n' '<json>' # agentstacks`, `<json>` being `airsstack.json.encode` of
  `{hookSpecificOutput = {hookEventName = <event>, additionalContext = <text>}}` with every `'`
  replaced by `'"'"'`. For the text `MSG-R8 line1⏎line2 it's "q" back\slash` it gives (`F-R8`):

  ```text
  printf '%s\n' '{"hookSpecificOutput":{"additionalContext":"MSG-R8 line1\nline2 it'"'"'s \"q\" back\\slash","hookEventName":"SessionStart"}}' # agentstacks
  ```

  and under `SHELL` zsh, sh and bash the model's first request held that text exactly (`F-R8`).

**Skip rules** (checked in this order; the first that applies is the reason):

| # | Case | Applies to | Reason |
|---|---|---|---|
| H1 | the hook's text or an argv element holds `${MUSE_SESSION_ID}` | both | `musecode gives hooks the session id in the event (event.session_id), not as {{var session_id}}` (in a subagent the variable is the parent's session while `session_id` is the child's, `B-B4`) |
| H2 | a message hook on `session.end` | neutral | `musecode shows no hook output to the model at session end` (`F-R6`: `SessionEnd` `additionalContext` reached no request, while the `SessionStart` control did) |
| H3 | an `agent.stop` hook with `agents` | neutral | `musecode gives hooks no agent name at agent stop, so it cannot run this hook for only the listed agents` (D6) |
| H4 | a message hook on `agent.stop` | neutral | `musecode passes no hook message to the model at agent stop` (`F-R6`: `SubagentStop` `additionalContext` reached no request) |
| H5 | a native hook on an event Muse Code does not run: any name but `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PostToolUseFailure`, `PostToolBatch`, `PreLLMCall`, `PostLLMCall`, `PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, `Stop`, `StopFailure`, `SessionEnd`, `Notification`, `SessionFork` | native | `musecode does not run hooks on "<event>"` (`F-R11`: these 18 run; `ToolUseStart` needs `async: true`, which the extension never writes, and an unknown name is dropped with `UnsupportedEvent`) |
| H6 | a native message hook on an event outside `SessionStart`, `SubagentStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure` | native | `musecode passes no hook message to the model on <event>` (the six are the events whose `additionalContext` reached the model in a run: `I§6`, `F-R5`, `H-P6`, `H-P9`; it reached no request on `SubagentStop` and `SessionEnd`, `F-R6`, and `Stop` rejects it, `H-P9`. The docs also list `PostToolBatch`, `PreLLMCall`, `PostLLMCall`, `probes/docs/guides_plugins_reference_hook-events_.txt:242`; unproven, so skipped) |
| H7 | a native hook with a non-empty `match` on `UserPromptSubmit` or `Stop` | native | `musecode refuses a matcher on <event>` (`H-P3`: ``InvalidMatcher: D11: `user_prompt_submit` is unmatchable``) |
| H8 | a Lua argv element with a `$` left after every var form is removed | both | `musecode runs hook commands through sh, which would expand the literal $ in argument "<arg>"` |

`<event>` and `<arg>` stand for the hook's event and the argument. H1, H6 and H8 keep the claude
extension's wording with `claude` replaced by `musecode`. A native Lua hook on an event `M.result`
does not handle (§4.7) runs for its side effects; any context or block it returns is not delivered
(§9).

With the official packs these hooks are skipped (`C3.1`): core's `session.end` message (H2) and the
three `agent.stop` handoff checks of core, sdlc and journal (H3). Core's `agent.stop` comm check
(no `agents`) renders. The two native hooks are `native/claude` only.

### 4.5 The shared hooks file (`lib/hookfile`)

`lib/hookfile` is `claude/lib/settings` with the file name changed: the same `decode`, `edits`,
`should_write` and the same rules for a rewritable event array. The adapter follows
`extensions/claude/adapter.lua`'s `propose_settings` with `SETTINGS = ".muse/hooks.json"`:

1. `old = agentstacks.read(".muse/hooks.json")`; when it is not `nil`,
   `agentstacks.json.edit(old, {"hooks", "agentstacks-check"}, nil)` refuses a file that is not JSON,
   not an object, or whose `hooks` is not an object, with the core's `json.edit` messages
   (`book/src/ch06-01-extensions.md` §11).
2. Per touched event (one that gets our groups or holds a marked handler), the old array minus our
   marked handlers, groups left empty dropped, then our groups appended in payload order; `nil`
   removes the event.
3. An event whose array cannot be rewritten is left alone, and each of our hooks on it is skipped:
   `.muse/hooks.json: hooks.<event> holds entries the musecode extension cannot rewrite (a null, an
   empty list, or a value that is not a list of hook groups)`.
4. The file is written when it existed or when there is at least one edit, with `covers` = the ids
   of the hooks written, sorted. It is shared: never locked (`book/src/ch05-00-plan-and-apply.md`
   §3).

Every group the extension writes has a string `matcher` or none, and a list of handlers with
`type` and `command` only, so it never triggers Muse Code's whole-file drop (`H-P8` b, e) or handler
drop (`H-P8` a).

### 4.6 Skips (all)

The frontmatter rules F1-F5 (§4.2), the hook rules H1-H8 (§4.4) and the hooks-file rule (§4.5
step 3). The core's own skips (missing values, `book/src/ch06-01-extensions.md` §6) apply as for
every extension.

### 4.7 `translate.lua` (`lib/translate`)

`lib/translate` returns this text, written verbatim to `<root>/translate.lua`:

```lua
-- agentstacks translator for musecode, api 0.
local M = {}

local EVENTS = {
  SessionStart = "session.start",
  SessionEnd = "session.end",
  UserPromptSubmit = "prompt.submit",
  PreToolUse = "tool.before",
  PostToolUse = "tool.after",
  SubagentStop = "agent.stop",
}
local TOOLS = { read_file = "read", edit_file = "edit", write_file = "write" }
local SOURCES = { startup = true, resume = true, clear = true, compact = true }

local function str(v)
  if type(v) == "string" then
    return v
  end
  return nil
end

-- Muse Code passes the path the model wrote; a relative one is made absolute against `cwd`.
-- An empty path, or a missing or empty `cwd`, is passed through unchanged.
local function absolute(path, cwd)
  if path == nil or path == "" or path:sub(1, 1) == "/" or type(cwd) ~= "string" or cwd == "" then
    return path
  end
  return cwd .. "/" .. path
end

function M.event(p)
  local name = EVENTS[p.hook_event_name]
  if name == nil then
    return nil
  end
  local e = { event = name, session_id = p.session_id, cwd = p.cwd }
  if name == "session.start" then
    if SOURCES[p.source] then
      e.source = p.source
    end
  elseif name == "prompt.submit" then
    e.prompt = str(p.prompt)
  elseif name == "tool.before" or name == "tool.after" then
    local tool = TOOLS[p.tool_name]
    if tool == nil then
      return nil
    end
    e.tool = tool
    if type(p.tool_input) == "table" then
      e.path = absolute(str(p.tool_input.path), p.cwd)
    end
  elseif name == "agent.stop" then
    e.last_message = str(p.last_assistant_message)
  end
  return e
end

local function encode(out)
  return airsstack.json.encode(out) .. "\n"
end

function M.result(r, p)
  if r == nil then
    return nil
  end
  local event = p.hook_event_name
  local ctx, blk = r.context, r.block
  if ctx == nil and blk == nil then
    return nil
  end
  if event == "PreToolUse" then
    local o = { hookEventName = event }
    if blk ~= nil then
      o.permissionDecision = "deny"
      o.permissionDecisionReason = blk
    end
    if ctx ~= nil then
      o.additionalContext = ctx
    end
    return encode({ hookSpecificOutput = o })
  elseif event == "UserPromptSubmit" or event == "PostToolUse" then
    local out = {}
    if blk ~= nil then
      out.decision = "block"
      out.reason = blk
    end
    if ctx ~= nil then
      out.hookSpecificOutput = { hookEventName = event, additionalContext = ctx }
    end
    return encode(out)
  elseif event == "SubagentStop" then
    if blk == nil then
      return nil
    end
    return encode({ decision = "block", reason = blk })
  elseif event == "SessionStart" then
    local parts = {}
    if ctx ~= nil then
      parts[#parts + 1] = ctx
    end
    if blk ~= nil then
      parts[#parts + 1] = blk
    end
    return encode({ hookSpecificOutput = { hookEventName = event, additionalContext = table.concat(parts, "\n") } })
  end
  return nil
end

return M
```

It differs from `claude/lib/translate` only in the header, `TOOLS`, the path field
(`tool_input.path`, made absolute) and the agent fields, which it never sets. Each output shape is
proven in Muse Code:

| Event | Output | Effect | Proof |
|---|---|---|---|
| `PreToolUse` | `hookSpecificOutput.permissionDecision = "deny"` + reason | the call is refused; the model sees `tool blocked by hook: <reason>` | `I§5`, `F-R5` |
| `PreToolUse` | `hookSpecificOutput.additionalContext`, alone or beside a deny | a developer message after the tool output (after the blocked output when denied) | `F-R5` |
| `PostToolUse`, `UserPromptSubmit`, `SessionStart` | `hookSpecificOutput.additionalContext` | a developer message after the tool output or prompt; session-start context in the first request | `I§6`, `H-P6` |
| `PostToolUse` | `decision = "block"` + reason | the model sees the reason in place of the tool output | `H-P6` |
| `UserPromptSubmit` | `decision = "block"` + reason | the run is cancelled before the model is called (`exec` exits 1) | `H-P5` |
| `SubagentStop` | `decision = "block"` + reason | the agent runs again (`stop_hook_active` true) | `F-R2` |
| `SessionStart` | block text joined into the context | `SessionStart` cannot block (`probes/docs/guides_extend_hooks_.txt:513`) | — |

The text compiles under airsl 0.1.5, and run through `crates/agentstacks/assets/runtime.lua` on
recorded Muse Code payloads it prints a deny with the absolute path for a relative `b.txt` write, a
`PostToolUse` context, a `SessionStart` context, and nothing for a `SubagentStop` context (`T`).

Payload fields read (`raw/out-root`, `raw/out3`, `H-P2`): `hook_event_name`, `session_id`, `cwd`,
`source`, `prompt`, `tool_name`, `tool_input.path`, `last_assistant_message`. No field names the
running agent (`B-B4`), so `agent` and `agent_id` are never set: every event reads as the main
agent's (§9).

## 5. Errors: locations and owners

The extension adds no refusal of its own: every unsupported item is a skip (§4.6), and operations
go through the core's existing checks (`book/src/ch06-01-extensions.md` §9).

| Check | Owner | What the user sees |
|---|---|---|
| `--tool musecode` accepted | `types/shorthand` (§3.1) | — |
| a `.muse/hooks.json` that is not JSON, not an object, or whose `hooks` is not an object | core `json.edit`, called by the adapter (§4.5 step 1) | the plan is refused with `musecode/adapter.lua:<line>: json.edit: <message>` (`book/src/ch06-01-extensions.md` §11-§12) |
| `.muse/hooks.json` changed between `plan` and `apply` | core | `.muse/hooks.json: changed after it was read; run plan again` (`book/src/ch05-00-plan-and-apply.md` §3) |
| skip decisions | `lib/frontmatter` (F1-F5), `lib/hooks` (H1-H8), `lib/hookfile` (§4.5 step 3) | a skip line in `plan` |
| path clashes, coverage, `translate.lua` presence | core (`book/src/ch06-01-extensions.md` §8-§9) | the core's messages |
| hook faults at run time | Muse Code | nothing on screen: a failed hook is recorded only in the session log (`H-P7`) |

## 6. Edge cases

| Case | Result | Owner |
|---|---|---|
| `claude` and `musecode` listed | both apply; separate roots, no path clash. Muse Code also scans `.claude/skills/`; a skill in both has one id and the `.agents` copy wins (`I§7`). It does not load `.claude/agents/` or the hooks in `.claude/settings.json` (`F-R10`). Skills only Claude has (`native/claude`, such as `core-statusline`) show in Muse Code with Claude's var forms; no setting turns project `.claude/skills` off as a whole, only one skill at a time with `muse skills disable <id> --scope project`, stored in the user's settings (`B-B7`) | documented (§9) |
| `opencode` and `musecode` listed | OpenCode also reads `.agents/skills/`; a skill in both has one id and the `.opencode` copy wins (`C2`). A skill the opencode extension skips but this one writes shows in OpenCode with Muse Code's var forms | documented (§9) |
| no pack has a hook, no `.muse/hooks.json` | no hooks file written, no `translate.lua` | `lib/hookfile`, adapter |
| no pack has a hook, `.muse/hooks.json` exists | rewritten with our marked handlers removed (unchanged when there are none) | `lib/hookfile` |
| `.muse/hooks.json` empty or whitespace only | treated as `{}` (`claude/lib/settings` `decode`; `book/src/ch06-01-extensions.md` §11) | `lib/hookfile` |
| the user's event array holds a `null`, an empty list, or a value that is not a list of groups | that event is left alone; our hooks on it are skipped (§4.5 step 3) | `lib/hookfile` |
| the user edits one of our handlers | the next `plan` replaces it (marked handlers are ours); a handler whose marker the user removed is the user's and stays | `lib/hookfile` |
| the `musecode` tool is removed | the core runs the extension with zero packs, which removes our handlers from the shared file, and deletes the locked files (`book/src/ch05-00-plan-and-apply.md` §3, §5) | core, `lib/hookfile` |
| every Lua hook skipped | the core still writes `runtime.lua`; no `translate.lua` (`book/src/ch06-01-extensions.md` §7) | core, adapter |
| Muse Code started in a subdirectory | the workspace root is the launch directory, so project hooks, skills and agents of the root are not loaded (`I§1`, `H-P10`). With `--workspace <root>` they load, but hooks run in the launch directory (`H-P10`), so relative grants and `project_dir` = `.` point at the subdirectory | documented (§9) |
| an untrusted workspace | project skills, hooks and agents are not loaded until the user trusts the folder (`probes/docs/user-configuration.txt:309`: "Skills, rules, and hooks … load only after you trust it"; `muse exec --trust-workspace` trusts for one run, `I§2`) | documented (§9) |
| a `session.start` hook matching `clear` or `compact` | rendered with that matcher. `startup` and `resume` are seen headless; `clear` needs the TUI and `compact` was not observed (`H-P2`) | `lib/hooks` (documented) |
| `SubagentStop` from Muse Code's own `goal-reminder` agent | fires on every turn, several times (`I§3`, `H-P4`, `F-R2`); an `agent.stop` hook without `agents` sees it, with `last_message` the reminder's text | documented (§9) |
| a tool path the model writes relative | made absolute against the payload `cwd` (§4.7). With `--workspace` from a subdirectory, `cwd` is the subdirectory while Muse Code resolves the path from the workspace root (`H-P10`, `H-P9`), so the absolute path is wrong | `translate.lua` (documented, §9) |
| a message hook with a newline, `'`, `"` or `\` | JSON-encoded and quoted for the shell; reaches the model exactly under zsh, sh and bash (`F-R8`) | `lib/hooks` |
| airsl missing from `PATH` | `require_binary` refuses at plan when a locked pack needs airsl (`book/src/ch06-02-hooks.md` §10). If it disappears later, each Lua hook exits 127, Muse Code shows nothing, the run goes on and a `may_block` guard blocks nothing (`H-P7`); `doctor` reports it | core (documented) |
| the user's `SHELL` is not zsh, sh or bash | Muse Code still runs each command as `$SHELL -c` (`H-P1`); the wrapper and the message form were checked under zsh, sh and bash only (`F-R7`, `F-R8`). A `SHELL` that cannot start makes every hook fail silently (`H-P1` `p1-false`, `p1-shell`) | documented (§9) |
| a user group in `.muse/hooks.json` that is valid JSON but invalid for Muse Code (for example a numeric `matcher`) | the merge keeps it (§4.5); Muse Code then drops the whole file, ours included (`H-P8` b) | documented (§9) |
| a `SessionStart` with `source` `fork` (in the docs, `probes/docs/guides_extend_hooks_.txt:118-119`; not observed) | `source` is left unset; a hook with no `match` runs, a hook with a `match` does not | `translate.lua` (documented) |
| an empty tool path, or an empty `cwd` | passed through unchanged (§4.7) | `translate.lua` |
| an agent with a native key Muse Code does not know | written; Muse Code drops the agent from its catalog with no message (`I§8`) | pack author (documented) |

## 7. Not carried over from Claude

| Claude feature | Muse Code | Visible as |
|---|---|---|
| `SubagentStop` matcher by agent name | no agent name in hook payloads (`B-B4`) | skip H3 |
| `agent_type` / `agent_id` on events | none (`B-B4`) | every event reads as the main agent's |
| model tier | Muse Code model ids only | not written |
| `cd "$CLAUDE_PROJECT_DIR"` | hooks run in the launch directory | `project_dir` = `.` (D7) |
| `$ARGUMENTS` in a skill | never substituted; the TUI appends the arguments (`B-B1`) | `<arguments>` |
| `/skill args` in headless runs | `muse exec` sends `/name args` as plain text (`B-B1`) | documented |

## 8. Testing

### 8.1 Unit (`airsl test extensions`, in `dod`)

One `<module>_test.lua` per `lib/` module, plain values in, asserted out:

- `values_test`: root; skill invoke and dir; agent names; each var of §4.1; a var not in the payload
  absent.
- `frontmatter_test`: skill with and without `invocation = "user"`, with `argument_hint`; agent with
  the core `coder` meta (§4.2 example, byte exact), with no `tools`, with `tools = []`, with
  `["grep","glob"]` (one `search`); native key replaces a generated one; each of F1-F5 with its exact
  reason, F2 at 128 (written) and 129 (skipped), F4 at 4096 (written) and 4097 (skipped), F5 at
  262144 and 262145 bytes, F3 with `core-coder2`, `core_x`, `Core-x`, and F3 and F4 applied to a
  native `name` and `description`.
- `hooks_test`: each row of the §4.4 events table; the Lua command for the `F-R7` argv and the
  message command for the `F-R8` text, both byte exact as §4.4 shows them; every skip rule H1-H8 with
  its exact reason; a native `SessionStart` message hook (renders), a native `PreCompact` message
  hook (H6), a native `OnSave` hook (H5), a native `UserPromptSubmit` hook with `match = []`
  (renders, no matcher).
- `hookfile_test`: the cases of `claude/lib/settings_test.lua` with the musecode reason text.
- `translate_test`: the text is byte-identical to §4.7.

### 8.2 Real host (`cargo test`, in `dod`)

- `crates/agentstacks/tests/musecode.rs`, mirroring `tests/opencode.rs`: plan fixtures under
  `tests/fixtures/musecode/<case>/expected` (every written file and skip line) for the demo pack, a
  project with no hooks, a project with an existing user `.muse/hooks.json` holding a user group and
  a non-rewritable event, and a two-tool project (`claude` + `musecode`); and `translate.lua` run on
  `airsl` against recorded Muse Code payloads under `tests/fixtures/musecode/payloads` (taken from
  `probes/raw/`: SessionStart, UserPromptSubmit, PreToolUse on `read_file`, `edit_file` and
  `write_file`, PostToolUse on `read_file` and `edit_file`, all with relative paths, and
  SubagentStop), asserting stdout byte for byte.
- `src/types/shorthand.rs`: `official_tool_musecode_expands_to_extensions_subdir`.

### 8.3 `musecode-check` (real Muse Code, not in `dod`)

Files under `crates/agentstacks-cli/tests/musecode-check/`:

| File | Role |
|---|---|
| `mock.mjs` | the mock model, `probes/harness/mock.mjs` logic (`GET <base>/muse-code/models`, `POST <base>/responses` as Responses SSE, scripted function calls, every request logged), plus: it listens on the port given (`0` = any free port) and prints `listening <port>` once bound |
| `check.mjs` | builds a scratch project, copies `extensions/musecode` to `<scratch>/ext/musecode` and `pack/` to `<scratch>/packs/check`, starts the mock and waits for its `listening` line (D9: no line within 10 s → `FAIL mock did not start`, exit 1), seeds `<scratch>/home/.config/muse/settings.json` = `{"schema_version":1,"endpoint_transport":{"base_url":"http://127.0.0.1:<port>","auth":"bearer"}}` and `auth.json` = `{"schema_version":1,"providers":{"meta":{"api_key":"dummy"}}}`, and runs each scenario as `muse exec --trust-workspace --yolo "<prompt>"` from the project root with `HOME` and `TMPDIR` in the scratch dir, `XDG_CONFIG_HOME` unset and `TBH_CREDENTIAL_BACKEND=file` (the shape of `probes/harness/msg-run.sh`). Prints one `ok`/`FAIL` line per scenario, exits 1 on any failure |
| `pack/` | the check pack, below |

The check pack (`pack.toml` name `check`, `requires.airsl = ">=0.1.5"`):

| Item | Definition |
|---|---|
| skill `hello` | `invocation = "any"`, body holding `HELLO-BODY`, plus `references/note.md` |
| skill `ask` | `invocation = "user"`, body `ASK-BODY args=<{{var arguments}}>` |
| agent `helper` | `tools = ["read", "grep"]`, body holding `HELPER-BODY` |
| hook 0 | `kind = "message"`, `session.start`, `match = ["startup"]`, text `CHECK-WELCOME` |
| hook 1 | `kind = "lua"`, `session.start`, no `match`, returns `{ context = "CHECK-START " .. event.source }` |
| hook 2 | `kind = "lua"`, `prompt.submit`, returns `{ context = "CHECK-PROMPT" }` |
| hook 3 | `kind = "lua"`, `tool.before`, `match = ["write"]`, `may_block = true`: a path ending `/blocked.txt` → `{ block = "CHECK-BLOCK " .. event.path }`, else `{ context = "CHECK-BEFORE " .. event.path }` |
| hook 4 | `kind = "lua"`, `tool.after`, `match = ["read"]`, returns `{ context = "CHECK-AFTER " .. event.path }` |
| hook 5 | `kind = "message"`, `session.end`, text `CHECK-END` (skipped, H2) |
| hook 6 | `kind = "lua"`, `agent.stop`, `agents = ["helper"]`, returns nothing (skipped, H3) |

Scenarios (each asserts on the requests the mock received and the files left):

| # | Scenario | Asserts |
|---|---|---|
| S1 | `agentstacks init --tool ext/musecode --pack packs/check --yes`, then `plan`, `doctor`, `update` | exit 0; `0 to write`; `healthy`; skip lines for hooks 5 and 6 with the H2 and H3 reasons; `update` prints `nothing to update` |
| S2 | skills: the model calls `read_skill` with `check-hello` | the first request lists `check-hello` with path `.agents/skills/check-hello/SKILL.md` and `check-ask` with `model-invocable="false"`; the `read_skill` result holds `HELLO-BODY` |
| S3 | agent: the model calls `subagent_spawn` with `subagent_type` `check-helper` | the catalog lists `check-helper`; the spawn is accepted; the child's request holds `HELPER-BODY` and offers `read_file` and `search` and no `write_file`, `edit_file` or `bash` |
| S4 | hooks (`<project>` is the realpath of the project directory: on macOS `TMPDIR` is under `/var`, a link to `/private/var`): prompt `hooks`; the model writes `blocked.txt`, writes `ok.txt`, reads `ok.txt`, runs `bash` `pwd` | the first request holds `CHECK-WELCOME`, `CHECK-START startup` and `CHECK-PROMPT`; `blocked.txt` absent and its tool output is `tool blocked by hook: CHECK-BLOCK <project>/blocked.txt`; the `ok.txt` write is followed by `CHECK-BEFORE <project>/ok.txt`; the read by `CHECK-AFTER <project>/ok.txt`; `pwd` prints `<project>` |
| S5 | shared file: before `init`, the project holds `.muse/hooks.json` with a user `SessionStart` group whose command prints context `USER-CTX` | after `apply` the file holds the user group first, then ours; a run's first request holds `USER-CTX` and `CHECK-START startup` |
| S6 | missing airsl: `PATH` without any directory holding `airsl`, the S4 write of `blocked.txt` | `muse exec` exits 0; `blocked.txt` is written (the guard is off, §9) |

### 8.4 Task and CI

- **Task** `[tasks.musecode-check]` in `Makefile.toml`, category `Muse Code`, not a dependency of
  `dod` (like `opencode-check`). Script:
  1. Pick the build from `uname -s`/`uname -m`: `Linux x86_64` → `file=muse-x86-linux`, SHA256
     `c1b728fb9732503c5e1869e3ad2f2c9bdb5b2638fb4cf398d6e2c46aa6d8f94a`; `Darwin arm64` →
     `file=muse-aarch64-macos`, SHA256
     `d8faeec8b3a2507125d97f1a79ae18781d254e8cd376f26531587fb6e043c27d` (`C1.2`; the manifest's
     sizes, 468724056 and 478310160 bytes, are not checked separately). Anything else:
     exit 1 with `musecode-check: no pinned Muse Code build for <os> <arch>`.
  2. `mkdir -p target/musecode-check/1.4.25-R9361.1`. When `target/musecode-check/1.4.25-R9361.1/muse`
     is missing: `curl -fsSL -o target/musecode-check/1.4.25-R9361.1/muse
     'https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=1.4.25-R9361.1&file=<file>'`
     (HTTP 200, no login, no redirect, `C1.3`, `C1.4`). Then, on every run, whether downloaded now
     or found (a partial download or a restored cache included), verify the SHA256 of the file
     (`sha256sum`, else `shasum -a 256`), as `opencode-check` does (`Makefile.toml:496-507`). A
     mismatch: remove the file, exit 1 with `musecode-check: <file>: checksum mismatch`. Then
     `chmod +x`. The download is the raw binary (`C1.3`, `C1.4`), the same bytes every probe here ran
     without the launcher (D10).
  3. `cargo build -p agentstacks-cli`.
  4. `node crates/agentstacks-cli/tests/musecode-check/check.mjs
     target/musecode-check/1.4.25-R9361.1/muse target/debug/agentstacks`.
- **CI job** `musecode-check` in `.github/workflows/ci.yml`: `ubuntu-latest`; the same checkout,
  toolchain, cache, cargo-make and airsl steps as the `opencode-check` job; `actions/setup-node@v4`
  with `node-version: 22`; `actions/cache@v4` on `target/musecode-check/1.4.25-R9361.1` keyed
  `musecode-1.4.25-R9361.1-linux-x64` (the binary is 468724056 bytes); then
  `cargo make musecode-check`; `timeout-minutes: 30`, as the `opencode-check` job sets. Same
  triggers as the other jobs.
- Bumping Muse Code means changing the version and both hashes together, as the OpenCode pin
  does.

### 8.5 Docs

- New book page `book/src/ch06-04-musecode.md` (in `SUMMARY.md` after `ch06-03`): the vars (§4.1),
  the files (§4.3), the hook mapping (§4.4) and delivery (§4.7 table), the skips (§4.6), the
  not-carried-over table (§7), and every limit of §9.
- `book/src/ch04-00-cli.md:59-61` and `book/src/ch01-02-first-project.md:15-18`: `musecode` in the
  official tools and the `--tool ext/musecode` note, as for OpenCode.
- `book/src/introduction.md`, `README.md` (tools table, layout tree, status lines) and `CLAUDE.md`
  (Status, layout, Commands gains `cargo make musecode-check`, the CI job list) name Muse Code as
  built.
- Text that says "both" official extensions names all three: `book/src/ch06-01-extensions.md:4`,
  `book/src/ch06-02-hooks.md:4`, `README.md:134`, `CLAUDE.md:5` ("both extensions") and the CLAUDE.md
  Invariants line "Both Claude and OpenCode run on it". `book/src/ch06-00-authoring.md` and `book/src/ch06-01-extensions.md`, where
  they list the official extensions, add `musecode`; their API text does not change.
- `book/src/ch06-03-opencode.md` §7 "Both tools" adds that OpenCode also reads `.agents/skills/`
  (`C2`); `packs/journal/README.md:23` (per-tool notes) adds Muse Code.
- Changelog entries: `crates/agentstacks/CHANGELOG.md` (`musecode` official tool),
  `crates/agentstacks-cli/CHANGELOG.md` (`--tool musecode`), `packs/CHANGELOG.md` (the extension).
- Org docs (`.github/profile/README.md`, workspace `README.md`) are outside this repository and are
  updated when the release is made (`CLAUDE.md` "How work is done").

## 9. Limits (documented, not fixed)

- Muse Code must be started in the project root: from a subdirectory the project's config does not
  load, and with `--workspace` hooks run in the subdirectory, so relative grants, `project_dir` and
  the translator's absolute paths point at the subdirectory (§6, `H-P10`).
- Muse Code reads `.muse/hooks.json` when a session starts: after `apply`, start a new session
  (`probes/docs/guides_extend_hooks_.txt:489-493`; the same holds for skills and agents, which a new
  run reads as shown in every probe).
- A user group in `.muse/hooks.json` that Muse Code finds invalid makes it drop the whole file,
  ours included (§6).
- A native Lua hook on an event other than `SessionStart`, `UserPromptSubmit`, `PreToolUse`,
  `PostToolUse` and `SubagentStop` runs, but its context or block is not delivered (§4.4, §4.7).
- Project config loads only in a trusted workspace (§6).
- No hook knows which agent is running: `agent.stop` hooks for named agents are skipped (H3), and
  tool hooks in a subagent read as the main agent's (§4.7, `B-B4`).
- `SubagentStop` fires several times per turn for Muse Code's own `goal-reminder` agent (§6).
- In hooks, `AIRSSTACK_HOME` is always the default `$HOME/.airsstack`: Muse Code passes hooks only an
  allowed list of variables (`I§2`). The `bash` tool sees the user's value (`B-B5`).
- `{{var project_dir}}` is `.`: right while the shell is in the workspace root.
- A hook's failure, a missing airsl included, is silent and fails open (`H-P7`); `doctor` checks
  airsl and its pin (`book/src/ch06-02-hooks.md` §10).
- `muse exec` does not expand `/<skill> args`; only the TUI does (`B-B1`).
- With `claude` listed, Claude-only skills show in Muse Code; they can be turned off one at a time
  (`B-B7`); with `opencode` listed,
  skills only this extension writes show in OpenCode (`C2`).
- Hook commands were checked under `SHELL` zsh, sh and bash only; a `SHELL` that cannot start fails
  every hook silently (§6).

## 10. Risks

| # | Risk | Owner |
|---|---|---|
| K1 | The Linux build has not run here (`C1.3` checked its hash and type only). The first CI run of `musecode-check` proves it; if it fails, the plan that adds the CI job reports it | CI plan, execute |
| K2 | Whether the versioned download URL stays reachable after stable moves past 1.4.25 is unproven (`C1.2`). A 404 fails the check loudly; the fix is a pin bump | maintainers |
| K3 | Muse Code moves fast (canary 1.4.26-R9515.1 on 2026-11-25, `C1.1`); hook and skill behaviour may change. The pin holds the check steady; a bump re-runs S1-S6 | maintainers |
| K4 | `TBH_CREDENTIAL_BACKEND` is not documented, and its effect rests on the author watching the screen during runs (`F-R4`). If a later build drops it, local macOS runs of the check may show the Keychain dialog again; CI on Linux is unaffected | maintainers |
| K5 | Muse Code's shell sandbox may refuse writes outside the workspace by a skill's script (for example under `$HOME/.airsstack`); not probed. Users choose the permission profile | documented as unprobed; book page |

## 11. Non-goals

- Any Muse Code version other than the pinned 1.4.25-R9361.1 in checks; Windows.
- Muse Code user-level config (`~/.config/muse/`), MCP servers, permission profiles, `AGENTS.md`
  rules and Muse Code plugins (`.muse-plugin/`).
- A test against Meta's real model.
- Pack content changes.
- Freezing the API (`foundation/03`).
- A way to run hooks for named agents (needs an agent name in Muse Code's payload).
