---
status: approved
created: 2026-11-22
built-on: a30291e280c8
---

# Spec: CLI commands and Claude Code extension

This spec turns the built core into something a user runs. It covers:

- the `agentstacks` commands `init`, `plan`, `apply`, `doctor` and `update`;
- the small library additions they need;
- the official `claude` extension, which renders packs into native `.claude/` project config through
  the public `api = 0` surface.

Proof convention:

- `probes/<file>:<line>` is a file in this chain directory holding raw probe output.
- `probes/docs/<page>.md:<line>` is Claude Code documentation fetched with curl on 2026-11-22
  (`probes/docs-results.md` lists the fetch commands).
- `src/…` is `crates/agentstacks/src/…` at commit `8107626`.
- `core §N` cites the approved core spec (`2026-11-20-core-plan-and-apply/spec.md`), and
  `formats §N` cites the approved formats spec.

Redesigned on 2026-11-22 from `spec-superseded-2026-11-22.md`. The plan-set review found six
gaps (G1-G6), settled as D19-D24. Its spec review found three more, settled as D25-D27. All are
proven in `probes/redesign-results.md` or the cited docs. Nothing else changed.

Versions probed:

| Tool | Version |
|---|---|
| Claude Code | 2.1.330 (`probes/claude-results.md`) |
| airsl, library and binary | 0.1.4 |
| clap | 4.6.9 |
| toml_edit | 0.25.20 |
| similar | 3.2.1 |
| rustc | 1.94.1 (`probes/cli-results.md`) |
| cargo, redesign probes | 1.94.1, the pinned toolchain; G1 and G2 also reproduce on 1.99.0 (`probes/redesign-results.md:3`) |
| toml (the toml_edit parse comparison) | 1.2.0; it shares `toml_parser` with toml_edit 0.25.20 (`probes/redesign-results.md`, `cargo tree -i toml_parser`) |

## 1. Settled decisions

All were settled with the author on 2026-11-22.

| # | Decision |
|---|---|
| D1 | `init` refuses when `agentstacks.toml` exists. Otherwise it writes the file, then plans, shows the plan and applies with the `apply` confirmation (§5). |
| D2 | `apply`, `init` and `update` print the plan and ask `Apply? [y/N]`. `--yes` skips the question. With no terminal on stdin and no `--yes`, the command refuses. A plan with nothing to change asks nothing (§5.3). |
| D3 | Exit codes: `0` success, `1` refused, `2` usage. A refusal is any diagnostic, any `doctor` finding, or a CLI refusal. A declined prompt exits `0`. Usage errors are clap's own. |
| D4 | `--pack` / `--tool` take an official name or a project-relative path. Git URLs are written into `agentstacks.toml` by hand (§4.2). |
| D5 | `update` rewrites each newer tag in `agentstacks.toml` with comments kept, then plans, shows and applies like `apply`. When nothing is newer, nothing is written (§7). |
| D6 | The project is the current directory. A global `-C <dir>` changes it. Parent directories are not searched. |
| D7 | Plan output has one line per changed path, a unified diff for an updated shared file, then skips, notes and a summary (§5.2). No colour. |
| D8 | When `init`'s plan is refused or declined, `agentstacks.toml` stays written, and the message says to run `apply`. |
| D9 | `init` writes a repeated identical argument once. A missing required pack is left to the existing set check; `init` adds no dependencies. |
| D10 | The claude extension marks every hook command it writes with a trailing ` # agentstacks` shell comment. On each run it removes its marked handlers and adds the current ones. |
| D11 | A pack's `native/claude/<rel>` file lands at `.claude/<rel>`. |
| D12 | `{{var session_id}}` renders `${CLAUDE_SESSION_ID}`. A hook whose command or text contains it is skipped with a reason (§9.6). The other vars are listed in §9.1. |
| D13 | A `may_block` hook on an event Claude cannot block is installed. Its block becomes that event's feedback (§9.7). |
| D14 | Tests: pure Lua modules get `airsl test`; a Rust golden test runs the real host; `translate.lua` is run with `airsl run` on recorded payloads (§12). |
| D15 | A refusal inside a `path`-source pack or extension names the file by the path as written in `agentstacks.toml`, not by its absolute path (§8.3). |
| D16 | Only `hooks.<Event>` arrays that held or will hold the extension's handlers are rewritten. User handlers in those arrays may be reformatted (§9.5). |
| D17 | The core's `runtime.lua` calls `translate.result(result, payload)`, so the translator knows the event for neutral and native hooks alike (§8.4). |
| D18 | A message hook renders as `printf` of Claude's `additionalContext` JSON. A message hook on `session.end` is skipped, because Claude has no channel there that reaches the model (§9.4, settled after review). |
| D19 | `init` with git missing from `PATH` gives one merged refusal at the location `init`, naming every official argument (§4.3). |
| D20 | The library's port fakes compile under `#[cfg(any(test, feature = "testing"))]`. The CLI enables `testing` only as a dev-dependency feature (§2, §12, §13). |
| D21 | A lock read error in `confirm` refuses with `agentstacks.lock: <io::Error Display>`, exit 1, nothing applied (§5.3). |
| D22 | `update` failures follow `init`'s shape. A retag or write refusal exits 1 with the file unchanged. A failed apply or lock read after the rewrite adds `agentstacks.toml updated; fix the above and run apply` (§7). |
| D23 | An absent `.claude/settings.json` decodes to `{}` without calling decode (§9.5). |
| D24 | A native message hook on a Claude event that takes no `additionalContext` is skipped with a reason (§9.4, §9.6). |
| D25 | On `Stop` and `SubagentStop`, Claude treats `additionalContext` as feedback that keeps the agent running. So message hooks there are skipped (neutral `agent.stop`, native `Stop`/`SubagentStop`), and `translate.result` drops a `context` on `SubagentStop`. Only a `may_block` hook's `block` continues the agent (§9.4, §9.6, §9.7). |
| D26 | `retag` refuses when the tag at re-read differs from the listed current tag: `agentstacks.toml: packs.core: tag changed during update (run update again)` (§7). |
| D27 | A lua hook whose argv holds a `$` outside the rendered var forms is skipped, because `sh` would expand it (§9.4, §9.6). |

## 2. Architecture

The author approved this architecture on 2026-11-22, presented on its own. The D20 change (gated fakes, the `testing` feature) was presented on its own and approved on 2026-11-22 during the redesign.

```text
crates/agentstacks/src/                       library: rules and flows (additions only)
├─ types/project/render.rs   NEW  rule: the entries of a new agentstacks.toml → its text
├─ types/project/retag.rs    NEW  rule: agentstacks.toml text + (key, new tag) → text, comments kept
├─ types/shorthand.rs        NEW  rule: an init argument → an official git source or a path; the unknown-name refusal
├─ port/workspace            +    write_project(text), and its fake
├─ port/**/fake, port/mod    ~    fakes gated #[cfg(any(test, feature = "testing"))] instead of #[cfg(test)]   (D20)
├─ fs/workspace              +    write_project via temp + rename; the path-source tree label is the path as written
├─ app/init.rs               NEW  flow: refuse an existing file, resolve every argument, render, write
├─ app/update.rs             +    retag(): write the newer tags through types/project/retag
└─ assets/runtime.lua        ~    translate.result(result, payload)   (D17)

crates/agentstacks-cli/src/                   composition root and presentation; no rules
├─ main.rs     parse → run → ExitCode
├─ cli.rs      clap grammar
├─ wire.rs     builds the real adapters for the project directory
├─ command/    one file per subcommand: init.rs plan.rs apply.rs doctor.rs update.rs, plus confirm.rs
│              (the shared show → ask → apply sequence)
├─ show/       text only: plan.rs, diff.rs, doctor.rs, update.rs
├─ prompt.rs   Answer {Yes, No, NoTerminal}, from a terminal flag and a line reader (both injected)
└─ exit.rs     Outcome → ExitCode

extensions/claude/                            Lua, public api 0 only
├─ extension.toml
├─ adapter.lua      the only file that calls agentstacks.*; wires the lib modules
└─ lib/             pure modules; each has a *_test.lua beside it
   ├─ values.lua       the values result
   ├─ frontmatter.lua  meta + native → YAML frontmatter text
   ├─ hooks.lua        a payload hook → {event, matcher, command}, quoting, the marker, the session_id check
   ├─ settings.lua     the decoded old settings + our groups → per-event edits
   └─ translate.lua    returns the text of <root>/translate.lua
```

**Dependency direction:**

```text
agentstacks-cli → app → port ← fs, process, host;  app → types   (unchanged rule, docs/architecture.md)
extensions/claude: adapter.lua → agentstacks.* (api 0) and lib/*;  lib/* → airsstack.json only
```

**Why each new unit exists:**

| Unit | Why it exists apart from its siblings |
|---|---|
| `types/project/render` | Writing a project file is a rule over `ProjectFile` entries; `ProjectFile` only parses |
| `types/project/retag` | Changing a tag in place without losing comments is a separate rule from rendering a new file |
| `types/shorthand` | The official repository, tag prefix and name list live in one place, as formats §7 requires |
| `app/init` | The order of steps (check → resolve → render → write) is a flow, and flows live in `app/` |
| `command/confirm` | `init`, `apply` and `update` share one show → ask → apply sequence |
| `show/*` | Presentation is kept apart from the flows; each renderer is unit-tested on `Plan::new` values (`src/types/plan/built.rs:35`, `pub fn new`) |
| `prompt` | Terminal detection and line reading are isolated so commands are tested without a terminal |
| `testing` feature (D20) | Command tests need the library's port fakes. One copy is reused rather than a CLI-local duplicate (modularity Rule 2). The fakes import only `std` and crate items, no dev-dependency (`src/port/fake.rs:19-25`, `src/port/git/fake.rs:23-29`, `src/port/adapter/fake.rs:26-36`, `src/port/binary/fake.rs:19-25`, `src/port/workspace/fake.rs:23-31`). Compiled under `--features testing`, the real fakes pass clippy `-D warnings` and rustdoc `-D warnings` (`probes/redesign-results.md`, G2b). A dev-dependency feature stays off in a normal or release build of the binary (`probes/redesign-results.md`, G2: `main TESTING=false`, `test TESTING=true`) |
| `lib/*.lua` | Pure Lua that `airsl test` can load, which requires no `agentstacks.*` table (`probes/airsl-test-results.md`) |

**Rust guideline rules applied** (claudestacks-guideline-rust 0.1.6):

| Rule | Applied |
|---|---|
| `modularity.md` Rule 1 | each unit above states one job; the CLI holds no rule |
| `modularity.md` Rule 2 | `Plan`, `Finding`, `UpdateEntry`, `Diagnostic` and `Owner` are reused, never copied |
| `static-dispatch.md` | `app/init` is generic over `W: Workspace, G: Git`. Each `command/*` function is generic over the ports it uses (`W: Workspace`, `G: Git`, `B: Binary`, `A: Adapter`). `main` passes the concrete adapters from `wire`, and tests pass the library fakes (D20). No new `dyn` |
| `mod-rs-export-only.md` | `command/mod.rs` and `show/mod.rs` hold only docs and `mod` / `pub use` |
| `strong-types.md` | arguments become `ToolName` / `PackName` / `LocalPath`; `Answer` and `Outcome` are enums, not `bool` |
| `unit-test-mandate.md` | every logic file has colocated tests (§12) |
| naming (project rule) | one-word file names. `*_test.lua` is the only exception, because `airsl test` finds only files named `*_test.lua` or `test_*.lua` (`probes/cli2-results.md`, T4, `airsl test --help`) |

## 3. Command line

```text
agentstacks [-C <DIR>] <COMMAND>
  init   --tool <TOOL>... --pack <PACK>... [--yes]
  plan
  apply  [--yes]
  doctor
  update [--yes]
```

- `--tool` and `--pack` are each required at least once.
- `-C` works before or after the subcommand (`probes/cli-results.md`, R1).

**clap behaviour** (`probes/cli-results.md` R1; `probes/cli2-results.md` T2 with `value_name`):

| Case | Exit | Stream | Text |
|---|---|---|---|
| a required argument missing (`init --tool claude`) | 2 | stderr | `error: the following required arguments were not provided:\n  --pack <PACK>\n\nUsage: agentstacks init --tool <TOOL> --pack <PACK>\n\nFor more information, try '--help'.` |
| unknown subcommand | 2 | stderr | `error: unrecognized subcommand 'frobnicate'` |
| no subcommand | 2 | stderr | full help |
| `--help` | 0 | stdout | help |
| `--version` | 0 | stdout | version |

The value names are `<TOOL>`, `<PACK>` and `<DIR>` (`value_name`).

**Streams:**

| Stream | Output |
|---|---|
| stdout | the plan; the `doctor` report; the `update` list; `nothing to change`; `nothing to update`; `agentstacks.toml updated`; `applied: …` |
| stderr | refusals; the prompt; `stdin is not a terminal; pass --yes to apply`; `nothing applied`; the `agentstacks.toml written; …` and `agentstacks.toml updated; …` follow-ups |
- Refusals are printed with `Diagnostics::into_sorted`, one `Diagnostic` Display per line (`src/types/report/diagnostic.rs:71`).

**Exit codes (D3):**

| Code | When |
|---|---|
| 0 | success: a plan shown, an apply done, nothing to change, a declined prompt, a healthy `doctor`, an `update` with nothing newer |
| 1 | any diagnostic, any `doctor` finding, `stdin is not a terminal; pass --yes to apply` |
| 2 | clap usage error |

**`-C <dir>`:** the project is `<dir>`, resolved from the current directory. When it cannot be opened (`StdWorkspace::open`, `src/fs/workspace/project.rs:66`), the CLI prints `<dir>: <io::Error Display>` and exits 1.

## 4. `init`

### 4.1 Flow (`app/init`)

```text
1 agentstacks.toml exists (Workspace::project_text Ok)   → refused, nothing written
  project_text Err other than NotFound                   → agentstacks.toml: <io::Error Display>
2 classify each --tool and --pack argument (types/shorthand)
3 resolve: an official name → latest packs-v* tag (Git::list_tags + types::resolve::latest_tag)
           a path → read that directory's manifest name (Workspace::source_tree, then its manifest parser)
4 collect every refusal of steps 2-3; git missing from PATH is merged into one refusal (D19); if any, stop, nothing written
5 render agentstacks.toml (types/project/render), Workspace::write_project
6 the CLI runs plan → show → ask → apply (§5)
```

### 4.2 Arguments (`types/shorthand`)

| Argument | Meaning |
|---|---|
| contains `/` or starts with `.` | a project-relative path (`LocalPath`, formats §7) |
| anything else | an official name |

**Official names:**

- packs: `core`, `sdlc`, `journal`, `guideline-rust`, `guideline-python`, `cmux` (the six packs the milestone ships);
- tools: `claude`.

**An official name expands to (formats §7):**

| Field | Pack | Tool |
|---|---|---|
| `git` | `https://github.com/airsstack/agentstacks` | same |
| `tag` | the latest `packs-v*` tag | same |
| `subdir` | `packs/<name>` | `extensions/<name>` |

**A path is written as given:** `path = "<argument>"`. The table key is the `name` in that directory's `pack.toml` or `extension.toml`.

| Case | Result |
|---|---|
| the same argument twice | written once (D9) |
| two arguments that resolve to the same key (`--pack core --pack ./packs/core`, whose pack.toml says `core`) | refused at the later one: `--pack "./packs/core": pack "core" is already given by --pack "core"` (tool form: `--tool "./ext": tool "claude" is already given by --tool "claude"`) |
| an official-looking name that is a directory (`core` while `./core` exists) | still a name: only `/` or a leading `.` makes a path |

### 4.3 Refusals

Each refusal is a `Diagnostic`, displayed as `<location>: <what>`. The location is the one each row shows: the flag and its argument as written, `agentstacks.toml`, the path as written, or the loader's own label (§10).

The unknown-name message replaces the formats §7 wording (`"x" is not an official pack; pass a git URL or a path`), because D4 drops git URLs as arguments.

| Rule | Owner | Message |
|---|---|---|
| file exists | `app/init` | `agentstacks.toml: already exists (edit it and run apply)` |
| unknown name | `types/shorthand` | `--pack "x": not an official pack; pass a path, or add a git source to agentstacks.toml` / `--tool "x": not an official tool; pass a path, or add a git source to agentstacks.toml` |
| no release tag | `app/init` | `--pack "core": no packs-v release tag in https://github.com/airsstack/agentstacks` |
| listing tags fails (other than git missing) | `app/init` | `--pack "core": ` + the `FetchError` Display that `Git::list_tags` returns (`src/port/git/seam.rs:44`, `src/types/resolve/error.rs:42`) |
| git missing from `PATH` (D19) | `app/init` | one refusal, `init: git not found on PATH (needed by --tool "claude", --pack "core")` (`probes/redesign-results.md`, G1a) |
| path absolute / climbs out | `types/shorthand` (`LocalPath` messages, formats §7) | `--pack "/x": absolute paths are not portable` / `--pack "../x": must stay inside the project` |
| path resolves outside through a symlink (`source_tree` returns `None`) | `app/init` | `--pack "./x": resolves outside the project` |
| path I/O | `app/init` | `<path as written>: <io::Error Display>` (core R1) |
| not a pack / not an extension, or a bad manifest | existing loaders (formats §2.1, §4.5), labelled per D15 | e.g. `packs/x: not a pack (no pack.toml)` |
| duplicate key | `app/init` | §4.2 |
| write fails | `fs` | `agentstacks.toml: <io::Error Display>` |

**Git missing (D19):**

- `Git::list_tags` reports git missing as `FetchError::GitNotFound` with an empty `needed_by` on every call, and the flow merges them (`src/port/git/seam.rs:25-26`). Passed through per argument, it would read `--pack "core": git not found on PATH (needed by )` (`probes/redesign-results.md`, G1b).
- `app/init` therefore collects every official argument that met `GitNotFound`. It emits one `Diagnostic` at `Location::file("init")` whose `needed_by` holds each argument's flag text (`--tool "claude"`), tools first, then packs, each in the order given. Repeated identical arguments are collapsed first (D9), so each appears once.
- This mirrors `app::update` (`src/app/update.rs:74-82`), which merges at `agentstacks.toml`. `init` has no file yet, so the location is `init`.

The repository is private today, so the official shorthand cannot list its tags yet (core §17). `init` passes git's own refusal through. Making the repository public belongs to `release-and-migration`.

### 4.4 The rendered file (`types/project/render`)

- Built with toml_edit 0.25.20.
- `tools` and `packs` are implicit tables, so no empty `[tools]` header is written (`probes/cli-results.md`: without `set_implicit` the output starts the tools section with an empty `[tools]` table).
- Tools come first in name order, then packs in name order.
- Keys are written as `key = value` in the order `git`, `tag`, `subdir` (or `path`), with a blank line before each table.

Exact bytes for `--tool claude --pack core --pack sdlc` (`probes/cli-results.md:407`):

```text
"api = 0\n\n[tools.claude]\ngit = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\nsubdir = \"extensions/claude\"\n\n[packs.core]\ngit = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\nsubdir = \"packs/core\"\n\n[packs.sdlc]\ngit = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\nsubdir = \"packs/sdlc\"\n"
```

That text parses with `toml::from_str::<toml::Table>` (R2 (c)). A path source renders `[packs.team]\npath = "packs/team"\n`.

### 4.5 After the file is written (D8)

| Outcome | stderr | Exit |
|---|---|---|
| plan refused | the refusals, then `agentstacks.toml written; fix the above and run apply` | 1 |
| declined | `agentstacks.toml written; nothing applied (run apply)` | 0 |
| no terminal, no `--yes` | `stdin is not a terminal; pass --yes to apply`, then `agentstacks.toml written; nothing applied (run apply)` | 1 |
| applied | as §5.4 | 0 |
| apply failed, or the lock read failed (D21) | the diagnostics (§5.3, §5.4), then `agentstacks.toml written; fix the above and run apply` | 1 |

## 5. `plan` and `apply`

### 5.1 Flow

- `plan` calls `app::plan::plan` with the real adapters (`src/app/plan/flow.rs:59`), prints the plan (§5.2) and exits 0. It writes nothing.
- `apply` does the same, then runs the confirmation (§5.3). On yes it calls `app::apply::apply(&plan, &workspace)` (`src/app/apply/flow.rs:35`) on the same `Plan` it showed. It never plans twice (core §7).

A refused plan prints its refusals and exits 1.

### 5.2 Plan text (`show/plan`)

```text
agentstacks plan

  create   .claude/skills/status/SKILL.md   core
  update   .claude/agentstacks/runtime.lua  claude
  delete   .claude/agents/old.md            team-lint
  update   .claude/settings.json            shared
    --- .claude/settings.json
    +++ .claude/settings.json
    @@ -3,5 +3,17 @@
    …
  skip     core/hook/2: claude gives hooks the session id in the event (event.session_id), not as {{var session_id}}

  airsl    0.1.4 (pinned)
  note     native/cluade: not for claude

3 to write, 1 to delete, 12 unchanged, 1 skipped
```

(The `…` stands for the rest of the diff; the rows are exact.)

**Rows:**

| Row | Rule |
|---|---|
| step | Every step whose `Action` is `Create`, `Update`, `Restore`, `Delete` or `Forget` (`src/types/plan/action.rs:24`), in the plan's path order. `Unchanged` is counted, not listed. Format: two spaces, the action in lower case padded to 8, a space, the path padded to the longest listed path, two spaces, then the owner. The owner is `Owner` Display, or `shared` when `Step.owner` is `None` (`src/types/plan/step.rs`). |
| diff | Below an `Update` step whose `old` is `Some`. similar 3.2.1 `TextDiff::from_lines(old, new).unified_diff().context_radius(3).header(path, path)`, each line indented four spaces. Bytes that are not UTF-8 are decoded lossily. Identical texts give no lines (R3). |
| skip | `  skip     <subject>: <reason>`. The subject is the item id, or `<pack>/<rel>` for a core root-file skip (`src/types/op/skip.rs:73`). The reason is `SkipReason` Display. |
| airsl | `  airsl    <v> (pinned)` when the locked and installed versions are equal; `  airsl    <locked> → <installed>` when they differ; `  airsl    <installed> (new pin)` when the lock had none (`AirslPin`, `src/types/plan/note.rs:79`). |
| binary | none. Binary refusals already stop the plan, and found binaries are not listed. |
| note | `  note     <Note Display>` (`src/types/plan/note.rs:59`) |
| summary | `<w> to write, <d> to delete, <u> unchanged, <s> skipped`, then `, <f> forgotten` when `f` > 0. `w` counts Create + Update + Restore; `d` counts Delete; `u` counts Unchanged; `s` counts skips; `f` counts Forget. |

Empty sections are left out, together with the blank line before them.

### 5.3 Confirmation (`command/confirm`, `prompt`)

```text
nothing to change? ── yes ─► print plan + "nothing to change", exit 0, no question, nothing written
       │ no
--yes? ── yes ─► apply
       │ no
stdin a terminal? ── no ─► "stdin is not a terminal; pass --yes to apply", exit 1
       │ yes
"Apply? [y/N] " on stderr, read one line
  trimmed, case-insensitive "y" or "yes" ─► apply
  anything else, empty, end of input, or a read error ─► "nothing applied", exit 0
```

- **Nothing to change** means no listed step, and `write_lock(plan.lock())` (`src/types/lock/write.rs:57`) equals the current lock text (an absent lock is never equal).
- **Lock read error (D21):** the current lock text is read with `Workspace::lock_text`. An `Err` prints `agentstacks.lock: <io::Error Display>`, the same location and shape `plan` uses (`src/app/plan/read.rs:58-59`). The command exits 1, asks nothing and applies nothing. `init` and `update` add their follow-up line (§4.5, §7).
- `IsTerminal` on a piped stdin and on `/dev/null` is `false`, and `read_line` on `/dev/null` returns `Ok(0)` (`probes/cli-results.md`, R4).

### 5.4 After apply

| Result | Output | Exit |
|---|---|---|
| `Ok` | stdout `applied: <w> written, <d> deleted` | 0 |
| `Err` | the diagnostics (core §7, e.g. `<path>: changed after plan; run plan again`) | 1 |

## 6. `doctor`

- Calls `app::doctor::doctor` (`src/app/doctor.rs:39`).
- Prints each `Finding` Display on stdout, one per line, in the order returned (`src/types/plan/finding.rs:31`, e.g. `changed: .claude/agents/coder.md`, `airsl: 0.1.5 installed, lock pins 0.1.4`).
- With no findings it prints `healthy` and exits 0. Any finding exits 1.

## 7. `update`

```text
1 app::update::update → entries (src/app/update.rs:43); refused → print, exit 1
2 print each entry on stdout: NewTag → "  <key>: <current> → <new>" ; Note → "  note     <UpdateNote Display>"
3 no NewTag → print "nothing to update", exit 0, nothing written
4 app::update::retag(workspace, entries) → agentstacks.toml rewritten (types/project/retag); stdout "agentstacks.toml updated"
  refused → print the diagnostics, exit 1, the file unchanged, no plan (D22)
5 plan → show → confirm → apply exactly as `apply` (§5), then the follow-up on stderr (D22)
```

| Step 5 outcome | stderr follow-up | Exit |
|---|---|---|
| plan refused, apply failed, or lock read failed | `agentstacks.toml updated; fix the above and run apply` | 1 |
| declined | `agentstacks.toml updated; nothing applied (run apply)` | 0 |
| no terminal, no `--yes` | `stdin is not a terminal; pass --yes to apply`, then `agentstacks.toml updated; nothing applied (run apply)` | 1 |
| applied, or nothing to change | none (§5.3, §5.4) | 0 |

**Step 4 refusals (D22):**

| Case | Message | Owner |
|---|---|---|
| re-read fails | `agentstacks.toml: <io::Error Display>` | `app/update::retag` |
| re-read text no longer parses | the `ProjectFile::parse` refusals, e.g. `agentstacks.toml:1: unclosed table, expected `]`` (`probes/redesign-results.md`, second run) | `app/update::retag` |
| a tag missing at re-read | `agentstacks.toml: packs.core: no tag to update (run update again)` | `types/project/retag` |
| a tag at re-read that differs from the entry's current tag (D26) | `agentstacks.toml: packs.core: tag changed during update (run update again)` | `types/project/retag` |
| write fails | `agentstacks.toml: <io::Error Display>` | `app/update::retag` |

`retag` checks the re-read text with `ProjectFile::parse` before editing it with toml_edit. Both use one parser, `toml_parser` (`cargo tree -i toml_parser`), and gave byte-identical errors on three bad inputs (`probes/redesign-results.md`). So text that `ProjectFile::parse` accepts also parses in toml_edit. A toml_edit parse error after that check is still refused, as `agentstacks.toml: ` + its Display, printed as its multi-line text (`probes/redesign-results.md`, P3).

- `<key>` is `SourceKey` Display (`packs.core`, `tools.claude`, `src/types/resolve/key.rs:64`).
- `UpdateNote` Display is the core's text, e.g. `agentstacks.toml: packs.core: no newer tag than packs-v0.1.0` (`src/types/resolve/update.rs:77`).

**Retag rule:**

- Find the old value with `get_mut` at each level, never with chained `IndexMut`. Indexing a missing pack with `IndexMut` writes `[packs]\nnope = {}` into the document (`probes/cli2-results.md`, T1 (d)).
- Compare the old value with the entry's current tag. A difference is refused (D26), and nothing is written. Otherwise set `<packs|tools>.<name>.tag` to the new tag, and copy the old value's decor onto the new value. A plain assignment drops the trailing comment (`# pinned` was lost); copying the decor keeps it, along with the alignment and every other comment (`probes/cli-results.md`, R2 (a) and (a2)).
- This works the same for a standard table, an inline table and dotted keys: only the tag text changes, and the trailing comments survive (`probes/cli2-results.md`, T1 (a)-(c)).
- The value missing at re-read (the file changed during `update`) is refused: `agentstacks.toml: packs.core: no tag to update (run update again)` (same shape for `tools.*`). Owner: `types/project/retag`.

**Ordering:** after step 4, `agentstacks.toml` names a new tag while the lock pins the old rev. The plan reconciles that entry as stale and resolves it again (core §9.1), so no special case is needed.

## 8. Library changes

### 8.1 `Workspace::write_project(text)`

- `fs`: write `.agentstacks.toml.agentstacks-tmp` beside the file, then rename it over `agentstacks.toml`, keeping the old mode as the lock does (core R21).
- Refusal: `agentstacks.toml: <io::Error Display>`.
- The fake records the text.

### 8.2 `app/init`, `app/update::retag`

- The flows are as §4.1 and §7.
- `retag` receives the `UpdateEntry` list, re-reads the text through `Workspace::project_text`, checks it with `ProjectFile::parse`, applies every `NewTag` and writes once. With no `NewTag` it writes nothing. Refusals are as §7 (D22).

### 8.3 Path-source labels (D15)

- **Today:** `StdWorkspace::source_tree` builds `StdFileTree::new(dir)` on the canonical directory (`src/fs/workspace/project.rs:135-143`). Messages from that tree are therefore absolute: `/private/tmp/…/host-project/packs/core/native/probe/hooks.toml: hooks[0].match: expected array, got string` (`probes/host-results.md`).
- **Change:** label the tree with the `LocalPath` as written, so the same refusal reads `packs/core/native/probe/hooks.toml: hooks[0].match: expected array, got string`. Reads still go to the canonical directory.
- This mirrors core R22 for git sources.

### 8.4 `runtime.lua` (D17)

```diff
-local out = translate.result(result)
+local out = translate.result(result, payload)
```

- **Why:** `airsstack.hook.payload()` returns the payload only once; a second call gives `nil` (`probes/payload-once-results.md`). For a native hook, `translate.event` is never called (core §11.3).
- **Proof:** the changed text ran on airsl 0.1.4 for both kinds and printed `{"context":"ctx","event":"PreToolUse"}` (`probes/runtime-payload-results.md`).
- **Translator contract** (core §11.4, `docs/hooks.md`): it becomes `M.result(result or nil, payload)`.

## 9. The claude extension

### 9.1 `extension.toml` and `values`

```toml
[extension]
name        = "claude"
version     = "0.1.0"
api         = 0
description = "Renders packs into Claude Code project config"
entry       = "adapter.lua"
reads       = [".claude/settings.json"]
```

**`values` result:**

- `root = ".claude/agentstacks"`.
- `skills["<p>/<s>"] = { invoke = "/<s>", dir = ".claude/skills/<s>" }`. Project skills live at `.claude/skills/<name>/SKILL.md` and are invoked as `/<name>` (`probes/docs/skills.md:164`, `:445`).
- `agents["<p>/<a>"] = "<a>"`. Project agents live at `.claude/agents/<name>.md` (`probes/docs/sub-agents.md:155-167`).
- `vars` holds only the vars named in the payload's `vars`, because any other key is refused (core §4.4).

| Var | Value | Proof |
|---|---|---|
| `session_id` | `${CLAUDE_SESSION_ID}` | substituted in skills: `ARGS=hello world SID=d6590e86-…` equal to the payload's `session_id` (`probes/claude-results.md:236`) |
| `arguments` | `$ARGUMENTS` | same run |
| `project_dir` | `${CLAUDE_PROJECT_DIR}` | a skill substitution (`probes/docs/skills.md:494`); set in hook commands (`probes/claude-env-results.md`) |
| `tmpdir` | `$TMPDIR` | set in hooks (`probes/claude-env-results.md`) |
| `home` | `$HOME` | same |
| `airsstack_home` | `${AIRSSTACK_HOME:-$HOME/.airsstack}` | the plugins' default (`probes/claude-env-results.md`, quoting `discuss.lua:57`) |

In skill and agent text, the shell forms are read by the model, which uses them in its Bash calls.

### 9.2 Frontmatter (`lib/frontmatter`)

- **Shape:** `---\n`, then one `key: <value>\n` line per key, then `---\n`, then the payload's `text`, which is the body after the closing `---` line (in the fixture it starts with the blank line that followed, `probes/host-results.md`).
- **Native keys:** a native key is written as is when it matches `^[A-Za-z0-9_-]+$`. Otherwise the item is skipped: `native.claude key "<key>" cannot be written as frontmatter`.
- **Native values:** a decoded JSON `{}` encodes back as `{}`, and `[]` as `[]` (`probes/settings-results.md`, S6).
- **Values:** every value is `airsstack.json.encode(value)`. A string is a JSON double-quoted string, which Claude Code loads as YAML, including an escaped `\"` (`probes/claude-results.md`, Q2). A list is a JSON flow list, and `tools: ["Read", "Bash"]` limits the agent's tools to exactly those (`probes/claude-agent-json-results.md`).
- **Key order:** the generated keys in the order below, then `native` keys that were not generated, sorted by name.
- **Native wins:** `meta.native` holds only this tool's entry (`probes/host-results.md`). Its keys replace a generated key of the same name.

| Skill (neutral) | Claude key | Proof |
|---|---|---|
| name | `name` (from the payload `name`; `meta` has none) | skills.md:401-422 |
| `description` | `description` | same |
| `invocation: user` | `disable-model-invocation: true`; `any` writes nothing | same |
| `argument_hint` | `argument-hint` | same |

| Agent (neutral) | Claude key | Proof |
|---|---|---|
| name | `name` | sub-agents.md:298-316 |
| `description` | `description` | same |
| `tools` | `tools`: read→Read, write→Write, edit→Edit, grep→Grep, glob→Glob, shell→Bash, skill→Skill, in the written order; absent writes nothing (all tools) | sub-agents.md:300; tools-reference.md |
| `model` | `model`: fast→`haiku`, balanced→`sonnet`, deep→`opus`; absent writes nothing | sub-agents.md:302 |
| `effort` | `effort`, same word | sub-agents.md:312 (low, medium, high, xhigh, max) |

On macOS and Linux, `Grep` and `Glob` are not in Claude's default tool set, and searches go through Bash (`probes/docs/tools-reference.md:36-37`). The mapping stays one to one, and the extension adds no tools.

**Limit:** an agent whose `tools` maps only to `Grep` and `Glob` usually fails to launch there (`probes/docs/sub-agents.md:300`). That depends on Claude's platform default, not on the pack, so it is documented, not skipped. All ten plugin agents include `shell`.

### 9.3 Files

| Item | Operation |
|---|---|
| skill `SKILL.md` | `write{ path = <dir>.."/SKILL.md", content = frontmatter .. text, covers = {id} }` |
| other skill file | `copy{ path = <dir>.."/"..rel, file = id }` (binary included) |
| agent | `write{ path = ".claude/agents/"..name..".md", content = frontmatter .. text, covers = {id} }` |
| native file | `copy{ path = ".claude/"..rel, file = id }` (D11) |
| hook, native hook | covered by the `.claude/settings.json` write (§9.5), or skipped (§9.6). Groups are added in payload order: packs in payload order; within a pack, `hooks` by index, then native hooks by index |
| `<root>/translate.lua` | written when at least one lua hook (neutral or native) is written, with the text from `lib/translate` (§9.7) |

Two packs that ship the same skill or agent name propose one path twice, which the core refuses (core §3.3, `path ".claude/skills/x/SKILL.md" is already proposed at adapter.lua:<line>`). The extension adds no check of its own.

### 9.4 Hooks (`lib/hooks`)

**Event and matcher.** Each hook becomes its own group, `{ matcher = <m> or absent, hooks = { { type = "command", command = <c> } } }`.

| Neutral event | Claude event | Matcher |
|---|---|---|
| `session.start` | `SessionStart` | `match` joined with `\|`; absent → no matcher |
| `session.end` | `SessionEnd` | none |
| `prompt.submit` | `UserPromptSubmit` | none (Claude ignores one, hooks.md:313) |
| `tool.before` | `PreToolUse` | `match` mapped read→Read, edit→Edit, write→Write, joined with `\|`; absent → `Read\|Edit\|Write` |
| `tool.after` | `PostToolUse` | same |
| `agent.stop` | `SubagentStop` | `agents` joined with `\|`; absent → no matcher |
| native hook | its `event` as written | its `match` list joined with `\|`; absent → none |

- A matcher made only of letters, digits, `_`, `-` and `|` is an exact list (`probes/docs/hooks.md:279-285`). `SubagentStop` matches on `agent_type`, the agent's `name`; the `probe-agent` matcher fired (`probes/claude-results.md:194`).
- The schema is `{"hooks":{"<Event>":[{"matcher","hooks":[{"type":"command","command"}]}]}}` (`probes/docs/hooks.md:321-337`).

**Command.**

| Kind | Command |
|---|---|
| message (D18) | `printf '%s\n' '<json>' # agentstacks`, where `<json>` is `airsstack.json.encode({ hookSpecificOutput = { hookEventName = <Claude event>, additionalContext = <text> } })` and each `'` in it becomes `'"'"'` |
| lua (neutral or native) | `cd "$CLAUDE_PROJECT_DIR" && ` + each argv element in double quotes, joined by spaces, + ` # agentstacks` |

**Message hooks:**

- Of the neutral events' Claude forms, `additionalContext` is accepted on `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse` and `SubagentStop` (`probes/docs/hooks.md:1179-1185`). The JSON form reached the model on `SessionStart` (`PLUM`, `probes/claude-results.md:350`).
- Claude adds plain stdout to the context only on `UserPromptSubmit`, `UserPromptExpansion`, `SessionStart` and `PostModelSwitch` (`probes/docs/hooks.md:837-840`). That is why `echo` is not used, and why `session.end` is skipped (§9.6).
- **Native message hooks (D24):** these may name any Claude event. Claude places `additionalContext` only on `SessionStart`, `SubagentStart`, `UserPromptSubmit`, `UserPromptExpansion`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PostToolBatch`, `Stop`, `SubagentStop` and `PostModelSwitch` (`probes/docs/hooks.md:1179-1185`). A native message hook on one of these renders as above, except on `Stop` and `SubagentStop` (D25). On any other event it is skipped (§9.6).
- **Stop and SubagentStop (D25):**
  - On these two events, `additionalContext` "keeps the subagent running" (`probes/docs/hooks.md:2736`). It "keeps the conversation going through the same loop protections as `decision: "block"`" (`:2944`), up to an 8-consecutive-continuation cap (`:2865`).
  - A static `printf` cannot read `stop_hook_active`, so a message hook there would act as a block every time it fires. Formats §6.2 says message hooks cannot block.
  - So message hooks on neutral `agent.stop` and on native `Stop`/`SubagentStop` are skipped (§9.6).
- `printf '%s\n'` with single-quoted JSON prints the JSON unchanged, including an escaped `'` and a `\` (`probes/cli2-results.md`, T3).

**Lua hooks:**

- Inside double quotes, `\`, `"` and `` ` `` are escaped with `\`, and `$` is left alone so `$TMPDIR` expands. The argv arrives with `$TMPDIR` literal (`probes/host-results.md`).
- **Example:** `cd "$CLAUDE_PROJECT_DIR" && "airsl" "run" "--fail-open" "--policy" "confined" "--allow-read" ".claude/agentstacks/core" "--allow-write" "$TMPDIR" "--allow-env" "HOME" ".claude/agentstacks/runtime.lua" "core.hooks.enforce" "0" "neutral" "--x" "a \"q\" \\b \`c\`" # agentstacks`.
- Run under `sh -c` with a stub `airsl` that prints its arguments, this example ran in `$CLAUDE_PROJECT_DIR` and expanded `$TMPDIR`. The last argument arrived as `a "q" \b `c``, and `# agentstacks` was not passed as an argument (`probes/cli2-results.md`, T3).
- **Literal `$` (D27):** `sh` expands every `$` inside double quotes. So for each argv element, every rendered var form from the §9.1 table (the values present in `values.vars`) is removed, longest form first, so `${AIRSSTACK_HOME:-$HOME/.airsstack}` goes before `$HOME`. If a `$` remains, the hook is skipped (§9.6). The var forms themselves are meant to expand.
  - Example: `"^a$"` is skipped; `"$TMPDIR/x"` is rendered.

**Both kinds:**

- Hook commands run under `sh -c` on macOS and Linux (`probes/docs/hooks.md:464-466`). The parent process was `/bin/sh` (`probes/claude-results.md:163`).
- The ` # agentstacks` comment does not change a command: `sh -c 'echo hi # agentstacks'` printed `hi` (`probes/cli2-results.md`, T3), and a Claude hook ending in the marker ran (`probes/claude-env-results.md`).

### 9.5 `.claude/settings.json` (`lib/settings`, D10, D16)

1. `old = agentstacks.read(".claude/settings.json")`.
2. **Start text:** `"{}\n"` when `old` is `nil`, otherwise `old`. With `"{}\n"` the result ends in `}\n`; with `""` it would end without a newline (`probes/settings-results.md`, S3).
3. **Validation** (when `old` is not `nil`): `agentstacks.json.edit(old, {"hooks", "agentstacks-check"}, nil)`, uncaught. It returns the text unchanged, or raises the core's located message (core §5, R14, R19):
   - `claude/adapter.lua:<line>: json.edit: <serde_json message>` (invalid JSON);
   - `claude/adapter.lua:<line>: json.edit: top level must be an object`;
   - `claude/adapter.lua:<line>: json.edit: "hooks" is not an object` (`hooks` is a number, a list, …; `probes/settings-results.md`, S5).
   Any of these refuses the whole plan, a removed tool's run included. Such a file is not valid Claude settings.
4. **Decode:** `airsstack.json.decode(old)`. When `old` is `nil` (D23), empty or whitespace-only, the decoded value is `{}` and decode is not called. Decode fails on empty text (`EOF while parsing a value`, S1), while `json.edit` treats it as `{}` (core §5).
5. **Events touched:** every event whose old array holds a marked handler, plus every event we add to.
6. **Rewritable check** for each touched event `E`. The array is rewritable when:
   - it is a list of tables;
   - each group's `hooks` is a list of tables;
   - each handler's `command` is a string;
   - no value anywhere inside it is a JSON `null` or an empty list.

   Why: `null` decodes to a userdata that `json.edit` refuses (`json.edit: value: userdata cannot be JSON`), and an empty list comes back from `json.edit` as `{}` (S1, S2). A list is a table whose keys are exactly `1..n`, or an empty table carrying the metatable decode gives a JSON array (S1). When `E` is not rewritable, it is left untouched, and each of our hooks for `E` is skipped (§9.6).
7. **For each rewritable touched `E`:**
   - Every handler whose `command` ends with ` # agentstacks` is removed.
   - A group whose handlers were all removed is dropped.
   - Our groups are appended (§9.3 order).
   - An empty result removes the key (`nil`).
8. **Edit:** each such `E` is written with `agentstacks.json.edit(text, {"hooks", E}, value)`, in event-name order.
9. **When to write:** propose `{ path = ".claude/settings.json", content = text, covers = <every hook id written> }` when `old` is not `nil` or there is at least one edit. With `old` `nil` and no hook, nothing is proposed.

**Checked example** (`probes/settings-results.md`, S4; run on the real host). Old text:

```json
{
  "permissions": {
    "allow": ["Bash(ls:*)"]
  },
  "hooks": {
    "SessionEnd": [
      { "hooks": [ { "type": "command", "command": "echo user-hook", "timeout": 5 } ] },
      { "hooks": [ { "type": "command", "command": "echo old # agentstacks" } ] }
    ]
  }
}
```

The example adds `PreToolUse` with one group and rewrites `SessionEnd` with the marked handler replaced. Result:

```json
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
```

What the example shows:

- Existing keys keep their position, and `PreToolUse` lands last.
- Values inside a rewritten array have their keys sorted, so the user handler reads `command, timeout, type` (D16).
- The whole file is pretty-printed: the one-line `allow` list is expanded (core §5).

**Edge rules:**

| Case | Result |
|---|---|
| `E` not rewritable (§9.5 step 6) | `E` untouched; our hooks for `E` skipped; marked handlers already there stay |
| `hooks` left empty | stays `"hooks": {}` |
| removed tool (core D8: zero packs) | the same steps with no groups of ours: marked handlers are removed from rewritable events |

`.claude/settings.json` is the committed project file (`probes/docs/settings.md:28`). Hooks from all settings files merge (`probes/docs/hooks.md:266`). `-p` sessions treat the folder as trusted, while interactive sessions hold hooks back until the trust dialog is accepted (`probes/docs/hooks.md:4300-4303`).

### 9.6 Skips

- A hook (message text or lua argv) that contains `${CLAUDE_SESSION_ID}` is skipped: `skip{ item = id, reason = "claude gives hooks the session id in the event (event.session_id), not as {{var session_id}}" }`. Hooks see `CLAUDE_CODE_SESSION_ID`, not `CLAUDE_SESSION_ID` (`probes/claude-env-results.md`).
- A message hook on `session.end` is skipped (D18): `skip{ item = id, reason = "claude shows no hook output to the model at session end" }`.
- A message hook on neutral `agent.stop`, or a native message hook on `Stop` or `SubagentStop`, is skipped (D25): `skip{ item = id, reason = "claude treats hook output at agent stop as feedback that keeps the agent running" }`.
- A lua hook with a literal `$` in its argv is skipped (D27): `skip{ item = id, reason = "claude runs hook commands through sh, which would expand the literal $ in argument \"<arg>\"" }`, where `<arg>` is the argv element as rendered, e.g. `claude runs hook commands through sh, which would expand the literal $ in argument "^a$"`.
- A native message hook whose `event` is not in the D24 list is skipped: `skip{ item = id, reason = "claude passes no hook message to the model on <Event>" }`, where `<Event>` is the event as written, e.g. `claude passes no hook message to the model on Notification`.
- A hook whose Claude event array is not rewritable is skipped (§9.5 step 6): `skip{ item = id, reason = ".claude/settings.json: hooks.<E> holds entries the claude extension cannot rewrite (a null, an empty list, or a value that is not a list of hook groups)" }`.
- A skill or agent with a native key that is not a plain word is skipped (§9.2): `skip{ item = id, reason = "native.claude key \"<key>\" cannot be written as frontmatter" }`. For a skill, the item is the skill group.
- Every other neutral field and event has a Claude form (§9.2, §9.4), so nothing else is skipped.

### 9.7 `translate.lua`

**`M.event(p)`:**

| `p.hook_event_name` | Neutral event | Extra fields |
|---|---|---|
| `SessionStart` | `session.start` | `source` = `p.source` when it is one of startup, resume, clear, compact; otherwise `nil` (Claude also sends `fork`, hooks.md:1311-1318) |
| `SessionEnd` | `session.end` | — |
| `UserPromptSubmit` | `prompt.submit` | `prompt` |
| `PreToolUse` / `PostToolUse` | `tool.before` / `tool.after` | `tool` = read, edit or write for `Read`, `Edit`, `Write`; any other tool returns `nil`. `path` = `p.tool_input.file_path`, absolute as Claude sends it (`probes/claude-results.md:176`) |
| `SubagentStop` | `agent.stop` | `last_message` = `p.last_assistant_message` (`probes/claude-results.md:194`) |
| any other | `nil` | — |

Always: `session_id`, `cwd`, and `agent` = `p.agent_type` when it is a non-empty string, else `nil` (hooks.md:753-769, :2709-2713).

**`M.result(r, p)`:** `nil` when `r` is `nil`. Otherwise `E = p.hook_event_name`, `ctx = r.context`, `blk = r.block`:

| `E` | Output (`airsstack.json.encode(out) .. "\n"`) | Proof |
|---|---|---|
| `PreToolUse` | `hookSpecificOutput = { hookEventName = E, permissionDecision = "deny", permissionDecisionReason = blk }` when `blk`, plus `additionalContext = ctx` when `ctx` | the deny blocked the Read (`probes/claude-results.md:287`); hooks.md:2013-2020 |
| `UserPromptSubmit`, `PostToolUse` | `decision = "block", reason = blk` when `blk`; `hookSpecificOutput = { hookEventName = E, additionalContext = ctx }` when `ctx` | PostToolUse block reached the model (`probes/claude-results.md:499`); hooks.md:1580, :2330, :1179-1185 |
| `SubagentStop` | `decision = "block", reason = blk` when `blk`; `ctx` is dropped (D25), so a context-only result gives `nil` | hooks.md:2736, :2931, :2944 |
| `SessionStart` | `hookSpecificOutput = { hookEventName = E, additionalContext = <ctx and blk joined by "\n", whichever are present> }` | `PLUM` (`probes/claude-results.md:350`) |
| `SessionEnd` | `nil`: it cannot block, and no output reaches the model (hooks.md:1034-1068, :837-840) | — |
| any other event (a native hook on, e.g., `Stop`) | `nil`. The translator maps the six events above; a native hook on another event gets no output. Documented limit (§15) | — |

When `ctx` and `blk` are both absent, the result is `nil`.

D13 follows from this table: a block on `PostToolUse` is feedback, since the write stays (`probes/claude-results.md:535`); on `SessionStart` it is context; on `SessionEnd` it is nothing. On `SubagentStop` only a `may_block` hook's block continues the subagent (D25).

## 10. Errors: locations and owners

| Location | Used for | Owner | Example |
|---|---|---|---|
| `--pack "<arg>"` / `--tool "<arg>"` | `init` argument refusals | `types/shorthand`, `app/init` | §4.3 |
| `agentstacks.toml` | exists, I/O, a missing or changed tag at retag, a re-read that no longer parses | `app/init`, `fs`, `app/update::retag`, `types/project/retag` | `agentstacks.toml: already exists (edit it and run apply)` |
| `<path as written>` | `init` path-argument I/O | `app/init` | `./packs/x: <io::Error Display>` |
| the loader label (path as written, D15) | a pack or extension refused by its loader | existing loaders | `packs/x: not a pack (no pack.toml)` |
| `<dir>` | `-C` open failure | CLI `wire` | `<dir>: <io::Error Display>` |
| `init` | git missing from `PATH` for official arguments (D19) | `app/init` | `init: git not found on PATH (needed by --tool "claude", --pack "core")` |
| `agentstacks.lock` | the lock read in `confirm` (D21) | CLI `command/confirm` | `agentstacks.lock: <io::Error Display>` |
| none (plain line) | `stdin is not a terminal; pass --yes to apply`, `nothing applied`, the `agentstacks.toml written; …` / `updated; …` follow-ups | CLI `command/confirm` | §4.5, §5.3, §7 |
| core and formats locations | everything the library already refuses | as core §13 | unchanged |
| `claude/adapter.lua:<line>` | `json.edit` refusals on settings | core host | §9.5 |

CLI messages are not `Diagnostic`s and are not sorted; they print in the order the flow reaches them.

## 11. Edge cases

| Case | Result | Owner |
|---|---|---|
| `plan` without `agentstacks.toml` | `agentstacks.toml: <io::Error Display>` (core R1), exit 1 | core |
| `apply` when only the lock changes (a forget, an adopted file, the airsl pin) | not "nothing to change", because the lock text differs; asks, then applies | `command/confirm` |
| `init --pack sdlc` without core | the set check refuses, `sdlc requires pack "core", which agentstacks.toml does not list (add it: --pack core)` (formats §7). The toml stays (D8) | core |
| `update` on a project with only path sources | no entries; `nothing to update`, exit 0 | `command/update` |
| `update` declined | toml updated, nothing applied, exit 0 | `command/update` |
| a message text with a newline, a `'` or a `\` | JSON-escaped by `airsstack.json.encode` (`\n`, `\\`), `'` shell-escaped; printed unchanged (`probes/cli2-results.md`, T3) | `lib/hooks` |
| no lua hook written | no `translate.lua` | adapter |
| every lua hook skipped by the extension | the core still writes `runtime.lua` (core R11) and no `translate.lua` is written: an unused file, harmless | core, adapter |
| `settings.json` absent and no hooks | no write | `lib/settings` |
| a user handler that ends with ` # agentstacks` | treated as ours and removed | `lib/settings` (documented) |
| a prompt answered `Y`, ` yes `, `YES` | yes | `prompt` |
| `init --tool claude --pack core --pack ./packs/team` with git missing | `init: git not found on PATH (needed by --tool "claude", --pack "core")`; the path argument resolves normally; nothing written | `app/init` |
| the lock deleted or unreadable between plan and confirm | `agentstacks.lock: <io::Error Display>`, exit 1 | `command/confirm` |
| `agentstacks.toml` edited into bad TOML between `update`'s list and its retag | the `ProjectFile::parse` refusal, exit 1, the file unchanged | `app/update::retag` |
| `.claude/settings.json` absent and hooks present | decoded as `{}`; the result is `"{}\n"` plus our events | `lib/settings` |
| a native message hook on `Notification` | skipped: `claude passes no hook message to the model on Notification` | `lib/hooks` |
| a native message hook on `Stop` | skipped: `claude treats hook output at agent stop as feedback that keeps the agent running` (D25) | `lib/hooks` |
| a native message hook on `PostToolUseFailure` | rendered (it takes `additionalContext`, D24) | `lib/hooks` |
| a lua hook on `agent.stop` returning only `context` | no output; the subagent stops (D25) | `translate.lua` |
| a lua argv element `^a$` / `$TMPDIR/x` | skipped (D27) / rendered | `lib/hooks` |
| `agentstacks.toml` tag edited by hand between `update`'s list and its retag | `agentstacks.toml: packs.core: tag changed during update (run update again)`, exit 1, the file unchanged (D26) | `types/project/retag` |

## 12. Testing

| What | How | Needs |
|---|---|---|
| `types/project/render`, `retag`, `types/shorthand` | colocated unit tests on plain values: every row of §4.2-§4.4 and §7, including the `# pinned` comment kept and the R2 byte string | nothing |
| `app/init`, `app/update::retag` | library fakes (`FakeWorkspace`, `FakeGit`): exists, every §4.3 row, duplicate key, write | nothing |
| `fs` `write_project`, the label | contract tests on a `tempfile` directory: mode kept; a refusal label reads `packs/core/…` | disk |
| `runtime.lua` | the existing runtime test, updated for D17, adds a native case reading `p.hook_event_name` | airsl binary when present |
| CLI `show/*`, `command/*`, `prompt`, `exit` | unit tests with `Plan::new` values, an injected terminal flag and line reader, and the library fakes through the `testing` feature (D20): every §5.2 row, every §5.3 branch including D21, every §7 outcome including D22, every exit code | nothing |
| CLI end to end | `crates/agentstacks-cli/tests/`: run the binary on a temp project with path sources and a copy of `extensions/claude`: `init` (plus `--yes`), `plan`, `apply --yes`, `doctor` healthy then changed, the no-terminal refusal | disk, airsl |
| `extensions/claude/lib/*.lua` | `*_test.lua` beside each module, run by `airsl test extensions/claude`: values, frontmatter (every mapping row, native override), hooks (every §9.4 row, quoting, marker, session_id), settings (every §9.5 row) | airsl binary |
| the whole extension | `crates/agentstacks/tests/claude.rs`: the real `plan` with `StdWorkspace` and `AirslAdapter` on fixtures in `crates/agentstacks/tests/fixtures/claude/`. Cases: a full pack with every item kind; an existing settings file with user keys, a user hook and an old marked hook; a removed tool. Every proposed file and skip is compared with checked-in expected files | airsl library |
| `translate.lua` | `crates/agentstacks/tests/claude.rs`: the expected `translate.lua` with the core `runtime.lua`, run with `airsl run` on recorded payloads (the §9.7 rows, anonymised from `probes/claude-results.md`), output compared byte for byte; skipped with a printed reason when the binary is absent | airsl binary |

- `Makefile.toml` gains a `lua` task (`airsl test extensions/claude`), which joins `dod`. A failing Lua test exits 1 (`probes/airsl-test-results.md`).
- So `cargo make dod` needs the airsl binary locally; without it the `lua` task fails with the shell's command-not-found error. Pack hooks need the same binary.
- CI already installs the airsl binary before `cargo make dod` (`.github/workflows/ci.yml:59-72`).
- Config examples parsed with the real crate: the §9.1 `extension.toml` loads through `Extension::load` with `reads = [.claude/settings.json]`, and a path-source project file parses through `ProjectFile::parse` (`probes/settings-results.md`, S7).
- No logic is tested only end to end. `cargo make dod` is the gate.
- The gate runs clippy and rustdoc with `--all-features` (`Makefile.toml`), which turns on `testing`. So the gated fakes must pass `missing_docs`, `unreachable_pub` and clippy outside `cfg(test)`.

## 13. Dependencies, policy and docs

| Change | Proof |
|---|---|
| `toml_edit = "0.25"` (workspace; used by `agentstacks`) | R2; `cargo deny check` passes with it (R5). The new duplicates, toml_edit 0.22/0.25 and winnow 0.7/1.0, come from airsl and fall under the existing `skip-tree`. R5 added the crate to `agentstacks-cli`; the workspace resolves one `Cargo.lock`, so the same crates are checked whichever member uses it |
| `similar = "3"` (workspace; used by `agentstacks-cli`), `Apache-2.0` | R3, R5 |
| `deny.toml` | unchanged (R5) |
| `agentstacks-cli` depends on `agentstacks` (workspace path dependency) | composition root (docs/architecture.md) |
| `agentstacks` gains `[features] testing = []`; `[workspace.dependencies]` gains `agentstacks = { path = "crates/agentstacks" }`; `agentstacks-cli` has `agentstacks = { workspace = true }` and `[dev-dependencies] agentstacks = { workspace = true, features = ["testing"] }` (D20) | `probes/redesign-results.md`, G2 (resolver 3: on in `cargo test`, off in `cargo run` and `--release`) |
| docs | `docs/hooks.md`: the translator contract gains `payload`, where `M.result` is documented (`docs/hooks.md:101`, `:153`). New `docs/cli.md`: the commands, output, prompt and exit codes. `docs/README.md` table updated. `CLAUDE.md` Status: CLI commands and the claude extension built |

## 14. Handed to other chains

| Chain | Receives |
|---|---|
| `plugins-to-packs` | the official pack names in `types/shorthand` must match the six pack directories; it checks rendered behaviour against the plugins, with claudevs |
| `claudevs-move` | checking the rendered `.claude/` config |
| `release-and-migration` | making the repository public and tagging `packs-v*`, so official shorthand resolves; the org docs status change |

## 15. Non-goals

- Git URLs as `init` arguments (D4); `ext add` / `ext test`.
- Searching parent directories for the project (D6).
- Colour or machine-readable (`--json`) output.
- Adding required packs automatically (D9).
- Byte-exact preservation of user handlers inside a rewritten `hooks.<Event>` array (D16).
- Commands and rules: reserved in api 0 (formats §2.1, §14). The intent's mention of them is met only by `native/claude/` files (D11).
- Output translation for native hooks on events other than the six neutral ones (§9.7).
- OpenCode or any other tool.
- Converting the plugins (`plugins-to-packs`).
- A stable API: everything stays `api = 0`.
