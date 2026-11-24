---
status: approved
created: 2026-11-24
built-on: a72a2ee643bb
---

# User docs Implementation Plan

**Goal:** Tell users how to install the release and how to migrate from the claudestacks plugins.

**Architecture:** `README.md` is the front page: status, install, usage. `docs/migration.md` is a
new standalone guide for claudestacks plugin users, indexed from `docs/README.md`. Both state only
what spec §7.2 and §7.3 give, with every command taken from the spec, and cite no
`.claudestacks/` planning path.

**Tech Stack:** Markdown.

**Content authority:** spec §7.2, §7.3, §10 (rows: plugins and packs both active; README example
without `--pack core`; `cargo install` without airsl).

---

## Files that change

```
README.md           — modify: status, Install, Usage, Tools, Packs, Layout, Roadmap (spec §7.2)
docs/migration.md   — create: the claudestacks plugins → packs guide (spec §7.3)
docs/README.md      — modify: one table row for migration.md
```

## Order of work

### Task 1 — README

**Files:**
- Modify `README.md`.

**Change:** Every spec §7.2 bullet, in order:
- The status block becomes the spec's text exactly.
- A new `## Install` before `## Usage`: the install.sh one-liner, the line that it also installs
  airsl when missing, and `cargo install agentstacks-cli` with the note that this route does not
  install airsl, which `plan` needs (≥0.1.5).
- `## Usage` leads with the official-names block
  `agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust`, then `plan`,
  `apply`, `doctor`, `update`. The local-paths block is retitled "Local extensions and packs work
  too" with its commands unchanged. The `--tool opencode` example and the "Planned, once the
  official packs are released" heading go.
- The `agentstacks.toml` example under "`init` writes the manifest that the project commits"
  becomes what that `init` line writes: `api = 0`, `[tools.claude]`, then `[packs.core]`,
  `[packs.sdlc]`, `[packs.guideline-rust]`. Each table has
  `git = "https://github.com/airsstack/agentstacks"`, `tag = "packs-v0.1.0"` and its `subdir`
  (`extensions/claude`, `packs/core`, `packs/sdlc`, `packs/guideline-rust`), the shape spec P3
  shows `init` writing. Spec §10's last row requires `core` beside `sdlc` in every example (P32:
  without it the file is refused).
- Tools table: the Claude Code status becomes `Released (`extensions/claude`, `packs-v0.1.0`)`.
- Packs section: one line saying they are released as `packs-v0.1.0`.
- Layout tree: add `install/` (installer scripts, served on Pages) and `RELEASING.md`.
- Roadmap M1 line: `M1  claude extension (api = 0) ✓ released (0.1.0); the claudestacks plugins are packs`.

**Proof:** checks written first and seen failing:
- `status-exact` — the README's status block equals the spec §7.2 text.
- `install-section` — `## Install` comes before `## Usage` and holds the install.sh one-liner and
  the `cargo install agentstacks-cli` airsl note.
- `usage-has-core` — every `init` line in the README that names `--pack sdlc` also names
  `--pack core` (spec P32).
- `manifest-example-has-core` — the README's `toml` block has `[packs.core]` whenever it has
  `[packs.sdlc]`, and its four tables are exactly `tools.claude`, `packs.core`, `packs.sdlc`,
  `packs.guideline-rust`, each with `git`, `tag = "packs-v0.1.0"` and `subdir`.
- `no-planned-leftovers` — `grep -n -e 'opencode' -e 'Planned, once' README.md` finds no line in
  the Usage section. The Tools table's OpenCode `Planned` row stays.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(repo): README for the first release`.

### Task 2 — Migration guide

**Files:**
- Create `docs/migration.md` — the seven sections of spec §7.3, in order.
- Modify `docs/README.md` — one row: `| [migration.md](migration.md) | moving a project from the claudestacks marketplace plugins to the packs | built |`.

**Change:** The guide covers, in order:
1. The plugin → pack map, with cmux kept as a plugin.
2. Installing agentstacks.
3. Removing the plugins: per plugin with `claude plugin uninstall <plugin>@claudestacks --scope <scope>`
   (default user scope), or all at once with `claude plugin marketplace remove claudestacks`, which
   also removes cmux.
4. Setting up each project: `agentstacks init --tool claude --pack core --pack sdlc --pack journal --pack guideline-rust`
   (pick the packs you used; `sdlc` requires `core`), the plan review, the apply prompt, and
   committing `agentstacks.toml`, `agentstacks.lock` and `.claude/`. The section ends with the
   sentence "Packs install per project, never machine-wide."
5. What changed: every row of packs spec §9 except the last, rewritten for users. The command and
   agent renames come first as an old → new table, built from §9 row 1 and §6.1's renamed-items
   table. Then airsl ≥0.1.5 on `PATH`, reported by `plan` and `doctor`; `style_reinject` under
   `[packs.core.options]`; `/core-statusline`; the enforce hook finding guideline packs in the same
   project; `sdlc` requiring `core`; the enforce pointer naming `/guideline-rust-guidelines`; the
   enforce doctor's changes; the SessionEnd reminder shown as a skip; the SubagentHandback deny
   text.
6. What stays: `.claudestacks/sdlc/` paths, the journal vault at `$AIRSSTACK_HOME/journal`
   (default `~/.airsstack/journal`), the snapshot store at `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/`.
7. claudevs users: the claudevs.sh one-liner; `cargo install claudevs-cli` unchanged; the old
   claudestacks installer URL forwards here once that change lands.

The source of the rows is `.claudestacks/sdlc/2026-11-20-packs-and-claudevs/spec.md` §6.1 and §9;
the guide itself cites no `.claudestacks/` path.

**Proof:** checks written first and seen failing:
- `sections-in-order` — the guide's `##` headings are the seven topics in spec §7.3 order.
- `rename-coverage` — every old name in packs spec §9 row 1 and in the §6.1 renamed-items table
  appears in the guide's old → new table, with its new name.
- `init-has-core` — every `init` line naming `--pack sdlc` also names `--pack core`.
- `no-planning-paths` — `grep -n '\.claudestacks/sdlc/20' docs/migration.md` prints nothing.
  `.claudestacks/sdlc/` itself appears only in "What stays".
- `index-row` — `docs/README.md` has the migration.md row.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(repo): migration guide from the claudestacks plugins`.

---

## Risks

- The guide restates packs spec §9 for users. A row dropped or misread changes what a migrating
  user expects; `rename-coverage` covers the names, and the other rows are listed one by one in
  Task 2.
- README links to `docs/migration.md` and to the installer URLs, which serve 200 only after the
  author's §9 step 2. Until then the URLs in the docs answer 404.

## Proof

- Every check above passes; `cargo make dod` green.
