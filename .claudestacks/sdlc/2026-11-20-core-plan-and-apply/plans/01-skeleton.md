---
status: approved
created: 2026-11-22
depends-on: []
built-on: f39b2a814319
---

# Skeleton Implementation Plan

**Goal:** Lay out the ports-and-adapters module skeleton of `crates/agentstacks`.

**Architecture:** The crate root gets the six spec §2 modules: `types` (rules), `port` (traits and
fakes), `app` (flows), and the adapters `fs`, `process` and `host`. `FileTree`, `EntryKind` and
`FakeFileTree` move unchanged from `src/types/port/` to `src/port/` (spec §12 last row). `types`
may import only `port::{FileTree, EntryKind}` (spec §2 dependency rule, §16 X2), so the `utf8_text`
helper that `types/extension/loaded.rs` uses moves the other way, into `types/tree/text.rs`, and
`port/tree.rs` imports it from `types` (`port` → `types` is allowed). The four empty seams
`executor.rs`, `host.rs`, `plan.rs` and `validation.rs` (`src/lib.rs:26-31`) are removed (spec §2
"removed:"). Every new module is a docs-only stub, so later plans only add files and `mod` /
`pub use` lines. `tests/isolation.rs` is extended to guard `src/port` and `src/app` the way it
guards `src/types`, to refuse a path into an adapter module (`crate::fs`, `crate::process`,
`crate::host`) inside `types`, `port` and `app` (spec §2 paragraph after the dependency rule), and
to allow exactly `crate::port::{FileTree, EntryKind}` inside `src/types`, plus
`crate::port::FakeFileTree` in its `#[cfg(test)]` code (§16 X2, R28(k)). The dependency
and `deny.toml` changes of spec §15 (rows 1-4, §16 X1) land here, so that no later plan edits
`Cargo.toml` or `deny.toml`.

Guideline rules applied (claudestacks-guideline-rust 0.1.6): `mod-rs-export-only.md` (every
`mod.rs` and `lib.rs` holds only `//!` docs, `mod` and `pub use`; every new module file opens with
the four-question block); `unit-test-mandate.md` exemption #1 (the docs-only stubs carry the
export-only line instead of a test block, as the removed seams do today); `modularity.md` Rule 1
(one job per module, the spec §2 table); naming (one-word folder and file names, no `_`, the folder
word never repeated: `port/tree.rs`, not `port/port_tree.rs`); `doc-comment-discipline.md` (no plan,
spec or chain names in any doc comment; module docs describe the code). Fakes are `#[cfg(test)]`
(spec §2 module table, `port/*` row), so no doc-comment example uses `FakeFileTree` (plan-set
rule); the moved doctests use `StdFileTree` and stay so.

**Tech Stack:** Rust 2024 (MSRV 1.94), crate `agentstacks`; cargo-make gate; cargo-deny 0.20;
`airsl 0.1.4` and `serde_json 1.0.154` from crates.io (declared only, not yet used).

**Content authority:** spec §2 (module tree, dependency rule, isolation paragraph, module table),
§12 last row (the `FileTree` move), §15 rows 1-5 (dependencies, `skip-tree`, `Zlib`, `tempfile`),
§16 X1, X2, R28(k).

---

## Files that change

```
crates/agentstacks/src/port/mod.rs              — create (moved from src/types/port/mod.rs): port docs, `mod`/`pub use` lines
crates/agentstacks/src/port/tree.rs             — create (moved from src/types/port/tree.rs): FileTree, unchanged; utf8_text imported from types
crates/agentstacks/src/port/kind.rs             — create (moved from src/types/port/kind.rs): EntryKind, unchanged
crates/agentstacks/src/port/fake.rs             — create (moved from src/types/port/fake.rs): FakeFileTree, unchanged
crates/agentstacks/src/types/port/              — delete (all four files, moved above)
crates/agentstacks/src/types/tree/text.rs       — create: utf8_text (moved out of port/tree.rs) and its test
crates/agentstacks/src/types/tree/mod.rs        — modify: `mod text;`, `pub(crate) use text::utf8_text;`, doc bullet
crates/agentstacks/src/types/mod.rs             — modify: drop `pub mod port` and its two re-exports; add the five new rule modules; docs
crates/agentstacks/src/types/op/mod.rs          — create: docs-only stub
crates/agentstacks/src/types/json/mod.rs        — create: docs-only stub
crates/agentstacks/src/types/plan/mod.rs        — create: docs-only stub
crates/agentstacks/src/types/resolve/mod.rs     — create: docs-only stub
crates/agentstacks/src/types/protocol/mod.rs    — create: docs-only stub
crates/agentstacks/src/app/mod.rs               — create: docs, `pub mod plan; apply; doctor; update;`
crates/agentstacks/src/app/plan.rs              — create: docs-only stub
crates/agentstacks/src/app/apply.rs             — create: docs-only stub
crates/agentstacks/src/app/doctor.rs            — create: docs-only stub
crates/agentstacks/src/app/update.rs            — create: docs-only stub
crates/agentstacks/src/process/mod.rs           — create: docs-only stub
crates/agentstacks/src/host/mod.rs              — create: docs-only stub (replaces src/host.rs)
crates/agentstacks/src/executor.rs              — delete
crates/agentstacks/src/host.rs                  — delete
crates/agentstacks/src/plan.rs                  — delete
crates/agentstacks/src/validation.rs            — delete
crates/agentstacks/src/lib.rs                   — modify: `pub mod app; fs; host; port; process; types;`, crate docs
crates/agentstacks/src/fs/disk.rs               — modify: use path and doctest import of FileTree/EntryKind
crates/agentstacks/src/fs/mod.rs                — modify: one doc sentence (the ports now live in `port`)
crates/agentstacks/src/types/extension/loaded.rs — modify: use paths of FileTree, FakeFileTree; utf8_text from types::tree
crates/agentstacks/src/types/extension/mod.rs   — modify: intra-doc link target of FileTree
crates/agentstacks/src/types/pack/error.rs      — modify: intra-doc link target of FileTree::read_text
crates/agentstacks/src/types/pack/load.rs       — modify: use paths of FileTree, FakeFileTree
crates/agentstacks/src/types/pack/mod.rs        — modify: intra-doc link target of FileTree
crates/agentstacks/src/types/set/testing.rs     — modify: use path of FakeFileTree
crates/agentstacks/src/types/tree/hash/tree.rs  — modify: use paths of FileTree, FakeFileTree
crates/agentstacks/src/types/tree/walk/run.rs   — modify: use paths of EntryKind, FileTree, FakeFileTree
crates/agentstacks/tests/isolation.rs           — modify: scan port and app; refuse adapter paths; types names only the tree port; header docs
Cargo.toml                                      — modify: workspace deps airsl, serde_json
crates/agentstacks/Cargo.toml                   — modify: add airsl, serde_json (workspace = true)
deny.toml                                       — modify: [bans] skip-tree line (§15, X1); "Zlib" in [licenses] allow
Cargo.lock                                      — modify: by cargo
```

### The 22 users of `FileTree` / `EntryKind`

`grep -rlE "FileTree|EntryKind" src tests` in `crates/agentstacks`, outside `src/types/port`, lists
22 files (spec §12 last row). Each one, and what this plan does to it:

| # | File | What matches | Edit |
|---|---|---|---|
| 1 | `src/fs/disk.rs` | `use crate::types::port::{EntryKind, FileTree}` (line 26); doctest `use agentstacks::types::{EntryKind, FileTree}` (line 39) | both paths to `port` |
| 2 | `src/fs/mod.rs` | doc text "the ports the `types` module declares", `StdFileTree` | the doc sentence only |
| 3 | `src/types/extension/loaded.rs` | `use crate::types::port::FileTree` / `utf8_text` (lines 33-34); test `use crate::types::port::FakeFileTree` (line 205) | `FileTree`, `FakeFileTree` to `crate::port`; `utf8_text` to `crate::types::tree::utf8_text` (X2) |
| 4 | `src/types/extension/mod.rs` | intra-doc link `crate::types::FileTree` (line 8) | link to `crate::port::FileTree` |
| 5 | `src/types/manifest/exclude.rs` | `StdFileTree` in a doctest only | none |
| 6 | `src/types/mod.rs` | `pub mod port`; `pub use port::EntryKind` / `FileTree` (lines 40, 192, 194) | removed (Task 1) |
| 7 | `src/types/pack/error.rs` | intra-doc link `crate::types::FileTree::read_text` (line 101) | link to `crate::port::FileTree::read_text` |
| 8 | `src/types/pack/item/loaded.rs` | `StdFileTree` in doctests only | none |
| 9 | `src/types/pack/load.rs` | `use crate::types::port::FileTree` (line 68); test `FakeFileTree` (line 502) | two paths to `crate::port` |
| 10 | `src/types/pack/mod.rs` | intra-doc link `crate::types::FileTree` (line 8) | link to `crate::port::FileTree` |
| 11 | `src/types/pack/native.rs` | `StdFileTree` in a doctest only | none |
| 12 | `src/types/set/check.rs` | `StdFileTree` in a doctest only | none |
| 13 | `src/types/set/testing.rs` | `use crate::types::port::FakeFileTree` (line 22) | path to `crate::port` |
| 14 | `src/types/set/valid.rs` | `StdFileTree` in a doctest only | none |
| 15 | `src/types/tree/hash/tree.rs` | `use crate::types::port::FileTree` (line 25); test `FakeFileTree` (line 113) | two paths to `crate::port` |
| 16 | `src/types/tree/path.rs` | `StdFileTree` in doctests only | none |
| 17 | `src/types/tree/walk/outcome.rs` | `StdFileTree` in doctests only | none |
| 18 | `src/types/tree/walk/run.rs` | `use crate::types::port::EntryKind` / `FileTree` (lines 32-33); test `FakeFileTree` (line 145) | three paths to `crate::port` |
| 19 | `tests/extension.rs` | `agentstacks::fs::StdFileTree` only | none |
| 20 | `tests/isolation.rs` | `FileTree` in the header doc | rewritten in Task 3 |
| 21 | `tests/pack.rs` | `agentstacks::fs::StdFileTree` only | none |
| 22 | `tests/set.rs` | `agentstacks::fs::StdFileTree` only | none |

Ten of the 22 match only through the word `StdFileTree` (the `fs` adapter type, whose path does not
change) and need no edit. The moved files' own doctests (`port/tree.rs`, `port/kind.rs`) import
`agentstacks::types::FileTree` / `EntryKind` today and switch to `agentstacks::port::…`.

## Order of work

### Task 1 — Move `FileTree` and `EntryKind` into `port`

**Files:**
- Create `crates/agentstacks/src/port/{mod,tree,kind,fake}.rs` — moved with `git mv` from
  `src/types/port/`, bodies unchanged except the internal `use crate::types::port::…` lines (now
  `crate::port::…`) and the doctest imports (now `agentstacks::port::{EntryKind, FileTree}`).
  `port/mod.rs` keeps `pub mod kind; pub mod tree; #[cfg(test)] pub mod fake;` and the three
  `pub use` lines, and drops `pub(crate) use tree::utf8_text;`. Its `//!` block is rewritten for
  the module's new place: the home of the traits the flows and rules read the outside through, each
  with a fake for tests; `FileTree` is the one it holds today; non-responsibilities: no logic, no
  adapter (spec §2 `port/` row, §2 module table `port/*` row).
- Create `crates/agentstacks/src/types/tree/text.rs` — `utf8_text`, body and doc unchanged, still
  `pub(crate)`, moved out of `port/tree.rs` with its test; `//!` block: a file's bytes as text,
  refused with the one `not valid UTF-8` wording every reader of a tree uses.
  `FileTree::read_text` in `port/tree.rs` calls it through `crate::types::tree::utf8_text`.
- Modify `crates/agentstacks/src/types/tree/mod.rs` — `mod text;`,
  `pub(crate) use text::utf8_text;`, one responsibility bullet.
- Modify `crates/agentstacks/src/types/mod.rs` — remove `pub mod port;`, the `pub use port::EntryKind`
  and `pub use port::FileTree` lines, and the `[`port`]` responsibility bullet.
- Modify `crates/agentstacks/src/lib.rs` — add `pub mod port;` (the rest of `lib.rs` changes in
  Task 2).
- Modify the 12 files marked with an edit in the table above (rows 1-4, 7, 9, 10, 13, 15, 18):
  use paths, doctest imports and intra-doc link targets only.

**Change:** `agentstacks::port::{FileTree, EntryKind}` (and `FakeFileTree` under `#[cfg(test)]`)
replace `agentstacks::types::{FileTree, EntryKind}`. Behaviour is unchanged: the trait, the enum,
the fake and `utf8_text` move byte for byte apart from paths. `agentstacks::types::FileTree` no
longer exists (no compatibility re-export: spec §2 puts `FileTree` in `port`, and modularity Rule 2
allows one home per concept). Outside tests, `src/types` now names only `crate::port::FileTree` and
`crate::port::EntryKind` (X2).

**Proof:** written first, seen failing, then passing.
- The doctests of `port::tree::FileTree` and `port::kind::EntryKind` are first edited to import
  from `agentstacks::port`; they fail to compile before the move and pass after it.
- `read_text_returns_utf8_text`, `read_text_refuses_bytes_that_are_not_utf8`,
  `read_text_passes_a_read_failure_through` (`port/tree.rs`), `the_kinds_are_distinct`
  (`port/kind.rs`) and every test of `port/fake.rs` run at their new path with unchanged assertions.
- `utf8_text_words_bad_bytes_as_read_text_does` runs in `types/tree/text.rs` with unchanged
  assertions (`a` plus a newline comes back as text; the bytes `0x66 0xFF` are refused as
  `InvalidData`, displayed `not valid UTF-8`).
- Every existing test of the rows 3, 9, 13, 15 and 18 files passes unchanged; they reach the fake
  through the new path.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `refactor(agentstacks): move FileTree and EntryKind to port`.

### Task 2 — Lay out the empty modules of the skeleton

**Files:**
- Delete `crates/agentstacks/src/{executor,host,plan,validation}.rs`.
- Create `crates/agentstacks/src/app/mod.rs` — `//!` block (the use cases, generic over ports, each
  calling ports in order and handing results to `types` rules; non-responsibilities: no rule, no
  adapter, no `std::fs` / `std::process` / `std::net`) and `pub mod apply; pub mod doctor;
  pub mod plan; pub mod update;`.
- Create `crates/agentstacks/src/app/{plan,apply,doctor,update}.rs` — one `//!` block each naming
  the flow it will hold (build a reviewable plan; apply a shown plan as the only writer; report
  drift against the lock; find newer tags for git sources) and the export-only exemption line.
- Create `crates/agentstacks/src/process/mod.rs` — `//!` block: the adapter that drives the `git`
  CLI and looks up binaries on `PATH`, the only `std::process` caller; translation only; the one
  adapter allowed to use another adapter's item, `fs::cache` (§16 X3).
- Create `crates/agentstacks/src/host/mod.rs` — `//!` block: the adapter that runs an extension's
  Lua adapter in the airsl sandbox, the only place Lua runs; translation only.
- Create `crates/agentstacks/src/types/{op,json,plan,resolve,protocol}/mod.rs` — `//!` block each,
  the one job from the spec §2 module table (`types/op`: an operation and its validation;
  `types/json`: editing a JSON document's text at one key path; `types/plan`: the step for each
  path from op, lock and disk state; `types/resolve`: reconciling the lock with
  `agentstacks.toml` and choosing tags; `types/protocol`: the hook command line and the text of
  the hook runtime). No items yet.
- Modify `crates/agentstacks/src/types/mod.rs` — add `pub mod json; pub mod op; pub mod plan;
  pub mod protocol; pub mod resolve;` and one responsibility bullet each. These modules get no flat
  `pub use` lines in `types/mod.rs`; callers use `types::op::…` (plan-set convention).
- Modify `crates/agentstacks/src/lib.rs` — modules become `pub mod app; pub mod fs; pub mod host;
  pub mod port; pub mod process; pub mod types;`. The `//!` responsibilities list the six modules
  with their spec §2 jobs and the dependency rule: `types` → `port::{FileTree, EntryKind}` only;
  `port` → `types`; `app` → `port`, `types`; `fs`, `process`, `host` → `port`, `types`; `process` →
  `fs::cache` only (spec §2, §16 X2, X3). Every intra-doc link to a removed module (`validation`,
  `plan`, `executor`) is gone.
- Modify `crates/agentstacks/src/fs/mod.rs` — the first two doc sentences say the adapter answers
  the ports `port` declares (today: "the ports the `types` module declares"). Nothing else in that
  file changes.

**Change:** The crate exposes `app`, `fs`, `host`, `port`, `process` and `types`, matching the spec
§2 tree. Each stub file carries the unit-test-mandate exemption #1 line (export-only, no executable
logic), worded as the removed seams word it today.

**Proof:** written first, seen failing, then passing.
- No unit test: every file this task creates or edits is export-only (unit-test-mandate
  exemption #1). The proof is the gate's doc step: the new `lib.rs` intra-doc links to
  `[`app`]`, `[`port`]`, `[`process`]`, `[`host`]` and `types::{op, json, plan, resolve, protocol}`
  are written first and fail `RUSTDOCFLAGS="-D warnings" cargo doc` until the modules exist.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `refactor(agentstacks): lay out the ports-and-adapters skeleton`.

### Task 3 — Guard the inside: no outside, no adapters, only the tree port in `types`

**Files:**
- Modify `crates/agentstacks/tests/isolation.rs` — header `//!` rewritten (the inside, `types`,
  `port` and `app`, reaches the disk, processes, the network and the adapters only through ports,
  and `types` reaches only the read-only tree port); the scans below.

**Change:**
- The existing `std::fs` / `std::process` / `std::net` / `File::open` scan (including grouped
  `use std::{..}`) runs over all three inside directories, not only `src/types` (spec §2: "extended
  to cover `src/port` and `src/app`").
- A new scan refuses, in code (comments ignored by the existing `without_comments`), the paths
  `crate::fs`, `crate::process` and `crate::host`, whole or as a direct member of a grouped
  `use crate::{..}`, inside the same three directories (spec §2: "refuse any `use crate::fs`,
  `crate::process` or `crate::host` inside those three"). A match must end at a word boundary, so
  `crate::fsx` or `crate::types::fs` are not flagged. The grouped form reuses the member walk of
  `grouped_imports`, parameterised by the root word (`std` or `crate`) and its forbidden members.
- A third scan applies to `src/types` only: a `crate::port` path in code is refused unless it names
  exactly `crate::port::FileTree` or `crate::port::EntryKind` (anywhere), or
  `crate::port::FakeFileTree` inside `#[cfg(test)]` code (§16 X2, R28(k)), whole or as the only
  members of a grouped `use crate::port::{..}`; a bare `crate::port` member of a grouped
  `use crate::{..}` is refused too. `crate::port::tree::…`, `crate::port::kind::…`, `FakeFileTree`
  outside test code, and every later port (`Workspace`, `Git`, `Binary`, `Adapter`) are refused
  there. Test code is, lexically: the brace-matched body of any item preceded by `#[cfg(test)]`
  (such as `#[cfg(test)] mod tests { … }`), and a whole file whose `mod` line in its parent carries
  `#[cfg(test)]` (such as `set/testing.rs`, declared `#[cfg(test)] mod testing;` in `set/mod.rs`).
- The offence line names the file, the line and the forbidden path, as today.

**Proof:** written first, seen failing, then passing.
- `port_never_names_the_disk_a_process_or_the_network` — every `.rs` file under `src/port` (at least
  one is found) has no forbidden `std` path.
- `app_never_names_the_disk_a_process_or_the_network` — the same over `src/app` (at least one file
  is found).
- `inside_never_names_an_adapter_module` — no `.rs` file under `src/types`, `src/port` or `src/app`
  names `crate::fs`, `crate::process` or `crate::host` in code.
- `types_names_only_the_tree_port` — no `.rs` file under `src/types` names a `crate::port` path
  other than `crate::port::FileTree` and `crate::port::EntryKind` in code, or
  `crate::port::FakeFileTree` in test code; the five test-only users of the fake after Task 1
  (`tree/walk/run.rs`, `tree/hash/tree.rs`, `extension/loaded.rs`, `pack/load.rs` in their
  `#[cfg(test)] mod tests`, and `set/testing.rs`) pass.
- `the_scan_flags_a_path_into_an_adapter_module` — `use crate::fs::StdFileTree;` is flagged as
  `crate::fs` on line 1; `crate::process::GitCli::new()` on line 2 of a two-line source is flagged
  as `crate::process` on line 2; `use crate::host::AirslAdapter;` is flagged as `crate::host`.
- `the_scan_flags_grouped_imports_of_an_adapter_module` — `use crate::{fs, types::Location};` is
  flagged as `crate::fs`; `use crate::{` / `types,` / `host::AirslAdapter,` / `};` on four lines is
  flagged as `crate::host` on line 3.
- `the_scan_does_not_flag_other_crate_paths` — `use crate::types::fs_like;`, `crate::fsx::a()`,
  `crate::hosts::b()` and `// use crate::fs` produce no offence.
- `the_port_scan_allows_only_the_tree_port` — `use crate::port::FileTree;`,
  `use crate::port::EntryKind;` and `use crate::port::{EntryKind, FileTree};` give no offence;
  `use crate::port::FakeFileTree;` is flagged as `crate::port::FakeFileTree`;
  `use crate::port::{FileTree, Workspace};` is flagged as `crate::port::Workspace`;
  `crate::port::tree::FileTree` is flagged as `crate::port::tree`.
- `the_port_scan_allows_the_fake_in_test_code_only` — `#[cfg(test)]` + `mod tests {` /
  `use crate::port::FakeFileTree;` / `}` gives no offence; the same `use` line outside any
  `#[cfg(test)]` item is flagged as `crate::port::FakeFileTree`; a file marked as declared
  `#[cfg(test)]` by its parent may name the fake (§16 R28(k)).

"Seen failing" for the directory scans: a temporary `use crate::fs::StdFileTree;` in
`src/app/plan.rs` makes `inside_never_names_an_adapter_module` fail, and a temporary
`use crate::port::FakeFileTree;` outside any test module in `src/types/op/mod.rs` makes
`types_names_only_the_tree_port` fail; both lines are removed before the commit.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `test(agentstacks): guard the inside against the outside and the adapters`.

### Task 4 — Declare airsl and serde_json under the deny policy

**Files:**
- Modify `Cargo.toml` — under `[workspace.dependencies]`: `airsl = "0.1.4"` and
  `serde_json = { version = "1.0.154", features = ["preserve_order"] }` (spec §15 rows 1-2; no
  `arbitrary_precision`, spec §5 "Dependency").
- Modify `crates/agentstacks/Cargo.toml` — under `[dependencies]`: `airsl = { workspace = true }`
  and `serde_json = { workspace = true }`. `tempfile` stays under `[dev-dependencies]` (spec §15
  row 5).
- Modify `deny.toml`:
  - under `[bans]`, `skip-tree = []` becomes, verbatim (spec §15 row 3, §16 X1):
    `skip-tree = [{ crate = "airsl@0.1.4", reason = "airsl 0.1.4 pins sha2 0.10 and toml 0.8; agentstacks uses sha2 0.11 and toml 1" }]`;
    the comment above `[bans]` keeps saying any duplicate fails unless acknowledged, and now names
    this one exemption;
  - `"Zlib"` joins `[licenses] allow`, in alphabetical place after `"Unlicense"` (spec §15 row 4:
    `preserve_order` pulls `foldhash 0.2.0`, `license = "Zlib"`, `probes/round3-results.md` D1,
    D2).
- `Cargo.lock` — written by cargo; it resolves `airsl 0.1.4`, the version the `skip-tree` entry
  names and `probes/deny-results.md` ran.

**Change:** The two crates later plans need are in the graph, and `cargo deny check` stays green.
Nothing uses the crates yet, and no lint the workspace enables refuses an unused dependency
(`Cargo.toml` `[workspace.lints]`).

**Proof:** written first, seen failing, then passing.
- `cargo make deny` after the dependency lines and before the `deny.toml` edit fails: duplicates
  `block-buffer`, `cpufeatures`, `crypto-common`, `digest`, `serde_spanned`, `sha2`, `toml`,
  `toml_datetime`, `winnow` (`bans FAILED`, `probes/deny-results.md` Run 1) and `foldhash` rejected
  with `license is not explicitly allowed` (probe D1).
- After the `deny.toml` edit, `cargo make deny` ends `advisories ok, bans ok, licenses ok, sources
  ok` (`probes/deny-results.md` Run 2).
- `cargo tree -p agentstacks -i foldhash -e features` shows `foldhash` reached through
  `serde_json feature "preserve_order"` (probe D2).

**Done when:** `cargo make dod` and `cargo make deny` pass and the Definition of Done is green.
Commit `build(repo): add airsl and serde_json under the deny policy`.

---

## Risks

- **Riskiest step: Task 1.** Removing the `types::FileTree` re-export breaks every path that named
  it. The 22-file table is the complete list from the spec's grep, and rows 1-4, 7, 9, 10, 13, 15
  and 18 are the only ones that name the moved items; a missed intra-doc link fails the doc step
  (`-D warnings`), not silently.
- `git mv` keeps history for the four `port` files; a copy-and-delete would lose it.
- The `crate::` scans are lexical, like the existing `std` scan. A relative path such as
  `super::super::fs` is not caught; the crate writes `crate::` paths throughout, and the final
  cross-check reads the inside for that form.
- The `skip-tree` entry names `airsl@0.1.4`. A later `cargo update` to a newer `0.1.x` (spec D4)
  makes it stop matching, and `cargo make deny` then fails until the entry is updated.
- Not done here: `docs/architecture.md`'s "State" column (owned by the docs plan) and any port
  beyond `FileTree` (the ports plan).

## Proof

- `cargo make dod` is green at the end of Task 4, and `cargo make deny` passes.
- `grep -rnE "types::port|types::FileTree|types::EntryKind|types::\{[^}]*(FileTree|EntryKind)" crates/agentstacks`
  prints nothing.
- `ls crates/agentstacks/src` shows exactly `app fs host lib.rs port process types`.
- Spec rows covered: §2 module tree and "removed:" line (Tasks 1-2), §2 dependency rule incl. X2,
  X3 as crate docs (Task 2), §2 isolation paragraph, §16 X2 and R28(k) (Task 3), §2 module table jobs as
  module docs (Task 2), §12 last row (Task 1), §15 rows 1-5 and §16 X1 (Task 4); the plan-set rule
  that no doc-comment example uses a fake (all tasks).
