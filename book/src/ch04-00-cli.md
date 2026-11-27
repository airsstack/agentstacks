# The Command Line

What the `agentstacks` commands do, what they print and which exit code each outcome gives. The
binary is a composition root and a presentation layer: every rule and flow lives in the library
(see [architecture.md](ch07-00-architecture.md)), and the planning and apply steps behind the commands are in
[plan-and-apply.md](ch05-00-plan-and-apply.md).

## 1. Commands

```text
agentstacks [-C <DIR>] <COMMAND>
  init   --tool <TOOL>... --pack <PACK>... [--yes | --dry-run]
  plan   [--summary | --verbose]
  apply  [--yes]
  doctor
  update [--yes]
  ext add <SOURCE> [--tag <TAG> | --rev <REV>] [--subdir <DIR>] [--yes]
  ext test <EXTENSION> [--pack <DIR>]...
```

| Command | Does |
|---|---|
| `init` | Writes `agentstacks.toml`, then plans, shows the plan and applies it. Refuses when `agentstacks.toml` exists, except with `--dry-run`, which proceeds and writes nothing: it prints `agentstacks.toml (dry run — not written)`, the rendered file and its plan (planned as a fresh project, against no lock). |
| `plan --verbose` | The full one-line-per-file plan; without it (or with `--summary`) the plan shows counts per owner. |
| `plan` | Prints the plan. Writes nothing. |
| `apply` | Prints the plan, asks, then applies the same plan it showed. It never plans twice. |
| `doctor` | Reports drift against the lock. |
| `update` | Rewrites each newer tag in `agentstacks.toml`, then plans, shows and applies like `apply`. |
| `ext add` | Adds a tool extension (a git URL with `--tag` or `--rev`, a project directory, or an official tool name) to `agentstacks.toml`, prints `agentstacks.toml: added tools.<name>`, then plans, shows and applies like `update`, with the same follow-ups and exit codes. |
| `ext test` | Checks an extension against the built-in conformance pack in a temporary project: runs its Lua tests, plans, applies and plans again, and prints the plan, `applied: <n> written, <m> deleted`, `second plan: nothing to change` and a last line `passed` or `failed`. Writes nothing into the project or the extension directory. `<EXTENSION>` and each `--pack` are resolved against `-C`. Exits 1 when the last line is `failed` or the run stopped before a verdict. |

- `init` requires `--tool` and `--pack` at least once each. `ext test`'s `--pack` is optional and may
  repeat; no other command takes `--tool`.
- `-C <DIR>` sets the project directory, resolved from the current directory. It works before or
  after the subcommand. Without it the project is the current directory. Parent directories are not
  searched.
- When `<DIR>` cannot be opened, the CLI prints `<dir>: <io::Error Display>` and exits 1.
- Usage errors are clap's own: a missing required argument, an unknown subcommand or no subcommand
  exits 2 on stderr. `--help` and `--version` print on stdout and exit 0.

## 2. Streams and exit codes

| Stream | Output |
|---|---|
| stdout | the plan; the `doctor` report; the `update` list; `nothing to change`; `nothing to update`; `agentstacks.toml: added tools.<name>`; `agentstacks.toml updated`; `applied: …`; the `ext test` report ending in `passed` or `failed` |
| stderr | refusals; the prompt; `stdin is not a terminal; pass --yes to apply`; `nothing applied` (`apply` only); the `agentstacks.toml written; …` and `agentstacks.toml updated; …` follow-ups; for `ext test`, what `airsl test` printed on its own stderr when the Lua tests fail, relayed as it came |

Refusals are printed one `Diagnostic` per line, sorted.

| Code | When |
|---|---|
| 0 | success: a plan shown, an apply done, nothing to change, a declined prompt, a healthy `doctor`, an `update` with nothing newer, an `ext test` that ends in `passed` |
| 1 | any diagnostic, any `doctor` finding, `stdin is not a terminal; pass --yes to apply`, an `ext test` that ends in `failed` or stops before a verdict |
| 2 | clap usage error |

## 3. `init`

`init` takes one or more `--tool` and `--pack` arguments:

| Argument | Meaning |
|---|---|
| contains `/` or starts with `.` | a project-relative path, written as `path = "<argument>"` |
| anything else | an official name |

- Official packs: `core`, `sdlc`, `journal`, `guideline-rust`, `guideline-python`. Official tools:
  `claude`, `musecode`, `opencode`. To use an extension from a checkout instead of a release, copy
  its directory (for example `extensions/opencode`) into the project and pass that path, such as
  `--tool ext/opencode`.
- An official name expands to `git = "https://github.com/airsstack/agentstacks"`, the latest
  `packs-v<api>.*` tag (the newest release whose major version is the `api` this agentstacks speaks, so `packs-v1.*` for `api = 1`), and `subdir = "packs/<name>"` (a tool: `extensions/<name>`).
- A path's table key is the `name` in that directory's `pack.toml` or `extension.toml`.
- Git URLs are not arguments. Add one to `agentstacks.toml` by hand.
- A repeated identical argument is written once. Two different arguments that resolve to the same
  key are refused at the later one: `--pack "./packs/core": pack "core" is already given by --pack "core"`.
- A name stays a name even when a directory of that name exists: only `/` or a leading `.` makes a
  path.
- `init` adds no dependencies. A missing required pack is left to the plan's set check.

Steps:

1. Refuse when `agentstacks.toml` exists: `agentstacks.toml: already exists (edit it and run apply)`.
2. Classify and resolve every argument. If any is refused, print every refusal and write nothing.
3. Render and write `agentstacks.toml`.
4. Plan, show, ask and apply, as in section 4.

The rendered file lists tools first in name order, then packs in name order, with keys in the order
`git`, `tag`, `subdir` (or `path`) and a blank line before each table.

### Refusals

| Case | Message |
|---|---|
| file exists | `agentstacks.toml: already exists (edit it and run apply)` |
| unknown name | `--pack "x": not an official pack; pass a path, or add a git source to agentstacks.toml` / `--tool "x": not an official tool; pass a path, or add a git source to agentstacks.toml` |
| no release tag | `--pack "core": no packs-v1 release tag in https://github.com/airsstack/agentstacks` |
| git missing from `PATH` | one refusal for all official arguments: `init: git not found on PATH (needed by --tool "claude", --pack "core")` |
| path absolute / climbs out | `--pack "/x": absolute paths are not portable` / `--pack "../x": must stay inside the project` |
| path resolves outside through a symlink | `--pack "./x": resolves outside the project` |
| path I/O | `<path as written>: <io::Error Display>` |
| write fails | `agentstacks.toml: <io::Error Display>` |

A path source in a pack or extension is named in refusals by the path as written in
`agentstacks.toml`, not by its absolute path.

### After the file is written

`init` leaves `agentstacks.toml` in place when the plan is refused or declined, and says so on
stderr:

| Outcome | stderr | Exit |
|---|---|---|
| plan refused | the refusals, then `agentstacks.toml written; fix the above and run apply` | 1 |
| declined | `agentstacks.toml written; nothing applied (run apply)` | 0 |
| no terminal, no `--yes` | `stdin is not a terminal; pass --yes to apply`, then `agentstacks.toml written; nothing applied (run apply)` | 1 |
| applied | as section 6 | 0 |
| apply failed, or the lock read failed | the diagnostics, then `agentstacks.toml written; fix the above and run apply` | 1 |

## 4. `plan` and `apply`

`plan` prints the plan and exits 0. A refused plan prints its refusals and exits 1. `apply` prints
the plan, runs the confirmation (section 5), and on yes applies that same plan.

A project written for the 0.x releases is refused by both, with nothing planned and
nothing written: `agentstacks.toml: api 0 is from agentstacks 0.x (run agentstacks update)`. Any other
`api` that is not 1 gives `agentstacks.toml: api <n> not supported (this agentstacks speaks 1)`. See
[moving a project from 0.x](ch03-01-from-0x.md).

### Plan text

By default the plan is a summary: one `  <owner>  <w> to write, <d> to delete, <u> unchanged`
row per owner (`shared` last), then the skip rows, the airsl row and notes, and the final
count line. `--verbose` shows the full text below: one row per file, with diffs. The
commands that show a plan before applying (`apply`, `init`, `update`, `ext add`) show the
summary; `ext test` shows the full text.

```text
agentstacks plan

  create   .claude/skills/core-status/SKILL.md   core
  update   .claude/agentstacks/runtime.lua       claude
  delete   .claude/agents/team-lint-old.md       team-lint
  update   .claude/settings.json                 shared
    --- .claude/settings.json
    +++ .claude/settings.json
    @@ -3,5 +3,17 @@
    …
  skip     core/hook/2: claude gives hooks the session id in the event (event.session_id), not as {{var session_id}}

  airsl    0.1.4 (pinned)
  note     native/cluade: not for claude

3 to write, 1 to delete, 12 unchanged, 1 skipped
```

(The `…` stands for the rest of the diff.)

| Row | Rule |
|---|---|
| step | Every step whose action is `Create`, `Update`, `Restore`, `Delete` or `Forget`, in the plan's path order. `Unchanged` is counted, not listed. Two spaces, the action in lower case padded to 8, a space, the path padded to the longest listed path, two spaces, then the owner. The owner is the pack or extension name, or `shared` when the step has none. |
| diff | Below an `Update` step that had an old text: a unified diff with three lines of context and the path as both headers, each line indented four spaces. Bytes that are not UTF-8 are decoded lossily. Identical texts give no lines. |
| skip | `  skip     <subject>: <reason>`. The subject is the item id, or `<pack>/<rel>` for a core root-file skip. |
| airsl | `  airsl    <v> (pinned)` when the locked and installed versions are equal; `  airsl    <locked> → <installed>` when they differ; `  airsl    <installed> (new pin)` when the lock had none. |
| binary | none. A missing binary already stops the plan, and found binaries are not listed. |
| note | `  note     <Note Display>` |
| summary | `<w> to write, <d> to delete, <u> unchanged, <s> skipped`, then `, <f> forgotten` when `f` is above 0. `w` counts Create, Update and Restore; `d` counts Delete; `u` counts Unchanged; `s` counts skips; `f` counts Forget. |

Empty sections are left out, together with the blank line before them. There is no colour.

## 5. Confirmation

`init`, `apply` and `update` share one sequence after the plan is shown:

```text
nothing to change? ── yes ─► print plan + "nothing to change", exit 0, no question, nothing written
       │ no
--yes? ── yes ─► apply
       │ no
stdin a terminal? ── no ─► "stdin is not a terminal; pass --yes to apply", exit 1
       │ yes
"Apply? [y/N] " on stderr, read one line
  trimmed, case-insensitive "y" or "yes" ─► apply
  anything else, empty, end of input, or a read error ─► decline line, exit 0
```

The decline line is `nothing applied` for `apply`. `init` and `update` print only their follow-up
line instead (`agentstacks.toml written; nothing applied (run apply)` and `agentstacks.toml updated;
nothing applied (run apply)`).

- **Nothing to change** means no listed step, and the lock the plan would write equals the current
  lock text. An absent lock is never equal.
- **Lock read error:** the current lock is read first. A failure prints
  `agentstacks.lock: <io::Error Display>`, exits 1, asks nothing and applies nothing. `init` and
  `update` add their follow-up line (sections 3 and 7).
- A piped stdin and `/dev/null` are not terminals, so without `--yes` they refuse.

## 6. After apply

| Result | Output | Exit |
|---|---|---|
| applied | stdout `applied: <w> written, <d> deleted`, then, when anything changed, `note: start a new session — hooks, skills and agents load at session start` | 0 |
| failed | the diagnostics, e.g. `<path>: changed after plan; run plan again` | 1 |

## 7. `update`

1. List the newer tags. A refusal is printed and exits 1.
2. Print each entry on stdout: `  <key>: <current> → <new>` for a new tag, `  note     <UpdateNote Display>`
   for a note. `<key>` is `packs.core` or `tools.claude`. A project written for the 0.x releases
   (`api = 0`) first prints `  api: 0 → 1`.
3. With no new tag and the `api` already current, print `nothing to update`, exit 0 and write
   nothing. A 0.x project is not in this case: it is moved to `api = 1` even when no tag is newer.
4. Rewrite `agentstacks.toml` with each new tag and, for a 0.x project, with `api = 1`, comments
   kept, and print `agentstacks.toml updated`. A refusal exits 1 with the file unchanged and no plan.
5. Plan, show, confirm and apply exactly as `apply` does (sections 4 to 6), then add the follow-up
   on stderr.

| Step 5 outcome | stderr follow-up | Exit |
|---|---|---|
| plan refused, apply failed, or lock read failed | `agentstacks.toml updated; fix the above and run apply` | 1 |
| declined | `agentstacks.toml updated; nothing applied (run apply)` | 0 |
| no terminal, no `--yes` | `stdin is not a terminal; pass --yes to apply`, then `agentstacks.toml updated; nothing applied (run apply)` | 1 |
| applied, or nothing to change | none | 0 |

Step 4 refusals:

| Case | Message |
|---|---|
| re-read fails | `agentstacks.toml: <io::Error Display>` |
| `api` no longer the legacy `0` at re-read | `agentstacks.toml: api changed during update (run update again)` |
| re-read text no longer parses | the project file's own parse refusals |
| a tag missing at re-read | `agentstacks.toml: packs.core: no tag to update (run update again)` |
| a tag that differs from the listed current tag | `agentstacks.toml: packs.core: tag changed during update (run update again)` |
| write fails | `agentstacks.toml: <io::Error Display>` |

After step 4, `agentstacks.toml` names a new tag while the lock still pins the old revision. The plan
reconciles that entry as stale and resolves it again, so `update` needs no special case.

## 8. `doctor`

Prints each finding on stdout, one per line, in the order returned, for example
`changed: .claude/agents/core-coder.md` or `airsl: 0.1.5 installed, lock pins 0.1.4`. With no findings it
prints `healthy` and exits 0. Any finding exits 1.

`doctor` reads the lock and the files it names, never `agentstacks.toml`, and always checks the
`airsl` runtime: missing airsl is reported with the install command, with or without a lock pin
(`airsl: not found on PATH (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)`, `airsl: not found on PATH (lock pins <v>) (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)`);
a parseable installed version is healthy without a pin, and an exact match is required with one.
A lock written by a 0.x release is read like a current one: its api is not a finding.

## 9. `ext add`

```text
agentstacks [-C <DIR>] ext add <SOURCE> [--tag <TAG> | --rev <REV>] [--subdir <DIR>] [--yes]
```

`<SOURCE>` is classified by its text alone:

| `<SOURCE>` | Meaning |
|---|---|
| contains `://` or starts with `git@` | a git URL; needs `--tag` or `--rev`, and takes `--subdir` |
| contains `/` or starts with `.` | a project-relative path, written as `path = "<SOURCE>"` |
| anything else | an official tool name (`claude`, `musecode`, `opencode`) |

- `--tag` and `--rev` exclude each other (clap, exit 2). `--tag`, `--rev` and `--subdir` apply only
  to a git URL.
- An official name is written as the official repository at its newest `packs-v1.*` tag, with
  `subdir = "extensions/<name>"`, the same rule as `init` (section 3).
- The table key is the `name` in the extension's `extension.toml`. The extension is fetched or read
  and loaded before anything is written, so a refused extension never reaches `agentstacks.toml`.

Steps:

1. Read `agentstacks.toml` and refuse a legacy or unsupported `api`.
2. Resolve the source and load the extension. Every refusal is printed and nothing is written.
3. Re-read `agentstacks.toml`; refuse when it changed since step 1.
4. Add `[tools.<name>]`, keeping comments, and print `agentstacks.toml: added tools.<name>` on stdout.
5. Plan, show, confirm and apply exactly as `update` does (sections 4 to 6), with the same follow-ups
   on stderr (`agentstacks.toml updated; …`) and exit codes as the `update` table in section 7. A
   refused plan keeps the new table.

A refusal in steps 1 to 4 exits 1 with no plan and the file as it was.

### Refusals

| Case | Message |
|---|---|
| file missing or unreadable | `agentstacks.toml: <io::Error Display>` |
| legacy project | `agentstacks.toml: api 0 is from agentstacks 0.x (run agentstacks update)` |
| other `api` | `agentstacks.toml: api <n> not supported (this agentstacks speaks 1)` |
| unknown name | `ext add "x": not an official tool; pass a path or a git URL` |
| git URL with neither flag | `ext add "<url>": a git URL needs --tag or --rev` |
| flag with a name or path | `ext add "<source>": --tag applies only to a git URL` (likewise `--rev`, `--subdir`; one line per flag) |
| empty value | `ext add "<url>": --tag: empty` (likewise `--rev`, `--subdir`) |
| `--rev` not a commit id | `ext add "<url>": --rev: not a commit id` |
| `--subdir` climbs out | `ext add "<url>": --subdir: must stay inside the repository` |
| path absolute / climbs out | `ext add "/x": absolute paths are not portable` / `ext add "../x": must stay inside the project` |
| path resolves outside through a symlink | `ext add "./x": resolves outside the project` |
| path I/O | `<path as written>: <io::Error Display>` |
| git missing from `PATH` | `ext add: git not found on PATH (needed by ext add "<source>")` |
| git fails | `ext add: git <subcommand> failed: <stderr>` |
| `--subdir` missing in the repository | `ext add "<url>": "<dir>" not found at <rev>` / `ext add "<url>": "<dir>" is a file, not a directory` |
| no tag of this major | `ext add "claude": no packs-v1 release tag in https://github.com/airsstack/agentstacks` |
| the extension is refused | the loader's own refusals, located at its files, such as `ext/mytool/extension.toml: extension.api: 2 not supported (this agentstacks speaks 1)` |
| tool already listed | `agentstacks.toml: tools.<name> already exists (edit it there)` |
| file changed meanwhile | `agentstacks.toml: changed during ext add (run it again)` |
| write fails | `agentstacks.toml: <io::Error Display>` |

## 10. `ext test`

```text
agentstacks [-C <DIR>] ext test <EXTENSION> [--pack <DIR>]...
```

Checks a tool extension the way `plan` and `apply` will use it, without touching the project or the
extension. It needs `airsl` on `PATH`. `<EXTENSION>` and each `--pack` are directories, resolved
against `-C` (or the current directory) and shown in refusals as written. `--pack` adds your own
pack to the temporary project beside the built-in `conformance` pack; a pack with that name, or two
packs with one name, are refused. The command never reads the project's `agentstacks.toml`.

Steps, in order:

1. `airsl` is on `PATH`.
2. The extension directory is opened and loaded like any extension.
3. When the extension holds a Lua test file (`*_test.lua` or `test_*.lua`), `airsl test` runs in its
   directory.
4. Each `--pack` is opened and named, and the temporary project is created: the extension, the
   conformance pack, your packs and an `agentstacks.toml` listing them.
5. The project is planned.
6. The plan is applied.
7. The project is planned again; the second plan must hold no step that writes, deletes or forgets
   (a skip is allowed).
8. The verdict.

A failure of step 1 or 2 stops the run with no verdict. A failure of step 4, 5 or 6 stops it with the
verdict `failed`. A failing step 3 or 7 does not stop the run and makes the verdict `failed`.

### Output

stdout holds, in order and only for the steps that ran:

```text
ext test demo 1.2.0
lua tests: 71 passed, 0 failed (5 files)
agentstacks plan

  create   …

2 to write, 0 to delete, 0 unchanged, 0 skipped
applied: 2 written, 0 deleted
second plan: nothing to change
passed
```

| Line | Rule |
|---|---|
| `ext test <name> <version>` | the extension's name and version from `extension.toml` |
| `lua tests: …` | `lua tests: <last line airsl test printed>` on success (`lua tests: passed` when it printed nothing); `lua tests: none` when the extension holds no test file; `lua tests: failed` followed by what `airsl test` printed on stdout |
| the plan | as in section 4 |
| `applied: <w> written, <d> deleted` | after step 6 |
| `second plan: nothing to change` | step 7 passed |
| `second plan: <w> to write, <d> to delete (apply is not repeatable)` | step 7 failed, followed by the second plan; `, <f> forgotten` follows `<d> to delete` when the second plan forgets `f` entries |
| `passed` or `failed` | the last line; absent when step 1 or 2 stopped the run |

stderr holds the refusals, and what `airsl test` printed on its own stderr when the Lua tests fail.
The exit code is 0 for `passed` and 1 for `failed` or for no verdict.

### Refusals

| Case | Message |
|---|---|
| `airsl` missing | `ext test: airsl not found on PATH (needed for the Lua tests and the hooks)` |
| `airsl test` cannot start | `ext test: airsl test: <io::Error Display>`, with `lua tests: failed` on stdout |
| a directory cannot be opened | `<path as written>: <io::Error Display>` |
| the extension is refused | the loader's own refusals, located at its files |
| `--pack` has no `pack.toml` | `--pack "<dir>": not a pack (no pack.toml)` |
| `--pack` repeats a name | `--pack "<dir>": pack "<name>" is already given by --pack "<first>"` |
| `--pack` named `conformance` | `--pack "<dir>": pack "conformance" is already given by the built-in conformance pack` |
| a symlink or special file in a tree | `<path>: symlinks are not allowed in a pack` and the like |
| plan or apply refused | the plan's own refusals (for example the extension does not cover an item), with verdict `failed` |

The temporary project is removed when the command returns.
