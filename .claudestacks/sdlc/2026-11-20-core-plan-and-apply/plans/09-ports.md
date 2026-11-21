---
status: draft
created: 2026-11-22
depends-on: [04, 06, 07]
built-on: ea104af7451e
---

# Ports Implementation Plan

**Goal:** declare the ports the flows need.

**Architecture:** `src/port/` holds traits and one fake per trait, no rule (spec §2 port row,
`docs/architecture.md` "Where logic lives"). Four ports join the built `FileTree` (moved to
`src/port/` by plan 01): `Workspace` (the project disk: read the two project files, resolve a path
to plan 06's `Resolution`, hand out a path-source directory as a `FileTree`, stage, swap, delete,
write the lock), `Git` (list tags, fetch a tag to its rev, give a rev's tree as a `FileTree`),
`Binary` (a `PATH` lookup, the `airsl --version` answer) and `Adapter` (open one extension, dispatch
`values`, dispatch `plan`). Every port is used as a generic bound (`W: Workspace`, `G: Git`,
`B: Binary`, `A: Adapter`, spec §2 static-dispatch row). A port that hands back a tree names it with
an associated type (`type Tree: FileTree`), and the adapter port names its per-extension engine
with `type Session`, so no trait object is needed anywhere. Each port is a folder holding its trait
and its fake, because the trait file must stay a pure trait (unit-test mandate exemption 3) while
the fake carries logic and tests. The fakes are compiled under `#[cfg(test)]`, as `port/fake.rs` is
today (`src/types/port/mod.rs:19-20`), and are built so the flows (plans 13-16) reach every §6.2
checked example, every §7 failure row, every §8 row and every §9.4 case without a disk, `git` or
airsl (spec §14 app row).

Answer types are the canonical ones other plans build, never copies (`modularity.md` Rule 2):
`Resolution`, `DiskState`, `DiskFile`, `Link`, `Reads`, `ReadOutcome`, `AirslAnswer` (plan 06);
`OpPath`, `Proposal`, `Proposals`, `ValuesInput`, `ValuesResult`, `PlanInput` (plan 04);
`FetchError` (plan 02); `GitUrl`, `GitTag`, `CommitId`, `Pin`, `SubPath`, `LocalPath`,
`BinaryName`, `Extension`, `Diagnostics`, `ContentHash` (built). I/O failures are the platform's
`std::io::Error`, as `FileTree` already does, so a flow reports `<path>: <io::Error Display>`
(spec §7 failure table, §13).

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6): `static-dispatch.md` (generic
bounds and associated types; no `dyn`); `mod-rs-export-only.md` (every `mod.rs` below holds only
docs, `mod` and `pub use`); `modularity.md` Rule 1 (one port per folder, the trait and its fake in
separate files) and Rule 2 (no port-local copy of a domain type); `unit-test-mandate.md` (each
`seam.rs` is a pure trait and carries the exemption-3 line; each `fake.rs` has colocated tests);
`strong-types.md` (paths are `OpPath` / `LocalPath`, names `BinaryName`, refs `GitTag` /
`CommitId`; no `bool` parameter); `doc-comment-discipline.md` (doc comments describe the ports and
never name plans, specs, chains or agents). Naming: one-word files; `seam.rs` is the guideline's
own name for a trait-only file.

**Tech Stack:** Rust 2024, crate `agentstacks`; `std::cell::RefCell` inside the fakes for their
call logs and recorded effects; no new dependency.

**Content authority:** spec §2 (port row, dependency rule, static-dispatch row), §6.1 (what each
flow step asks of a port), §6.3, §7, §8, §9.2, §9.3, §9.4, §10, §4.3-§4.4 (the adapter calls),
§14 (app row: fakes only).

---

## Files that change

```
crates/agentstacks/src/port/mod.rs               — modify: docs list the five ports; mod + pub use lines for the four folders
crates/agentstacks/src/port/workspace/mod.rs     — create: export-only
crates/agentstacks/src/port/workspace/seam.rs    — create: trait Workspace
crates/agentstacks/src/port/workspace/fake.rs    — create (#[cfg(test)]): FakeWorkspace, WorkspaceCall
crates/agentstacks/src/port/git/mod.rs           — create: export-only
crates/agentstacks/src/port/git/seam.rs          — create: trait Git
crates/agentstacks/src/port/git/fake.rs          — create (#[cfg(test)]): FakeGit, GitCall
crates/agentstacks/src/port/binary/mod.rs        — create: export-only
crates/agentstacks/src/port/binary/seam.rs       — create: trait Binary
crates/agentstacks/src/port/binary/fake.rs       — create (#[cfg(test)]): FakeBinary, BinaryCall
crates/agentstacks/src/port/adapter/mod.rs       — create: export-only
crates/agentstacks/src/port/adapter/seam.rs      — create: trait Adapter
crates/agentstacks/src/port/adapter/fake.rs      — create (#[cfg(test)]): FakeAdapter, ToolScript, ScriptStep, AdapterCall
```

`port/tree.rs`, `port/kind.rs` and `port/fake.rs` (`FileTree`, `EntryKind`, `FakeFileTree`) are
plan 01's and are not touched. The fakes below reuse `FakeFileTree` as their tree type.

## Order of work

### Task 1 — Declare the `Workspace` port and its fake

**Files:**
- Create `crates/agentstacks/src/port/workspace/seam.rs` — the trait.
- Create `crates/agentstacks/src/port/workspace/fake.rs` — the fake.
- Create `crates/agentstacks/src/port/workspace/mod.rs` — export-only: `mod seam; #[cfg(test)] mod fake;` and `pub use` of `Workspace` (and, under `#[cfg(test)]`, `FakeWorkspace`, `WorkspaceCall`).
- Modify `crates/agentstacks/src/port/mod.rs` — `pub mod workspace;` plus `#[doc(inline)] pub use` lines; the module doc's responsibilities list gains `Workspace`.

**Change:** `Workspace` is the project directory as the flows see it. Every method takes `&self`;
paths are project-relative `OpPath`s (normalised, spec §3.3 "Normalisation"):

- `type Tree: FileTree` — the tree a path source is read through.
- `project_text()` → `io::Result<String>`: the text of `agentstacks.toml` (spec §6.1 step 1). A
  missing file is the platform's `NotFound`; bytes that are not UTF-8 fail as `FileTree::read_text`
  does (`not valid UTF-8`).
- `lock_text()` → `io::Result<Option<String>>`: the text of `agentstacks.lock`, `None` when there is
  none (spec §6.1 step 1; §8 row 1 "a lock exists").
- `resolve(path)` → `io::Result<Resolution>`: the §6.3 resolution of one proposed, deleted, locked
  or `read` path, with the disk state of what it resolves to (spec §6.1 step 8, §6.2 "Disk states",
  §6.3, §8 rows 3-5). The answer is plan 06's `Resolution`: `Outside`, or `Inside { link, state }`
  where `link` says whether the path itself is a symlink (§6.2 second table, symlink row) and
  `state` is `Absent`, `File(DiskFile)` (bytes, hash, mode) or `NotRegular` (a dangling symlink
  included, §6.3 step 1). Apply's race check (§7 step 3) calls the same method.
- `source_tree(dir: &LocalPath)` → `io::Result<Option<Self::Tree>>`: a `path` source directory put
  through the same §6.3 rule (spec §6.3 last paragraph); `None` when it resolves outside the
  project, else the tree to load the pack or extension from.
- `stage(path, bytes)` → `io::Result<()>`: write the bytes to the step's temp
  `<dir>/.<name>.agentstacks-tmp`, creating parents, overwriting a leftover temp (spec §7 step 2).
- `discard(path)` → `io::Result<()>`: remove that temp; a temp already gone is not an error
  (spec §7 failure rows "2 or 3" and "4": "the temps … are removed").
- `swap(path)` → `io::Result<()>`: rename the temp onto the (canonical) target, an updated file
  keeping its old mode (spec §7 step 4, "Modes").
- `delete(path)` → `io::Result<()>`: remove a locked file (spec §7 step 4); directories it leaves
  empty stay (§7, §17).
- `write_lock(text)` → `io::Result<()>`: write `agentstacks.lock` via temp + rename (spec §7 step 5).

The trait doc states the contract each adapter keeps (plan 10 proves it for the real disk): paths
compare by bytes (§17), the lock records the proposed path and never the canonical one (§6.3), and
nothing outside these methods writes.

`FakeWorkspace` is an in-memory project, rooted at a display path, keyed by the normalised path text:

- Builders (consume `self`, like `FakeFileTree`): `project(text)`, `lock(text)`,
  `file(path, bytes)` (mode `FakeWorkspace::NEW_MODE`, the 644 that `std::fs::write` gave in
  `probes/round3-results.md` S2), `file_with_mode(path, bytes, mode)`, `not_regular(path)` (a
  directory or special file), `symlink(path, target)` (a link inside the project; a target with no
  entry makes it dangling), `outside(path)` (resolves outside, e.g. a link to `/etc/hosts`, §6.2
  last checked example), `source(dir, FakeFileTree)` and `source_outside(dir)`, and
  `fail(WorkspaceCall)` (that exact call fails every time with `FakeWorkspace::denied()`, a
  `PermissionDenied` error, mirroring `FakeFileTree::denied()`).
- `resolve` answers from the entries: no entry → `Inside { Direct, Absent }`; a file →
  `Inside { Direct, File(DiskFile::new(bytes, mode)) }`; `not_regular` → `Inside { Direct,
  NotRegular }`; a symlink → `Inside { Symlink, <state of its target> }`, `NotRegular` when the
  target has no entry or is not regular; `outside` → `Outside`.
- `source_tree` returns the configured `FakeFileTree`, `None` for `source_outside`, and
  `FakeFileTree::missing(dir)` for an unconfigured directory.
- `stage` keeps the bytes as a temp; `discard` drops it; `swap` moves the temp's bytes onto the
  target, through a symlink to its target (the link stays, §6.3 "written through"), keeping an
  existing file's mode and giving a new one `NEW_MODE`; `delete` removes the entry; `write_lock`
  replaces the lock text. Each runs only when its call is not set to fail.
- Mid-test edits for the apply race (spec §14 "the apply race"): `set_file(&self, path, bytes)` and
  `remove(&self, path)` change the disk between `plan` and `apply`.
- Observers: `calls()` → the ordered `Vec<WorkspaceCall>` (`ProjectText`, `LockText`,
  `Resolve(path)`, `SourceTree(dir)`, `Stage(path)`, `Discard(path)`, `Swap(path)`, `Delete(path)`,
  `WriteLock`), failed calls included; `file_bytes(path)`, `mode(path)`, `temps()` (paths with a
  temp still present), `lock_written()` (the last text given to `write_lock`).

**Proof:** written first, seen failing, then passing.
- `resolve_reports_each_disk_state` — absent, file (hash = `ContentHash::of_bytes`, mode kept),
  not regular, and `outside` each give the matching `Resolution`.
- `resolve_follows_an_inside_symlink` — a link to an inside file gives `Inside { Symlink, File }`
  with the target's bytes; a link to a missing target gives `Inside { Symlink, NotRegular }`.
- `project_and_lock_text` — `project_text` returns the text; `lock_text` is `None` without a lock;
  `project_text` without a project fails with `NotFound`.
- `source_tree_answers_configured_outside_and_missing` — configured tree, `None`, and a missing tree.
- `stage_then_swap_installs_bytes_and_keeps_mode` — an existing 755 file updated through
  stage + swap holds the new bytes with mode 755; a new path gets `NEW_MODE`; no temp is left.
- `swap_through_a_symlink_updates_the_target` — the link still resolves as `Symlink`, the target
  holds the new bytes.
- `discard_removes_a_temp_and_tolerates_none` — after `discard` no temp remains; a second
  `discard` succeeds.
- `delete_removes_the_file` — the path resolves `Absent` afterwards.
- `write_lock_records_the_text` — `lock_written` and `lock_text` both return it.
- `a_failing_call_returns_denied_and_changes_nothing` — `fail(Swap(".claude/b"))`: swapping
  `.claude/a` succeeds, `.claude/b` fails with `denied()`'s kind and Display, `.claude/b`'s temp
  stays, and its old bytes stay.
- `calls_are_logged_in_order` — stage, resolve, swap, write_lock come back in that order.
- `set_file_changes_the_disk_between_calls` — a `resolve` after `set_file` sees the new hash.
- `fake_is_usable_through_a_generic_bound` — a helper generic over `W: Workspace` reads the
  project text from the fake (the port is generic-friendly, spec §2).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): workspace port and its fake`.

### Task 2 — Declare the `Git` port and its fake

**Files:**
- Create `crates/agentstacks/src/port/git/seam.rs` — the trait.
- Create `crates/agentstacks/src/port/git/fake.rs` — the fake.
- Create `crates/agentstacks/src/port/git/mod.rs` — export-only.
- Modify `crates/agentstacks/src/port/mod.rs` — `pub mod git;` plus `pub use` lines.

**Change:** `Git` is the remote side of a git source (spec §9.2, §9.3). Failures are plan 02's
`FetchError`, without a location: the flow adds `agentstacks.toml` and the entry's key (§13).

- `type Tree: FileTree` — a rev's files, read into memory once (spec §9.2 "The source's tree is read
  into memory once and served through the `FileTree` port").
- `list_tags(url, prefix: &str)` → `Result<Vec<GitTag>, FetchError>`: the tags under
  `refs/tags/<prefix>*`, in the order git lists them (lexical, `probes/git-results.md` G1); empty
  when nothing matches (§9.2 "list tags", §9.4 step 1). Choosing among them is plan 07's.
- `fetch_tag(url, tag)` → `Result<CommitId, FetchError>`: fetch the tag into the cache and return
  the commit it names, `^{commit}` for an annotated tag (§9.2 "fetch a tag", §9.3 "A tag fetch
  learns its rev here").
- `tree(url, rev, subdir: Option<&SubPath>, at: &Pin)` → `Result<Self::Tree, FetchError>`: the
  files under `subdir` at `rev`, fetching the rev first only when the cache does not hold it (§9.2
  "fetch a rev", "list files", "read blobs"; §9.3 "A locked rev whose `<rev>.git` exists resolves
  without network"). `at` is the pin the source names, used as `<tag or rev>` in the §9.2 `subdir`
  refusals. A symlink is `EntryKind::Symlink` and a gitlink `EntryKind::Other` in the tree (§9.2).

The trait doc states two contracts the flows rely on: "git not on `PATH`" comes back as its own
`FetchError` outcome on every call (§9.2 row 1, so the flow can report it once for all keys), and a
non-zero exit carries git's trimmed stderr (§9.2 row 2, D11).

`FakeGit` answers from configuration and logs every call:

- Builders: `tags(url, tags)` (`list_tags` returns those whose text starts with `prefix`, sorted
  by bytes, as git does), `tag(url, tag, rev)` (`fetch_tag`), `tree(url, rev, subdir, FakeFileTree)`
  (`tree`; the `FakeFileTree` is returned as a clone, so the same rev can be served twice),
  `fail(url, FetchError)` (queued: the next call on that URL returns it, once per queued error) and
  `git_missing()` (every call returns the §9.2 row 1 outcome).
- A call with no configured answer returns the §9.2 row 1 outcome too, so a test that forgot to
  configure a source sees a refusal it did not expect rather than a silent success.
- `calls()` → the ordered `Vec<GitCall>`: `ListTags { url, prefix }`, `FetchTag { url, tag }`,
  `Tree { url, rev, subdir }`. This is how plan 13 proves a moved tag uses no tag call (§9.1 last
  paragraph, §14 "moved tag") and a removed pack makes no call, and how plan 16 proves the prefix it
  asked for.

**Proof:**
- `list_tags_filters_by_prefix_in_byte_order` — configured `packs-v0.2.0`, `other-v9.9.9`,
  `packs-v0.10.0`, `packs-v0.1.0`; prefix `packs-v` gives `packs-v0.1.0`, `packs-v0.10.0`,
  `packs-v0.2.0` (the G1 order); prefix `packs-v9` gives an empty list.
- `fetch_tag_returns_the_configured_rev` — and logs `FetchTag`.
- `tree_returns_the_configured_files_twice` — two `tree` calls for one rev both serve the files.
- `queued_failure_is_returned_once` — the first call on the URL fails with the queued error, the
  second gets the configured answer.
- `git_missing_answers_every_call` — all three methods return the §9.2 row 1 outcome.
- `unconfigured_call_reports_git_missing` — a `tree` for an unknown rev returns that outcome.
- `calls_are_logged_in_order` — list, fetch, tree come back in call order with their arguments.
- `fake_is_usable_through_a_generic_bound` — a helper generic over `G: Git` reads a file from
  `G::Tree`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): git port and its fake`.

### Task 3 — Declare the `Binary` port and its fake

**Files:**
- Create `crates/agentstacks/src/port/binary/seam.rs` — the trait.
- Create `crates/agentstacks/src/port/binary/fake.rs` — the fake.
- Create `crates/agentstacks/src/port/binary/mod.rs` — export-only.
- Modify `crates/agentstacks/src/port/mod.rs` — `pub mod binary;` plus `pub use` lines.

**Change:** `Binary` answers the two questions of spec §10 and §8 rows 6-8:

- `on_path(name: &BinaryName)` → `bool`: whether `name` is found on `PATH` (§10 rows 1-2: a pack's
  `requires.binaries`, an extension's `require_binary`).
- `airsl_version()` → `AirslAnswer` (plan 06): `NotFound` when `airsl` is not on `PATH`, else
  `Printed(text)`, what `airsl --version` printed (§10 row 3, §8 rows 6-8). Parsing `airsl <semver>`
  and comparing versions are plan 06's and plan 04's rules, never the port's.

`FakeBinary`: builders `with(name)` (on `PATH`) and `airsl(text)` (`airsl` on `PATH`, printing
`text`); without `airsl(…)` the answer is `NotFound`. `asked()` → the ordered `Vec<BinaryCall>`
(`OnPath(name)`, `AirslVersion`), so plan 13 proves a repeated name is looked up once (§3.3
"checked once") and plans 13 and 15 prove airsl is never asked when no pack declares
`requires.airsl` or the lock has no `[airsl]`.

**Proof:**
- `on_path_answers_configured_names` — `rg` configured → true; `node` not → false.
- `airsl_version_is_not_found_by_default` — `NotFound`.
- `airsl_version_prints_the_configured_text` — `airsl("airsl 0.1.4")` → `Printed("airsl 0.1.4")`,
  and `on_path` of `airsl` is true.
- `questions_are_logged` — two lookups and one version query come back in order.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): binary port and its fake`.

### Task 4 — Declare the `Adapter` port and its fake

**Files:**
- Create `crates/agentstacks/src/port/adapter/seam.rs` — the trait.
- Create `crates/agentstacks/src/port/adapter/fake.rs` — the fake.
- Create `crates/agentstacks/src/port/adapter/mod.rs` — export-only.
- Modify `crates/agentstacks/src/port/mod.rs` — `pub mod adapter;` plus `pub use` lines.

**Change:** `Adapter` runs one tool extension through the §4.4 flow. The engine must live from the
entry run through `values` to `plan`, because the entry run registers the handlers (§4.4 steps 1-4,
D5), so the port names it as an associated type:

- `type Session` — one extension's live engine.
- `open<T: FileTree>(extension: &Extension, files: &T)` → `Result<Self::Session, Diagnostics>`:
  load the extension's `.lua` files from `files` into memory (§4.1 step 3; `Extension` keeps no file
  contents) and run the entry file at the top level (§4.4 step 1). `files` is the tree the extension
  was loaded from (`G::Tree` or `W::Tree`).
- `values(session, input: &ValuesInput)` → `Result<ValuesResult, Diagnostics>`: dispatch `values`
  (§4.4 step 2) and return the checked result.
- `plan<W: Workspace>(session, input: &PlanInput, workspace: &W, proposals: &mut Proposals)` →
  `Result<Reads, Diagnostics>`: dispatch `plan` (§4.4 step 4). Each accepted operation goes into
  `proposals`, which already holds the core's writes and every earlier tool's operations, so the
  call-time duplicate rule names `… already proposed by <tool>` or `by core` (§3.3 row "path not
  proposed twice", §4.4 "Tools run in name order"). `workspace` serves `agentstacks.read`
  (resolution and disk state, §4.3, §6.3). The result is the read record: what each `read` returned
  (§4.3 "The core records what each read returned").
- Refusals are `Diagnostics` located as §13 rows 1-3 say; any refusal refuses the whole plan (§4.6).

`FakeAdapter` plays a scripted extension per tool, keyed by the extension's `ToolName`
(`extension.manifest().name()`):

- `FakeAdapter::new().tool(name, ToolScript)`. A `ToolScript` holds an optional open refusal
  (`Diagnostics`), the `values` answer (`Result<ValuesResult, Diagnostics>`) and the plan steps, a
  list of `ScriptStep`: `Read(OpPath)`, `Propose(Proposal)` and `Refuse(Diagnostic)`.
- `open` returns the tool's name as the session, or its scripted refusal; an unscripted tool's
  `open` returns an empty `Diagnostics` as `Err`, so the test's exact-refusal assertion fails visibly.
- `values` returns the scripted answer once (taken from the script).
- `plan` runs the steps in order: `Read` asks `workspace.resolve` and records, like the host does,
  `ReadOutcome::Bytes(hash)` for an inside regular file and `ReadOutcome::Absent` for an inside path
  with no regular file (§4.3 "or `nil` when no regular file is there"); an outside path records
  nothing (the located refusal is the host's, §4.3, and a test scripts it with `Refuse`).
  `Propose` inserts the `Proposal` through `Proposals`' own insertion, so the duplicate-path and
  skip/cover rules (§3.2, §3.3) run exactly as with the real host, and a refusal it returns is
  collected. `Refuse` collects its `Diagnostic`. The call returns the `Reads` when nothing was
  collected, else every collected refusal.
- `calls()` → the ordered `Vec<AdapterCall>`: `Open(tool)`, `Values(tool, ValuesInput)`,
  `Plan(tool, PlanInput)`, with clones of the inputs, so plan 13 proves tools run in name order
  (§4.4), the rendered `PlanInput`, and a removed tool's empty `packs` list (§6.4 bullet 1).

**Proof:**
- `open_values_plan_follow_the_script` — the scripted `ValuesResult` comes back; the plan's
  proposals land in `Proposals`; the result is an empty `Reads`.
- `plan_records_what_read_returned` — a `Read` of a `FakeWorkspace` file records `Bytes` with its
  hash; a `Read` of an absent path records `Absent`; an outside path records nothing.
- `propose_runs_the_duplicate_rule` — a `Proposals` already holding `.claude/x` from another tool:
  the scripted write to `.claude/x` comes back as the refusal `Proposals` returns for it, exactly as
  it displays.
- `refuse_collects_and_fails_the_call` — one `Refuse` step makes `plan` return that diagnostic.
- `open_refusal_is_returned` — a scripted open refusal comes back; `values` is never reached.
- `unscripted_tool_is_refused_empty` — `open` of an unknown tool is `Err` with no diagnostics.
- `calls_log_inputs_in_order` — open, values, plan are logged with clones of their inputs.
- `fake_is_usable_through_a_generic_bound` — a helper generic over `A: Adapter, W: Workspace`
  drives open, values and plan on the fakes.

**Done when:** `cargo make dod` passes (end of plan) and the Definition of Done is green.
Commit `feat(agentstacks): adapter port and its fake`.

---

## Risks

- **Seams with the plans that build the answer types.** `Resolution`, `DiskFile::new(bytes, mode)`,
  `Reads`, `ReadOutcome`, `AirslAnswer` (plan 06), `Proposals`' insertion and the payload types
  (plan 04) and `FetchError` (plan 02) are used by name. The fakes need: `ValuesInput` and
  `PlanInput` to be `Clone` (the call log keeps copies), and a `FetchError` value for "git not on
  `PATH`" that carries no "needed by" list, because a port call sees one source and plan 13 merges
  the keys (§9.2 row 1 "once"). If plan 02 shapes that variant differently, plan 02's shape wins and
  the fake builds it the way plan 02 allows.
- **The adapter signature is wider than plan 13's wording.** Plan 13 calls `values` and `plan`
  directly; the port adds `open` and a `Session`, which plan 12 needs (one engine across both
  dispatches, the extension's `FileTree` for its `.lua` files). Plan 13's coder follows the names
  here.
- **The fakes are `#[cfg(test)]`.** A doc-comment example in `app/` cannot use them (doctests do not
  compile `cfg(test)` items); plans 13 and 14 promise such examples and need a non-fake example or a
  `no_run` text instead. Flagged for the orchestrator.
- **Riskiest task: Task 1.** `FakeWorkspace` must model write-through, mode keeping and failure
  injection faithfully enough that plan 14's tests mean the same as plan 10's contract tests. Each
  behaviour has a fake test here and a disk test there with the same name stem.
- Option not taken: one port per flow (a read-only workspace for doctor, a writing one for apply).
  The spec names one `Workspace` (§2); `apply` and `doctor` simply use the methods they need.
- Option not taken: pre-reading every `extension.reads` path in the flow and passing bytes to the
  adapter. `Adapter::plan` takes the workspace instead, so the record holds only paths actually read
  (§6.2 "Shared": "read it in this run").

## Proof

- `cargo make dod` is green at the end of the plan.
- `grep -rn "dyn " crates/agentstacks/src/port` finds nothing.
- `tests/isolation.rs` (plan 01) passes: no `std::fs`, `std::process`, `std::net`, `crate::fs`,
  `crate::process` or `crate::host` in `src/port`.
- Spec rows covered: §2 port row and static-dispatch row (all tasks, the generic-bound tests);
  §6.1 steps 1, 3, 5, 7, 8 (what each step asks: Tasks 1-4); §6.3 resolution answer incl. symlink and
  path-source rule (Task 1); §7 steps 2-5 and the fake I/O failure for each failure row (Task 1);
  §8 rows 1, 3-5 (lock text, resolution, Task 1) and 6-8 (airsl answer, Task 3); §9.2 list tags,
  fetch tag, fetch rev, list files, read blobs, git-missing outcome, symlink and gitlink kinds as
  the port contract (Task 2); §9.3 cache-hit contract (Task 2); §9.4 step 1 listing (Task 2); §10
  `PATH` lookup and airsl answer (Task 3); §4.3 `read` record and §4.4 values/plan dispatch with
  cross-tool duplicates (Task 4); §14 app row (fakes for every flow). The real adapters' behaviour
  is proven by plans 10, 11 and 12.
