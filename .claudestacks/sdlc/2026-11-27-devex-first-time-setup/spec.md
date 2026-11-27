---
status: approved
created: 2026-11-27
built-on: eb05481d902a
---

# Spec: Devex improvements from first-time setup

This spec delivers the six improvements of GitHub issue #17, in the reporter's priority
order, for agentstacks 1.0 (`api = 1`, additive only). Every behavioural claim below carries
its proof: a probe in `probes/` (Rust, the project's own stack, locked versions) or a
`file:line` opened in this task. Exact refusal and output texts are settled here; no plan or
execute step invents a message.

Proof convention: `probes/results.md` summarises, `probes/probe-output.txt` is the raw run,
`probes/probe-main.rs` is the probe source. Versions proved: clap 4.6.9, serde_json 1.0.154,
semver 1.0.28, agentstacks 1.0.0 (path), rustc/cargo 1.94.1, airsl 0.1.5 on PATH.

## 1. Plan summary (item 1)

`agentstacks plan` shows counts by default; the full per-file list is opt-in.

- CLI: `plan [--summary | --verbose]` in
  [cli.rs](/Users/hiraq/.herdr/worktrees/agentstacks/devex-first-time-setup/crates/agentstacks-cli/src/cli.rs:43).
  `--summary` and `--verbose` conflict; giving both is a clap usage error, exit 2, text
  `error: the argument '--summary' cannot be used with '--verbose'` (probe R1).
  Neither flag = summary. `--summary` alone = summary (explicit form of the default).
- Rendering: a strong type `PlanView::{Summary, Full}` (no `bool` parameter; guideline-rust
  strong-types) in `crates/agentstacks-cli/src/show/plan.rs`, beside the existing
  `plan_text` (which becomes the `Full` rendering, unchanged byte for byte).
- Summary layout, exactly:

```text
agentstacks plan

  claude  5 to write, 0 to delete, 2 unchanged
  core    40 to write, 0 to delete, 30 unchanged
  shared  1 to write, 0 to delete, 0 unchanged
  skip     core/hook/2: <reason>

  airsl    0.1.5 (new pin)
  note     <note>

46 to write, 0 to delete, 32 unchanged, 1 skipped
```

- Rows: one per owner with at least one step. Format per row, byte-exact:
  `  {owner:<width}  {w} to write, {d} to delete, {u} unchanged` where `width` is the
  longest owner name in the plan (byte length; `shared` counts), counts are unpadded
  decimal, single spaces inside the count text. `shared` is the name for owner `None`.
  Owners sort by Rust `str` byte order, `shared` last regardless of sort. Counts:
  write = Create+Update+Restore, delete = Delete only, unchanged = Unchanged. Forget is
  not counted in any per-owner row; it appears only in the final line's existing
  `, N forgotten` suffix, so per-owner deletes sum to the final delete count.
- Sections mirror `Full`: section 1 = owner rows followed by skip rows, section 2 =
  airsl row followed by notes, section 3 = the final count line (the existing
  `summary()` text, unchanged). A blank line precedes each non-empty section; the header
  `agentstacks plan` is first. A shared-file `Update` diff is shown only in `Full`.
- Empty plan (no steps, no skips, no airsl, no notes): header, blank line, final count
  line — the same shape `Full` gives today for that case.
- Scope: the view applies wherever a plan is shown — `plan`, and the confirm sequence of
  `apply`, `init`, `update`, `ext add` (default summary; `--verbose` exists only on
  `plan`, the other commands always show summary). `ext test` keeps `Full` (its report is
  a diagnostic transcript).
- `written_and_deleted` and `nothing_to_change` are unchanged.

Checked example (counted from the existing fixture in design): the
`spec_example_rows_render_exactly` fixture in `show/plan.rs` (1 create by `core`,
1 update by `claude`, 1 delete by `team-lint`, 1 shared update, 12 unchanged by `core`,
1 skip, airsl pinned) renders in Summary with owner rows in byte order, `shared` last:
`claude 1 to write, 0 to delete, 0 unchanged`, `core 1 to write, 0 to delete, 12 unchanged`,
`team-lint 0 to write, 1 to delete, 0 unchanged`, `shared 1 to write, 0 to delete, 0 unchanged`, then the skip and
airsl rows, then the fixture's existing final line `3 to write, 1 to delete, 12 unchanged,
1 skipped`. Execute proves this byte-for-byte with a golden test on that fixture.

## 2. Cache home override (item 2)

- Rule (`types/resolve/cache.rs`): one root, resolved once. When `AIRSSTACK_HOME` is set
  and non-empty it is the root (hooks expand `${AIRSSTACK_HOME:-$HOME/.airsstack}`, probed
  in shell, so the variable already names the `.airsstack` directory); otherwise the root
  is `<HOME>/.airsstack`. The cache is `<root>/cache/git/<sha256(url)>/<rev>.git`;
  `url_dir` takes the root. Probe R2 confirmed today's `url_dir` appends
  `.airsstack/cache/git/<sha256>` to a raw home, so the `HOME` branch composes the root
  as `<HOME>/.airsstack` before calling it.
- Empty `AIRSSTACK_HOME` is treated as unset (matches `${AIRSSTACK_HOME:-...}`; shell
  probe). Empty `HOME` likewise (today's `cache_home` behaviour, probe R2).
- Refusal: neither set/non-empty gives the existing unlocated error, reworded in
  `types/resolve/error.rs` (`HomeUnset`) to
  `AIRSSTACK_HOME and HOME are not set, so the cache (~/.airsstack/cache) has no home`,
  located by callers at `agentstacks.toml` as today. This is a message change, named here.
- Unwritable root (the reported sandbox case: HOME set, `mkdir` refused): every
  `FetchError::CacheIo` refusal gains the remedy in its displayed text, exactly
  `<path>: <io error> (set AIRSSTACK_HOME to a writable directory to move the cache)`.
  The suffix is part of the `CacheIo` display in `types/resolve/error.rs`, so CLI, plan
  and hook-adjacent callers all show it; other `FetchError` variants are unchanged.
- Wiring: `process/git/cli.rs::from_env` reads `var_os("AIRSSTACK_HOME")` and
  `var_os("HOME")` and passes both to the rule; `fs/cache.rs` keeps taking one resolved
  root. `types` reads no environment (layering unchanged).
- Docs: `agentstacks --help` gains no text (clap help is per-command; the variable is
  documented in book ch05 cache section and ch01 installation), and the sandbox remedy is
  discoverable from the refusal plus that section. Hooks already honour the variable;
  no Lua change.
- Edge: a relative `AIRSSTACK_HOME` is accepted as given (parity with today's relative
  `HOME` handling; `cache_home` passes any non-empty value through, probe R2). No new
  refusal for it.

## 3. Doctor runtime checks (item 3)

- `app/doctor` always asks the `Binary` port for `airsl --version`, on every path —
  missing lock, unreadable lock (`LockIo`), refused lock (any number of `Finding::Lock`),
  and healthy lock, pinned or not. Findings order: lock findings (all of them), then
  file findings, then the airsl finding last.
- Outcome matrix for `airsl_finding(pin: Option<&PackVersion>, answer)` in
  `types/plan/drift.rs`. New variant name: `Finding::AirslMissing` (no pin).
  - NotFound, with pin: `airsl: not found on PATH (lock pins 0.1.4) (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)` (pin text is the lock's).
  - NotFound, no pin: `airsl: not found on PATH (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)`.
  - Printed and parseable, with pin, equal: no finding. With pin, different:
    `airsl: 0.1.5 installed, lock pins 0.1.4` (no install suffix — reinstalling installs
    the newest, it cannot restore a pin). No pin: no finding, any parseable version.
  - Printed/Failed unparseable, either pin state: today's
    `airsl: --version printed "<text>", expected "airsl <version>"`, unchanged.
  - SpawnFailed, either pin state: today's `agentstacks.toml: airsl: <io>`, unchanged.
  The install URL is the one the book already publishes for installing agentstacks and
  airsl (`book/src/ch01-01-installation.md`, Installation section).
- Version comparison stays exact-pin equality when a pin exists. Pack `requires.airsl`
  ranges (semver, probe R4) remain a plan-time check; doctor does not re-resolve packs
  (offline contract kept: no network, no git, no extension — `doctor` signature stays
  `Workspace + Binary`). Lock-parse refusals are existing behaviour, restated in §9 as
  out of scope, not new coverage.
- Exit code unchanged: any finding exits 1 (`doctor` command), healthy exits 0.

## 4. Generated-file stamps (item 4)

- Who: each official extension stamps the files it owns, in its Lua adapter/lib — the
  core learns no file shapes (tool-agnostic invariant). New file per extension:
  `lib/stamp.lua` with `stamp(path, content) -> content`, plus `lib/stamp_test.lua`, the
  pattern every existing lib file follows. The adapter calls it on every Write
  operation's content before proposing; Copy operations never pass through it.
- Mechanism: the helper decides from the Write target's suffix only. Shared files are
  exactly the extension's `reads` targets (`.claude/settings.json`,
  `.muse/hooks.json`), both `.json`, so the suffix rule already excludes them; no
  owner signal is needed at proposal time.
- Stamp set (suffix → line), inserted as specified below:
  - `.md` → `<!-- generated by agentstacks — change agentstacks.toml and re-run apply, do not edit -->`
  - `.lua` → `-- generated by agentstacks — change agentstacks.toml and re-run apply, do not edit`
  No `.toml` row: no extension proposes a `.toml` Write today (checked across the three
  adapters in design), so there is nothing to stamp.
- Placement: for `.lua`, the stamp is the first line. For `.md`, when the content
  starts with a frontmatter block (`---\n` … `\n---\n`), the stamp goes immediately
  after the closing `---` line — prepending before `---` breaks frontmatter parsing
  (probe R6: prepended content no longer starts with `---`); otherwise it is the first
  line. Suffix match is case-sensitive (`.MD` is not stamped). Empty content is stamped
  (result is the stamp line plus a trailing newline).
- Excluded, by name: `.json` (comments break parsing — probe R3 with serde_json 1.0.154:
  `#`, `//` and `<!--` prefixes all fail at line 1 column 1), Copy operations (verbatim
  pack files), and any suffix not in the set. The lock keeps its existing stamp
  (`types/lock/write.rs` `# Generated by agentstacks. Do not edit.`), unchanged.
- Idempotence: the helper checks the proposed content at the insertion point (first
  line, or the line after frontmatter) for the exact stamp line and does not add a
  second. Because the adapter stamps pack-derived content deterministically, the
  stamped bytes are what the plan hashes and the lock records; a second plan is
  `nothing to change`. CRLF content: the check compares the line without its trailing
  `\r`; a `\r\n` stamp already present counts as stamped.
- `ext test` conformance gains a case: a stamped `.md` write is proposed with the stamp
  after its frontmatter, a `.json` write without.

## 5. Activation hint (item 5)

- After a successful apply that wrote or deleted at least one file, the confirm sequence
  (`command/confirm.rs`, shared by `apply`/`init`/`update`/`ext add`) prints on stdout,
  after the `applied:` line, exactly:
  `note: start a new session — hooks, skills and agents load at session start`
- Not printed on `nothing to change`, on decline, on no-terminal, or on failure. `ext
  test` does not print it (no user session). The em dash and wording are fixed here.

## 6. `init --dry-run` (item 6)

- CLI: `init --dry-run` conflicts with `--yes` (clap, exit 2; probed verbatim in R7:
  `error: the argument '--dry-run' cannot be used with '--yes'`, `probes/probe2-output.txt`).
- Behaviour: resolve the arguments exactly as `init` does (official names need git
  `ls-remote`, which needs no cache/HOME), render `agentstacks.toml` in memory, create a
  scratch project (the `Scratch` port pattern `ext test` already uses) holding that file,
  plan against it, and print on stdout, byte layout: the line
  `agentstacks.toml (dry run — not written)`, the rendered TOML (which ends with one
  newline), one blank line, then the plan in the default Summary view, the command's
  usual trailing newline after the last plan line. Exit 0 on a successful plan, 1 with
  the plan's refusals on stderr.
- "Writes nothing" is scoped: nothing in the project directory, no lock anywhere. The
  plan against the scratch project fetches official sources through the git cache, so a
  dry run can write the git cache under the §2 root, need network, and refuse on the §2
  cache-home rules — exactly as `plan` does today.
- The scratch project is removed by dropping the `Scratch` value (its port contract:
  the project lasts as long as the returned value); the command holds it until after
  printing, on the refusal path as well, so cleanup needs no explicit step.
- Refusals of argument resolution are identical to `init`'s and write nothing. The
  existing-file check belongs to the write step only: `init` runs it first, before
  resolving (today's ordering, unchanged); dry-run skips it, so an existing
  `agentstacks.toml` does not refuse a dry run.
- The split that makes this possible: `app/init.rs` separates resolve+render (returning
  the text) from the write; `init` keeps writing that text, dry-run keeps it in memory.

## 7. Architecture

Approved by the author in the design dialogue of 2026-11-27, on its own, before this
spec was written (the approval is recorded at this spec's approval gate). Module tree (only files
that change; `~` modified, `+` new concept inside an existing file):

```text
crates/agentstacks/src/
├─ types/resolve/cache.rs   ~ root rule: AIRSSTACK_HOME (non-empty) else HOME/.airsstack;
│                             url_dir(root, url) = <root>/cache/git/<hex> (drops its join)
├─ types/resolve/error.rs   ~ HomeUnset reworded; CacheIo display gains the remedy suffix
├─ types/plan/finding.rs    ~ NotFound gains the install suffix; new AirslMissing variant
├─ types/plan/drift.rs      ~ airsl_finding takes Option<&pin> (§3 matrix)
├─ app/doctor.rs            ~ always asks Binary; airsl finding on every lock path
├─ app/init.rs              ~ resolve+render split from write_project
├─ fs/cache.rs              ~ Cache holds the resolved root; url_dir composes from it
└─ process/git/cli.rs       ~ from_env reads AIRSSTACK_HOME and HOME
crates/agentstacks-cli/src/
├─ cli.rs                   ~ Plan { --summary, --verbose }, Init { --dry-run }
├─ show/plan.rs             ~ PlanView::{Summary, Full}; per-owner count rendering
├─ show/doctor.rs           (unchanged — Finding Display carries the fix text)
├─ command/mod.rs           ~ Ports gains the Scratch port for the init command
├─ command/plan.rs          ~ passes the view from the flags
├─ command/confirm.rs       ~ activation hint after Applied with changes
├─ command/init.rs          ~ dry-run: scratch project, plan, print, write nothing
└─ main.rs                  ~ passes the Scratch port into the init dispatch
extensions/{claude,musecode,opencode}/
├─ lib/stamp.lua            + stamp(path, content) for owned .md/.lua writes
├─ lib/stamp_test.lua       + its airsl tests
└─ adapter.lua              ~ calls stamp on every Write before proposing
book/src/                   ~ ch01-01, ch01-02, ch04-00, ch05-00 for the new texts
```

Why each unit exists and the rules applied (guideline-rust pack, read in design):

- **Modularity** — one concept per file, its error inside: the view type lives in the
  file that renders plans; the home rule in the file that owns cache paths; no new file
  was needed because each change lands in its concept's existing home.
- **mod-rs-export-only** — no `mod.rs`/`lib.rs` gains logic; new items are re-exported
  through the existing export-only modules.
- **Static dispatch** — flows stay generic over the existing ports (`Workspace`, `Git`,
  `Binary`, `Adapter`, `Scratch`); no `Box<dyn>` is introduced.
- **Strong types** — `PlanView` is a two-variant enum, never a `bool` parameter; the
  resolved cache root is one `PathBuf` the rule produced, not two raw env strings
  threaded through callers.
- **Unit-test mandate** — every touched logic file keeps/gains its colocated
  `#[cfg(test)] mod tests`; CLI behaviour is also covered by the command tests on fakes.
- **Doc discipline** — source comments state the rule (e.g. why JSON is excluded), never
  this chain, the issue, or the plan.
- Dependency direction is unchanged: `types → port → app → adapters`; the CLI remains
  presentation over the library. `tests/isolation.rs` must stay green.

How logic is tested: rules in `types` by unit tests with plain values (home precedence
matrix, finding texts); flows in `app` by fakes (doctor asks Binary with no lock; init
render-without-write); rendering in `show` by golden text tests (the §1 fixture in both
views); commands by fake-port tests (hint printed once, dry-run writes nothing, flag
conflicts exit 2 — clap probe R1); Lua stamp helper by `airsl test` in each extension;
end-to-end by the existing CLI integration tests. The gate is `cargo make dod`.

## 8. Error and edge summary

| Case | Result |
|---|---|
| `plan --summary --verbose` | clap exit 2, conflict text (§1) |
| `plan` with no steps | header + final count line only |
| owner with only Unchanged steps | row shown with 0 to write |
| `AIRSSTACK_HOME=""`, `HOME=/home/u` | root `/home/u/.airsstack` |
| `AIRSSTACK_HOME=/x`, `HOME` unset | root `/x`; no refusal |
| both unset/empty | refusal reworded in §2, at `agentstacks.toml` |
| doctor, no lock, no airsl | lock-missing finding, then `AirslMissing` finding; exit 1 |
| doctor, refused lock, no airsl | every lock refusal, then the airsl finding; exit 1 |
| doctor, pin 0.1.4, airsl 0.1.5 | mismatch finding, no install suffix; exit 1 |
| HOME set but unwritable, no override | CacheIo refusal with the AIRSSTACK_HOME remedy suffix |
| stamp target `.json` / Copy / shared | no stamp (§4 exclusions) |
| already-stamped first line | left single-stamped |
| apply writes 0 files | no activation hint |
| `init --dry-run --yes` | clap exit 2, conflict text (§6 pattern) |
| `init --dry-run`, project file exists | proceeds; header marks not written |
| dry-run plan refused | refusals on stderr, exit 1, nothing written |

## 9. Non-goals

- Fetching extension/pack sources in `doctor` (breaks its offline contract).
- Stamping JSON, copied or shared files, or stamping in the core.
- A `--verbose` flag on `apply`/`init`/`update`/`ext add`, or `Full` view in `ext test`
  changing at all.
- Changing lock/project/pack/extension formats, or the `api = 1` surface beyond the
  additive CLI flags above.
- Cleaning the pre-existing sandbox failure `fs::disk::tests::a_unix_socket_is_other`
  (environment `PermissionDenied` on a unix socket, reproduced in design on untouched
  code; not this chain's defect).

## Claim ledger (proof index)

- clap conflict/exit-2 and `--dry-run` parsing — probe R1 (`probes/probe-output.txt`);
  the `--dry-run`/`--yes` conflict verbatim and the frontmatter placement check —
  probes R7/R6 (`probes/probe2-output.txt`, source `probes/probe2-main.rs`).
- `cache_home` empty/unset refusal and `url_dir` layout — probe R2, same output.
- JSON comment intolerance — probe R3 (serde_json 1.0.154), same output.
- semver range matching — probe R4, same output. Shell `${AIRSSTACK_HOME:-...}`
  empty-as-unset — raw run in `probes/shell-output.txt`.
- Current plan/doctor/init behaviour and texts — `file:line` citations in §§1–6, the
  green `cargo test -p agentstacks-cli` run (142 tests) in `probes/cli-test-output.txt`,
  and the core lib run (1858 passed, 1 environment failure named in §9) summarised in
  `probes/results.md`. Probe manifests retained as `probes/probe-Cargo.toml`.
- New behaviours (Summary layout, stamp placement, hint, dry-run output) are normative
  examples, counted/derived from the cited fixtures — not claims about current code;
  execute proves each byte-for-byte with the golden tests §7 names.
