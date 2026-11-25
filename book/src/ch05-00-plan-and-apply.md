# Plan, Apply and the Lock

How the core turns a project into a plan a human reads, applies it as the only writer, and checks
it later. This guide is for core contributors. The layering it sits in is
[architecture.md](ch07-00-architecture.md). What an extension proposes is in [extensions.md](ch06-01-extensions.md),
and how hooks run is in [hooks.md](ch06-02-hooks.md).

The core is tool-neutral: it knows no tool's file shapes. Where a message names a tool
(`claude/adapter.lua`, `.claude/settings.json`), it is an example: claude extension.

Where the code lives:

| Piece | Location |
|---|---|
| rules | `crates/agentstacks/src/types/` (`op`, `plan`, `resolve`, `json`, `protocol`) |
| flows | `crates/agentstacks/src/app/` (`plan`, `apply`, `doctor`, `update`) |
| ports | `crates/agentstacks/src/port/` (`FileTree`, `Workspace`, `Git`, `Binary`, `Adapter`) |
| adapters | `crates/agentstacks/src/fs/`, `process/`, `host/` |

## 1. The flow

`app/plan` runs these ten steps. Each asks a port for facts and hands them to a rule in `types/`.

```text
1  read agentstacks.toml + agentstacks.lock                         Workspace
2  reconcile lock vs toml → what to fetch                           types/resolve
3  fetch / read sources, tree hashes                                Git, Workspace
4  load + check the set (the pack and extension format checks)      types
5  per tool, in name order: values → render → plan                  Adapter
   per removed tool: values → plan with zero packs
6  validate ops, coverage, duplicates                               types/op
7  binary checks: packs' requires.binaries, extension require_binary, airsl   Binary
8  read each target's disk state                                    Workspace
9  decide each path's step                                          types/plan
10 return Plan, or every refusal                                    none
```

A refusal in steps 1 to 7 skips steps 8 and 9. No step is decided, so no false delete appears.

**Order of tools.** Listed tools run in name order, then removed tools in name order. Each listed
tool's core writes are recorded just before its `plan` dispatch, so the order decides which proposal
is the earlier one. Two messages name the earlier proposer when it is the core:

- the earlier proposer is the core's `runtime.lua`: `path "<p>" is already proposed by core (runtime.lua of claude)`
- a core write that meets an earlier tool's path: `agentstacks.toml: tools.opencode: path "<p>" is already proposed by claude`

**A `Plan` holds:**

- steps, each with path, action, owner and op origin, plus the new bytes; for a shared file, the old
  bytes too, so the CLI can show a diff;
- skips with their reasons;
- binary checks;
- notes;
- the disk state each step was decided on;
- the new lock contents.

**Notes** list what the pack format loaders ignore, and a source that was resolved again. Three
messages:

- `<pack>/<path>: ignored (dot entry)`
- `native/cluade: not for claude`
- `agentstacks.toml: packs.core: source changed, resolved again`

**Refusals** are collected, never first-only, and sorted. A plan with any refusal applies nothing.

## 2. Operations

The kinds and their fields are in [extensions.md](ch06-01-extensions.md#7-operations), and their validation
in [extensions.md](ch06-01-extensions.md#9-validation). In short:

| Kind | Proposed by | Effect |
|---|---|---|
| `write`, `copy`, `skip`, `require_binary` | extension | put bytes at a path, put a pack file's bytes at a path, install nothing with a reason, require a binary |
| `write` (core) | core | each pack root file goes to `<root>/<pack>/<rel>`; `runtime.lua` goes to `<root>/runtime.lua` when a pack of this tool has a `lua` hook or a native `lua` hook, whether or not the extension later skips it |
| `require_binary` (core) | core | each pack's `requires.binaries`; `airsl` when any locked pack declares `requires.airsl` |
| `delete` | core | a locked file no longer proposed |

`by`, the lock's owner of a file, is derived: a copy, or a write whose `covers` all belong to one pack,
is owned by that pack; a core write of a root file is owned by its pack; any other write is owned by
the tool. A shared file has no `by`.

## 3. Steps

**Disk states**, read through `Workspace`: absent, a regular file (bytes, hash, mode), or not a
regular file.

**Shared** means the path is in this tool's `extension.reads` and the extension read it in this run.
A shared file is never written to the lock, so `doctor` never sees it, and a pack or tool removal
handles it by rerunning the extension.

| Proposed path | In the lock | Disk | Step |
|---|---|---|---|
| any | any | not a regular file | refused `<path>: is not a regular file` |
| shared | none | absent, and `read` returned absent | `create` |
| shared | none | equals what `read` returned | `update`, or `unchanged` when the new bytes equal the disk; old bytes kept for the diff. A lock entry for it, if any, is dropped |
| shared | none | differs from what `read` returned | refused `<path>: changed after it was read; run plan again` |
| not shared | yes | absent | `restore` |
| not shared | yes | hash = lock hash | `update`, or `unchanged` when equal |
| not shared | yes | hash ≠ lock hash, bytes = new bytes | `unchanged` (the user made the same edit; the lock takes the new hash) |
| not shared | yes | hash ≠ lock hash, bytes ≠ new bytes | refused `<path>: changed since the last apply` |
| not shared | no | absent | `create` |
| not shared | no | bytes = new bytes | `unchanged` (adopted into the lock) |
| not shared | no | bytes ≠ new bytes | refused `<path>: exists and was not written by agentstacks` |

| Locked path no longer proposed | Disk | Step |
|---|---|---|
| any | the path itself is now a symlink (dangling or pointing outside included) | `forget`; the user replaced it, so it is left alone. Checked before any other row |
| any | absent | `forget` (dropped from the lock) |
| any | hash = lock hash | `delete` |
| any | hash ≠ lock hash | refused `<path>: changed since the last apply; not deleting it` |
| any | not a regular file | refused `<path>: is not a regular file` |

**Checked examples:**

| Situation | Step |
|---|---|
| first apply; `.claude/skills/core-status/SKILL.md` absent | `create` |
| user edited that file after apply; pack unchanged | refused `.claude/skills/core-status/SKILL.md: changed since the last apply` |
| user deleted it | `restore` |
| pack `team-lint` removed from toml; its file `.claude/agents/team-lint-old.md` untouched | `delete` |
| `.claude/settings.json` holds the user's permissions; the claude extension read it and proposes it with hooks added | `update` (shared), diff shown, not locked |
| `.claude/settings.json` absent; the extension read `nil` and proposes it | `create` (shared), new-file mode |
| `.claude/agents/core-coder.md` exists, never written by agentstacks, different content | refused `.claude/agents/core-coder.md: exists and was not written by agentstacks` |
| a proposed path is a symlink to `/etc/hosts` | refused `<path>: resolves outside the project` |

## 4. Inside the project

Every proposed path, deleted path and `read` path is resolved by `Workspace` with one rule:

1. If the path is a symlink, follow it. A dangling symlink is "not a regular file".
2. Otherwise canonicalise the deepest existing ancestor.
3. The result must start with the canonical project root. Otherwise: refused
   `<path>: resolves outside the project` (for `read`, at the caller's line).

A symlink that stays inside the project is written through: the write goes to its canonical target
and the link survives. The lock records the normalised proposed path, never the canonical one.

Two details of the built `Workspace`: a dangling symlink at stage or swap is an I/O `NotFound`
(`symlink leads nowhere`), and a path that resolves to the project directory itself is refused at
stage and swap (`resolves to the project directory`).

A `path` source directory goes through the same rule. Outside: refused
`agentstacks.toml: packs.team-lint.path: resolves outside the project`.

## 5. Removed pack and removed tool

A pack removed from `agentstacks.toml` gets `delete` steps for its locked files, under the rules of
[section 3](#3-steps).

A tool removed from `agentstacks.toml` runs its locked extension once with zero packs, so it cleans
its own shared-file entries:

- the extension is resolved from the lock, and its git tree is checked against its lock `tree` like
  any locked entry;
- its ops are still checked against each other;
- only its writes to paths it read in that run (shared files) are kept, and only those meet the
  other tools' proposals. Every other proposal is dropped and never reaches the duplicate check, and
  no core write is made for its root;
- all of its locked files then get delete steps under the rules of [section 3](#3-steps);
- nothing from that tool is locked.

## 6. Apply

`app/apply` takes the `Plan` that `app/plan` returned, the same `Plan` the CLI showed. It never
re-plans.

```text
1  refuse a Plan that holds refusals
2  stage: for each create/update/restore write <dir>/.<name>.agentstacks-tmp
      (parents created; a leftover temp of the same name is overwritten)
3  check: every target's disk state still equals the state the Plan recorded
      (for a shared file: what read returned); any differs →
      remove the temps, refused `<path>: changed after plan; run plan again`
4  swap: rename each temp onto its (canonical) target; perform each delete
5  lock: write agentstacks.lock last (`types::lock::write_lock`), via temp + rename
```

- Temps sit beside the canonical target. The lock temp is `.agentstacks.lock.agentstacks-tmp`, and
  the lock keeps its mode.
- A path that resolves outside the project at apply time is refused: `<path>: resolves outside the project`.
- A write step with no content is refused `<path>: has no content to write`. A plan built from valid
  proposals never holds one.
- The check compares content hashes only, so a mode-only change between plan and apply is not
  "changed".

**Modes:**

- A new or restored file gets the mode `std::fs::write` creates.
- An updated file keeps its old mode. A rename installs the temp's mode, so `fs` copies the old
  permissions onto the temp before the rename.

**Failures:**

| Step | Result |
|---|---|
| 2 or 3 | the temps written so far are removed, nothing is swapped, the lock is untouched; I/O failure: `<path>: <io::Error Display>`. Directories already created are left |
| 4 | stop; remove the remaining temps; write the lock with only what landed (renamed files carry their new hash, the rest keep their old entries); report `<path>: <io::Error Display>` |
| 5 | `agentstacks.lock: <io::Error Display>`. The files have landed, so the next plan finds each one equal to its new bytes (the "bytes = new bytes" rows of [section 3](#3-steps)), and the lock heals |

**The lock after a step 4 failure:** `[[tool]]`, `[[pack]]` and `[airsl]` take the new values.
`[[file]]` holds new entries for files that landed and old entries for those that did not. `forget`
steps and every `unchanged` step apply, because they never touch the disk.

Directories a delete leaves empty are left in place.

**Lock content:**

- `[[tool]]` and `[[pack]]` with rev and tree;
- `[[file]]` for every non-shared path in a create, update, restore or unchanged step, with its `by`;
- `[airsl] version` exactly when a locked pack declares `requires.airsl`, so `check_airsl_presence`
  passes.

## 7. The lock

`agentstacks.lock` records tools, packs, the non-shared files with their `by`, and `[airsl]`.
Shared files are never recorded.

- There is no `[[merge]]` table. The key is unknown: `agentstacks.lock: merge: unknown key`.
- A `[[file]].path` containing `\` is refused when the lock is read:
  `agentstacks.lock: file[0].path: "a\b" must use / as the separator`.
- A lock path naming `agentstacks.toml` or `agentstacks.lock` is refused:
  `agentstacks.lock: file[0].path: "agentstacks.lock" is managed by agentstacks` (the same shape for
  `agentstacks.toml`).
- `check_airsl_presence` runs on the new lock contents only. The lock read in step 1 of the flow is
  not checked with it.

## 8. Doctor

`app/doctor` is offline. It reads the lock and the disk and returns findings; none means healthy. It
needs no extension, because shared files are not in the lock.

| Check | Finding |
|---|---|
| a lock exists | `agentstacks.lock: not found (run apply)` (then no other check) |
| lock parses | the lock's own refusals |
| each `[[file]]` exists | `missing: <path>` |
| each `[[file]]` is a regular file (a symlink to one inside the project counts) | `not a regular file: <path>` |
| each `[[file]]` hash matches | `changed: <path>` |
| `[airsl]` present: the binary is on `PATH` | `airsl: not found on PATH (lock pins 0.1.4)` |
| its `--version` parses | `airsl: --version printed "<text>", expected "airsl <version>"` |
| installed version = lock version | `airsl: 0.1.5 installed, lock pins 0.1.4` |

Two more findings: a `[[file]]` that resolves outside the project is `outside the project: <path>`,
and an I/O failure is `<path>: <io::Error Display>`.

A locked path that normalises to nothing (such as `.`, which names the project directory itself)
cannot form a project path, and is reported `not a regular file: <path>`.

Findings follow the lock's path order, then airsl.

## 9. Sources

### Reconciliation

| `agentstacks.toml` | `agentstacks.lock` | Result |
|---|---|---|
| entry | none | new: resolve (git: tag → rev; path: read) |
| entry | entry with the same source fields (`git`, `tag`, `rev`, `subdir` / `path`) | use the lock's `rev`; the git tree must equal the lock's `tree`, else refused `agentstacks.toml: packs.core: tree sha256:<a> at rev <rev> differs from the lock's sha256:<b>`; a changed path tree is a normal change, shown in the plan |
| entry | entry with different source fields | stale: resolve as new; plan note `agentstacks.toml: packs.core: source changed, resolved again` |
| none | pack entry | removed: its files get `delete` steps |
| none | tool entry | removed: [section 5](#5-removed-pack-and-removed-tool) |

A moved remote tag does not move a locked entry. The lock's `rev` wins, and no network is used.
`update` is the only way to move it.

"Same source fields" compares normalised values (`./packs/core` equals `packs/core`). Messages quote
a value as written.

### Git

Every command runs with `GIT_TERMINAL_PROMPT=0` and `-c core.autocrlf=false`, and passes the URL
after `--`. `subdir` is normalised before use. The variables that name a repository (`GIT_DIR` and
its kin) are removed from the environment, so an inherited one cannot redirect a command. The code is
`crates/agentstacks/src/process/git/`.

| Step | Command |
|---|---|
| list tags | `git ls-remote --tags --refs -- <url> 'refs/tags/<prefix>*'` → lines `<40hex>\t<ref>`; empty output with exit 0 when nothing matches |
| fetch a tag | `git init --bare -q <tmp>`; `git -C <tmp> fetch --no-tags --depth 1 -- <url> refs/tags/<tag>:refs/tags/<tag>`; rev = `rev-parse --verify --end-of-options refs/tags/<tag>^{commit}` (annotated tags need `^{commit}`) |
| fetch a rev | `git -C <tmp> fetch --no-tags --depth 1 -- <url> <rev>` |
| list files | `git -C <repo> ls-tree -r -z --full-tree <rev> -- <subdir>`; records `<mode> SP <type> SP <oid> TAB <path> NUL`, byte-ordered |
| read blobs | `git -C <repo> cat-file --batch`; bytes unconverted even with `core.autocrlf=true` |

For a source with no `subdir`, `ls-tree` omits the pathspec and lists the whole tree.

The source's tree is read into memory once and served through the `FileTree` port. Git modes map as:
`100644` and `100755` are a file, `120000` is a symlink (so the pack format's "symlinks are not
allowed" refusal applies), and any other mode (a gitlink, `160000`, included) is other. A git
source's tree label, which prefixes its load messages, is the `subdir` as written, or `.` without
one.

| Failure | Refusal |
|---|---|
| `git` not on `PATH` | once: `agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)` |
| git exits non-zero | `agentstacks.toml: packs.core: git <subcommand> failed: <git's stderr, trimmed>`, e.g. `… fatal: could not read Username for 'https://github.com': terminal prompts disabled`, `… fatal: couldn't find remote ref refs/tags/packs-v9.9.9` |
| URL starting with `-` | never reaches a flag: git refuses it itself, `fatal: strange pathname '-oops' blocked` |
| `subdir` lists nothing at that rev | `agentstacks.toml: packs.core.subdir: "packs/nope" not found at <tag or rev>` |
| `subdir` names a file | `agentstacks.toml: packs.core.subdir: "packs/core/pack.toml" is a file, not a directory` |
| a removed tool whose locked source is neither cached nor fetchable | the git refusal above; the plan is refused, and nothing is removed until the source is reachable |

Git or airsl output that does not parse (a `cat-file` frame, a `<oid> missing` answer, `rev-parse`
output that is not a commit id, an `ls-tree` record or an `ls-remote` line) is decoded as lossy UTF-8
and trimmed of surrounding whitespace. airsl exiting non-zero counts as unparseable. The refusals:

- `agentstacks.toml: packs.core: git <subcommand> printed output agentstacks cannot read: "<first line>"`
- `agentstacks.toml: airsl --version printed "<text>", expected "airsl <version>"` (`<text>` trimmed)

The decode and trim cover quoted text and `ls-remote` lines. `ls-tree -z` paths stay raw bytes.

### Cache

- **Layout:** `~/.airsstack/cache/git/<hex sha256 of url>/<rev>.git` (bare). A locked rev whose
  `<rev>.git` exists resolves without network. The directory helper is
  `crates/agentstacks/src/fs/cache.rs`, the one piece of `fs` the `process` adapter uses.
- **Filling it:**
  1. Fetch into `<url dir>/tmp-<pid>-<n>` (`<n>` counts up within the process). A tag fetch learns
     its rev here.
  2. Rename it to `<rev>.git`. A rename onto an existing non-empty directory fails with
     `DirectoryNotEmpty`; another run cached it first, so the temp is removed and the existing one is
     used.
- A leftover cache temp of the same name is removed and reused.
- A failed cache rename is reported at the `<rev>.git` target.
- **Home directory:** taken from `$HOME`. When it is unset or empty, reported once:
  `agentstacks.toml: HOME is not set, so the cache (~/.airsstack/cache) has no home`.
- Temps left by a killed process are never cleaned up.

There is no lock file for the cache. Concurrency is handled by fetching into a temporary directory
and renaming it into place.

## 10. Update

`app/update` finds the latest tag for each git source. `latest_tag(url, prefix)` returns
`Option<tag>`:

1. List `refs/tags/<prefix>*`.
2. Take each tag's suffix after `prefix`. Keep those that parse as semver and are not pre-releases.
3. Return the highest by semver. If none remains, return `None`. Equal precedence (build metadata
   only) returns the greater tag in byte order.

**Splitting a current tag into prefix and version:** the version starts at the leftmost ASCII digit
whose remainder parses as semver.

- `packs-v0.1.0` → prefix `packs-v`, version `0.1.0`
- `x-1.2.3-4.5.6` → prefix `x-`, version `1.2.3-4.5.6`

| Case | Result |
|---|---|
| current tag has no such split | note `agentstacks.toml: packs.core: tag "main-latest" has no version to update from` |
| `latest_tag` returns `None`, or nothing newer than the current tag | note `agentstacks.toml: packs.core: no newer tag than packs-v0.1.0` |
| source pinned by `rev` | note `agentstacks.toml: packs.core: pinned to rev <rev>, not updated` |
| `path` source | nothing |

`app/update` returns one entry per git source: its key, the current tag, and the new tag or the
note. Entries follow key order: `packs.*`, then `tools.*`.

The CLI owns writing a new tag into `agentstacks.toml`, shorthand expansion (prefix `packs-v`) and
its refusal when `latest_tag` returns `None`.

## 11. Binaries and the airsl pin

| Requirement | Check | Refusal |
|---|---|---|
| a pack's `requires.binaries` | `PATH` lookup through `Binary` | `agentstacks.toml: packs.core: binary "rg" not found on PATH` |
| an extension's `require_binary` | `PATH` lookup | `claude/adapter.lua:7: require_binary: "node" not found on PATH` |
| `airsl`, when any locked pack declares `requires.airsl` | `airsl --version` → `airsl 0.1.4`, parsed as `airsl <semver>` | not found: `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)` / unparseable: `agentstacks.toml: airsl --version printed "<text>", expected "airsl <version>"` / out of range: `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4` |

A binary required by both a pack and an extension is reported once for each.

"On `PATH`" means a regular file with an execute bit. Empty `PATH` entries are skipped, and an unset
`PATH` finds nothing. A symlink on `PATH` counts when it resolves to an executable regular file.

The installed airsl version is written to `[airsl] version`. A newer version in range moves the pin,
and the plan shows `airsl 0.1.4 → 0.1.5`. A missing airsl after install is caught in two ways: the
tool's own error for a missing command (under OpenCode, the shim's one-time notice to the model,
see [OpenCode](ch06-03-opencode.md#41-missing-airsl)), and `doctor` ([section 8](#8-doctor)).

## 12. Where a refusal is located

Every refusal is a `Diagnostic` with a `Location`: a file, plus a line or key path. It is displayed
as `<file>:<line>: <what>` or `<file>: <key>: <what>`, and sorted.

| Location file | Used for | Example | Raised in |
|---|---|---|---|
| `<tool>/<extension file>` with line | op arguments, `read`, `require`, `json.edit` | `claude/adapter.lua:12: write: path: expected string, got number` | `host` (reading) + `types/op` (rule) |
| `<tool>/<entry>` with key | the `values` result | `claude/adapter.lua: values.root: required` | `host` + `types/op` |
| `<tool>/<entry>`, no line | coverage, missing handler, Lua failure | `claude/adapter.lua: core/agent/coder got no operation (copy it, write it, or skip it with a reason)` | `types/op`, `host` |
| `<path>` (project-relative) | file steps | `<path>: changed since the last apply` | `types/plan`, with the resolution from `Workspace` |
| `<path>` | apply race, I/O | `<path>: changed after plan; run plan again` | `app/apply`, `fs` |
| `agentstacks.toml` with key | sources, a pack's binaries | `agentstacks.toml: packs.core: git fetch failed: <stderr>` | `process`, `types/resolve` |
| `agentstacks.toml`, no key | git, airsl and cache-home checks | `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)` | `types/op`, from a `Binary` answer |
| `agentstacks.lock` | lock write | `agentstacks.lock: <io::Error Display>` | `fs` |
| pack, extension, project and lock files | the format messages | none | `types` |

`<extension file>` is the file's path inside the extension, for example `claude/lib/settings.lua:3`.

Any I/O failure with no message of its own (the cache, a `reads` path, the lock, `agentstacks.toml`,
or spawning git or airsl) is `<path>: <io::Error Display>` at the path involved. A failed spawn:

- `agentstacks.toml: packs.core: git: <io::Error Display>`
- `agentstacks.toml: airsl: <io::Error Display>`

## 13. Not done

- A merge or run-command operation.
- Tool-specific code in the core.
- Writing `agentstacks.toml` (the CLI does).
- Network access in `doctor`.
- A lock file for the cache, and cleaning up cache temps left by a killed process.
- Exact preservation of number spelling or of the inner layout of a shared JSON file.
- Pure-policy pack hooks.
- Removing directories a delete leaves empty.
- Case-insensitive file systems: paths are compared by bytes, so `A.md` and `a.md` are two paths.
