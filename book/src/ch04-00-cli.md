# The Command Line

What the `agentstacks` commands do, what they print and which exit code each outcome gives. The
binary is a composition root and a presentation layer: every rule and flow lives in the library
(see [architecture.md](ch07-00-architecture.md)), and the planning and apply steps behind the commands are in
[plan-and-apply.md](ch05-00-plan-and-apply.md).

## 1. Commands

```text
agentstacks [-C <DIR>] <COMMAND>
  init   --tool <TOOL>... --pack <PACK>... [--yes]
  plan
  apply  [--yes]
  doctor
  update [--yes]
```

| Command | Does |
|---|---|
| `init` | Writes `agentstacks.toml`, then plans, shows the plan and applies it. Refuses when `agentstacks.toml` exists. |
| `plan` | Prints the plan. Writes nothing. |
| `apply` | Prints the plan, asks, then applies the same plan it showed. It never plans twice. |
| `doctor` | Reports drift against the lock. |
| `update` | Rewrites each newer tag in `agentstacks.toml`, then plans, shows and applies like `apply`. |

- `--tool` and `--pack` are each required at least once.
- `-C <DIR>` sets the project directory, resolved from the current directory. It works before or
  after the subcommand. Without it the project is the current directory. Parent directories are not
  searched.
- When `<DIR>` cannot be opened, the CLI prints `<dir>: <io::Error Display>` and exits 1.
- Usage errors are clap's own: a missing required argument, an unknown subcommand or no subcommand
  exits 2 on stderr. `--help` and `--version` print on stdout and exit 0.

## 2. Streams and exit codes

| Stream | Output |
|---|---|
| stdout | the plan; the `doctor` report; the `update` list; `nothing to change`; `nothing to update`; `agentstacks.toml updated`; `applied: …` |
| stderr | refusals; the prompt; `stdin is not a terminal; pass --yes to apply`; `nothing applied` (`apply` only); the `agentstacks.toml written; …` and `agentstacks.toml updated; …` follow-ups |

Refusals are printed one `Diagnostic` per line, sorted.

| Code | When |
|---|---|
| 0 | success: a plan shown, an apply done, nothing to change, a declined prompt, a healthy `doctor`, an `update` with nothing newer |
| 1 | any diagnostic, any `doctor` finding, `stdin is not a terminal; pass --yes to apply` |
| 2 | clap usage error |

## 3. `init`

`init` takes one or more `--tool` and `--pack` arguments:

| Argument | Meaning |
|---|---|
| contains `/` or starts with `.` | a project-relative path, written as `path = "<argument>"` |
| anything else | an official name |

- Official packs: `core`, `sdlc`, `journal`, `guideline-rust`, `guideline-python`. Official tools: `claude`.
- An official name expands to `git = "https://github.com/airsstack/agentstacks"`, the latest
  `packs-v*` tag, and `subdir = "packs/<name>"` (a tool: `extensions/<name>`).
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
| no release tag | `--pack "core": no packs-v release tag in https://github.com/airsstack/agentstacks` |
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

### Plan text

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
| applied | stdout `applied: <w> written, <d> deleted` | 0 |
| failed | the diagnostics, e.g. `<path>: changed after plan; run plan again` | 1 |

## 7. `update`

1. List the newer tags. A refusal is printed and exits 1.
2. Print each entry on stdout: `  <key>: <current> → <new>` for a new tag, `  note     <UpdateNote Display>`
   for a note. `<key>` is `packs.core` or `tools.claude`.
3. With no new tag, print `nothing to update`, exit 0 and write nothing.
4. Rewrite `agentstacks.toml` with each new tag, comments kept, and print `agentstacks.toml updated`.
   A refusal exits 1 with the file unchanged and no plan.
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
