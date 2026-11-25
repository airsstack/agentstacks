---
status: approved
created: 2026-11-25
depends-on: [02, 03, 04, 05, 06]
built-on: c187d693fbea
---

# Documentation for 1.0 Implementation Plan

**Goal:** The book and repository docs describe agentstacks 1.0 as built.

**Architecture:** Four new book pages (pack authoring, pack format, stability, moving from 0.x) plus
edits to the pages and READMEs that name api 0 or the CLI surface. Every fact on a page is read
from the code as built by plans 01–06, never copied from the spec where the two could differ
(CLAUDE.md: a chapter describes what the code does). Links in pack READMEs stay absolute.

**Tech Stack:** Markdown, mdBook (`cargo make book-build`).

**Content authority:** spec §7, §9, D8, D15, D16, D18, E24, F14.

---

## Files that change

```
book/src/ch06-04-pack-authoring.md     — create: Writing a Pack, step by step for an agent
book/src/ch06-05-pack-format.md        — create: the pack format contract at api = 1
book/src/ch06-06-stability.md          — create: the 1.x promise (§7)
book/src/ch03-01-from-0x.md            — create: moving a project from 0.x
book/src/SUMMARY.md                    — modify: the four pages
book/src/ch04-00-cli.md                — modify: ext add, ext test, update's api move, D16
book/src/ch05-00-plan-and-apply.md     — modify: lock reads api 0; latest-tag major rule
book/src/ch06-00-authoring.md          — modify: api 1, [requires], ext test in step 6 and the checklist
book/src/ch06-01-extensions.md         — modify: api 1, [requires] in §2, stability link
book/src/ch06-02-hooks.md              — modify: api 1, stability link
book/src/ch06-03-opencode.md           — modify: api 1
book/src/introduction.md, book/src/ch01-02-first-project.md — modify: api 1
README.md                              — modify: api 1, status, ext commands, stability link
crates/agentstacks/README.md           — modify: stability link; D8 sentence
crates/agentstacks-cli/README.md       — modify: ext commands; stability link
CLAUDE.md                              — modify: status, invariant, CLI surface
```

## Order of work

### Task 1 — The stability page and the 0.x move page

**Files:**
- Create `book/src/ch06-06-stability.md` — §7 in full: covered, allowed additions, not allowed before api 2 (D15), the R1–R7 table, what authors do, not covered, 2.0
- Create `book/src/ch03-01-from-0x.md` — install 1.0 first; `agentstacks update` (its `api: 0 → 1` output); the §3.3 legacy messages for path and third-party sources and what to do; why a 0.x binary refuses `packs-v1.0.0` (E24); `doctor` is unaffected (D18)
- Modify `book/src/SUMMARY.md` — both pages

**Change:** The promise and the move are documented.

**Proof:**
- `cargo make book-build` prints no `[ERROR]` line.
- Every message quoted on `ch03-01` matches its string in the code (`grep` each one in `crates/`).

**Done when:** `cargo make dod` and `cargo make book-build` pass. Commit `docs(repo): the 1.x stability promise and moving from 0.x`.

### Task 2 — The pack pages

**Files:**
- Create `book/src/ch06-05-pack-format.md` — layout, `pack.toml` (every field and refusal), tokens and vars, frontmatter, hooks and events, options, `native/<tool>/`, `requires.agentstacks`, each refusal quoted from the code
- Create `book/src/ch06-04-pack-authoring.md` — in the style of `ch06-00`: a one-line hand-off for an agent, read first (ch06-05, ch06-02, a worked example `packs/guideline-rust` with an absolute URL), what you build, rules, steps (layout, `pack.toml`, skills, agents, hooks, options, native, `requires.agentstacks`, try it with `agentstacks init --tool claude --pack ./packs/<name> --yes` in a scratch project, then publish it in a git repository and add it to a project by hand with a `[packs.<name>]` table of `git`, `tag` and `subdir`, as D5 says, because `ext add` adds extensions only), a checklist, common refusals
- Modify `book/src/SUMMARY.md` — both pages under "Writing a Tool Extension"'s part

**Change:** A third party can write a pack from the book alone (intent outcome).

**Proof:**
- `cargo make book-build` prints no `[ERROR]` line.
- Every refusal quoted on `ch06-05` is found by `grep` in `crates/agentstacks/src`.

**Done when:** `cargo make dod` and `cargo make book-build` pass. Commit `docs(repo): writing a pack and the pack format`.

### Task 3 — The CLI and extension pages at 1.0

**Files:**
- Modify `book/src/ch04-00-cli.md` — §1 grammar and table gain `ext add` and `ext test`; §2 streams; new sections for both with their refusals (§5.4, §6.5) and output (§6.2); §3's tag rule names `packs-v1`; §7 `update` gains the api step and its re-read row
- Modify `book/src/ch05-00-plan-and-apply.md` — the lock reads api 0 (D10); §10 latest tag gains D16/D19
- Modify `book/src/ch06-00-authoring.md` — api 1 throughout; `[requires] agentstacks` in step 2; step 6 and the checklist run `agentstacks ext test ./ext/<tool>`; stability link
- Modify `book/src/ch06-01-extensions.md`, `ch06-02-hooks.md`, `ch06-03-opencode.md` — api 1; the "unstable until…" sentences become the stability link; ch06-01 §2 shows `[requires]`
- Modify `book/src/introduction.md`, `book/src/ch01-02-first-project.md` — api 1

**Change:** The book matches the code. ch06-00 changes with ch06-01, as CLAUDE.md requires.

**Proof:**
- `cargo make book-build` prints no `[ERROR]` line.
- The api-0 gate of plan 01 ("Kept lines") over `book/src` prints only the kept book lines listed there.

**Done when:** `cargo make dod` and `cargo make book-build` pass. Commit `docs(repo): the CLI and extension pages at api 1`.

### Task 4 — READMEs and CLAUDE.md

**Files:**
- Modify `README.md` — status says 1.0; `api = 1`; `ext add` and `ext test`; the stability link
- Modify `crates/agentstacks/README.md` — "unstable during 0.x" becomes the stability link; D8's sentence (the Rust API is the CLI's engine, not covered by the 1.x promise)
- Modify `crates/agentstacks-cli/README.md` — `ext add`, `ext test`; the stability link
- Modify `CLAUDE.md` — Status: 1.0, `api = 1` frozen; the invariant becomes "frozen as `api = 1`; 1.x changes are additive only (book ch06-06)"; the CLI surface line gains `ext add` and `ext test`

**Change:** The repository front pages say 1.0.

**Proof:**
- The api-0 gate over `README.md crates/*/README.md CLAUDE.md` prints only the `README.md` roadmap line `M1  claude extension (api = 0) ✓ released (0.1.0)`, a kept 0.x history line (plan 01, "Kept lines").

**Done when:** `cargo make dod` and `cargo make book-build` pass. Commit `docs(repo): READMEs and CLAUDE.md for 1.0`.

---

## Risks

- The pack-format page is the longest new text. Reading it from the spec instead of the code would bring back any spec/code gap. The proof greps every quoted refusal in `crates/`.
- `cargo make book-build` fails on a broken internal link. New pages are linked from `SUMMARY.md` in the same task that creates them.

## Proof

- `cargo make book-build` is green.
- The api-0 gate of plan 01 prints only kept lines.
