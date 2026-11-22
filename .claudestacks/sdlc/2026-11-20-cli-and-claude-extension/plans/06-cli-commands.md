---
status: done
created: 2026-11-22
depends-on: [02, 05, 10]
built-on: cefc42fbbc4c
---

# CLI commands Implementation Plan

**Goal:** Wire the five `agentstacks` subcommands to the library flows.

**Architecture:**
- `crates/agentstacks-cli` is the composition root:
  - `cli.rs` is the clap grammar;
  - `wire.rs` builds the real adapters for the project directory;
  - `command/<name>.rs` runs one subcommand;
  - `command/confirm.rs` is the shared show → ask → apply sequence;
  - `main.rs` is parse → run → `ExitCode`.
- Each `command/*` function is generic over the ports it uses (`W: Workspace`, `G: Git`,
  `B: Binary`, `A: Adapter`). `main` passes the concrete adapters from `wire`, and tests pass the
  library fakes. The CLI gets the fakes by enabling the library's `testing` feature as a
  dev-dependency (plan 10, spec D20). Tests also inject a terminal flag, a line reader and output
  buffers. Static dispatch, no `dyn`.
- `command/mod.rs` is export-only. The CLI holds no rule.

**Tech Stack:** Rust 2024, clap 4 (derive), `agentstacks` library, plans 02 and 05.

**Content authority:** spec D1-D9, D20, D21, D22, §3, §4.5, §5.1, §5.3, §5.4, §6, §7, §10 (CLI rows), §11, §13
(docs row: `docs/cli.md`, `docs/README.md`).

---

## Files that change

```
crates/agentstacks-cli/src/cli.rs              — modify: the full grammar
crates/agentstacks-cli/src/main.rs             — modify: parse → wire → run → ExitCode
crates/agentstacks-cli/src/wire.rs             — create: real adapters for -C <dir> or the current directory
crates/agentstacks-cli/src/command/mod.rs      — create: export-only
crates/agentstacks-cli/src/command/confirm.rs  — create: shared show → ask → apply sequence
crates/agentstacks-cli/src/command/plan.rs     — create: plan
crates/agentstacks-cli/src/command/apply.rs    — create: apply
crates/agentstacks-cli/src/command/doctor.rs   — create: doctor
crates/agentstacks-cli/src/command/update.rs   — create: update
crates/agentstacks-cli/src/command/init.rs     — create: init
crates/agentstacks-cli/src/show/*.rs, prompt.rs, exit.rs — modify: remove the temporary dead_code expectations
crates/agentstacks-cli/Cargo.toml              — modify: dev-dependencies agentstacks (features = ["testing"]) and tempfile
docs/cli.md                                    — create: commands, output, prompt, exit codes
docs/README.md                                 — modify: table row for cli.md
```

## Order of work

### Task 1 — Command-line grammar

**Files:**
- Modify `crates/agentstacks-cli/src/cli.rs`:
  - the global `-C <DIR>`, valid before or after the subcommand;
  - the subcommands `init --tool <TOOL>... --pack <PACK>... [--yes]`, `plan`, `apply [--yes]`,
    `doctor`, `update [--yes]`;
  - `value_name` set to `TOOL`, `PACK` and `DIR`;
  - `--tool` and `--pack` required at least once;
  - replace the test `subcommand_is_rejected` (`:28-30`).

**Change:** clap produces the spec §3 table: usage errors exit 2 with clap's text, and `--help` /
`--version` exit 0.

**Proof:**
- `command_definition_is_valid` (kept).
- `init_without_pack_is_a_usage_error`: `try_parse_from(["agentstacks","init","--tool","claude"])`
  fails, and its rendered text contains `--pack <PACK>`.
- `init_collects_repeated_flags`: two `--pack` values both parse.
- `dash_c_works_before_and_after_the_subcommand`.
- `no_subcommand_is_an_error`.
- `unknown_subcommand_is_an_error`.

**Done when:** `cargo test -p agentstacks-cli cli` passes and `cargo make dod` is green. Commit
`feat(agentstacks-cli): command-line grammar for the five commands`.

### Task 2 — The confirm sequence

**Files:**
- Create `crates/agentstacks-cli/src/command/mod.rs` — module docs plus `mod` / `pub(crate) use`.
- Create `crates/agentstacks-cli/src/command/confirm.rs`.
- Modify `crates/agentstacks-cli/Cargo.toml` — `[dev-dependencies] agentstacks = { workspace = true,
  features = ["testing"] }` (spec §13).

**Change:**
- `Confirmed` enum `{ NothingToChange, Applied { written, deleted }, Declined, NoTerminal,
  Failed(Diagnostics) }`. A lock read error is `Failed` with one diagnostic.
- `confirm(plan, workspace, yes: bool, is_terminal, input, out, err) -> Confirmed`:
  1. Print `show::plan_text` to `out`.
  2. Read `workspace.lock_text()`. An `Err` prints `agentstacks.lock: <io::Error Display>` to `err`
     and returns `Failed`: no question, nothing applied (spec D21, §5.3). Otherwise, if
     `show::nothing_to_change(plan, <the text as Option<&str>>)`, print `nothing to change` to `out`
     and return `NothingToChange`: no question, nothing written.
  3. If `yes`, apply.
  4. Otherwise call `prompt::ask`:
     - `NoTerminal` prints `stdin is not a terminal; pass --yes to apply` to `err`;
     - `No` prints `nothing applied` to `err`;
     - `Yes` applies.
- **Apply:** `app::apply::apply(plan, workspace)` (`src/app/apply/flow.rs:35`) on the same `Plan`.
  - `Ok` prints `applied: <w> written, <d> deleted` to `out`, counted as in the plan summary.
  - `Err` prints each diagnostic from `into_sorted` to `err`.

**Proof:** on `FakeWorkspace` with `Plan::new` values.
- `nothing_to_change_asks_nothing`: the input reader is never read, and nothing is written.
- `yes_flag_applies_without_asking`.
- `no_terminal_without_yes_refuses`: the exact stderr line, and the result is `NoTerminal`.
- `declined_prints_nothing_applied`.
- `accepted_applies_and_prints_counts`: `applied: 1 written, 0 deleted`.
- `apply_failure_prints_diagnostics`: a staged race on the fake gives `<path>: changed after plan;
  run plan again`.
- `lock_only_change_still_asks`: no listed step, but the lock text differs, so it asks.
- `lock_read_error_refuses_without_asking`: `fail(WorkspaceCall::LockText)` on the fake gives
  `agentstacks.lock: <Display>` on stderr and `Failed`, and the reader is never read.

**Done when:** `cargo test -p agentstacks-cli command::confirm` passes and `cargo make dod` is green.
Commit `feat(agentstacks-cli): shared show, ask and apply sequence`.

### Task 3 — `plan`, `apply`, `doctor`

**Files:**
- Create `crates/agentstacks-cli/src/command/plan.rs`, `apply.rs`, `doctor.rs`.
- Modify `crates/agentstacks-cli/src/show/*.rs`, `prompt.rs`, `exit.rs` — remove every temporary
  `#[expect(dead_code, …)]` that plan 05 added.

**Change:**
- `plan`:
  - calls `app::plan::plan` (`src/app/plan/flow.rs:59`), prints `plan_text`, returns `Success`;
  - a refusal prints the sorted diagnostics to `err` and returns `Refused`.
- `apply`:
  - the same `plan` call, then `confirm`;
  - `Success` for `NothingToChange`, `Applied` and `Declined`;
  - `Refused` for `NoTerminal` and `Failed`.
- `doctor`:
  - calls `app::doctor::doctor` (`src/app/doctor.rs:39`) and prints `doctor_text`;
  - `Success` only with no findings.

**Proof:** on the library fakes (`FakeWorkspace`, `FakeGit`, `FakeBinary`, `FakeAdapter` from
`src/port/`).
- `plan_prints_and_writes_nothing`.
- `plan_refusal_exits_refused`: a fake project without `agentstacks.toml` gives
  `agentstacks.toml: <Display>`.
- `apply_declined_is_success`.
- `apply_no_terminal_is_refused`.
- `doctor_healthy_is_success`.
- `doctor_with_a_finding_is_refused`.

**Done when:** `cargo test -p agentstacks-cli command` passes and `cargo make dod` is green (no
`dead_code` expectation left: `grep -rn "dead_code" crates/agentstacks-cli/src` prints nothing).
Commit `feat(agentstacks-cli): plan, apply and doctor commands`.

### Task 4 — `update` and `init`

**Files:**
- Create `crates/agentstacks-cli/src/command/update.rs`, `init.rs`.

**Change:**
- `update`:
  1. `app::update::update` (`src/app/update.rs:43`); a refusal prints the diagnostics and returns
     `Refused`.
  2. Print `update_text` to `out`.
  3. If `!has_newer`, print `nothing to update` and return `Success`, with nothing written.
  4. Call `app::update::retag`. A refusal (re-read failure, text that no longer parses, a tag
     missing or changed, a failed write) prints the diagnostics and returns `Refused`, with the file
     unchanged and no plan (spec D22). Otherwise print `agentstacks.toml updated` to `out`.
  5. Plan and confirm as `apply`, then the spec §7 follow-up on `err`:
     - a refused plan, or `Failed` (failed apply or lock read error):
       `agentstacks.toml updated; fix the above and run apply`, exit 1;
     - declined: `agentstacks.toml updated; nothing applied (run apply)`, exit 0;
     - no terminal: the confirm line, then `agentstacks.toml updated; nothing applied (run apply)`,
       exit 1;
     - applied or nothing to change: no follow-up, exit 0.
- `init`:
  - calls `app::init::init`; a refusal prints the diagnostics and returns `Refused`;
  - then plan and confirm, with the follow-ups of spec §4.5 on `err`:
    - a refused plan: the refusals, then `agentstacks.toml written; fix the above and run apply`
      (exit 1);
    - declined: `agentstacks.toml written; nothing applied (run apply)` (exit 0);
    - no terminal: the confirm line, then `agentstacks.toml written; nothing applied (run apply)`
      (exit 1);
    - applied: as confirm (exit 0);
    - a failed apply or a lock read error (`Failed`): the diagnostics, then
      `agentstacks.toml written; fix the above and run apply` (exit 1).

**Proof:**
- `update_with_only_path_sources_prints_nothing_to_update`.
- `update_writes_tags_then_applies_with_yes`.
- `update_declined_keeps_the_toml_and_succeeds`: the stderr follow-up is exact.
- `update_refused_plan_adds_the_fix_line`.
- `update_retag_refusal_is_refused_with_no_plan`: a tag changed at re-read gives the D26 message, and
  the adapter is never called.
- `update_failed_apply_adds_the_fix_line`.
- `update_no_terminal_prints_both_lines_and_is_refused`.
- `init_lock_read_error_adds_the_fix_line`.
- `init_existing_file_is_refused_with_nothing_written`.
- `init_refused_plan_keeps_the_toml`: the stderr ends with the exact `written; fix the above` line,
  and the result is `Refused`.
- `init_declined_is_success_with_the_follow_up`.
- `init_no_terminal_prints_both_lines_and_is_refused`.
- `init_with_yes_applies`.

**Done when:** `cargo test -p agentstacks-cli command` passes and `cargo make dod` is green. Commit
`feat(agentstacks-cli): init and update commands`.

### Task 5 — Wire the binary

**Files:**
- Create `crates/agentstacks-cli/src/wire.rs`:
  - opens `StdWorkspace::open(dir)` (`src/fs/workspace/project.rs:66`), where `dir` is `-C` or the
    current directory, with no parent search;
  - builds `GitCli::from_env()`, `SystemBinary::from_env()` and `AirslAdapter`;
  - an open failure prints `<dir>: <io::Error Display>` and returns `Refused`.
- Modify `crates/agentstacks-cli/src/main.rs`:
  - `Cli::parse()` (clap exits 2 itself), then `wire`, then dispatch;
  - real stdin, stdout and stderr, with `io::stdin().is_terminal()`;
  - returns `ExitCode` from `Outcome`;
  - rewrite the module docs, which say it has no branching: it now dispatches, and its logic is
    tested in `command/*`.
- Modify `crates/agentstacks-cli/Cargo.toml` — `tempfile` dev-dependency.

**Change:** The binary runs the commands.

**Proof:**
- `missing_dash_c_directory_is_refused`: `wire` on a nonexistent path returns `Refused`, and its
  message starts with that path.
- `tests/version.rs` (existing) still passes.

**Done when:** `cargo test -p agentstacks-cli` passes and `cargo make dod` is green. Commit `feat(agentstacks-cli): wire the commands to the real adapters`.

### Task 6 — CLI docs

**Files:**
- Create `docs/cli.md`:
  - the five commands and `-C`;
  - the plan text rows (spec §5.2);
  - the prompt and `--yes` (§5.3);
  - the stream and exit-code tables (§3);
  - the `init` follow-ups (§4.5);
  - the `update` steps (§7).
- Modify `docs/README.md` — one row for `cli.md` in its table.

**Change:** Contributor docs for the CLI.

**Proof:** None; this is a docs-only task. Every message quoted in `docs/cli.md` is copied from the
spec section it cites.

**Done when:** `cargo make dod` is green. Commit `docs(repo): document the CLI commands`.

---

## Risks

- **`init` duplicates the confirm follow-ups.** `init` and `update` add follow-up lines around
  `confirm`. Ordering mistakes are caught by the exact stderr assertions in Task 4.
- **Fake coverage.** The real end-to-end run is in plan 09. Here, every branch is reached through
  fakes, as the spec §12 requires.
- **Not done:** colour and `--json` output (spec §15).

## Proof

- `cargo make dod` green.
- `cargo run -p agentstacks-cli -- --help` lists `init`, `plan`, `apply`, `doctor` and `update`.

## Review findings

- 🟡 `command/followup.rs` had logic and no tests (blocking). Fixed: 10 tests (Display, `kept`, `fix`, every `Confirmed`→`Outcome` arm). `cargo test -p agentstacks-cli` → 83 unit + 1 integration passed.
- 🟡 `command/apply.rs:51` `Confirmed::Failed => Outcome::Refused` unguarded. Fixed: `apply_failed_apply_is_refused`, `apply_lock_read_error_is_refused`; flipping the arm to `Success` failed both. `cargo make dod-crate agentstacks-cli` → green.
- 🔵 `docs/cli.md` said a decline prints `nothing applied` for every command. Fixed: apply only; init/update print their follow-up line.
- 🔵 open: `Confirmed`/`confirm` are `pub(crate)` where siblings use `pub(super)`; `yes: bool` threaded through (plan-prescribed); `init.rs` refused-plan and lock-read tests do not assert stdout; `ports.rs` lacks an exemption note; `main.rs` dispatch has no inline tests (plan-sanctioned); `version = "0.0.0"` must move with the crate version; `arg_required_else_help = true` added.
- ❓ open: `command/console.rs` drops stdout write failures (`plan >/dev/full` exits 0); the module doc says it is deliberate.

## Deviations

- 2026-11-22 Tasks 2–5: commands take `&Ports<W,G,B,A>` and `&mut Console<I,O,E>` instead of many separate arguments; new files `command/{console,ports,followup,testing}.rs`. Dispatch stays static.
- 2026-11-22 Task 2: `confirm` takes a `declined` line. A decline prints only that line: `nothing applied` for apply, the follow-up line for init/update (e.g. `Apply? [y/N] agentstacks.toml written; nothing applied (run apply)`). No terminal prints the refusal, then the follow-up.
- 2026-11-22 Task 2: `Confirmed::Failed` is a unit variant (`Diagnostics` is not `Clone`; confirm already printed them).
- 2026-11-22 Task 4: `update` prints the update list only when it has entries.
- 2026-11-22 Task 3: `doctor::run(workspace, binary, console)` takes only the ports it uses.
- 2026-11-22 Task 2: `serde_json` dev-dependency added (fake adapter scripts build `ValuesResult` input).
- 2026-11-22 Task 6: written by a separate coder in parallel with Tasks 1–5.
- 2026-11-22 (process): coders read their task sections straight from this plan file instead of a task-briefer brief; the ledger came from `grep '^### Task'`. Earlier batches were staged in the git index (not committed) so each review saw only its own batch.
