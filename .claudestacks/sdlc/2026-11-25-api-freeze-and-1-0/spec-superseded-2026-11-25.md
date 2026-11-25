---
status: superseded
created: 2026-11-25
built-on: 53fcbbc5d1f9
---

# Spec: Freeze api = 1 and release agentstacks 1.0

agentstacks 0.2.0 runs Claude Code and OpenCode on the unstable `api = 0`. This spec:

- freezes the extension API and the pack format as `api = 1`;
- adds the one key the freeze needs, `requires.agentstacks`;
- moves 0.x projects with `agentstacks update`;
- adds `agentstacks ext add` and `agentstacks ext test`;
- writes the pack-authoring and stability chapters;
- releases agentstacks 1.0.0 with `packs-v1.0.0`.

Probe outputs live in `probes/` beside this file and are cited as `probes/<file>`. Each was run on
2026-11-25 against HEAD `33e3925` with the exact versions it names. The Rust probes are
`probes/deps/` and `probes/version/`. The shell probes are `probes/cli-probes.sh`. Build output
was deleted after each run.

## 1. Decisions

| # | Decision | By |
|---|---|---|
| D1 | One intent and one chain cover all of `foundation/03`. | author, 2026-11-25 |
| D2 | A review pass ran before the freeze. It found one gap that needs code: there is no way for a pack or extension to say which agentstacks it needs (§4). The other findings (R1–R6, §7) are settled in the stability promise. | author, 2026-11-25 |
| D3 | `requires.agentstacks` is added, optional, to `pack.toml` and `extension.toml`. | author, 2026-11-25 |
| D4 | `agentstacks update` moves a 0.x project to `api = 1`. `plan`, `apply`, `doctor` and `ext add` refuse an `api = 0` project and name `update`. | author, 2026-11-25 |
| D5 | `ext add` adds tool extensions only. This narrows the intent's "add an extension (and its packs)": a third-party pack from git is still written into `agentstacks.toml` by hand, as the new pack chapter shows. | author, 2026-11-25 |
| D6 | `ext test` runs the full check: Lua tests, a plan against a built-in conformance pack, an apply in a scratch directory, and a second plan that must change nothing. | author, 2026-11-25 |
| D7 | The Architecture section (§10) is approved, including the `Scratch` port added after review. | author, 2026-11-25 (twice) |
| D8 | The library crate `agentstacks` and the CLI crate `agentstacks-cli` are both 1.0.0, in lockstep as they are today (both 0.2.0). The library README says its Rust API is the CLI's engine and is not covered by the 1.x promise. | author, 2026-11-25 |
| D9 | Every official pack and extension version becomes 1.0.0. | settled (simplest) |
| D10 | A lock with `api = 0` is read as it is, in every 1.x. The next apply that writes anything writes `api = 1`. An apply with nothing to change writes nothing (book ch04 §5), so a lock can stay at `api = 0`, which is harmless. The lock format is unchanged (F9). | settled (simplest) |
| D11 | `ext add` with a git URL requires `--tag` or `--rev`; it does not pick a latest tag. A repository can hold several tag families (this one has `packs-v*`, `agentstacks-cli-v*` and others, F12), so no default is safe. | settled |
| D12 | `ext test` prints the plan with the same renderer `plan` uses. There is no new item table. | settled (simplest) |
| D13 | `ext test` does not run `airsl test` when the extension holds no test file, because `airsl test` exits 1 when it finds none (F5). It prints `lua tests: none`, and that is not a failure. | settled |
| D14 | claudevs and claudevs-cli are not released or changed. | intent non-goal |
| D15 | The vocabularies are frozen for 1.x. New item kinds, new neutral events and new `match`, `tools` or `model` words need `api = 2`. The coverage rule (book ch06-01 §8) would make any of them break a working 1.0 extension. | author, 2026-11-25 |
| D16 | The official shorthand (`init`, `ext add <official>`) and `update` of an official-repository `packs-v*` source pick the newest stable `packs-v` tag **whose major version equals `API`**. A 1.x binary therefore never moves to a future `packs-v2.*`, and 1.0 moves a project at `packs-v0.2.0` to the newest `packs-v1.*`. | settled (review R6a) |
| D17 | Written-file headers that say `api 0` (`assets/runtime.lua:1`, `extensions/{claude,opencode}/lib/translate.lua:7`, `extensions/opencode/lib/plugin.lua:8`) say `api 1`. Each such file changes once, so the first apply after `update` shows it as an `update` step of a locked file (not a user edit). | settled |

## 2. Proven facts

| # | Fact | Proof |
|---|---|---|
| F1 | semver 1.0.28: `">=1.3"` does not match 1.0.0 and matches 1.3.0; `">= 1.0"`, `"^1"`, `"*"`, `">=1.0, <2"` and `"1.x"` match 1.0.0. No range matches the pre-release `1.1.0-rc.1`. `"nope"` fails to parse. | `probes/deps-output-final.txt` §P1 |
| F2 | toml_edit 0.25.20: inserting a `[tools.mytool]` table placed after the last `tools.*` table keeps every comment, lands before `[packs.core]` in a file of standard tables, joins the inline table in a file with `tools = { … }`, and becomes a `[tools.mytool]` table in a file with dotted `tools.claude.path`. toml 1.2.0 re-parses all three, and the result has the keys `claude` and `mytool`. | `probes/deps-output-final.txt` §P2, §P2b |
| F3 | toml_edit 0.25.20: setting `api` to 1 with the old value's decor keeps `api = 1 # pinned by hand`. | `probes/deps-output-final.txt` §P3 |
| F4 | toml 1.2.0 parses the `pack.toml` and `extension.toml` examples of §4.1 with `requires.agentstacks = ">=1.0"` and `api = 1`. | `probes/deps-output-final.txt` §P4 |
| F5 | airsl 0.1.5 `airsl test <dir>`: passing tests exit 0 and print `1 passed, 0 failed (1 files)` as the last stdout line. A failing test exits 1 and prints `1 passed, 1 failed (1 files)` followed by the failure detail. A directory with no test file exits 1 with stderr `airsl: no test files found under …`. A missing directory exits 1 with an IO error on stderr. | `probes/airsl-test-output.txt` |
| F6 | clap 4.6.9: `ext add`/`ext test` as a nested `#[command(subcommand)]` parse with the global `-C` before or after them, and `--pack` repeats. `--tag` with `--rev` declared `conflicts_with` exits 2. `ext` alone exits 2 with its help. A field named `dir` inside `ext test` collides with the global `-C` arg id `dir`: with it, `ext test x -C proj` exits 2 with `unexpected argument '-C' found` (`probes/deps-output-2.txt` §P6b, from the earlier source with that field). The saved source names the positional `extension`, and every placement parses. | `probes/deps-output-final.txt` §P6, §P6b; `probes/deps-output-2.txt` |
| F7 | agentstacks 0.2.0 today: `api = 1` is refused as `agentstacks.toml: api 1 not supported (this agentstacks speaks 0)`, `agentstacks.lock: api 1 not supported (this agentstacks speaks 0)`, `packs/demo/pack.toml: demo: api 1 not supported (this agentstacks speaks 0)` and `ext/claude/extension.toml: extension.api: 1 not supported (this agentstacks speaks 0)`. Each exits 1. | `probes/api-refusals-output.txt` (script: `probes/cli-probes.sh` §P7) |
| F8 | agentstacks 0.2.0 today: `plan` with no `agentstacks.toml` prints `agentstacks.toml: No such file or directory (os error 2)` and exits 1. `requires.airsl` set to `""` or `"   "` gives `requires.airsl: "" is not a version range` (with the text as written); set to `1`, `requires.airsl: expected string, got integer`. `requires.agentstacks` gives `requires.agentstacks: unknown key`, and `[requires]` in `extension.toml` gives `ext/claude/extension.toml: requires: unknown key`. | `probes/edges-output.txt` (script §P8) |
| F9 | The lock format has not changed since the first release: `git log agentstacks-cli-v0.1.0..HEAD -- crates/agentstacks/src/types/lock` is empty, while the control path `types/shorthand.rs` shows commit `3aa0763`. | `probes/history-output.txt` |
| F10 | rustc 1.94.1 (`rust-toolchain.toml`), semver 1.0.28: `const VERSION: Version = Version::new(number(env!("CARGO_PKG_VERSION_MAJOR")), …)`, with `number` a `const fn` over `u64::from_str_radix` that maps `Err` to 0, passes `cargo clippy --all-targets -- -D warnings` under the workspace lint set (pedantic, nursery, `unwrap_used` and `panic` denied), and its test shows it equals `CARGO_PKG_VERSION`. | `probes/version-output.txt`, `probes/version/src/lib.rs` |
| F11 | agentstacks 0.2.0 with the conformance pack of §6.4 at `api = 0`, without `requires.agentstacks` (0.2.0 refuses that key, F8), in a project under `$TMPDIR`: `init --tool ./ext/claude --pack ./packs/conformance --yes` writes 10 files with 0 skipped, and a second `plan` reads `0 to write, 0 to delete, 10 unchanged, 0 skipped`. With `./ext/opencode` it writes 11 files, skips `conformance/hook/2` (`opencode has no hook when a session ends`) and `conformance/hook/6` (`opencode has no hook when a subagent stops`), and the second plan reads `0 to write, 0 to delete, 11 unchanged, 2 skipped`. Every run exits 0. | `probes/conformance-output.txt` (script §P15) |
| F12 | The repository's tags: `agentstacks-cli-v0.1.0`, `agentstacks-cli-v0.2.0`, `agentstacks-v0.1.0`, `agentstacks-v0.2.0`, `claudevs-cli-v0.1.2`, `claudevs-v0.1.1`, `packs-v0.1.0`, `packs-v0.2.0`. | `git tag`, run in this task |
| F13 | Current versions: packs core 0.1.1, sdlc 0.1.1, journal 0.2.0, guideline-rust 0.1.0, guideline-python 0.1.0; extensions claude 0.1.0, opencode 0.1.0; crates agentstacks 0.2.0, agentstacks-cli 0.2.0. | `probes/edges-output.txt` §versions |
| F14 | 19 lines in `book/src`, `README.md`, the crate READMEs, `CLAUDE.md` and `RELEASING.md` say `api = 0`, `api 0` or `speaks 0`. | `probes/history-output.txt` |
| F15 | `API` is defined once, at `crates/agentstacks/src/types/api.rs:26`. It is read by `types/extension/manifest.rs:205`, `types/manifest/pack/file.rs:239`, `types/lock/file.rs:162`, `types/project/file.rs:124`, `types/project/render.rs:46` and `host/payload.rs` (`"api": API`). The refusal phrase is `types/report/phrase.rs:139`. Text outside `API` that names api 0: `types/hooks/invalid.rs:106` (the `pure` refusal), `types/pack/error.rs:54` (`reserved, not supported in api 0`), the D17 headers, the doc comments `/// api = 0` in `types/hooks/{action,grants,lua,message,neutral,run}.rs` and `//!` lines in `types/hooks/{grants,invalid,policy}.rs` and `types/lock/write.rs:8`. | `grep -rn API crates/agentstacks/src/types`; `git grep -nE 'api 0\|api = 0\|speaks 0' -- crates extensions packs`; both run in this task |
| F16 | `include_str!` of an asset in `types/` has a precedent: `types/protocol/runtime.rs:33` includes `assets/runtime.lua`. | file opened in this task |
| F17 | The airsl crate 0.1.5 exports no test runner: its `lib.rs` exports the engine, extension, modules, sandbox and types, and none of its files mentions `_test.lua`. `airsl test` exists only in the binary. | `~/.cargo/registry/src/*/airsl-0.1.5/src/lib.rs:50-75`, grep run in this task |
| F18 | Fixtures and test text with `api = 0`: 233 files under `crates/agentstacks/tests` (232 `.toml` and `tests/fixtures/claude/removed/agentstacks.lock`) and 2 under `crates/agentstacks-cli/tests`; 49 `.rs` files under `crates/` with inline `api = 0` TOML; `extensions/{claude,opencode}/lib/values_test.lua:5`. Tests that use `api = 1` as the refused case: `types/set/check.rs:176-179`, `tests/pack.rs:256`, `tests/set.rs:201`, `types/lock/file.rs:562`, `types/project/file.rs:360`, `types/manifest/pack/file.rs:429`, `types/lock/error.rs:121`, `types/manifest/error.rs:254`, `tests/fixtures/extensions/invalid/api-one/`. | `grep -rlE '^api *= *0'` and `git grep -lE 'api *= *0' -- '*.rs'`, run in this task and by the reviewer |

## 3. api = 1 and the move from 0.x

### 3.1 The number

`API` becomes `1`. Every reader takes it from `types/api.rs`, which also gains `VERSION`, this
agentstacks' own version as a `const semver::Version` built as in F10. `init` writes `api = 1` (it
renders `API`). The `values` and `plan` payloads carry `"api": 1`. Every text of F15 that names
api 0 says api 1. The two refusals take the number from `API`:

- `hooks[1].grants.policy: "pure" is not supported in api 1 (airsl has no require under pure, airsstack/airsl#12)`
- `<pack>/commands: reserved, not supported in api 1`

### 3.2 What each file accepts

| File | `api = 1` | `api = 0` | any other integer |
|---|---|---|---|
| `pack.toml` | loads | refused, legacy (§3.3) | refused, unsupported |
| `extension.toml` | loads | refused, legacy | refused, unsupported |
| `agentstacks.toml` | loads | read, but only `update` goes on; every other command refuses (§3.4) | refused, unsupported, by every command |
| `agentstacks.lock` | read | read as is (D10) | refused, unsupported |

A missing `api` key and a key of the wrong type keep today's refusals (`api: required`,
`api: expected integer, got string`). The `api` checks run where they run today (F15). The project
and lock readers now return which api they read, so a flow can tell legacy from current.

### 3.3 Messages

`<found>` is what the reader names today: `api 0` for `agentstacks.toml`, `agentstacks.lock` and a
pack whose name is known, and `0` after `extension.api:` or `pack.api:` (F7).

| Case | Message |
|---|---|
| legacy pack | `packs/demo/pack.toml: demo: api 0 is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)` |
| legacy pack, name missing or invalid | `packs/demo/pack.toml: pack.api: 0 is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)` |
| legacy extension | `ext/claude/extension.toml: extension.api: 0 is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)` |
| legacy project, in `plan`, `apply`, `doctor`, `ext add` | `agentstacks.toml: api 0 is from agentstacks 0.x (run agentstacks update)` |
| unsupported, any file (here 2) | `<location>: <found> not supported (this agentstacks speaks 1)`, e.g. `agentstacks.toml: api 2 not supported (this agentstacks speaks 1)`, `agentstacks.lock: api 2 not supported (this agentstacks speaks 1)` |

The unsupported shape is today's phrase with the number from `API` (F7, F15). A negative number is
unsupported. `this agentstacks speaks 1` is the same phrase in both cases, so the reader of a
legacy file learns the target number.

### 3.4 `update` on a legacy project

Step numbers are the book's (ch04 §7). `update` runs as today, with these changes:

- **Before step 1:** it reads `agentstacks.toml` with `api` 0 or 1.
- **Step 1:** newer tags are listed as today, with D16's major rule for official `packs-v` sources.
- **Step 2:** with `api = 0`, it prints `  api: 0 → 1` on stdout before the tag entries.
- **Step 3:** `nothing to update` is printed only when there is no new tag **and** `api` is 1.
- **Step 4:** the rewrite sets each new tag (as today) and, with `api = 0`, sets `api = 1`,
  keeping the value's decor (F3), then prints `agentstacks.toml updated`. Step 4's re-read refusals
  gain one row: `api` is no longer 0 at re-read (the file was edited after `update` read it) gives
  `agentstacks.toml: api changed during update (run update again)`.
- **Step 5:** it plans, confirms and applies as today. The plan reads the old lock (D10).

A path source or third-party git source still at `api = 0` is refused by the step-5 plan with the
legacy pack or extension message. Step 5's existing follow-up covers that:
`agentstacks.toml updated; fix the above and run apply`, exit 1.

Expected output, new behaviour (not a run; it follows today's `  <key>: <current> → <new>` format
and the packs-then-tools order). A 0.2.0 project with `api = 0` and `[tools.claude]` plus
`[packs.core]` at `packs-v0.2.0`, after `packs-v1.0.0` exists:

```text
  api: 0 → 1
  packs.core: packs-v0.2.0 → packs-v1.0.0
  tools.claude: packs-v0.2.0 → packs-v1.0.0
agentstacks.toml updated
```

A case that could break the rule: a 0.2.0 binary already ran `update` against `packs-v1.0.0`, so
the tags say `packs-v1.0.0` while `api` still says 0. 1.0's `update` prints `  api: 0 → 1` and no
tag entries, rewrites only `api`, and plans. It never prints `nothing to update` (step 3).

## 4. `requires.agentstacks`

### 4.1 Format

`requires.agentstacks` is an optional semver range. It goes in `pack.toml` `[requires]`, and in a
new optional `[requires]` table in `extension.toml` that holds only this key. When it is absent,
any 1.x is accepted. These examples parse (F4):

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

```toml
[extension]
name        = "mytool"
version     = "1.0.0"
api         = 1
description = "Renders packs into mytool project config"
entry       = "adapter.lua"

[requires]
agentstacks = ">=1.0"
```

### 4.2 Rules

- **One range type.** Today's `AirslReq` becomes `VersionRange`, and its error `InvalidAirslReq`
  becomes `InvalidRange`, with the same messages. The file moves from
  `types/manifest/airsl.rs` to `types/manifest/range.rs`. `requires.airsl` and
  `requires.agentstacks` both hold a `VersionRange` (modularity Rule 2), and the eight-operator
  allow-list applies to both.
- **Matching.** The range is matched against `VERSION` with `VersionReq::matches` (F1).
  agentstacks is never released as a pre-release (§8), so F1's pre-release behaviour cannot exclude
  a released binary.
- **Checked first.** `Pack::load` (`types/pack/load.rs`) and `Extension::load`
  (`types/extension/loaded.rs`) read `pack.toml` or `extension.toml` from the tree before anything
  else: before the walk's refusals, layout, frontmatter, bodies and every other key. When the text
  is readable, parses as TOML and has a `requires.agentstacks` that parses and excludes
  `VERSION`, the load returns that one refusal and nothing else. A file written for a later 1.x may
  use things this version does not know, so its other refusals would mislead. That covers an
  `api` refusal, unknown keys, unknown tokens in `SKILL.md` and walk refusals. The rule is pure (in
  `types/`), because `VERSION` is a constant.
- **Otherwise as today.** When the file is unreadable or not valid TOML, or the range does not
  parse, loading goes on as today. A bad range is one refusal among the file's others, as
  `requires.airsl` is today.

### 4.3 Messages

| Case | Message |
|---|---|
| excludes this version (pack) | `packs/team-lint/pack.toml: requires.agentstacks: ">=1.3" does not match agentstacks 1.0.0 (install a newer agentstacks)` |
| excludes this version (extension) | `ext/mytool/extension.toml: requires.agentstacks: ">=1.3" does not match agentstacks 1.0.0 (install a newer agentstacks)` |
| not a range, empty or whitespace-only | `packs/team-lint/pack.toml: requires.agentstacks: "nope" is not a version range` (the text as written; same as `requires.airsl`, F8) |
| wrong type | `packs/team-lint/pack.toml: requires.agentstacks: expected string, got integer` |
| unsupported operator | `packs/team-lint/pack.toml: requires.agentstacks: "<range>" uses an operator this agentstacks does not support` (same as `requires.airsl`) |
| `extension.toml` unknown key in `[requires]` | `ext/mytool/extension.toml: requires.foo: unknown key` |
| `extension.toml` `requires` not a table | `ext/mytool/extension.toml: requires: expected table, got string` |

The range is quoted as written, as `VersionRange`'s `Display` does today.

## 5. `agentstacks ext add`

### 5.1 Grammar

```text
agentstacks [-C <DIR>] ext add <SOURCE> [--tag <TAG> | --rev <REV>] [--subdir <DIR>] [--yes]
```

`--tag` and `--rev` conflict (clap `conflicts_with`, exit 2, F6). `ext` with no subcommand is a clap
usage error, exit 2 (F6).

### 5.2 Classifying `<SOURCE>`

The rule extends `types/shorthand.rs` `classify` with a new `Flag::Ext`. It displays as `ext add`,
its noun is `tool`, and it uses the official tool list.

| `<SOURCE>` | Kind |
|---|---|
| contains `://` or starts with `git@` | git URL (`GitUrl`, which keeps its own refusals) |
| else contains `/` or starts with `.` | project path (`LocalPath`, init's rules) |
| else | official tool name (`claude`, `opencode`): the official repository, the newest `packs-v1.*` tag (D16), `subdir = "extensions/<name>"`, as `init --tool` does |

`init` is unchanged: it takes no git URL.

### 5.3 Steps

1. Read `agentstacks.toml` and keep its bytes. When it is absent, print today's
   `agentstacks.toml: No such file or directory (os error 2)` (F8) and exit 1. Parse refusals are
   printed, exit 1. With `api = 0`, print the legacy project refusal of §3.3, exit 1.
2. Classify `<SOURCE>` and check the flags (§5.4). Every refusal is printed, nothing is written,
   exit 1.
3. Resolve the source as a plan does: git fetches the tag or rev into the cache, a path reads the
   directory. Load it with `Extension::load`. Load refusals (including §3.3 and §4.3) are printed,
   exit 1.
4. The key is `tools.<extension.name>`. When `agentstacks.toml` already has it:
   `agentstacks.toml: tools.<name> already exists (edit it there)`, exit 1.
5. Re-read `agentstacks.toml`. When its bytes differ from step 1's:
   `agentstacks.toml: changed during ext add (run it again)`, exit 1, nothing written. Otherwise
   insert the table (F2): keys in the order `git`, `tag` or `rev`, `subdir` (or `path`), the path
   as written, placed after the last `tools.*` table. Inline and dotted `tools` forms are kept as
   F2 shows. Write the file and print `agentstacks.toml: added tools.<name>` on stdout.
6. Plan, show, confirm and apply exactly as `update` step 5 does (book ch04 §7), with the same
   follow-ups and exit codes.

### 5.4 Refusals

The location is `ext add "<SOURCE>"` unless a row says otherwise.

| Case | Message |
|---|---|
| unknown bare name | `ext add "x": not an official tool; pass a path or a git URL` |
| git URL with neither `--tag` nor `--rev` | `ext add "https://example.com/x.git": a git URL needs --tag or --rev` |
| `--tag`, `--rev` or `--subdir` with a path or an official name | `ext add "./ext/x": --tag applies only to a git URL` (named per flag; one refusal per flag given) |
| `--rev` not a commit id | `ext add "<url>": --rev: not a commit id` |
| `--subdir` leaves the repository | `ext add "<url>": --subdir: must stay inside the repository` |
| empty `--tag`, `--rev` or `--subdir` | `ext add "<url>": --tag: empty` (same shape per flag) |
| path absolute, climbing out, or out through a symlink | init's path refusals at this location: `ext add "/x": absolute paths are not portable` / `ext add "../x": must stay inside the project` / `ext add "./x": resolves outside the project` |
| git not on `PATH` | today's variant: `ext add: git not found on PATH (needed by ext add "<SOURCE>")` |
| git fails | today's variant (`types/resolve/error.rs:73`), located `ext add`: `ext add: git <subcommand> failed: <git's stderr, trimmed>` |
| no `packs-v1.*` tag (official name) | `ext add "claude": no packs-v1 release tag in https://github.com/airsstack/agentstacks` |
| key exists | `agentstacks.toml: tools.<name> already exists (edit it there)` |
| file changed since step 1 | `agentstacks.toml: changed during ext add (run it again)` |
| write fails | `agentstacks.toml: <io::Error Display>` |

`init`'s no-tag refusal gains the same major: `--pack "core": no packs-v1 release tag in https://github.com/airsstack/agentstacks`.

## 6. `agentstacks ext test`

### 6.1 Grammar

```text
agentstacks [-C <DIR>] ext test <EXTENSION> [--pack <DIR>]...
```

The positional is named `extension`, not `dir`, because a field named `dir` collides with the
global `-C` arg (F6). `<EXTENSION>` and each `--pack` are resolved against `-C`'s directory, or
the current directory without it. Absolute paths are accepted, because nothing is written into
them.

### 6.2 Steps

Nothing is ever written into the user's project or the extension directory. Steps 1, 2, 4, 5 and
6 stop the run when they fail. A step-3 failure does not stop it.

| # | Step | Pass | Fail |
|---|---|---|---|
| 1 | `airsl` on `PATH` (`Binary::on_path`) | — | `ext test: airsl not found on PATH (needed for the Lua tests and the hooks)`; stops, exit 1, no verdict line |
| 2 | Open `<EXTENSION>` (`Scratch::open`) and load it with `Extension::load` (all manifest, §3.3 and §4.3 rules) | prints `ext test <name> <version>` | its open or load refusals; stops, exit 1, no verdict line |
| 3 | Lua tests: when the extension's walked file list (`types::tree::walk`, as `Extension::load` walks at `types/extension/loaded.rs:101-102`) holds a file named `*_test.lua` or `test_*.lua`, run `airsl test <EXTENSION>` (`Binary::test`) | exit 0: `lua tests: <last non-empty stdout line>`, for example `lua tests: 71 passed, 0 failed (5 files)` (F5). With no test file: `lua tests: none` (D13), not a failure | exit ≠ 0: `lua tests: failed`, then airsl's stdout on our stdout and its stderr on our stderr; the run goes on and the verdict is `failed` |
| 4 | Stage the scratch project (§6.3): `types/check/stage.rs` builds the file set, `Scratch::create` writes it | — | a staging refusal (§6.5); stops, exit 1, verdict `failed` |
| 5 | Plan in the scratch project | the plan text, rendered as `plan` renders it (D12) | the plan's refusals; stops, exit 1, verdict `failed` |
| 6 | Apply that plan, without asking | `applied: <n> written, <m> deleted` | the apply refusal; stops, exit 1, verdict `failed` |
| 7 | Plan again | `second plan: nothing to change` | `second plan: <n> to write, <m> to delete (apply is not repeatable)`, then that plan's steps as `plan` renders them; verdict `failed` |
| 8 | Verdict (from step 3 on, every run ends with it) | last stdout line `passed`, exit 0 | last stdout line `failed`, exit 1 |

Any step in the second plan, whether it writes or deletes, fails step 7. The scratch project is
removed when the command ends, on success or failure. Refusals go to stderr and everything else to
stdout, as in book ch04 §2.

### 6.3 The scratch project

`types/check/stage.rs` computes the file set, a pure rule over `FileTree`s. `Scratch::create`
writes it (translation only):

```text
<tmp>/
  agentstacks.toml                 api = 1; [tools.<name>] path = "ext/<name>"; [packs.conformance] path = "packs/conformance";
                                   [packs.<p>] path = "packs/<p>" for each --pack, rendered like init (tools, then packs in name order)
  ext/<name>/                      every walked file of <EXTENSION>
  packs/conformance/               the built-in pack (§6.4), plus native/<name>/note.md
  packs/<p>/                       every walked file of each --pack, <p> being the name in its pack.toml
```

The files come from `types::tree::walk` over the tree `Scratch::open` returns. The walk skips dot
entries, and its refusals for a symlink or a special file (`types/tree/walk/error.rs:39-42`) are
the staging refusals. No adapter decides any of this.

### 6.4 The conformance pack

The pack is embedded in the binary (`include_str!`/`include_bytes!`, F16) from
`crates/agentstacks/assets/conformance/`. It holds every **neutral** item kind (skill file, skill
group, agent, hook, native file), every neutral hook event, every token kind, every var and every
option type. The reviewer checked these counts against the code: events 6/6, token kinds 6/6, vars
6/6, option types 3/3, agent tools 7/7.

Its text is the probe pack `probes/conformance/` with two changes: `api = 1`, and
`agentstacks = ">=1.0"` in `[requires]`. The probe used `api = 0` without that key, because 0.2.0
refuses it (F8). The asset tree has no `native/`. Instead `types/check/stage.rs` adds
`native/<name>/note.md` with the text `Native note for <name>.\n` for the tool under test.

**Native hooks are not included.** Their `event` and `match` are the tool's own names (book ch06-01
§6), so no generic one is valid for an unknown tool. The extension's own Lua tests cover them.

`hooks/event_test.lua` becomes a real test of `hooks/event.lua`. The `lua` task in `Makefile.toml`
adds `crates/agentstacks/assets/conformance` to its `airsl check` and `airsl test` roots, so the
embedded Lua is checked by `cargo make dod`.

```text
conformance/
  pack.toml                        render data/settings.json; exclude **/*_test.lua; options flag/count/label;
                                   hooks: message session.start; lua session.start, session.end,
                                   prompt.submit, tool.before (may_block), tool.after, agent.stop (agents, grants)
  skills/check/SKILL.md            invocation user, argument_hint, every token kind, every var, an escaped \{{
  skills/check/references/notes.md a rendered skill file
  skills/check/logo.png            a binary skill file (copy)
  agents/checker.md                model, effort, all seven tools
  hooks/event.lua                  the hook module (a root file)
  hooks/event_test.lua             excluded from install; tests event.lua
  data/settings.json               a rendered root file
```

Against 0.2.0 with `api = 0`, both official extensions cover this pack with no refusal, and the
second plan writes nothing (F11). That is the expected `ext test` result for them at 1.0.

### 6.5 Refusals

| Case | Message |
|---|---|
| `airsl` missing | `ext test: airsl not found on PATH (needed for the Lua tests and the hooks)` |
| `<EXTENSION>` or a `--pack` path does not exist, is not a directory, or I/O | `<path as written>: <io::Error Display>` |
| `<EXTENSION>` load refusals | `Extension::load`'s own messages, labelled with `<EXTENSION>` as written |
| `--pack` directory without `pack.toml` | `--pack "<dir>": not a pack (no pack.toml)` |
| `--pack` `pack.toml` unreadable, or `pack.name` missing or invalid | that file's own refusals |
| two `--pack` with one name | `--pack "<b>": pack "x" is already given by --pack "<a>"` |
| a `--pack` named `conformance` | `--pack "<dir>": pack "conformance" is already given by the built-in conformance pack` |
| a symlink or special file in a walked tree | the walk's own wording: `<path>: symlinks are not allowed in an extension` / `in a pack` / `<path>: only regular files and directories are allowed` |
| scratch directory or write I/O | `ext test: <io::Error Display>` |

A `--pack` that requires a pack not given is refused by the plan's set check, with its own message
(`… requires pack "core", which agentstacks.toml does not list …`). The author passes every pack
it needs.

## 7. The 1.x stability promise

The new book page `ch06-06-stability.md` states the promise.

**Covered.** Anything that works on agentstacks 1.N works on every 1.M with M ≥ N:

- the pack format: layout, `pack.toml`, skill and agent frontmatter, tokens and vars, the hook
  schema and event vocabulary, and `native/<tool>/`;
- the extension API: `extension.toml`, the sandbox and its limits, the `agentstacks` table, the
  `values` and `plan` events and their payload fields, operations, and the coverage and validation
  rules;
- the hook runtime: the command line, `runtime.lua`'s behaviour, the neutral event and result, and
  the translator contract;
- `agentstacks.toml`;
- `agentstacks.lock`, whose format is frozen for all of 1.x: every 1.x reads and writes the same
  lock, and reads an `api = 0` lock (D10);
- the CLI's commands, flags and exit codes.

**Allowed in a 1.x minor release (additive only):**

- new optional keys that only the core reads;
- new payload fields (extensions ignore them);
- new `agentstacks.*` functions;
- new vars, because a var an extension does not supply is already a visible skip (book ch06-01 §6);
- new commands and flags;
- allowing a refused value that creates no new item, such as `grants.policy = "pure"` (R1).

**Not allowed before `api = 2` (D15):**

- new item kinds, such as `commands/` and `rules/` (R2);
- new neutral events;
- new `match`, agent `tools` or `model` words;
- any change that would give a 1.0 extension an item it cannot cover.

| # | Finding | Settled as |
|---|---|---|
| R1 | `grants.policy = "pure"` is refused while airsl#12 is open; its text says `api 1` | allowing it later is an addition |
| R2 | `commands/` and `rules/` are reserved; their refusal says `api 1` | supporting them needs `api = 2` (D15) |
| R3 | a hook with no `match` gets every value | the vocabulary is frozen (D15), so no value it does not know can appear in 1.x |
| R4 | item ids such as `core/hook/0` | ids are opaque strings, copied back and never parsed |
| R5 | exact refusal and message text | not covered; only the location and key shapes are |
| R6 | a future `packs-v2.*` would be fetched by 1.x shorthand and `update` | D16 limits them to `packs-v1.*` |
| R7 | a 1.N key in `agentstacks.toml` read by 1.0 gives `unknown key` | stated on the page: a team that uses a 1.N key needs 1.N everywhere. No gate is added (simple design) |

Authors declare `requires.agentstacks = ">=1.N"` when a pack or extension uses a 1.N addition.

**Not covered:** the exact wording of refusals, notes and prompts; the plan text layout; `--help`
text; the Rust API of the `agentstacks` crate (D8); claudevs.

`2.0` is the only release that may remove or change a covered behaviour. It would ship as `api = 2`.

## 8. Release 1.0

- **Versions.** `agentstacks` and `agentstacks-cli` become 1.0.0, with the workspace dependency
  `agentstacks = { path = …, version = "1.0.0" }`. Every pack and extension becomes 1.0.0 (D9,
  from F13). The tag is `packs-v1.0.0`. claudevs is untouched (D14). No version is a pre-release
  (§4.2).
- **`api = 1` everywhere valid.** This covers the five `pack.toml`, the two `extension.toml`, and
  every fixture and test text of F18 meant to be valid: the `.toml` and `.lock` fixtures, the
  inline TOML in `.rs` files, and `values_test.lua`. The F18 tests that use `api = 1` as the
  refused case are rewritten to test §3.3's two cases: a legacy `api = 0` and an unsupported
  `api = 2`. Gate: afterwards,
  `git grep -nE 'api *= *0|api 0|speaks 0' -- crates extensions packs book README.md CLAUDE.md RELEASING.md`
  prints only legacy-case tests and 0.x history lines, and the plan names each kept line.
- **CHANGELOGs.** `crates/agentstacks/CHANGELOG.md`, `crates/agentstacks-cli/CHANGELOG.md` and
  `packs/CHANGELOG.md` each get a `1.0.0` entry. The crate entries cover the api freeze,
  `requires.agentstacks`, the 0.x move, `ext add`, `ext test` and D16; the packs entry covers
  api 1, the versions and the D17 headers. The date stays `Unreleased` until the release pull
  request.
- **`RELEASING.md`.** Today's steps stay, titled as the 0.1.0 record. A new **Later releases**
  section is the reusable runbook, written for `<version>`:
  1. The release pull request is green: `cargo make dod`, `cargo make claudevs-check`,
     `cargo make opencode-check`, `cargo deny check`, the `installers` and `book` jobs, and the
     release workflow's plan job. Replace `Unreleased` with the day of steps 2–5, then merge.
  2. Push `packs-v<version>` first. A 0.x binary that runs `init` or `update` after this point
     fetches `api = 1` packs and is refused (F7's shape), so keep the window to step 3 short.
  3. Push `agentstacks-cli-v<version>` and wait for its release workflow to finish green.
  4. Verify in one subshell with a throwaway home, as today's step 5 does. The installer also
     installs airsl.

     ```sh
     (
       export HOME="$(mktemp -d)" PATH=/usr/bin:/bin
       curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh
       export PATH="$HOME/.local/bin:$PATH"
       agentstacks --version    # agentstacks <version>
       src="$(mktemp -d)" && git clone -q --depth 1 --branch packs-v<version> https://github.com/airsstack/agentstacks "$src"
       agentstacks ext test "$src/extensions/claude"     # passed
       agentstacks ext test "$src/extensions/opencode"   # passed
       cd "$(mktemp -d)" && agentstacks init --tool claude --pack core --yes \
         && grep 'tag = "packs-v<version>"' agentstacks.toml && agentstacks doctor    # healthy
     )
     ```

  5. Run `cargo publish -p agentstacks`, then `cargo publish -p agentstacks-cli`. Push
     `agentstacks-v<version>`.
  6. Commit the org docs (§9) and run `/claudestacks-sdlc:milestone foundation/03` to roll up.
- **No change to dist, the installers or CI.** The release workflow starts on
  `agentstacks-cli-v[0-9]+.[0-9]+.[0-9]+*` and `claudevs-cli-v…` tags
  (`.github/workflows/release.yml:48-50`). `packs-v*` tags start no release
  (`.github/workflows/release.yml:42-43`). The installer looks up `"tag_name": "<app>-v<x.y.z>"`
  (`install/install.sh:89`). None of these names changes.

## 9. Documentation

| File | Change |
|---|---|
| `book/src/ch06-04-pack-authoring.md` (new) | "Writing a Pack", step by step for an agent to follow, in the style of ch06-00: read first, what you build, rules, steps (layout, `pack.toml`, skills, agents, hooks, options, native, `requires.agentstacks`, try it with `init` on a scratch project), checklist, common refusals |
| `book/src/ch06-05-pack-format.md` (new) | the pack format contract at `api = 1`, taken from the code as built: layout, `pack.toml` fields and refusals, tokens and vars, frontmatter, hooks, options, `native/<tool>/`, `requires.agentstacks` |
| `book/src/ch06-06-stability.md` (new) | §7 |
| `book/src/ch03-01-from-0x.md` (new) | moving a project from 0.x: install 1.0 first, run `agentstacks update`, what to do about path and third-party sources (the §3.3 messages), why an old binary refuses `packs-v1.0.0` (E24) |
| `book/src/SUMMARY.md` | the four pages above, under their chapters |
| `book/src/ch04-00-cli.md` | §1 commands and §2 streams gain `ext add` and `ext test`; new sections for both (§5, §6); `update` gains §3.4; D16's major rule in §3 |
| `book/src/ch05-00-plan-and-apply.md` | the lock reads api 0 (D10); §10's latest-tag rule gains D16 |
| `book/src/ch06-00-authoring.md` | `api = 1`; `[requires] agentstacks`; step 6 and the checklist use `agentstacks ext test`; a link to the stability page |
| `book/src/ch06-01-extensions.md`, `ch06-02-hooks.md`, `ch06-03-opencode.md` | every `api = 0` becomes `api = 1`; the "unstable until …" sentences become a link to the stability page; `[requires]` in ch06-01 §2 |
| `book/src/introduction.md`, `ch01-02-first-project.md`, `README.md`, `crates/agentstacks/README.md`, `crates/agentstacks-cli/README.md` | `api = 1`; the "unstable during 0.x" sentence becomes the stability link; the library README adds D8's sentence |
| `CLAUDE.md` | Status: 1.0 released, `api = 1` frozen. The invariant "stays `api = 0` (unstable) until it is frozen as `api = 1` in M3" becomes "frozen as `api = 1`; 1.x changes are additive only (book ch06-06)". The CLI surface line gains `ext add` and `ext test` |
| org root `~/Projects/airsstack/README.md` and `~/Projects/airsstack/.github/profile/README.md` | the agentstacks status line says 1.0. Committed in the org root after merge (RELEASING step 6), outside this repository |

The §8 grep gate covers these files too. `cargo make book-build` has no `[ERROR]` line.

## 10. Architecture

The author approved this section on 2026-11-25 (D7). After review it gained the `Scratch` port,
and that was approved too. The rules applied come from `claudestacks-guideline-rust:rust-guidelines`
and `book/src/ch07-00-architecture.md`:

| Rule (reference) | Applied as |
|---|---|
| modularity Rule 2, one type per concept (`modularity.md`) | one `VersionRange` for both ranges; `Flag::Ext` extends the existing classifier instead of adding a second one; the scratch project reuses the plan and apply flows and the existing tree walk |
| modularity Rule 1, no God object (`modularity.md`) | `app/ext/` is split by role (`add`, `check`); `types/check/` holds rules only, one concept per file |
| `mod.rs` is export-only (`mod-rs-export-only.md`) | every new `mod.rs` holds the four-question `//!` block and `mod`/`pub use` lines only |
| static dispatch (`static-dispatch.md`) | the new flows are generic over `Workspace`, `Git`, `Binary`, `Adapter`, `Scratch`; `Scratch` uses associated types (`type Tree: FileTree; type Project: Workspace`), with no `dyn` |
| strong types (`strong-types.md`) | `VersionRange`, `GitUrl`, `LocalPath`, `SubPath` and `ToolName` carry the parsed values; there is no `bool` parameter for a semantic flag |
| unit-test mandate (`unit-test-mandate.md`) | every new logic file has a colocated `#[cfg(test)] mod tests` |
| layering (ch07; adapters make no decisions) | rules in `types/`, flows in `app/`, translation in `fs/` and `process/`; the walk refusals stay in `types::tree::walk`; `tests/isolation.rs` is unchanged and green |
| naming (memory: one word, no `_`) | `range`, `upgrade`, `add`, `check`, `scratch`, `ext`. `check`, not `test`, so that a `test.rs` is never confused with `#[cfg(test)] mod tests` |
| errors live with their context | each new error enum sits in its concept's file or its folder's `error.rs` |

```text
crates/agentstacks/src/
├─ types/
│  ├─ api.rs              ~ API = 1; VERSION (F10); the api decision (current · legacy 0 · unsupported) and its error
│  ├─ manifest/range.rs   ← airsl.rs renamed: VersionRange + InvalidRange (same messages)
│  ├─ manifest/requires.rs ~ + agentstacks: Option<VersionRange>
│  ├─ manifest/pack/file.rs ~ legacy and unsupported api; requires.agentstacks parsed
│  ├─ pack/load.rs        ~ the excluded-range check first (§4.2)
│  ├─ pack/error.rs       ~ reserved text from API (§3.1)
│  ├─ hooks/invalid.rs    ~ the pure text from API (§3.1); doc comments in hooks/* say api = 1
│  ├─ extension/manifest.rs ~ + [requires] agentstacks; legacy and unsupported api
│  ├─ extension/loaded.rs ~ the excluded-range check first (§4.2)
│  ├─ project/file.rs     ~ reads api 0 or 1; the parsed file says which
│  ├─ project/upgrade.rs  + api 0 → 1 rewrite keeping decor (F3), with the re-read refusal of §3.4
│  ├─ project/add.rs      + insert [tools.<name>] (F2), refusing an existing key
│  ├─ lock/file.rs        ~ reads api 0 or 1; lock/write.rs writes API
│  ├─ report/phrase.rs    ~ the legacy and unsupported phrases
│  ├─ resolve/update.rs   ~ latest tag: D16's major rule for the official packs-v family
│  ├─ shorthand.rs        ~ Flag::Ext; Argument::Git; ext add's not-official text; D16
│  └─ check/              + ext test rules
│     ├─ mod.rs             exports only
│     ├─ stage.rs           the scratch file set from the walked trees and the assets, its agentstacks.toml text, the --pack name rules (§6.3, §6.5)
│     ├─ conformance.rs     the embedded pack (include_str!/include_bytes!, F16)
│     ├─ lua.rs             whether the walked files hold a test; the summary line of an airsl test run (F5)
│     └─ verdict.rs         the second-plan check and the passed/failed verdict
├─ port/
│  ├─ binary/seam.rs      ~ + fn test(&self, dir: &Path) -> LuaTestRun (exit status, stdout, stderr); FakeBinary gains it
│  └─ scratch/            + the Scratch port
│     ├─ mod.rs             exports only
│     ├─ seam.rs            trait Scratch { type Tree: FileTree; type Project: Workspace;
│     │                       fn open(&self, dir: &Path) -> io::Result<Self::Tree>;
│     │                       fn create(&self, files: &[StagedFile]) -> io::Result<Self::Project>; }
│     └─ fake.rs            FakeScratch: FakeFileTree per dir, FakeWorkspace preloaded with the files
├─ app/
│  ├─ update.rs           ~ the api step (§3.4)
│  ├─ init.rs             ~ D16 through shorthand
│  ├─ plan/, apply/, doctor.rs ~ refuse a legacy project (§3.3)
│  └─ ext/
│     ├─ mod.rs             exports only
│     ├─ add.rs             §5.3 steps 1–5
│     └─ check.rs           §6.2 steps 1–8, in order, generic over the ports
├─ fs/scratch.rs          + Scratch on disk: open = the existing disk FileTree at dir; create = a tempfile::TempDir
│                           holding the files, wrapped in the disk Workspace; the TempDir is dropped with it
├─ process/binary.rs      ~ + runs `airsl test <dir>` (F5)
└─ assets/
   ├─ runtime.lua         ~ header says api 1 (D17)
   └─ conformance/        + the conformance pack (§6.4)

crates/agentstacks-cli/src/
├─ cli.rs                 ~ + Ext(Ext) with Add { source, tag, rev, subdir, yes } and Test { extension, pack } (F6)
├─ command/ext/{mod,add,check}.rs + wiring, confirmation, follow-ups, exit codes
└─ show/check.rs          + the ext test lines of §6.2

extensions/{claude,opencode}/lib/translate.lua, extensions/opencode/lib/plugin.lua   ~ headers say api 1 (D17)
Makefile.toml             ~ the lua task also covers crates/agentstacks/assets/conformance
```

`tempfile` moves from `[dev-dependencies]` to `[dependencies]` of `agentstacks`, and only `fs/`
uses it. It is already in `Cargo.lock` (`Cargo.lock:987`, as a dev-dependency), so no new crate
enters the tree.

The dependency direction is unchanged: `app/ext` uses `port` and `types`, `fs/scratch` implements
`port::Scratch`, and the CLI wires the adapters in.

## 11. Edge cases and owners

| # | Case | Result | Owner |
|---|---|---|---|
| E1 | `agentstacks.toml` `api = 0`, command `plan`/`apply`/`doctor`/`ext add` | legacy project refusal | the `types/api.rs` rule, called by `app/plan`, `app/apply`, `app/doctor.rs`, `app/ext/add.rs` |
| E2 | `api = 0`, `update`, no newer tag | `api: 0 → 1`, rewrite, plan | `app/update.rs` |
| E3 | `api = 0` edited to 1 between `update`'s read and rewrite | `api changed during update` | `types/project/upgrade.rs` |
| E4 | lock `api = 0`, project `api = 1` | read; the next apply that writes something writes `api = 1` | `types/lock/file.rs` |
| E5 | `api = -1` or `2` in any file | unsupported refusal | each reader |
| E6 | `requires.agentstacks` excludes VERSION, and the pack also has an unknown key, `api = 2`, an unknown token in `SKILL.md` and a symlink | only the range refusal | `types/pack/load.rs`, `types/extension/loaded.rs` |
| E7 | `requires.agentstacks = ""` / `"   "` / `1` / `"nope"` | §4.3 rows | `types/manifest/range.rs`, `requires.rs` |
| E8 | `requires.agentstacks` absent | any 1.x | `types/manifest/requires.rs` |
| E9 | `ext add` of a tool already listed, under another source | `tools.<name> already exists (edit it there)` | `types/project/add.rs` |
| E10 | `ext add` with `tools` inline or dotted | inserted as F2 shows | `types/project/add.rs` |
| E11 | `ext add` URL with both `--tag` and `--rev` | clap exit 2 | `cli.rs` |
| E12 | `ext add` source whose extension is legacy or too new | its load refusal; nothing written | `Extension::load` |
| E13 | `ext add` plan refused after the write | `agentstacks.toml updated; fix the above and run apply`, exit 1; the file keeps the new table | CLI `command/ext/add.rs` |
| E14 | `ext add` with no `agentstacks.toml` | today's I/O message (F8) | `app/ext/add.rs` |
| E15 | `ext add`: the file is edited during the fetch | `changed during ext add (run it again)` | `app/ext/add.rs` |
| E16 | `ext test` extension with no test file | `lua tests: none`, not a failure | `types/check/lua.rs` |
| E17 | `ext test` Lua tests fail but the plan passes | the run goes on; verdict `failed`, exit 1 | `app/ext/check.rs` |
| E18 | `ext test` second plan has a write or a delete | listed; `failed`, exit 1 | `types/check/verdict.rs` |
| E19 | `ext test` `--pack` duplicate name or named `conformance` | §6.5 | `types/check/stage.rs` |
| E20 | `ext test` `--pack` requiring a pack not given | the set check's refusal | the plan flow |
| E21 | `ext test` tree with a symlink or special file | the walk's refusal | `types::tree::walk`, via `types/check/stage.rs` |
| E22 | `ext test` scratch under a symlinked `$TMPDIR` (macOS `/var`) | works; F11 ran there | `fs/scratch.rs` (its contract test runs under `$TMPDIR`) |
| E23 | `ext test` extension that skips many items | not a failure; skips are shown with reasons | `types/check/verdict.rs` (only a refusal or a second-plan step fails) |
| E24 | 0.2.0 binary after `packs-v1.0.0` exists | refused by 0.2.0 (F7's shape); ch03-01 says to install 1.0 first | docs |
| E25 | a future `packs-v2.0.0` and a 1.x binary | shorthand and `update` stay on `packs-v1.*` | `types/resolve/update.rs`, `types/shorthand.rs` (D16) |
| E26 | `ext add -- -x` (a bare name starting with `-`) | `ext add "-x": not an official tool; pass a path or a git URL` | `types/shorthand.rs` |

## 12. Testing

| Area | Tests |
|---|---|
| `types/api.rs` | `api_is_one`; `version_is_the_package_version` (F10); `legacy_project_names_update`; `legacy_pack_and_extension_name_api_one`; `two_and_minus_one_are_unsupported` |
| `types/manifest/range.rs` | today's `AirslReq` tests, renamed; `agentstacks_range_messages_match_airsl` |
| `types/pack/load.rs`, `types/extension/loaded.rs` | `excluding_range_is_the_only_refusal` (E6: an unknown key, `api = 2`, an unknown token in `SKILL.md` and a symlink in the same pack; for the extension, a walk refusal and an unknown key); `invalid_range_is_one_refusal_among_others`; `absent_range_loads` |
| `types/extension/manifest.rs` | `extension_requires_unknown_key`; `extension_requires_not_a_table` |
| `types/project/file.rs`, `lock/file.rs` | `project_reads_api_zero_as_legacy`; `lock_reads_api_zero`; `lock_refuses_api_two` |
| `types/project/upgrade.rs` | `api_zero_becomes_one_keeping_comment` (F3); `api_changed_since_read_is_refused` |
| `types/project/add.rs` | `inserts_after_last_tool_table`; `inline_tools`; `dotted_tools` (F2 texts); `existing_key_refused`; `keys_in_order_git_tag_subdir`; `path_source_as_written` |
| `types/shorthand.rs`, `types/resolve/update.rs` | `ext_classifies_url_path_and_official`; `ext_not_official_text`; `dash_name_is_not_official` (E26); `official_packs_tag_keeps_api_major` (E25: `packs-v2.0.0` ignored, `packs-v1.4.0` chosen); `third_party_tags_unchanged` |
| `types/check/*` | `stage_lists_every_file_and_project_text`; `native_note_for_the_tool`; `duplicate_pack_names`; `conformance_name_reserved`; `walk_refusal_is_staging_refusal`; `has_tests_by_name`; `summary_is_last_nonempty_line`; `second_plan_write_fails`; `second_plan_delete_fails`; `skips_do_not_fail` |
| `app/update.rs` | `legacy_project_upgrades_with_no_newer_tag`; `legacy_and_new_tags_print_api_first`; `current_project_unchanged_behaviour` |
| `app/plan`, `app/apply`, `app/doctor.rs` | `legacy_project_refused` in each (E1) |
| `app/ext/add.rs` | with `FakeWorkspace`/`FakeGit`: every §5.4 row reachable through the flow; `no_project_file` (E14); `changed_during_fetch` (E15); the happy paths for URL, path and official name |
| `app/ext/check.rs` | with `FakeBinary`/`FakeScratch`/`FakeAdapter`: `airsl_missing_stops_without_verdict`; `tests_fail_run_continues_and_fails`; `no_tests_is_none`; `plan_refusal_stops_with_failed`; `not_repeatable_fails`; `skips_pass` (E23) |
| `fs/scratch.rs` | contract test, under `$TMPDIR`: `open` reads a directory like `FakeFileTree`; `create` writes the files and the directory is gone after drop |
| `process/binary.rs` | contract test: `test()` on a passing and a failing directory matches F5 |
| CLI integration (`crates/agentstacks-cli/tests/`) | `ext add` of a path extension end to end; `ext test extensions/claude` and `ext test extensions/opencode` print `passed` and exit 0 (they need airsl, as the existing CLI tests do); `update` of a 0.x fixture project; clap parse tests for F6's argv list |
| Lua | `cargo make lua` stays green with every pack and extension at `api = 1`, plus the conformance assets |
| Gate | `cargo make dod`, `cargo make claudevs-check`, `cargo make opencode-check`, `cargo deny check`, `cargo make book-build`, and the §8 grep gate |

## 13. Non-goals

- A marketplace, registry or discovery service for packs and extensions.
- Official tools beyond Claude Code and OpenCode.
- A promise for the Rust API of the `agentstacks` crate (D8).
- claudevs and claudevs-cli changes or releases (D14).
- An automatic move of third-party packs or extensions from `api = 0`. Their authors change the
  number (ch03-01).
- `pack add` (D5).
- Picking a latest tag for a git URL in `ext add` (D11).
- Running the real tool in `ext test`. The book's "Run it for real" step stays manual.
- New item kinds, events or vocabulary words in 1.x (D15).
- A version gate in `agentstacks.toml` (R7).
- Native hooks in the conformance pack (§6.4).
