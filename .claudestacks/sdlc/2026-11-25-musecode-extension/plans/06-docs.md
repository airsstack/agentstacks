---
status: approved
created: 2026-11-25
depends-on: [01, 04, 05]
built-on: c1dbedb148db
---

# Muse Code Docs Implementation Plan

**Goal:** The book, READMEs, CLAUDE.md and changelogs describe the built musecode extension.

**Architecture:** Documentation only (spec §8.5). A new book page `book/src/ch06-04-musecode.md`
holds the extension's reference; the other pages and files change only where they list tools or
say "both".

**Tech Stack:** mdBook (`cargo make book-build`), Markdown.

**Content authority:** spec §4.1, §4.3, §4.4, §4.6, §4.7, §7, §8.5, §9. Spec §8.5 names
`book/src/ch06-00-authoring.md` "where they list the official extensions"; that chapter holds no
such list (only the claude adapter as its worked example), so it does not change.

---

## Files that change

```
book/src/ch06-04-musecode.md          — create: the Muse Code extension page
book/src/SUMMARY.md                   — modify: link the page after ch06-03
book/src/ch04-00-cli.md               — modify: musecode in the official tools (lines 59-61)
book/src/ch01-02-first-project.md     — modify: --tool musecode note (lines 15-18)
book/src/introduction.md              — modify: Muse Code named as built
book/src/ch06-01-extensions.md        — modify: "both official" (line 4) → all three
book/src/ch06-02-hooks.md             — modify: "both official" (line 4) → all three
book/src/ch06-03-opencode.md          — modify: §7 "Both tools" adds .agents/skills (line 151)
packs/journal/README.md               — modify: per-tool note gains Muse Code (line 23)
README.md                             — modify: tools table, layout tree, status lines, line 134
CLAUDE.md                             — modify: Status (line 5), layout, Invariants, Commands, CI job list
crates/agentstacks/CHANGELOG.md       — modify: musecode official tool
crates/agentstacks-cli/CHANGELOG.md   — modify: --tool musecode
packs/CHANGELOG.md                    — modify: the musecode extension
```

## Order of work

### Task 1 — The Muse Code book page

**Files:**
- Create `book/src/ch06-04-musecode.md`; modify `book/src/SUMMARY.md`.

**Change:** A page in the shape of `book/src/ch06-03-opencode.md`: checked against Muse Code
1.4.25-R9361.1; `agentstacks init --tool musecode --pack <p>`; the vars table (spec §4.1); the files
table (§4.3); the hook mapping (§4.4 events table) and how results reach the model (§4.7 table); the
skips F1-F5, H1-H8 and the hooks-file rule with their exact reasons (§4.2, §4.4, §4.5); the
not-carried-over table (§7); every limit of §9 and risk K5 (shell sandbox, unprobed); starting Muse
Code in the project root, trusting the workspace, and starting a new session after `apply`.
`SUMMARY.md` gains `  - [Muse Code](ch06-04-musecode.md)` after the OpenCode line.

**Proof:** `cargo make book-build` prints no `[ERROR]` line and the page appears in the built book's
table of contents; every skip reason on the page matches spec §4.2/§4.4/§4.5 character for character
(checked by grep of each reason against the page).

**Done when:** `cargo make book-build` is clean and the Definition of Done is green. Commit
`docs(repo): Muse Code extension book page`.

### Task 2 — Tool lists and "both" wording

**Files:**
- Modify `book/src/ch04-00-cli.md`, `book/src/ch01-02-first-project.md`, `book/src/introduction.md`,
  `book/src/ch06-01-extensions.md`, `book/src/ch06-02-hooks.md`,
  `book/src/ch06-03-opencode.md`, `packs/journal/README.md`, `README.md`, `CLAUDE.md`.

**Change:** Each file per spec §8.5: `musecode` beside `claude` and `opencode` wherever official tools
are listed, with the `--tool ext/musecode` note until the next `packs-v*` release, as for OpenCode;
"both official" → the three official extensions (`ch06-01:4`, `ch06-02:4`, `README.md:134`,
`CLAUDE.md:5`, the Invariants line, and the Planned layout line
`extensions/{claude,opencode}/ … both are built`, which becomes `extensions/{claude,musecode,opencode}/`
… all three built); `ch06-03` §7 "Both tools" adds that OpenCode also reads
`.agents/skills/` and the `.opencode` copy wins; `packs/journal/README.md:23` adds Muse Code;
`README.md` tools table row `| Muse Code | Built (extensions/musecode), ships with the next packs-v*
release |`, layout tree `musecode/ built`; `CLAUDE.md` Status, layout (`extensions/musecode/`),
Commands (`cargo make musecode-check # real Muse Code + a mock model; deliberately not part of
dod`) and the CI job list (`musecode-check`).

**Proof:** `grep -rn -i "both official\|both extensions\|both are built" book/src README.md CLAUDE.md`
finds no stale line; `grep -n musecode` finds each listed file; `cargo make book-build` clean.

**Done when:** those greps hold, `cargo make book-build` is clean and the Definition of Done is green.
Commit `docs(repo): name Muse Code among the official tools`.

### Task 3 — Changelogs

**Files:**
- Modify `crates/agentstacks/CHANGELOG.md`, `crates/agentstacks-cli/CHANGELOG.md`,
  `packs/CHANGELOG.md`.

**Change:** An unreleased entry in each, in the existing style: `musecode` is an official tool
(agentstacks); `--tool musecode` (agentstacks-cli); the musecode extension, Muse Code 1.4.25 (packs).

**Proof:** each file's unreleased section holds its entry (grep).

**Done when:** the greps hold and the Definition of Done is green. Commit
`docs(repo): changelog entries for the musecode extension`.

## Risks

- Line numbers in spec §8.5 may have moved by the time this plan runs; the coder finds each line by
  its quoted text.
- Org docs (`.github/profile/README.md`, workspace `README.md`) are outside this repository: not in
  this plan (spec §8.5).

## Proof

- `cargo make book-build` clean; `cargo make dod` green.
