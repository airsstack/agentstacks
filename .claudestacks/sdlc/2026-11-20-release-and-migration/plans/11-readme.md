---
status: approved
created: 2026-11-24
depends-on: [10, 12]
built-on: d3463785d0cb
---

# README Implementation Plan

**Goal:** Make `README.md` the front page of the first release, pointing into the book.

**Architecture:** `README.md` keeps status, install and usage; everything longer lives in the book,
linked by its Pages URL. It states only what spec §7.2 gives, names no `docs/` path and cites no
`.claudestacks/` planning path. Depends on plan 10: the README's raw link names
`book/src/ch06-00-authoring.md`, which plan 10 creates, and its layout tree lists `RELEASING.md`,
which plan 12 creates. Supersedes plan 04's README task; the
migration guide is now the book chapter of plan 10.

**Tech Stack:** Markdown.

**Content authority:** spec §7.2, §10 (rows: plugins and packs both active; README example
without `--pack core`; `cargo install` without airsl; old raw URL), P44, P48.

---

## Files that change

```
README.md   — modify: status, Install, Usage, Tools, Extending, Packs, Layout, Documentation, Roadmap (spec §7.2)
```

## Order of work

### Task 1 — README

**Files:**
- Modify `README.md`.

**Change:** Every spec §7.2 bullet, in order:
- The status block becomes the spec's text exactly, its migration link
  `https://airsstack.github.io/agentstacks/ch03-00-migration.html`.
- A new `## Install` before `## Usage`: the install.sh one-liner, the line that it also installs
  airsl when missing, and `cargo install agentstacks-cli` with the note that this route does not
  install airsl, which `plan` needs (≥0.1.5).
- `## Usage` leads with the official-names block
  `agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust`, then `plan`,
  `apply`, `doctor`, `update`. The local-paths block is retitled "Local extensions and packs work
  too" with its commands unchanged. The `--tool opencode` example and the "Planned, once the
  official packs are released" heading go.
- The `agentstacks.toml` example under "`init` writes the manifest that the project commits"
  (not a §7.2 bullet: the spec §10 row "README example without `--pack core`" with P48)
  becomes its existing first line `# agentstacks.toml` followed by exactly what that `init` line
  writes: the `agentstacks.toml` of spec P48 (`probes/book-results.md:236-256`, unaligned `=`),
  whose tables are `[tools.claude]`, `[packs.core]`,
  `[packs.guideline-rust]`, `[packs.sdlc]` in that order, each with
  `git = "https://github.com/airsstack/agentstacks"`, `tag = "packs-v0.1.0"` and its `subdir`.
- Tools table: the Claude Code status becomes `Released (`extensions/claude`, `packs-v0.1.0`)`;
  the "Your own" row (line 86) becomes the spec §7.2 text with the
  `https://airsstack.github.io/agentstacks/ch06-00-authoring.html` link.
- "Extending agentstacks" (lines 115-119): the guide link becomes the spec §7.2 book link, and the
  raw link line becomes exactly
  `Follow https://raw.githubusercontent.com/airsstack/agentstacks/main/book/src/ch06-00-authoring.md`.
- Packs section: one line saying they are released as `packs-v0.1.0`.
- Layout tree: add `install/` (installer scripts, served on Pages) and `RELEASING.md`; remove the
  `docs/` line; the `book/` line becomes the spec §7.2 line in the tree's column layout.
- `## Documentation` becomes the spec §7.2 paragraph, exactly, with `book/src/` in backticks.
- Roadmap M1 line: `M1  claude extension (api = 0) ✓ released (0.1.0); the claudestacks plugins are packs`.

**Proof:** checks written first and seen failing:
- `status-exact` — the README's status block equals the spec §7.2 text.
- `install-section` — `## Install` comes before `## Usage` and holds the install.sh one-liner and
  the `cargo install agentstacks-cli` airsl note.
- `usage-has-core` — every `init` line in the README that names `--pack sdlc` also names
  `--pack core` (spec P32).
- `manifest-example-exact` — the README's `toml` block without its first line
  (`# agentstacks.toml`) equals the spec P48 `agentstacks.toml` (`probes/book-results.md:236-256`),
  `cmp`.
- `no-planned-leftovers` — `grep -n -e 'opencode' -e 'Planned, once' README.md` finds no line in
  the Usage section. The Tools table's OpenCode `Planned` row stays.
- `no-docs-paths` — `grep -n 'docs/' README.md` prints no line naming this repository's `docs/`
  (spec P44: the hits today are lines 86, 115, 119, 144, 150-151).
- `raw-link-target` — the file the raw link names, `book/src/ch06-00-authoring.md`, exists, and
  every `https://airsstack.github.io/agentstacks/<file>.html` link in the README names a chapter
  of `book/src/SUMMARY.md`.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(repo): README for the first release`.

---

## Risks

- The README links to Pages URLs, which serve 200 only after the author's spec §9 step 2. Until
  then they answer 404.
- The `agentstacks.toml` example reflects `packs-v0.1.0`; a later pack release does not change
  it, since the tag is pinned in the example.

## Proof

- Every check above passes; `cargo make dod` green.
