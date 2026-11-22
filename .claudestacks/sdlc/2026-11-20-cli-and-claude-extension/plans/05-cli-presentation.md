---
status: done
created: 2026-11-22
depends-on: [01]
built-on: cefc42fbbc4c
disproves: cefc42fbbc4c
---

# CLI presentation Implementation Plan

**Goal:** Give the CLI its text renderers, prompt reader and exit mapping, each testable without a terminal.

**Architecture:**
- These are presentation-only units in `crates/agentstacks-cli`:
  - `show/*` turns library values into text;
  - `prompt` turns a terminal flag and a line reader into an `Answer`;
  - `exit` maps an `Outcome` to an `ExitCode`.
- None calls a flow or touches disk. Each is unit-tested on `Plan::new` values
  (`src/types/plan/built.rs:35`), `Finding` and `UpdateEntry` values, and injected readers.
- `show/mod.rs` is export-only.

**Tech Stack:** Rust 2024, similar 3 (new workspace dependency), the `agentstacks` library (new
path dependency of the CLI).

**Content authority:** spec §2 (CLI layout), §3 (streams, exit codes), §5.2, §5.3 (prompt), §5.4
(applied line), §6, §7 steps 2-3, §11 (prompt row), §13 (similar row).

---

## Files that change

```
Cargo.toml                                — modify: similar = "3"; agentstacks = { path = "crates/agentstacks" } in [workspace.dependencies]
crates/agentstacks-cli/Cargo.toml         — modify: agentstacks, similar
crates/agentstacks-cli/src/main.rs        — modify: declare show, prompt, exit (main body unchanged until plan 06)
crates/agentstacks-cli/src/show/mod.rs    — create: export-only
crates/agentstacks-cli/src/show/diff.rs   — create: unified diff text of an updated file
crates/agentstacks-cli/src/show/plan.rs   — create: the plan text
crates/agentstacks-cli/src/show/doctor.rs — create: the doctor report text
crates/agentstacks-cli/src/show/update.rs — create: the update list text
crates/agentstacks-cli/src/prompt.rs      — create: Answer from a terminal flag and a line reader
crates/agentstacks-cli/src/exit.rs        — create: Outcome → ExitCode
```

## Order of work

### Task 1 — Unified diff text

**Files:**
- Modify `Cargo.toml`, `crates/agentstacks-cli/Cargo.toml` — the dependencies above.
- Modify `crates/agentstacks-cli/src/main.rs` — `mod show;`.
- Create `crates/agentstacks-cli/src/show/mod.rs` — module docs plus `mod diff;` and its `pub(crate) use`. Tasks 2 and 3 each add their own `mod` line.
- Create `crates/agentstacks-cli/src/show/diff.rs`.

**Change:** `diff_lines(path, old: &[u8], new: &[u8]) -> Vec<String>`:
- Both sides are decoded lossily as UTF-8.
- The diff is `similar::TextDiff::from_lines(old, new).unified_diff().context_radius(3).header(path, path)`.
- Each line comes back indented by four spaces.
- Identical texts give no lines.

**Proof:**
- `identical_texts_give_no_lines`.
- `changed_line_gives_headers_and_a_hunk`: the first lines are `    --- <path>` and `    +++ <path>`,
  and a hunk header starting with `    @@ ` follows.
- `invalid_utf8_is_decoded_lossily`: `[0xff]` against `b"a\n"` gives lines without a panic.

**Done when:** `cargo test -p agentstacks-cli show::diff` passes and `cargo make dod` is green.
Commit `feat(agentstacks-cli): unified diff for updated shared files`.

### Task 2 — Plan text

**Files:**
- Create `crates/agentstacks-cli/src/show/plan.rs`.
- Modify `crates/agentstacks-cli/src/show/mod.rs` — `mod plan;` and its re-export.

**Change:** `plan_text(plan: &Plan) -> String`, exactly the spec §5.2 layout:
- The header line is `agentstacks plan`.
- **Step rows:** one per `Create`, `Update`, `Restore`, `Delete` or `Forget` step, in plan order
  (`src/types/plan/action.rs:24`).
  - Each row is two spaces, the action in lower case padded to 8, a space, the path padded to the
    longest listed path, two spaces, then the owner.
  - The owner is `Owner` Display, or `shared` when `Step.owner` is `None`.
- **Diff:** below an `Update` step whose `old` is `Some`, `show::diff` lines are inserted.
- **Skip rows:** `  skip     <subject>: <reason>`. `Skipped` and `SkipSubject` have no Display, so `show/plan` formats
  the subject itself:
  - an item subject is `ItemId` Display (`src/types/op/id.rs:173`);
  - a core root-file subject is `<pack>/<rel>` (`src/types/op/skip.rs:73`).
  The reason is `SkipReason` Display (`skip.rs:65`).
- **airsl row:** from `AirslPin` (`src/types/plan/note.rs:79`):
  - `<v> (pinned)` when the locked and installed versions are equal;
  - `<locked> → <installed>` when they differ;
  - `<installed> (new pin)` when the lock had none.
- **Note rows:** `  note     <Note Display>`.
- **Binary rows:** none.
- **Summary:** `<w> to write, <d> to delete, <u> unchanged, <s> skipped`, plus `, <f> forgotten`
  when `f > 0`.
  - `w` counts Create + Update + Restore;
  - `d` counts Delete;
  - `u` counts Unchanged;
  - `s` counts skips;
  - `f` counts Forget.
- An empty section is left out together with the blank line before it. No colour.

Also `nothing_to_change(plan, current_lock: Option<&str>) -> bool`. It is true when no step is
listed and `write_lock(plan.lock())` (`src/types/lock/write.rs:57`) equals `current_lock`. An absent
lock is never equal.

**Proof:**
- `spec_example_rows_render_exactly`: a `Plan::new` with the spec §5.2 steps (create, update with a
  runtime owner, delete, a shared update with old text), one skip, a pinned airsl and one note. The
  text equals the spec's rows byte for byte, with the diff block in place of `…` checked by prefix.
- `root_file_skip_subject_is_pack_slash_rel`.
- `unchanged_steps_are_counted_not_listed`.
- `path_column_pads_to_the_longest_listed_path`.
- `owner_none_prints_shared`.
- `airsl_changed_prints_an_arrow` and `airsl_new_pin_prints_new_pin`.
- `forget_count_appears_only_when_positive`.
- `empty_sections_are_left_out`: a plan with only unchanged steps prints the header, a blank line
  and the summary.
- `nothing_to_change_is_false_without_a_lock`, `nothing_to_change_is_false_when_lock_text_differs`,
  `nothing_to_change_is_true_when_equal_and_no_listed_step`.

**Done when:** `cargo test -p agentstacks-cli show::plan` passes and `cargo make dod` is green.
Commit `feat(agentstacks-cli): render the plan as text`.

### Task 3 — Doctor and update text

**Files:**
- Create `crates/agentstacks-cli/src/show/doctor.rs`.
- Create `crates/agentstacks-cli/src/show/update.rs`.
- Modify `crates/agentstacks-cli/src/show/mod.rs` — `mod doctor; mod update;` and their re-exports.

**Change:**
- `doctor_text(findings: &[Finding]) -> String` prints one `Finding` Display per line in the given
  order (`src/types/plan/finding.rs:31`), or `healthy` when there are none.
- `update_text(entries: &[UpdateEntry]) -> String`:
  - a newer tag prints `  <key>: <current> → <new>`, where `<key>` is `SourceKey` Display;
  - a note prints `  note     <UpdateNote Display>` (`src/types/resolve/update.rs:77`).
- `has_newer(entries) -> bool`. When false, the command prints `nothing to update`.

**Proof:**
- `no_findings_prints_healthy`.
- `findings_print_in_order`: `changed: .claude/agents/coder.md`, then the airsl finding.
- `newer_tag_row_has_an_arrow`: `  packs.core: packs-v0.1.0 → packs-v0.2.0`.
- `note_row_is_prefixed`: `  note     agentstacks.toml: packs.core: no newer tag than packs-v0.1.0`.
- `has_newer_is_false_for_notes_only`.

**Done when:** `cargo test -p agentstacks-cli show::doctor show::update` passes and `cargo make dod`
is green. Commit `feat(agentstacks-cli): render doctor and update results`.

### Task 4 — Prompt and exit codes

**Files:**
- Modify `crates/agentstacks-cli/src/main.rs` — `mod prompt; mod exit;`.
- Create `crates/agentstacks-cli/src/prompt.rs`.
- Create `crates/agentstacks-cli/src/exit.rs`.

**Change:**
- `Answer` enum `{ Yes, No, NoTerminal }`.
- `ask(is_terminal: bool, input: impl BufRead, err: impl Write) -> Answer`:
  - when not a terminal, it returns `NoTerminal` without writing or reading;
  - otherwise it writes `Apply? [y/N] ` to `err` and reads one line;
  - a trimmed, case-insensitive `y` or `yes` is `Yes`;
  - anything else, an empty line, end of input (`Ok(0)`) or a read error is `No`.
- `Outcome` enum `{ Success, Refused }`, and `Outcome::code(self) -> ExitCode`, giving 0 or 1. Usage
  errors stay clap's own exit 2.

**Proof:**
- `no_terminal_returns_without_prompting`: `err` stays empty.
- `y_yes_and_padded_uppercase_are_yes`: `Y\n`, ` yes \n` and `YES\n` are all `Yes`.
- `empty_line_and_eof_are_no`.
- `read_error_is_no`: a reader that fails.
- `prompt_text_goes_to_err`: `err` holds exactly `Apply? [y/N] `.
- `success_is_zero_and_refused_is_one`.

**Done when:** `cargo test -p agentstacks-cli prompt exit` passes and `cargo make dod` is green.
Commit `feat(agentstacks-cli): confirmation prompt and exit codes`.

---

## Risks

- **Unused code between plans.** Units unused until plan 06 can trip `dead_code` under `-D
  warnings`. Each new item gets a temporary `#[cfg_attr(not(test), expect(dead_code, reason = "wired by the commands"))]`, because a plain `expect` is unfulfilled in the test target and fails `-D warnings`.
  Plan 06 must remove every one; its Task 3 states this.
- **The spec example.** It counts `3 to write` with the listed rows (`spec §5.2`). The test builds
  the plan to match those counts and does not retype the spec's ellipsis.

## Proof

- `cargo make dod` green.
- `cargo test -p agentstacks-cli` green.

## Review findings

- No blocking finding.
- 🔵 open: plan path column padded by byte length; a test named `spec_example_*`; a CLI unit test walks a fixture in `crates/agentstacks/tests/fixtures`; prompt write-failure path, diff `context_radius(3)` and the `update_text` NewTag-without-current arm untested.

## Deviations

- 2026-11-22 Task 2: spec example `spec.md:300-303` lists steps create, update, delete, update — not path order — while `spec.md:322` says "in the plan's path order" and `Plan::new` sorts by path. Built: path order (delete `old.md`, update `runtime.lua`, update `settings.json`, create `SKILL.md`). Spec claim disproved.
- 2026-11-22 Task 1: `spec.md:771` gives the workspace dependency `agentstacks = { path = "crates/agentstacks" }`. `cargo deny check` (cargo-deny 0.20.2, `deny.toml:74` `wildcards = "deny"`) → `error[wildcard]: found 2 wildcard dependencies for crate 'agentstacks-cli'` … `advisories ok, bans FAILED, licenses ok, sources ok`. Built: `agentstacks = { path = "crates/agentstacks", version = "0.0.0" }` → `advisories ok, bans ok, licenses ok, sources ok`. Spec claim disproved.
- 2026-11-22 Task 3: `update_text` renders a NewTag with no current tag as `  <key>: <new>`.
- 2026-11-22: tests written together with code; red not observed separately.
- 2026-11-22 (process): coders read their task sections straight from this plan file instead of a task-briefer brief; the ledger came from `grep '^### Task'`. Earlier batches were staged in the git index (not committed) so each review saw only its own batch.
