---
status: done
created: 2026-11-23
built-on: 0f89fdb3ebb3
---

# Official packs Implementation Plan

**Goal:** Make the official pack list name exactly the five converted packs.

**Architecture:** `OFFICIAL_PACKS` in `crates/agentstacks/src/types/shorthand.rs:39-46` is the one
list the `init` shorthand reads. It loses `cmux`. The docs that list official packs follow it (spec
§5 second bullet, §12).

**Tech Stack:** Rust (`crates/agentstacks`), Markdown.

**Content authority:** spec §1 (`OFFICIAL_PACKS` row), §5 second bullet, §12 (official pack lists).

---

## Files that change

```
crates/agentstacks/src/types/shorthand.rs  — modify: OFFICIAL_PACKS is [&str; 5]; its test; a cmux test
docs/cli.md                                — modify: official pack list (:59)
README.md                                  — modify: drop the cmux row of the pack table (:102)
CLAUDE.md                                  — modify: planned layout pack list (:105)
```

## Order of work

### Task 1 — Drop cmux from the official packs

**Files:**
- Modify `crates/agentstacks/src/types/shorthand.rs`, `docs/cli.md`, `README.md`, `CLAUDE.md`.

**Change:**
- `OFFICIAL_PACKS: [&str; 5] = ["core", "sdlc", "journal", "guideline-rust", "guideline-python"]`.
- The list assertion in the test at `shorthand.rs:222-234` lists the same five.
- `docs/cli.md:59`: "Official packs: `core`, `sdlc`, `journal`, `guideline-rust`,
  `guideline-python`."
- `README.md:102`: the `cmux` row is removed from the pack table.
- `CLAUDE.md:105`: `packs/  official packs: core, sdlc, journal, guideline-rust, guideline-python`.
- `name/binary.rs:104` uses `"cmux"` as a sample binary name, not as a pack; it is not changed.

**Proof:**
- The existing list test passes with five names.
- `every_official_pack_name_is_accepted` still passes for each of the five.
- New `cmux_is_not_an_official_pack`: `--pack cmux` is refused with
  `--pack "cmux": not an official pack; pass a path, or add a git source to agentstacks.toml`
  (the existing refusal, `shorthand.rs:129,156`).
- `grep -rn cmux docs README.md CLAUDE.md crates/agentstacks/src/types/shorthand.rs` prints
  nothing.

**Done when:** `cargo make dod` is green. Commit
`feat(agentstacks): official packs are the five converted packs`.

---

## Risks

- A CLI test or fixture that runs `init --pack cmux` would now fail. The grep over `crates/` in Task 1
  (`grep -rn '"cmux"\|pack cmux' crates`) finds none besides `binary.rs:104`.

## Proof

- `cargo make dod` green.

## Review findings

- 🔵 fixed — `docs/cli.md:59` not reflowed after dropping `cmux`; "Official tools: `claude`." joined onto the line. Workspace `cargo make dod` exit 0 afterwards.
- Verification: `cargo test -p agentstacks --lib shorthand` 12 passed (incl. `every_official_pack_name_is_accepted`, `cmux_is_not_an_official_pack`); `cargo make dod-crate agentstacks` exit 0.

## Deviations

- 2026-11-23 — Proof "`grep -rn cmux docs README.md CLAUDE.md crates/agentstacks/src/types/shorthand.rs` prints nothing": it prints `shorthand.rs:236,238,239`, all inside the new `cmux_is_not_an_official_pack`, which must name `cmux`. docs, README.md and CLAUDE.md are clean. `grep -rn '"cmux"\|pack cmux' crates` prints only that test and `name/binary.rs:104`.
- 2026-11-23 — Not committed (execute does not commit unless told).
