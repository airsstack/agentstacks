---
status: approved
created: 2026-11-24
depends-on: [09]
built-on: d3463785d0cb
---

# Book Implementation Plan

**Goal:** Publish all agentstacks and claudevs documentation as one mdBook on Pages.

**Architecture:** `book/` in the shape of the org's other books (spec P38): `book.toml`, the two
mermaid scripts, `src/SUMMARY.md` and 40 chapters. The six `docs/` files move in with `git mv` and
only the spec §7.6 edits; the 22 claudevs files are copied from claudestacks `e099300` through the
§7.6 claudevs map; the five pack pages each include their pack's README; the rest is new. `docs/`
is removed. CI builds the book on every pull request and `pages.yml` publishes it with the two
installers beside it, both through the spec §7.7 build step, which also fails on mdBook's `[ERROR]`
lines. Depends on plan 09: `pages.yml` copies `install/install.sh` and `install/claudevs.sh`, and
both plans edit `ci.yml`.

**Tech Stack:** mdBook 0.4.52, mdbook-mermaid 0.16.2, Markdown, cargo-make, GitHub Actions YAML.

**Content authority:** spec §5.1 (`pages.yml`), §7.1 (`book` job), §7.3 (migration chapter), §7.6
(all of it), §7.7 (build step), §10 (book rows: missing chapter, missing include target, pack README
change, relative link in a pack README, broken internal link, old raw URL, Pages before its source
is set, README examples with `core`), §12 (book builds, book links, moves are moves), P38-P57.

---

## Files that change

```
docs/cli.md → book/src/ch04-00-cli.md                       — git mv + docs link map + title (spec §7.6)
docs/plan-and-apply.md → book/src/ch05-00-plan-and-apply.md — git mv + docs link map + title
docs/authoring.md → book/src/ch06-00-authoring.md           — git mv + docs link map + title + raw URLs
docs/extensions.md → book/src/ch06-01-extensions.md         — git mv + docs link map (#values) + title
docs/hooks.md → book/src/ch06-02-hooks.md                   — git mv + docs link map
docs/architecture.md → book/src/ch07-00-architecture.md     — git mv + docs link map + title
docs/README.md                                              — delete (its paragraph goes to CLAUDE.md in plan 12)
book/src/ch08-*.md … ch12-*.md, appendix-01-claudevs-cli.md — create: 22 files from claudestacks e099300, claudevs map
book/src/ch02-01-core.md … ch02-05-guideline-python.md      — create: one {{#include}} line each
packs/{core,sdlc,journal,guideline-rust,guideline-python}/README.md — modify: relative links → absolute
book/src/introduction.md, ch01-00, ch01-01, ch01-02, ch02-00, appendix-00 — create: new chapters (spec §7.6)
book/src/ch03-00-migration.md                               — create: the migration guide (spec §7.3)
book/book.toml                                              — create: spec §7.6 block, exactly
book/mermaid.min.js, book/mermaid-init.js                   — create: `mdbook-mermaid install book` (0.16.2)
book/src/SUMMARY.md                                         — create: spec §7.6 block, exactly
.gitignore                                                  — modify: + book/book/ with its comment
Makefile.toml                                               — modify: + book-mdbook, book-mermaid, book, book-build
.github/workflows/ci.yml                                    — modify: + the `book` job (spec §7.1, §7.7)
.github/workflows/pages.yml                                 — create: book + installers (spec §5.1, §7.7)
```

## Order of work

### Task 1 — Move `docs/` into the book

**Files:**
- `git mv` each of the six `docs/` files to its spec §7.6 name (table above).
- Modify each moved file — the spec §7.6 docs link map in one pass (the seven replacements), and
  the title line: line 1 becomes its `SUMMARY.md` entry (`# The Command Line`,
  `# Plan, Apply and the Lock`, `# Writing a Tool Extension`, `# The Extension API`,
  `# Under the Hood`; `# Hooks` stays). In `ch06-00-authoring.md` only, the three raw URLs of spec
  §7.6 change to their `…/main/book/src/…` form, with the link texts `ch06-01-extensions.md` and
  `ch06-02-hooks.md`.
- Delete `docs/README.md` (`git rm`).

**Change:** The reference docs become chapters with history kept, their cross-links pointing at
the new names, and the one broken anchor fixed (`#missing-values` → `#values`, spec P40). No other
byte changes.

**Proof:** checks written first and seen failing (the files are still in `docs/`):
- `docs-gone` — `test ! -e docs` succeeds.
- `moves-are-renames` — `git diff --cached -M --stat` shows the six files as renames from `docs/`
  to `book/src/`.
- `only-map-edits` — for each moved file, `git show HEAD:docs/<old>.md` piped through the spec §7.6
  docs map, the title line and (authoring) the raw URLs, written as one `sed` script, equals the
  new file (`cmp`).
- `no-old-links` — `grep -n -E '\]\((cli|plan-and-apply|authoring|extensions|hooks|architecture)\.md' book/src/ch0[4-7]-*.md`
  and `grep -n 'missing-values\|main/docs/' book/src/ch0[4-7]-*.md` print nothing.
- `titles` — `head -1` of the six files prints exactly the six titles above, in table order.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(repo): move docs/ into the book`.

### Task 2 — Move the claudevs chapters

**Files:**
- Create the 22 files of spec §7.6 (`ch08-*` … `ch12-*`, `appendix-01-claudevs-cli.md`): each is
  `git -C /Users/hiraq/Projects/airsstack/claudestacks show e099300:book/src/<old name>` piped
  through the spec §7.6 claudevs map: the full-name rename map applied in one pass (so a renamed
  name is never renamed again), then the three replacements (`…/claudestacks/install.sh` →
  `…/agentstacks/claudevs.sh`, `https://github.com/airsstack/claudestacks/` →
  `https://github.com/airsstack/agentstacks/`, the output line `claudevs 0.1.1` →
  `claudevs 0.1.2`). File names follow the same rename rule (`ch05-` → `ch08-`, `ch06-` →
  `ch09-`, `ch07-` → `ch10-`, `ch08-` → `ch11-`, `ch09-` → `ch12-`).

**Change:** claudevs' documentation lives beside its code, pointing at this repository's examples
and installer.

**Proof:** checks written first and seen failing (the files are absent):
- `moved-identically` — the spec §12 "moves are moves" method: each claudestacks `e099300` chapter
  through the map `cmp`-equals its new file; 22 identical, 0 differ (spec P57).
- `no-claudestacks` — `cat book/src/ch08-*.md book/src/ch09-*.md book/src/ch10-*.md book/src/ch11-*.md book/src/ch12-*.md book/src/appendix-01-claudevs-cli.md | grep -c -i claudestacks`
  prints 0 (spec P46).
- `examples-exist` — every `https://github.com/airsstack/agentstacks/(tree|blob)/main/<path>` in
  those files names a path that exists in this repository.
- `version-line` — `grep -n '^claudevs 0\.1\.' book/src/ch08-01-installation.md` prints only
  `claudevs 0.1.2`.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(repo): claudevs chapters move into the agentstacks book`.

### Task 3 — Pack pages and pack README links

**Files:**
- Create `book/src/ch02-01-core.md`, `ch02-02-sdlc.md`, `ch02-03-journal.md`,
  `ch02-04-guideline-rust.md`, `ch02-05-guideline-python.md` — each exactly one line,
  `{{#include ../../packs/<pack>/README.md}}`, and a newline.
- Modify the five `packs/*/README.md` — the spec §7.6 pack README link rule: `../../LICENSE` →
  `https://github.com/airsstack/agentstacks/blob/main/LICENSE`; a directory link (ending `/`) →
  `https://github.com/airsstack/agentstacks/tree/main/packs/<pack>/<path>`; a file link →
  `https://github.com/airsstack/agentstacks/blob/main/packs/<pack>/<path>`. Absolute links stay.

**Change:** Each pack has one page whose text is its README, with links that work both on GitHub
and on the site. No pack installs its README and the pack text gates skip it (spec P47), so
nothing installed changes.

**Proof:** checks written first and seen failing:
- `include-lines` — each of the five pages is exactly its include line (`cmp` against
  `printf '{{#include ../../packs/%s/README.md}}\n' <pack>`).
- `readme-links` — `grep -on '\]([^)]*)' packs/*/README.md` from the repository root equals
  `probes/book-results.md:12-27` with each line's `packs/` path unchanged (16 links, 12 of them
  rewritten; spec §7.6).
- `no-relative-readme-links` — that grep piped through `grep -v '](https://'` prints nothing.
- `pack-gates` — `cargo make lua` exits 0.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(packs): absolute README links for the book's pack pages`.

### Task 4 — New chapters

**Files:**
- Create `book/src/introduction.md`, `ch01-00-getting-started.md`, `ch01-01-installation.md`,
  `ch01-02-first-project.md`, `ch02-00-packs.md`, `appendix-00.md` — each with the content its
  spec §7.6 "New chapters" bullet lists, in that order, under a `#` title equal to its
  `SUMMARY.md` entry.
- Create `book/src/ch03-00-migration.md` — title `# Moving from the claudestacks Plugins`, then
  the seven spec §7.3 `##` sections in order:
  1. `## What maps to what`: the plugin → pack map (spec §7.3 item 1), with cmux kept as a plugin.
  2. `## Install agentstacks`: the install.sh one-liner, linking [Installation](ch01-01-installation.md).
  3. `## Remove the plugins`: `claude plugin uninstall <plugin>@claudestacks --scope <scope>` per
     plugin (default user scope), or `claude plugin marketplace remove claudestacks`, which also
     removes cmux (spec P27).
  4. `## Set up each project`:
     `agentstacks init --tool claude --pack core --pack sdlc --pack journal --pack guideline-rust`
     (pick the packs you used; `sdlc` requires `core`), the plan review, the apply prompt, then
     committing `agentstacks.toml`, `agentstacks.lock` and `.claude/`. The section ends with the
     sentence "Packs install per project, never machine-wide."
  5. `## What changed`: every row of packs spec §9 except the last, for users. The command and agent
     renames come first as an old → new table built from §9 row 1 and §6.1's renamed-items table
     (source: `.claudestacks/sdlc/2026-11-20-packs-and-claudevs/spec.md`). Then airsl ≥0.1.5 on
     `PATH`, reported by `plan` and `doctor`; `style_reinject` under `[packs.core.options]`;
     `/core-statusline`; the enforce hook finding guideline packs in the same project; `sdlc`
     requiring `core`; the enforce pointer naming `/guideline-rust-guidelines`; the enforce
     doctor's changes; the SessionEnd reminder shown as a skip; the SubagentHandback deny text.
  6. `## What stays`: `.claudestacks/sdlc/` paths, the journal vault at `$AIRSSTACK_HOME/journal`
     (default `~/.airsstack/journal`), the snapshot store at
     `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/` (spec P29).
  7. `## claudevs users`: `curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh`;
     `cargo install claudevs-cli` unchanged; the old claudestacks installer URL forwards here once
     that change lands; a link to [its installation](ch08-01-installation.md).

**Change:** A user can install, set up a first project, pick packs and migrate from the plugins
from the book alone. Every command and output line shown is a spec text with its proof; nothing
else is shown as output.

**Proof:** checks written first and seen failing (the files are absent):
- `titles` — `head -1` of each of the seven files equals its `SUMMARY.md` entry as `# <entry>`.
- `init-has-core` — every line in `book/src/` naming `--pack sdlc` also names `--pack core`
  (spec P32).
- `first-project-output` — `ch01-02-first-project.md` holds these lines verbatim, each from spec
  P48: `76 to write, 0 to delete, 0 unchanged, 1 skipped`; `applied: 76 written, 0 deleted`;
  `stdin is not a terminal; pass --yes to apply`;
  `agentstacks.toml written; nothing applied (run apply)`;
  `0 to write, 0 to delete, 76 unchanged, 1 skipped`; `healthy`; `nothing to update`; the skip line
  `skip     core/hook/3: claude shows no hook output to the model at session end`; and the
  `agentstacks.toml` block equal to `probes/book-results.md:236-256`'s body.
- `installation-texts` — `ch01-01-installation.md` holds the two spec §5.3 usage lines, the M8 and
  M9 texts of spec §5.5 verbatim, `cargo install agentstacks-cli`, the airsl note and
  `agentstacks 0.1.0`.
- `packs-table` — `ch02-00-packs.md` names all five packs with the spec §7.6 needs (airsl ≥0.1.5
  for `core`, `journal`, `sdlc`; `core` for `sdlc`; nothing for the guideline packs) and quotes
  the spec P32 refusal exactly.
- `migration-sections` — the `##` headings of `ch03-00-migration.md` are exactly the seven above,
  in order (the spec §7.3 names); the check below reads "What stays" by that heading.
- `rename-coverage` — every old name in packs spec §9 row 1 and in the §6.1 renamed-items table
  appears in the migration chapter's old → new table with its new name.
- `no-planning-paths` — `grep -rn '\.claudestacks/sdlc/20' book/src/` prints nothing, and
  `.claudestacks/sdlc/` appears in `ch03-00-migration.md` only in "What stays".

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(repo): the book's new chapters: install, first project, packs, migration`.

### Task 5 — Book configuration and build

**Files:**
- Create `book/book.toml` — the spec §7.6 `toml` block (the only fence indented two spaces with
  language `toml`), with its two-space indent removed, exactly.
- Run `mdbook-mermaid install book` (mdbook-mermaid 0.16.2) and commit the `book/mermaid.min.js`
  and `book/mermaid-init.js` it writes.
- Create `book/src/SUMMARY.md` — the spec §7.6 `markdown` block (the only two-space-indented
  `markdown` fence), indent removed, exactly.
- Modify `.gitignore` — append a blank line, `# The book's build output. Pages builds its own
  copy; nothing built is committed.` and `book/book/`.
- Modify `Makefile.toml` — add the comment block and the four tasks of
  `/Users/hiraq/Projects/airsstack/claudestacks/Makefile.toml:529-584` verbatim, in a "Book"
  section, and its two usage lines (`claudestacks/Makefile.toml:7-8`) in this file's header
  comment. `dod` does not change.

**Change:** `cargo make book` serves the book locally and `cargo make book-build` builds it into
the ignored `book/book/`.

**Proof:** checks written first and seen failing (no `book.toml` yet):
- `config-exact` — `book.toml` and `SUMMARY.md` `cmp`-equal their spec blocks (indent stripped),
  and still do after `mdbook-mermaid install book` (spec P53).
- `src-count` — `ls book/src | wc -l` prints 41 (spec P39).
- `build-step` — the spec §7.7 `bash` block (the first column-0 `bash` fence), extracted and run
  under `bash -e` from the repository root with `RUNNER_TEMP="$(mktemp -d)"`, exits 0.
- `links-resolve` — the spec §12 "Book links" method over `book/book/`: every `href` that is not
  `http:`, `https:`, `mailto:` or `//`, on every page but `print.html`, `toc.html` and `404.html`,
  names an existing file or directory and, with a `#`, an `id` in that file; 0 broken.
- `mermaid-renders` — the built chapters hold 5 `class="mermaid"` blocks (spec P39).
- `edge-missing-chapter` — with a `- [Gone](gone.md)` line appended to a copy of the book's
  `SUMMARY.md` in a temp copy of `book/`, the §7.7 step exits 1 (spec P41, P43).
- `edge-missing-include` — in a temp copy where one pack page includes
  `../../packs/gone/README.md`, the §7.7 step exits 1 and prints
  `mdbook build book logged an error; see the [ERROR] lines above` (spec P42, P43).
- `ignored` — after `cargo make book-build`, `git status --porcelain book/book` prints nothing.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(repo): the agentstacks book`.

### Task 6 — CI book job and the Pages workflow

**Files:**
- Modify `.github/workflows/ci.yml` — add the job `book`: claudestacks' `book` job
  (`/Users/hiraq/Projects/airsstack/claudestacks/.github/workflows/ci.yml:144-162`: name `Book`,
  `ubuntu-latest`, `timeout-minutes: 10`, its comments, checkout,
  `taiki-e/install-action@v2` with `tool: mdbook@0.4.52,mdbook-mermaid@0.16.2`), with its
  `mdbook build book` step replaced by a step named `Build the book` whose `run:` is the spec §7.7
  block. No other job changes.
- Create `.github/workflows/pages.yml` — spec §5.1: claudestacks' `pages.yml` shape; triggers
  `workflow_dispatch` and push to `main` with paths `book/**`, `install/**`, `packs/*/README.md`,
  `.github/workflows/pages.yml`; `permissions: contents: read`; concurrency group `pages`,
  `cancel-in-progress: false`; build job (`ubuntu-latest`, `timeout-minutes: 10`): checkout with
  `persist-credentials: false`, the install-action step, `Build the book` (spec §7.7),
  `cp install/install.sh install/claudevs.sh book/book/`, `actions/upload-pages-artifact@v5` with
  `path: book/book`; deploy job unchanged from claudestacks (`needs: build`,
  `actions/deploy-pages@v5`, `pages: write`, `id-token: write`, environment `github-pages`). The
  header comment is claudestacks', adapted to name the book, `install.sh` (agentstacks plus airsl)
  and `claudevs.sh`, and says the Pages source must be "GitHub Actions".

**Change:** A pull request that breaks the book fails CI, and a merge publishes the book with both
installers beside it.

**Proof:** checks written first and seen failing:
- `book-job-local` — the `Build the book` step's `run:` body, extracted from `ci.yml` and from
  `pages.yml`, is byte-equal to the spec §7.7 block in both files.
- `ci-other-jobs-unchanged` — written first as a script over `git diff .github/workflows/ci.yml`
  that fails unless the diff is non-empty, has no removed line, and adds only lines inside the
  `book` job; seen failing on the untouched file (empty diff), then passing.
- `pages-site-contents` — the build job's steps after checkout, run locally in a temp copy of the
  repository with `RUNNER_TEMP="$(mktemp -d)"` (build step, then the `cp`), leave `book/book/` holding `index.html`, `install.sh`
  and `claudevs.sh`, with the two scripts `cmp`-equal to `install/`.
- `pages-triggers` — the `paths:` list of `pages.yml` is exactly the four spec §5.1 entries.
- `pins-agree` — `mdbook@0.4.52,mdbook-mermaid@0.16.2` appears in `ci.yml` and `pages.yml`, and
  `0.4.52` and `0.16.2` in the `Makefile.toml` book tasks.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`ci(repo): build the book in CI and publish it on Pages`.

---

## Risks

- Task 1 moves files that `README.md` and `CLAUDE.md` link to. Those links stay broken until
  plans 11 and 12 land; all three plans ship in the release pull request.
- Task 2 reads claudestacks at `e099300`. If claudestacks has moved on, the commit is still read
  by its hash; any later claudevs chapter change there is not carried (the claudevs part freezes
  at 0.1.1 per spec §8.2).
- The link check runs once, here. A later chapter edit can break a link with CI green (spec §10,
  §13).
- The Pages upload and deploy run only on GitHub. The local runs cover the build
  step and the copy, not `upload-pages-artifact` or `deploy-pages`.

## Proof

- The §7.7 build step exits 0; every relative href resolves; `book/src/` holds 41 files.
- `cargo make dod` green.
