---
status: approved
created: 2026-11-24
built-on: 97ea787d2a1e
---

# Spec: First release and migration

This spec ships agentstacks to users who have no Rust toolchain. It covers the first release of the
four crates on crates.io, prebuilt `agentstacks` and `claudevs` binaries through dist, the
`packs-v0.1.0` tag that makes `init --tool claude --pack <p>` resolve official names, installers
that also install `airsl` when it is missing, a migration guide from the claudestacks marketplace
plugins, an mdBook that holds all agentstacks and claudevs documentation and is published on Pages
beside the installers, the README and org-doc status changes, and the archive of claudestacks
after all of that is out. `docs/` moves into the book and is removed. No Rust module is added,
moved or changed. The work is versions, release configuration, installer scripts, the book and
docs.

This spec replaces `spec-superseded-2026-11-24.md`, which had no book. Everything except the book
and the links it moves is carried over unchanged.

Probe outputs live in `probes/` next to this file, and the scripts that produced them in
`probes/scripts/`. Every probe ran on 2026-11-24 against worktree HEAD `0fe1f61`, cargo 1.94.1,
dist (`cargo-dist`) 0.33.1 and jq 1.7.1. The book probes (P38-P57) ran against HEAD `e4e1acd`
(which changed only `.claudestacks/`), mdBook 0.4.52, mdbook-mermaid 0.16.2, airsl 0.1.5 and
claudestacks HEAD `e099300`.

## 1. Premises (proven)

| # | Fact | Proof |
|---|---|---|
| P1 | `airsstack/agentstacks` is public: an anonymous `git ls-remote --tags` succeeds and lists no tags | `probes/init-results.md` §3 (`exit=0`, empty) |
| P2 | Today `init --tool claude --pack core` refuses with `--pack "core": no packs-v release tag in https://github.com/airsstack/agentstacks` and `--tool "claude": …`, exit 1 | `probes/init-results.md:7-9` |
| P3 | With a `packs-v0.1.0` tag on the official repository, `init --tool claude --pack core --pack guideline-rust` writes `[tools.claude]`/`[packs.*]` with `git = "https://github.com/airsstack/agentstacks"`, `tag = "packs-v0.1.0"` and the `subdir`, and `plan` exits 0. Proven by redirecting the official URL to a bare copy of HEAD with `insteadOf` | `probes/init-results.md` §2 |
| P4 | `plan` with no `airsl` on `PATH` prints `agentstacks.toml: airsl not found on PATH (needed by core)` and exits 1; with an airsl that reports 0.1.4 it prints `agentstacks.toml: airsl 0.1.4 installed, packs need >=0.1.5` and exits 1 | `probes/airsl-results.md:6-11` |
| P5 | `core`, `journal` and `sdlc` require `airsl = ">=0.1.5"`; `guideline-rust` and `guideline-python` do not | `probes/docs-excerpts.md` (last block) |
| P6 | claudevs never runs the `airsl` binary: there is no `"airsl"` in `crates/claudevs{,-cli}/src` (grep exit 1), while the same grep finds `const AIRSL: &str = "airsl";` in `crates/agentstacks/src/process/binary.rs:26` | `probes/airsl-results.md:18-27` |
| P7 | crates.io has no `agentstacks` or `agentstacks-cli`; it has `claudevs` 0.1.0 and `claudevs-cli` 0.1.0, 0.1.1 | `probes/publish-results.md` (crates.io state) |
| P8 | `cargo publish --workspace --dry-run --allow-dirty` with agentstacks 0.1.0, agentstacks-cli 0.1.0, claudevs 0.1.1, claudevs-cli 0.1.2 and `publish = true` packages all four crates in dependency order and exits 0, with no warning except `aborting upload due to dry run` | `probes/publish-results.md`, `probes/spec-examples-results.md` §2 (both filtered to the lines named in their headers) |
| P9 | dist's plan step fails on a `packs-v0.1.0` tag: `unexpected character 'p' while parsing major version number` | `probes/dist-lockstep-results.md:68` |
| P10 | dist's default tag trigger is `'**[0-9]+.[0-9]+.[0-9]+*'`, which matches `packs-v0.1.0`. In GitHub's filter syntax `**` matches any characters, `+` one or more of the preceding character, `[]` one character of a range | `probes/dist-lockstep-results.md:78`; `probes/workflow-syntax-filter.md:6-9` |
| P11 | With the trigger edited to `agentstacks-cli-v…` and `claudevs-cli-v…`, `dist plan --tag=agentstacks-cli-v0.1.0` exits 255 (`help: run 'dist init' to update the file ('allow-dirty' in Cargo.toml to ignore out of date contents)`). With `allow-dirty = ["ci"]` it exits 0 and releases only `agentstacks-cli` 0.1.0 with `agentstacks-cli-installer.sh`. `claudevs-cli-v0.1.2` releases only `claudevs-cli` 0.1.2. `dist generate` then keeps the edited trigger | `probes/dist-results.md:36-64`; `probes/spec-examples-results.md` §1 (the §4 texts) |
| P12 | With per-binary versions, an untagged `dist plan` fails under `pr-run-mode = "upload"` (`There are too many unrelated apps in your workspace to coherently Announce!`, exit 255) and passes under `"plan"` (exit 0) | `probes/dist-pr-results.md:7-20` |
| P13 | Under `pr-run-mode = "plan"` the build job runs only when publishing or in upload mode. `custom-smoke` needs `build-local-artifacts`, and GitHub skips a job whose needed job was skipped. So a pull request runs only the plan job. `host` needs `custom-smoke` to succeed or be skipped | `probes/dist-pr-results.md` (plan-mode section: generated line 96 `if:`, `custom-smoke` `needs`, line 239 `host` `if:`) and `probes/dist-pr-results.md:68-72` (GitHub's `needs` rule) |
| P14 | The six targets build on `macos-15` (aarch64-apple-darwin), `macos-15-intel`, `ubuntu-22.04` and `ubuntu-22.04-arm`, and each release has a `<app>-installer.sh` | `probes/dist-lockstep-results.md` (runner lines); `probes/dist-results.md:51,54` |
| P15 | GitHub's latest release is the most recent non-prerelease, non-draft release of the whole repository, and new releases default to latest | `probes/rest-releases.md:196-197,379` |
| P16 | GitHub creates no push event for tags when more than three tags are pushed at once | `probes/webhook-events-tags.md:15` |
| P17 | Unauthenticated GitHub REST requests are limited to 60 an hour | `probes/rate-limits.md:17` |
| P18 | This POSIX sh pipeline finds the newest stable `<app>-vX.Y.Z` release from `GET /repos/{o}/{r}/releases?per_page=100` whatever the API's order: `0.1.1` for claudestacks `claudevs-cli` (mixed with airsclaude tags), `0.1.5` for airsl `airsl-cli`, empty for agentstacks today; the sort orders `0.1.10` below `0.2.0` | `probes/latest-results.md` (script lines 6-12, output lines 30-45) |
| P19 | Archiving makes a repository read-only for all users. Archived repositories keep serving the Pages site they already had (`microsoft/autogen`: archived, `build_type` workflow, 200). claudestacks' Pages is workflow-built | `probes/archiving.md:3,17`; `probes/pages-results.md` |
| P20 | `https://airsstack.github.io/airsl/install.sh` and `…/claudestacks/install.sh` serve 200; `…/agentstacks/install.sh` serves 404 and agentstacks has no Pages site (API 404) | `probes/pages-results.md` |
| P21 | airsl's Pages deploys from an Actions artifact and needs the repository's Pages source set to "GitHub Actions" | `/Users/hiraq/Projects/airsstack/airsl/.github/workflows/pages.yml:6-9` |
| P22 | airsl's and claudestacks' pointer scripts are POSIX sh with `say`/`fail`/`download` helpers and `main "$@"` as the last line; `sh -n` passes both | `/Users/hiraq/Projects/airsstack/airsl/install/install.sh`; `/Users/hiraq/Projects/airsstack/claudestacks/install/install.sh:44-74`; `probes/misc-results.md` |
| P23 | `sh -n` reads commands without executing them (POSIX). The macOS `sh -n` exits 2 on a script with an unclosed `if` and 0 on a good one | `probes/docs-excerpts.md` (POSIX block); `probes/misc-results.md` |
| P24 | crates.io keywords: at most 5, ASCII, at most 20 characters, starting alphanumeric, only letters, numbers, `_`, `-`, `+`. The category slugs `command-line-utilities`, `development-tools` and `development-tools::testing` exist | `probes/docs-excerpts.md`; `probes/misc-results.md` |
| P25 | The agentstacks smoke steps need no tool on `PATH` (no git, no airsl). With `PATH=/nonexistent`, `init --tool ext/claude --pack packs/guideline-rust --yes` exits 0 and prints `applied: 11 written, 0 deleted`; `plan` prints `0 to write, 0 to delete, 11 unchanged, 0 skipped` and exits 0; `doctor` prints `healthy` and exits 0 | `probes/smoke-results.md`, script `probes/scripts/smoke2-probe.sh` |
| P26 | `claudevs --version` prints `claudevs <version>`, and `agentstacks --version` prints `agentstacks <version>` (`claudevs 0.1.1`, `agentstacks 0.0.0` today). The claudevs smoke fixtures from claudestacks exist under `crates/claudevs/tests/fixtures/` | `probes/smoke-results.md`; `probes/init-results.md` (`--version`); `probes/paths-results.md` (fixtures) |
| P27 | Claude Code: `claude plugin uninstall <plugin>@<marketplace> --scope <scope>` uninstalls one plugin (uninstall defaults to user scope); `claude plugin marketplace remove <name>` uninstalls every plugin installed from that marketplace and removes their `enabledPlugins` entries | `probes/discover-plugins.md:328-338,380-382` |
| P28 | The claudestacks marketplace is named `claudestacks` and lists `claudestacks`, `claudestacks-sdlc`, `claudestacks-guideline-rust`, `claudestacks-guideline-python`, `claudestacks-journal`, `claudestacks-cmux` | `/Users/hiraq/Projects/airsstack/claudestacks/.claude-plugin/marketplace.json` |
| P29 | The sdlc pack uses the same `.claudestacks/sdlc/` paths as the plugin (identical path strings in both skill sets). The journal vault root is the plugin's (`$AIRSSTACK_HOME/journal`, default `~/.airsstack/journal`), and `lib/vault.lua` without comment lines is identical to the plugin's. The snapshot store is `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/` in both | `packs/journal/scripts/lib/vault.lua:127-133`; `probes/paths-results.md`; `probes/round2-results.md` §e |
| P30 | The pack text gates: `Makefile.toml:173` greps every file under `packs/` except `README.md` and `*_test.lua` for plugin syntax and plugin-qualified names; `Makefile.toml:181` greps every `*.md` under `packs/` except `README.md` and two guideline files for unprefixed skill, agent or command names | `Makefile.toml:171-196` |
| P31 | The plugin-to-pack renames and user-visible behaviour changes are listed in the packs spec | `.claudestacks/sdlc/2026-11-20-packs-and-claudevs/spec.md` §6.1 and §9 |
| P32 | `init --tool claude --pack core --pack sdlc --pack guideline-rust` (official names, `packs-v0.1.0`) writes the project file and `plan` exits 0 (`76 to write, 0 to delete, 0 unchanged, 1 skipped`). Without `--pack core` it refuses: `agentstacks.toml: packs.sdlc: sdlc requires pack "core", which agentstacks.toml does not list (add it: --pack core)` | `probes/round2-results.md` §a |
| P33 | airsl's pointer, under `/bin/sh` and `/bin/dash`: `''` and `latest` take the latest path; `v0.1.5` and `0.1.5` give `fetching the installer for airsl 0.1.5`; `0.1 5`, `a/b` and `v` give `AIRSL_VERSION='<value>' is not a version (expected MAJOR.MINOR.PATCH or latest)`; `0.2.0-rc.1` is accepted as given. A failed download prints one error. With `PATH=/nonexistent` it dies on `mktemp: command not found` (exit 127) before any helper runs | `probes/round2-results.md` §b |
| P34 | The dist plan manifest has `.releases[]` with `app_name`, `app_version` and `artifacts` (a list of artifact names), and `.artifacts[<name>]` with `kind` (`executable-zip` for archives) and `target_triples`. The §6 jq program over it yields six `{archive, target, app, bin, version}` rows per binary tag, `bin` being `agentstacks` or `claudevs`, `alpine` true only on the musl rows, merged with claudestacks' runner table | `probes/round2-results.md` §c (raw keys); `probes/jq-results.md` (the exact §6 program) |
| P35 | dist's generated shell installer installs to `$HOME/.local/bin` (help text and the default branch), unless `AGENTSTACKS_CLI_INSTALL_DIR` (or `CARGO_DIST_FORCE_INSTALL_DIR`) is set | `probes/round2-results.md` §d |
| P36 | `cargo publish` uploads one package, then polls until it appears in the index; that poll may time out, which does not affect the upload | `probes/docs-excerpts.md` (cargo publish block) |
| P37 | GitHub's "List releases" section states no order (no `sort`, `order` or `created_at` in it), while the latest-release section does say `sorted by the created_at attribute` | `probes/round2-results.md` §f |
| P38 | claudestacks' book shape: `book/book.toml` (`create-missing = false`, the `mermaid` preprocessor, `site-url`, `git-repository-url`, `edit-url-template`, `additional-js`), `mermaid.min.js` and `mermaid-init.js` beside it, the Makefile tasks `book-mdbook`, `book-mermaid`, `book`, `book-build` pinning mdBook 0.4.52 and mdbook-mermaid 0.16.2, a `book` CI job installing both with `taiki-e/install-action@v2`, a `pages.yml` that builds the book and copies `install/install.sh` into `book/book/`, and `book/book/` in `.gitignore` | `/Users/hiraq/Projects/airsstack/claudestacks/book/book.toml`; `claudestacks/Makefile.toml:542-584`; `claudestacks/.github/workflows/ci.yml:144-162`; `claudestacks/.github/workflows/pages.yml`; `claudestacks/.gitignore:40-41` |
| P39 | The §7.6 book (docs moved by the §7.6 link map, the claudevs chapters moved by the §7.6 rename and URL map, pack READMEs included after the §7.6 link edit, placeholders for the new chapters) builds with exit 0 and no `ERROR` line. All 843 relative hrefs of its pages (chapter links, anchors, assets) resolve: the file or directory exists, and the anchor when there is one. The moved pages carry their `SUMMARY.md` titles. The 5 mermaid diagrams render. No relative link is left in the pack chapters | `probes/book-results.md:10-158` |
| P40 | Today `docs/extensions.md:297` links `(#missing-values)`, but "Missing values" is a bold paragraph (`:175`) under the heading `` ### `values` `` (`:133`), so no heading makes that anchor. The §7.6 map points it at `#values`, which resolves (P39) | `docs/extensions.md:133,175,297`; `probes/book-results.md:52-64` |
| P41 | A chapter listed in `SUMMARY.md` with no file fails the build: `Chapter file not found`, exit 101 | `probes/book-results.md:159-162` |
| P42 | `{{#include ../../packs/<p>/README.md}}` in `book/src/` pulls in a file outside `book/`, but does not fix up its relative links: a `.md` file link only becomes `.html` (`skills/guidelines/SKILL.md` → `skills/guidelines/SKILL.html`), while a directory link (`skills/guidelines/references/`) and `../../LICENSE` stay as written. All of them break on the site. A missing include target logs `[ERROR] (mdbook::preprocess::links): Error updating …` and the build still exits 0, leaving the raw `{{#include …}}` in the page | `probes/book-results.md:332-341` (the unedited README); `probes/book-results.md:288-302` (missing target) |
| P43 | The §7.7 build step, run under `bash -e`, exits 0 on a clean book and 1 on a missing include target or a missing chapter, printing its own line after mdBook's | `probes/book-results.md:303-328` |
| P44 | A repo-wide `git grep 'docs/'` outside `.claudestacks/`, outside URLs dropped, finds this repository's `docs/` only in `README.md:86,115,144,150-151`, `CLAUDE.md:18,73,95-96,103,108` and `crates/claudevs/Cargo.toml:13` (`exclude = ["docs/"]`). Every other hit names some other `docs/`: test data in `crates/agentstacks/src/types/pack/`, a plugin-folder comment and fixture in `crates/claudevs/src/contract/site.rs`, and generic `docs/specs/` text and glob tests in `packs/` (control: the same grep finds `packs/` 12 times in `Makefile.toml`). `README.md:119` and `docs/authoring.md:7,18-19` hold raw `…/main/docs/…` URLs. Nothing in `Makefile.toml`, `.github/` or `deny.toml` names `docs/` | `probes/book-results.md:347-380` (repo-wide), `:270-282` (build files, and the raw URLs in `docs/`); `probes/book-structure-results.txt:5-17` |
| P45 | `crates/claudevs/docs/` holds nine one-line pointers to the claudestacks book, and nothing links to them (grep exit 1; control: the same grep shape finds `claudevs/examples`) | `probes/book-structure-results.txt:18-30` |
| P46 | The claudestacks book's claudevs part is 21 chapter files (ch05-ch09) plus `appendix-01-claudevs-cli.md`. No link in them leaves that set (grep exit 1; control: 7 in-set links in `ch05-00`). All 19 of their `claudestacks` mentions are URLs: the examples on GitHub and `install.sh` | `probes/book-structure-results.txt:31-63` |
| P47 | Every pack excludes `README.md` from what it installs, and both pack text gates skip `README.md` (P30). So the README link edit changes no installed file and no gate result | `probes/book-structure-results.txt:82-86`; `Makefile.toml:173,181` |
| P48 | With official names and `packs-v0.1.0` (official URL redirected to a bare copy of HEAD): `init --tool claude --pack core --pack sdlc --pack guideline-rust` with no terminal prints the plan, ending `76 to write, 0 to delete, 0 unchanged, 1 skipped`, `stdin is not a terminal; pass --yes to apply` and `agentstacks.toml written; nothing applied (run apply)`, exit 1. `apply --yes` ends `applied: 76 written, 0 deleted`. `plan` then ends `0 to write, 0 to delete, 76 unchanged, 1 skipped`, and its skip line is `skip     core/hook/3: claude shows no hook output to the model at session end`. `doctor` prints `healthy`. `update` prints one `no newer tag than packs-v0.1.0` note per entry and `nothing to update`. `init … --yes` in a fresh project ends `76 to write, 0 to delete, 0 unchanged, 1 skipped` and `applied: 76 written, 0 deleted`, exit 0. An `init` whose plan is accepted applies at once, as `--yes` does (`docs/cli.md:96-107`, row "applied") | `probes/book-results.md:182-269` |
| P49 | In GitHub's path filters, `*` matches zero or more characters but not `/` | `probes/workflow-syntax-filter.md:5` |
| P50 | The org README's Projects table has a Docs column, with `[Book](https://airsstack.github.io/<project>/)` for airsl, buildl and claudestacks | `/Users/hiraq/Projects/airsstack/README.md:99,104-106` |
| P51 | The §3 `packs/CHANGELOG.md` text, with the book link, passes both pack text gates (grep exit 1 each) | `probes/book-results.md:283-287` |
| P52 | The claudevs README book links are `crates/claudevs/README.md:47,49,51,53,55,57` and `crates/claudevs-cli/README.md:36-42`, all under `https://airsstack.github.io/claudestacks/` | `probes/book-structure-results.txt:67-80` |
| P53 | GitHub sets `RUNNER_TEMP` on every runner: "The path to a temporary directory on the runner. This directory is emptied at the beginning and end of each job." `mdbook-mermaid install book` leaves the §7.6 `book.toml` unchanged (diff exit 0) | `probes/variables.md:67`; `probes/book-probe5-results.txt` |
| P54 | mdBook 0.4.52 builds a chapter holding a link to a missing chapter file (`nope.md`) and an anchor no heading makes (`gr.md#no-such-anchor`) with exit 0 and no `ERROR` or `WARN` line | `probes/book-results.md:342-346` |
| P55 | The org's other books run no link checker: a grep for `linkcheck`, `lychee`, `link-check`, `markdown-link` over the `.github/`, `book/book.toml` and `Makefile.toml` of airsl, buildl and claudestacks exits 1 for each (control: each `pages.yml` pins `mdbook@0.4.52`) | `probes/book-results.md:381-387` |
| P56 | GitHub's default shell for `run:` on Linux and macOS is `bash -e {0}` | `probes/workflow-syntax.md:910` |
| P57 | Each claudestacks `e099300` chapter of the claudevs part, piped through the §7.6 claudevs map, equals the moved file of the probe book (`cmp`): 22 identical, 0 differ | `probes/book-results.md:392-393` |
| P58 | `journal` requires `core`, as `sdlc` does: its manifest reads `[requires]`, `packs = ["core"]`, `airsl = ">=0.1.5"`. Found at execute by plan 10 (`disproves: d3463785d0cb`), which corrected the earlier §7.6 text "`core`, `journal` → airsl ≥0.1.5" | `packs/journal/pack.toml:9-11` |

## 2. Versions and crate metadata

| Crate | Version | Why |
|---|---|---|
| `agentstacks` | 0.1.0 | first release (P7) |
| `agentstacks-cli` | 0.1.0 | first release (P7) |
| `claudevs` | 0.1.1 | crates.io has 0.1.0 (P7) |
| `claudevs-cli` | 0.1.2 | crates.io has 0.1.1 (P7) |

Root `Cargo.toml`:

- `[workspace.package] publish = true` (was `false`).
- `[workspace.dependencies]` `agentstacks = { path = "crates/agentstacks", version = "0.1.0" }` and
  `claudevs = { path = "crates/claudevs", version = "0.1.1" }`.

Member manifests:

| Crate | Adds |
|---|---|
| `agentstacks` | `readme = "README.md"`, `keywords = ["agents", "claude-code", "cli", "lua", "setup"]`, `categories = ["development-tools"]` |
| `agentstacks-cli` | `readme = "README.md"`, `keywords = ["agents", "claude-code", "cli", "lua", "setup"]`, `categories = ["command-line-utilities", "development-tools"]` |
| `claudevs`, `claudevs-cli` | version only; their `readme`, `keywords` and `categories` stay |

The keywords and categories meet the crates.io rules (P24). The publish dry run accepts this exact
metadata (`probes/spec-examples-results.md` §2).

New files. Every link in them is an absolute `https://github.com/airsstack/agentstacks/…` URL,
because crates.io renders the README away from the repository:

- `crates/agentstacks/README.md`: what the library is, that it backs the `agentstacks` CLI, that
  its API is unstable during `0.x`, and a link to the book's Under the Hood chapter
  (`https://airsstack.github.io/agentstacks/ch07-00-architecture.html`).
- `crates/agentstacks-cli/README.md`: install with the §5.3 one-liner, or with
  `cargo install agentstacks-cli` plus the note "this route does not install airsl, which `plan`
  needs (≥0.1.5)"; usage (`init --tool claude --pack core`, `plan`, `apply`, `doctor`, `update`);
  links to the repository README, the book (`https://airsstack.github.io/agentstacks/`) and its
  migration chapter (`https://airsstack.github.io/agentstacks/ch03-00-migration.html`).

`crates/claudevs-cli/README.md:12` becomes
`curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh   # prebuilt binary, no Rust needed`.
The book links in both claudevs READMEs (P52) move to the agentstacks book: the prefix
`https://airsstack.github.io/claudestacks/` becomes `https://airsstack.github.io/agentstacks/`, and
the chapter file is renamed by the §7.6 claudevs map (`ch05-00-claudevs-getting-started.html` →
`ch08-00-claudevs-getting-started.html`, `ch07-03-ci.html` → `ch10-03-ci.html`, and so on;
`appendix-01-claudevs-cli.html` keeps its name). The link texts stay.

`crates/claudevs/docs/` (nine pointers to the claudestacks book that nothing links to, P45) is
deleted, and with it the line `exclude    = ["docs/"]` in `crates/claudevs/Cargo.toml:13`, which
then excludes nothing. The publish dry run (§12) covers the manifest after the change.

## 3. CHANGELOGs

Every file follows Keep a Changelog 1.1.0 with the header shape of
`crates/claudevs/CHANGELOG.md:1-5`. Every new entry is dated `Unreleased`. The author replaces
`Unreleased` with the release day in the release pull request (§9 step 1).

| File | Entry |
|---|---|
| `crates/agentstacks/CHANGELOG.md` (create) | `## [0.1.0] - Unreleased`: initial release, minimum supported Rust 1.94; Added: the pack, extension, project and lock formats; the plan, apply, doctor and update flows; the extension host and the hook runtime |
| `crates/agentstacks-cli/CHANGELOG.md` (create) | `## [0.1.0] - Unreleased`: initial release; Added: `init`, `plan`, `apply`, `doctor`, `update`; official names `--tool claude` and `--pack core\|sdlc\|journal\|guideline-rust\|guideline-python`, resolved to the newest `packs-v*` tag; prebuilt archives for the six targets of §4 and the installer `curl -LsSf https://airsstack.github.io/agentstacks/install.sh \| sh`, which also installs airsl when it is missing |
| `crates/claudevs/CHANGELOG.md` | `## [0.1.1] - Unreleased`: Changed: the source moved to https://github.com/airsstack/agentstacks, and the documentation to the agentstacks book, https://airsstack.github.io/agentstacks/ch08-00-claudevs-getting-started.html; behaviour is unchanged. (The packs spec §2 is the source of "unchanged"; the entry carries no citation) |
| `crates/claudevs-cli/CHANGELOG.md` | `## [0.1.2] - Unreleased`, built on `claudevs` 0.1.1: Changed: built and released from https://github.com/airsstack/agentstacks; the prebuilt installer is `curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh \| sh` |
| `packs/CHANGELOG.md` (create) | exactly the text below |

`packs/CHANGELOG.md`, exactly:

```markdown
# Changelog

All notable changes to the official packs and tool extensions are recorded here. They are released
together under `packs-v*` tags, which `agentstacks init` and `agentstacks update` resolve. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and each pack and extension
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [packs-v0.1.0] - Unreleased

First release.

### Added

- Packs `core`, `sdlc`, `journal`, `guideline-rust` and `guideline-python`, each at 0.1.0.
- The `claude` tool extension at 0.1.0, on extension API `api = 0`.

Coming from the claudestacks plugins? See
[Moving from the claudestacks Plugins](https://airsstack.github.io/agentstacks/ch03-00-migration.html).
```

Text rule for later entries: name no skill, agent, command or plugin, only pack and extension names,
versions and docs. The pack text gates scan this file (P30), and `cargo make dod` runs them. Both
gates pass the text above (P51).

## 4. Release workflow (dist)

### 4.1 `dist-workspace.toml` (create)

Keys exactly as below. The comments say why each non-default key exists.

```toml
[workspace]
members = ["cargo:."]

# Config for 'dist'
[dist]
# The preferred dist version to use in CI (Cargo.toml SemVer syntax)
cargo-dist-version = "0.33.1"
# CI backends to support
ci = "github"
# Shell only: agentstacks and claudevs run on Unix, not Windows.
installers = ["shell"]
# macOS and Linux on both architectures, plus a musl build of each Linux one.
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "aarch64-unknown-linux-musl", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"]
# Not CARGO_HOME: the prebuilt binaries exist for engineers who have no Rust toolchain.
install-path = "~/.local/bin"
hosting = "github"
install-updater = false
# An attestation ties each archive's digest to this repository, commit and workflow.
github-attestations = true
# `host` waits for this workflow, so an archive that fails its smoke test is never published.
global-artifacts-jobs = ["./smoke"]
# agentstacks-cli and claudevs-cli carry their own versions. An untagged plan that builds
# ("upload") refuses two apps with different versions, so pull requests run `dist plan` only.
pr-run-mode = "plan"
# release.yml's tag trigger is edited by hand (see the comment there). Without this, dist
# reports the edit as out of date and fails its plan step.
allow-dirty = ["ci"]

# GitHub retired every macOS 14 image on 2026-11-02, and dist's default runner for
# aarch64-apple-darwin is still macos-14. Keep this entry whenever this file is rewritten.
[dist.github-custom-runners]
aarch64-apple-darwin = "macos-15"
```

### 4.2 `.github/workflows/release.yml` (create)

To produce it: write §4.1 without the `allow-dirty` line, run `dist generate` (dist 0.33.1), edit
the trigger, then add the `allow-dirty` line. The only hand edit replaces the `on:` block with the
block below, comment included:

```yaml
# Edited by hand, kept by allow-dirty = ["ci"] in dist-workspace.toml: only agentstacks-cli and
# claudevs-cli tags start a release. Library tags (agentstacks-v*, claudevs-v*) and packs-v* tags
# do not; dist's plan step fails on a packs-v* tag. To regenerate this file, remove allow-dirty,
# run `dist generate`, then reapply this edit and allow-dirty.
on:
  pull_request:
  push:
    tags:
      - 'agentstacks-cli-v[0-9]+.[0-9]+.[0-9]+*'
      - 'claudevs-cli-v[0-9]+.[0-9]+.[0-9]+*'
```

Why: the default trigger matches `packs-v0.1.0` (P10), and dist fails on that tag (P9). With the
edit and `allow-dirty`, each binary tag plans exactly one app, and an untagged plan passes (P11,
P12; the two texts above are what `probes/spec-examples-results.md` §1 ran).

### 4.3 Tags

| Tag | Marks | Starts a release |
|---|---|---|
| `agentstacks-v<version>` | the `agentstacks` crate on crates.io | no |
| `agentstacks-cli-v<version>` | the `agentstacks` binary release | yes |
| `claudevs-v<version>` | the `claudevs` crate on crates.io | no |
| `claudevs-cli-v<version>` | the `claudevs` binary release | yes |
| `packs-v<version>` | the official packs and extensions that `init`/`update` resolve | no |

- Every tag is annotated (`git tag -a <tag> -m <tag>`) and pushed alone (`git push origin <tag>`),
  because GitHub creates no event when more than three tags are pushed at once (P16).
- The first release tags the merged release commit with all five, in the order of §9.
- Later, each crate and the packs release on their own. A new `packs-vX.Y.Z` needs no binary
  release; `update` picks the newest stable `packs-v*` tag (`docs/plan-and-apply.md:336`, which §7.6 moves to `book/src/ch05-00-plan-and-apply.md`).

## 5. Installers

### 5.1 Hosting: `.github/workflows/pages.yml` (create)

The shape of `/Users/hiraq/Projects/airsstack/claudestacks/.github/workflows/pages.yml` (P38), which
publishes the book with its installer beside it. Triggers: `workflow_dispatch`, and a push to `main`
touching `book/**`, `install/**`, `packs/*/README.md` (the pack chapters include them, P42; `*`
does not cross `/`, P49) or `.github/workflows/pages.yml`. `permissions: contents: read`,
`concurrency` group `pages` with `cancel-in-progress: false`.

- **build** (`ubuntu-latest`, `timeout-minutes: 10`): checkout with `persist-credentials: false`;
  `taiki-e/install-action@v2` with `tool: mdbook@0.4.52,mdbook-mermaid@0.16.2`; the §7.7 build
  step; `cp install/install.sh install/claudevs.sh book/book/`;
  `actions/upload-pages-artifact@v5` with `path: book/book`.
- **deploy**: claudestacks' job unchanged (`needs: build`, `actions/deploy-pages@v5`,
  `pages: write`, `id-token: write`, environment `github-pages`).
- The header comment is claudestacks', adapted: it names the book, `install.sh` (agentstacks plus
  airsl) and `claudevs.sh`, and says the Pages source must be "GitHub Actions" (P21).

Served: the book at `https://airsstack.github.io/agentstacks/`, the scripts at
`https://airsstack.github.io/agentstacks/install.sh` and `…/claudevs.sh`. The Pages source is an
author step (§9 step 2). A failed book build stops the workflow before deploy (deploy `needs`
build; GitHub skips a job whose needed job failed, P13), so the installers are never served
without the book or the other way round.

### 5.2 Shared shape

Both scripts are POSIX sh, the shape of airsl's pointer (P22): a header comment with the usage
lines, `set -eu`, `say` (prints `<p>: <text>` to stderr), `fail` (`say "error: <text>"`, exit 1),
`download` (curl with `--proto '=https' --tlsv1.2 -sSfL`, else `wget -qO-`), a `main`, and
`main "$@"` as the last line. Why sh and not Lua: they run before airsl exists, as airsl's and
claudestacks' pointers do (P22). `main` runs in this order:

```text
1. no curl and no wget (command -v, a builtin) → M1. Nothing else runs before this check, so M1 is
   the only message (airsl's script would die on mktemp first, P33).
2. tmp = mktemp -d; trap removes it on exit.
3. version = ${<VAR>:-latest}, one leading "v" stripped.
4. version = latest:
     say M3
     download https://api.github.com/repos/airsstack/agentstacks/releases?per_page=100 > "$tmp/releases.json"
       (a plain redirect in the current shell, never inside $( ); fails → M4)
     version = the P18 grep | sed | sort | tail over "$tmp/releases.json"; empty → M5
   contains a char outside [0-9A-Za-z.+-], or empty after the strip → M2
   anything else → used as given (a pre-release such as 0.2.0-rc.1 is allowed, never chosen as latest)
5. url = https://github.com/airsstack/agentstacks/releases/download/<app>-v<version>/<app>-installer.sh
   say M6; download url > "$tmp/<app>-installer.sh"; fails → M7
6. sh "$tmp/<app>-installer.sh" "$@"; a non-zero exit ends the script with that status.
7. (install.sh only) the airsl step, §5.3.
```

The rules of steps 3-4 are airsl's `case` (P33). `releases/latest` is not used: it is repo-wide,
so after a claudevs-only release it would hold no agentstacks installer (P15).

### 5.3 `install/install.sh` (create)

Prefix `agentstacks-install`, `<VAR>` = `AGENTSTACKS_VERSION`, `<app>` = `agentstacks-cli`,
`<name>` = `agentstacks`. Usage lines in the header:

```text
curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh
curl -LsSf https://airsstack.github.io/agentstacks/install.sh | AGENTSTACKS_VERSION=0.1.0 sh
```

The airsl step, because agentstacks needs airsl (P4):

```text
command -v airsl succeeds                   → say M8 with that path; done
else "$HOME/.local/bin/airsl" is executable → say M8 with that path; done
else → say M9; download https://airsstack.github.io/airsl/install.sh > "$tmp/airsl-install.sh"
       and run it with sh (download or run fails → M10)
```

`$HOME/.local/bin` is checked by path because that is where dist's installers put binaries by
default (P35; airsl's `dist-workspace.toml` sets the same `install-path`), and the running shell
may not have it on `PATH` yet. An airsl that is found is never replaced or upgraded, even when it
is too old; `plan` then names the version it needs (P4). Arguments go only to the agentstacks
installer. Environment variables reach both installers unchanged (for example
`AGENTSTACKS_CLI_INSTALL_DIR`, `AIRSL_VERSION`).

### 5.4 `install/claudevs.sh` (create)

Prefix `claudevs-install`, `<VAR>` = `CLAUDEVS_VERSION`, `<app>` = `claudevs-cli`,
`<name>` = `claudevs`. No airsl step, because claudevs never runs the airsl binary (P6). Usage lines:

```text
curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh
curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | CLAUDEVS_VERSION=0.1.2 sh
```

### 5.5 Messages (exact; `<p>` is the script's prefix, `<name>`/`<VAR>`/`<app>` per script)

| # | When | Text (stderr) | Exit |
|---|---|---|---|
| M1 | neither curl nor wget | `<p>: error: neither curl nor wget is installed` | 1 |
| M2 | bad version | `<p>: error: <VAR>='<raw value>' is not a version (expected MAJOR.MINOR.PATCH or latest)` | 1 |
| M3 | latest lookup starts | `<p>: finding the latest <name> release` | — |
| M4 | API download failed | `<p>: error: could not list the releases at https://api.github.com/repos/airsstack/agentstacks/releases (GitHub allows 60 unauthenticated requests an hour); set <VAR> to a version to skip this lookup` | 1 |
| M5 | no stable `<app>-v*` release | `<p>: error: found no <name> release at https://github.com/airsstack/agentstacks/releases` | 1 |
| M6 | fetching | `<p>: fetching the installer for <name> <version>` | — |
| M7 | installer download failed | `<p>: error: could not download the installer for <name> <version>; is <version> a released version? See https://github.com/airsstack/agentstacks/releases` | 1 |
| M8 | airsl present (install.sh only) | `agentstacks-install: airsl found at <path>; leaving it as it is` | — |
| M9 | airsl missing (install.sh only) | `agentstacks-install: agentstacks needs airsl, which is not installed; installing the latest airsl with https://airsstack.github.io/airsl/install.sh` | — |
| M10 | airsl install failed (install.sh only) | `agentstacks-install: error: agentstacks is installed, but installing airsl failed; install it with: curl -LsSf https://airsstack.github.io/airsl/install.sh \| sh` | 1 |

The dist installer's own failures print dist's text, and the script exits with its status.

## 6. Smoke workflow: `.github/workflows/smoke.yml` (create)

Written by hand, with `on: workflow_call` taking an input `plan` (string), and
`permissions: contents: read`. The shape and the runner table come from
`/Users/hiraq/Projects/airsstack/claudestacks/.github/workflows/claudevs-cli-smoke.yml`, with the
header comment adapted to two apps.

**targets job.** It builds the matrix with this jq program over the plan (`jq -c --argjson runners
"$runners"`), proven on both binary tags (P34):

```text
. as $all
| [.releases[] as $r
   | $r.artifacts[]
   | select($all.artifacts[.].kind == "executable-zip")
   | {archive: ., target: $all.artifacts[.].target_triples[0],
      app: $r.app_name, bin: ($r.app_name | sub("-cli$"; "")), version: $r.app_version,
      alpine: false}
     + ($runners[$all.artifacts[.].target_triples[0]] // {})]
```

`$runners` is claudestacks' six-row table. The empty-matrix and missing-runner errors are
claudestacks' (`the plan lists no archives to smoke-test`, `no smoke runner for: <targets>`). The
version comes per archive, never from `.releases[0]`.

**smoke job** (per archive, `fail-fast: false`, bash, env `ARCHIVE`, `BIN_NAME` = matrix `bin`,
`VERSION` = matrix `version`): checkout, download `artifacts-build-local-*`, check the checksum,
unpack, then find the file named `$BIN_NAME`. If it is missing: `no $BIN_NAME binary inside $ARCHIVE`.
musl archives run inside `alpine:3.20` through a wrapper, as in claudestacks. The binary's path
(or the wrapper's) goes into `$GITHUB_ENV` as `BIN`; claudestacks calls this variable `CLAUDEVS`,
and every copied step uses `"$BIN"`. Then:

| Step | When | Check |
|---|---|---|
| version | always | `"$BIN" --version` equals `$BIN_NAME $VERSION`, else `$BIN_NAME --version printed '<got>', expected '$BIN_NAME $VERSION'` (P26) |
| claudevs doctor, wiring fixtures, test-stage fixtures | `BIN_NAME == claudevs` | claudestacks' three steps with `$CLAUDEVS` → `$BIN`, nothing else changed, including `if: !matrix.alpine` on the test stage (the fixtures exist here, P26) |
| agentstacks project | `BIN_NAME == agentstacks` | In `.smoke/project` (inside the workspace, so the Alpine mount sees it): copy `extensions/claude` to `ext/claude` and `packs/guideline-rust` to `packs/guideline-rust`. Run `init --tool ext/claude --pack packs/guideline-rust --yes` (exit 0, a line matching `^applied: [0-9]+ written, 0 deleted$`), then `plan` (exit 0, a line matching `^0 to write, 0 to delete, [0-9]+ unchanged, 0 skipped$`), then `doctor` (exit 0, a line equal to `healthy`). On a miss: `agentstacks <command> exited <code>, expected 0` or `agentstacks <command> did not print '<expected>'` |

`guideline-rust` is used because it needs no airsl (P5), and these commands need no tool on `PATH`,
not even git (P25). So the Alpine archives run every agentstacks step. The counts are regexes, so
a change to the pack's file count does not break smoke.

A pull request does not run this workflow, because `pr-run-mode = "plan"` builds nothing (P13). It
runs on each binary tag, and `host` publishes only after it succeeds (P13). Its first run is
therefore on the first binary tag, which §9 pushes before anything irreversible happens.

## 7. CI and docs in this repository

### 7.1 `.github/workflows/ci.yml` (modify)

Add a job `installers` (`runs-on: ubuntu-latest`, `timeout-minutes: 5`) that checks out the
repository and runs, for each of `install/install.sh` and `install/claudevs.sh`:

| Check | Expect |
|---|---|
| `sh -n <script>` | exit 0 (P23) |
| `env -i PATH=/nonexistent /bin/sh <script>` | exit 1, and stderr is exactly the M1 line. This proves §5.2 step 1 comes first. `command -v` and `printf` are shell builtins, so no external tool is needed to reach M1 |
| `env -i PATH=/usr/bin:/bin HOME="$RUNNER_TEMP" <VAR>='a/b' /bin/sh <script>`, with `curl` and `wget` present on the runner | exit 1, and stderr is exactly the M2 line with `'a/b'` |

All three are offline.

Add a job `book`: claudestacks' `book` job (P38: `ubuntu-latest`, `timeout-minutes: 10`, checkout,
`taiki-e/install-action@v2` with `tool: mdbook@0.4.52,mdbook-mermaid@0.16.2`, and its comments),
with the §7.7 build step in place of its `mdbook build book` step. Nothing else in `ci.yml`
changes.

### 7.2 `README.md` (modify)

- Status block becomes: `> **Status: pre-release (0.1.0).** The CLI and the Claude Code extension
  are released, with the five official packs (`packs-v0.1.0`). OpenCode is not here yet. The
  extension API is `api = 0` and will change. Coming from the claudestacks plugins? See
  [the migration guide](https://airsstack.github.io/agentstacks/ch03-00-migration.html).`
- A new `## Install` section before `## Usage`: the install.sh one-liner, the line that it also
  installs airsl when it is missing, and `cargo install agentstacks-cli` with the §2 airsl note.
- `## Usage` leads with the official-names block, which works now:
  `agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust` (P32: `sdlc`
  needs `core`), then `plan`, `apply`, `doctor`, `update`. The "Works today, with … local paths"
  block becomes "Local extensions and packs work too", with the same commands. The
  `--tool opencode` example and the "Planned, once the official packs are released" heading go.
- Tools table: Claude Code → `Released (`extensions/claude`, `packs-v0.1.0`)`.
- Packs: one line saying they are released as `packs-v0.1.0`.
- Tools table, the "Your own" row (`README.md:86`): `Write a tool extension against the public API:
  [Writing a Tool Extension](https://airsstack.github.io/agentstacks/ch06-00-authoring.html)`.
- "Extending agentstacks" (`README.md:115-119`): the guide link becomes
  `[Writing a Tool Extension](https://airsstack.github.io/agentstacks/ch06-00-authoring.html)`, and
  the raw link in the text block becomes
  `Follow https://raw.githubusercontent.com/airsstack/agentstacks/main/book/src/ch06-00-authoring.md`.
- Layout tree: add `install/` (installer scripts, served on Pages) and `RELEASING.md`; the `docs/`
  line goes; the `book/` line becomes
  `├─ book/                 the agentstacks book (mdBook), on airsstack.github.io/agentstacks   built`,
  in the tree's column layout.
- `## Documentation` becomes: `The agentstacks book, https://airsstack.github.io/agentstacks/, covers
  installing, the packs, moving from the claudestacks plugins, the CLI, plan and apply, writing a
  tool extension, the architecture, and claudevs. Its source is book/src/; contributors start with
  [Under the Hood](https://airsstack.github.io/agentstacks/ch07-00-architecture.html): the layers,
  the dependency rule, where logic lives, and how it is tested.` (`book/src/` in backticks.)
- Roadmap: `M1  claude extension (api = 0) ✓ released (0.1.0); the claudestacks plugins are packs`.
- After the edits, `README.md` names no `docs/` path (P44).

### 7.3 `book/src/ch03-00-migration.md` (create): Moving from the claudestacks Plugins

For a user of the claudestacks marketplace plugins. A `#` title (`# Moving from the claudestacks
Plugins`), then these `##` sections, in order:

1. **What maps to what**: `claudestacks` → `core`, `claudestacks-sdlc` → `sdlc`,
   `claudestacks-journal` → `journal`, `claudestacks-guideline-rust` → `guideline-rust`,
   `claudestacks-guideline-python` → `guideline-python`, `claudestacks-cmux` → not converted, keep
   the plugin if you use it (P28; packs spec §9 last row).
2. **Install agentstacks**: the install.sh one-liner (installs airsl when missing).
3. **Remove the plugins**, so their skills and hooks don't run twice. Per plugin:
   `claude plugin uninstall <plugin>@claudestacks --scope <scope>`, with the scope you installed it
   at (default user). Or remove the whole marketplace with `claude plugin marketplace remove
   claudestacks`, which uninstalls every plugin from it, cmux included (P27).
4. **Set up each project**: `agentstacks init --tool claude --pack core --pack sdlc --pack journal
   --pack guideline-rust` (pick the packs you used; `sdlc` and `journal` require `core`, P32, P58), review the plan,
   answer the apply prompt, then commit `agentstacks.toml`, `agentstacks.lock` and `.claude/`.
   Packs install per project, never machine-wide.
5. **What changed**: packs spec §9's rows, rewritten for users (every row except the last, which is
   step 1's cmux line). The command and agent renames come first, as an old → new table
   (`/claudestacks:x` → `/core-x`, … from §9 row 1 and §6.1). Then the rest: airsl on `PATH`
   ≥0.1.5, reported by `plan` and `doctor`; `style_reinject` in `agentstacks.toml`
   `[packs.core.options]`; `/core-statusline`; the enforce hook finds guideline packs installed in
   the same project; `sdlc` and `journal` require `core` (P58); the enforce pointer names `/guideline-rust-guidelines`;
   the enforce doctor's changes; the SessionEnd reminder shown as a skip by `plan`; the
   SubagentHandback deny text.
6. **What stays**: `.claudestacks/sdlc/` chain files are read and written at the same paths. The
   journal vault stays at `$AIRSSTACK_HOME/journal` (default `~/.airsstack/journal`), and the
   snapshot store at `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/` (P29).
7. **claudevs users**: `curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh`;
   `cargo install claudevs-cli` is unchanged; the old claudestacks installer URL forwards here once
   §8.2 lands.

The chapter cites no `.claudestacks/` planning path for its content. Install steps link to
[Installation](ch01-01-installation.md); the claudevs section links to the claudevs
[Installation](ch08-01-installation.md).

### 7.4 `RELEASING.md` (create)

The author's checklist: §9 as numbered steps with its commands and recovery lines, and the tag
table of §4.3. It says that `cargo publish` is irreversible (a version can be yanked, never
replaced) and that each tag is pushed alone. It also says to revisit the installers' release
lookup when the repository passes 100 releases (§10). It stays at the repository root, outside the
book: the book is for users and contributors, and this is the maintainer's runbook.

### 7.5 `CLAUDE.md` (modify)

- Status heading: `## Status: core, CLI commands, claude extension, packs and claudevs built; first release configured`.
- The layout block gains `dist-workspace.toml`, `install/` and `RELEASING.md`.
- In Commands, the line `When dist is added, set aarch64-apple-darwin = "macos-15" … smoke
  workflow.` becomes: `dist-workspace.toml sets aarch64-apple-darwin = "macos-15" under
  [dist.github-custom-runners], and smoke.yml uses the same runner. Releases follow RELEASING.md.`
- The CI sentence names the `installers` job too.
- The "Archiving claudestacks … is `2026-11-20-release-and-migration` work" sentence becomes
  `claudestacks is archived after the first release (RELEASING.md); never delete it: releases,
  install.sh and the old marketplace still point there.`
- The book replaces `docs/` everywhere it is named (P44): the layout line `docs/ …` (line 18)
  becomes `book/                     the agentstacks book (mdBook); book/src/ch07-00-architecture.md
  is the layering contract`; the sentence "`book/` has not landed." goes; line 73's contract link
  becomes `[`book/src/ch07-00-architecture.md`](book/src/ch07-00-architecture.md)`; lines 95-96
  name `book/src/ch06-00-authoring.md`, `book/src/ch06-01-extensions.md` and
  `book/src/ch06-02-hooks.md`, and the "keep it in step" sentence names those files; in the
  planned layout, line 103's `(see docs/architecture.md)` becomes
  `(see book/src/ch07-00-architecture.md)`, the `docs/` line (108) goes, and the `book/` line ends
  `(built)`.
- Commands gain `cargo make book                # serve the book at http://localhost:3000` and
  `cargo make book-build          # build the book once into book/book/ (CI also fails on [ERROR] lines)`, and the
  CI sentence names the `book` job too.
- A new paragraph after the Architecture bullets, carried from `docs/README.md:15-17`: `A book
  chapter describes what the code does, or, while marked planned, what it is agreed to do. When
  the two disagree, the code is wrong or the chapter is out of date. Fix one of them in the same
  change. Links in pack READMEs are absolute URLs, because the book includes those files.`
- After the edits, `CLAUDE.md` names no `docs/` path.

### 7.6 The book: `book/` (create)

The shape of the org's other books (P38). `docs/` moves in and is removed; the claudevs chapters
move in from the claudestacks book; the rest is new. The proposed layout builds with every internal
link resolving (P39).

**Files.**

- `book/book.toml`, exactly (claudestacks' file with its name swapped, P38; `mdbook-mermaid
  install` leaves it unchanged, P53):

  ```toml
  [book]
  title = "The agentstacks Book"
  authors = ["airsstack"]
  language = "en"
  src = "src"

  # A chapter named in SUMMARY.md with no file behind it fails the build
  # (`Chapter file not found`) instead of being created empty.
  [build]
  create-missing = false

  # Turns ```mermaid blocks into diagrams. mermaid.min.js and mermaid-init.js
  # beside this file are written by `mdbook-mermaid install book`.
  [preprocessor.mermaid]
  command = "mdbook-mermaid"

  [output.html]
  # Pages serves this repository as a project site under /agentstacks/, so the 404
  # page needs the prefix to load its own stylesheets.
  site-url = "/agentstacks/"
  git-repository-url = "https://github.com/airsstack/agentstacks"
  edit-url-template = "https://github.com/airsstack/agentstacks/edit/main/book/{path}"
  additional-js = ["mermaid.min.js", "mermaid-init.js"]
  ```

- `book/mermaid.min.js`, `book/mermaid-init.js`: written by `mdbook-mermaid install book` with
  mdbook-mermaid 0.16.2, and committed (P38). The moved claudevs chapters hold 5 diagrams (P39).
- `.gitignore`: append a blank line, `# The book's build output. Pages builds its own copy; nothing
  built is committed.` and `book/book/` (claudestacks' two lines, P38).
- `Makefile.toml`: claudestacks' tasks `book-mdbook`, `book-mermaid`, `book` and `book-build`
  with the comment block above them, verbatim (`claudestacks/Makefile.toml:529-584`, P38), and its
  two header usage lines (`claudestacks/Makefile.toml:7-8`) in this file's header comment. They are
  not part of `dod`.
- `book/src/SUMMARY.md`, exactly:

  ```markdown
  # Summary

  [Introduction](introduction.md)

  # agentstacks

  - [Getting Started](ch01-00-getting-started.md)
    - [Installation](ch01-01-installation.md)
    - [A First Project](ch01-02-first-project.md)
  - [The Packs](ch02-00-packs.md)
    - [core](ch02-01-core.md)
    - [sdlc](ch02-02-sdlc.md)
    - [journal](ch02-03-journal.md)
    - [guideline-rust](ch02-04-guideline-rust.md)
    - [guideline-python](ch02-05-guideline-python.md)
  - [Moving from the claudestacks Plugins](ch03-00-migration.md)
  - [The Command Line](ch04-00-cli.md)
  - [Plan, Apply and the Lock](ch05-00-plan-and-apply.md)
  - [Writing a Tool Extension](ch06-00-authoring.md)
    - [The Extension API](ch06-01-extensions.md)
    - [Hooks](ch06-02-hooks.md)
  - [Under the Hood](ch07-00-architecture.md)

  # claudevs

  - [Getting Started](ch08-00-claudevs-getting-started.md)
    - [Installation](ch08-01-installation.md)
    - [A First Plugin](ch08-02-first-plugin.md)
    - [A First Case](ch08-03-first-case.md)
    - [Running the Gate](ch08-04-running-the-gate.md)
  - [Writing Cases](ch09-00-writing-cases.md)
    - [Hook Decisions](ch09-01-hook-decisions.md)
    - [Scripts and Flows](ch09-02-scripts-and-flows.md)
    - [Cases in Lua](ch09-03-cases-in-lua.md)
    - [Native Suites](ch09-04-native-suites.md)
    - [Selecting Cases and Reading Skips](ch09-05-selecting-and-skips.md)
  - [The check Gate](ch10-00-check-gate.md)
    - [Stages](ch10-01-stages.md)
    - [Wiring](ch10-02-wiring.md)
    - [Running in CI](ch10-03-ci.md)
  - [Embedding](ch11-00-embedding.md)
    - [Your First Suite](ch11-01-first-suite.md)
    - [Recipes](ch11-02-recipes.md)
  - [Under the Hood](ch12-00-claudevs-under-the-hood.md)
    - [Why It Works This Way](ch12-01-design.md)
    - [How the Engine Fits Together](ch12-02-engine.md)

  # Appendix

  - [Appendix](appendix-00.md)
    - [A. claudevs CLI Reference](appendix-01-claudevs-cli.md)
  ```

  (`probes/book-results.md:92-142` is this file.) It lists 40 chapter files; with `SUMMARY.md`,
  `book/src/` holds 41 files and nothing else (P39).

**Where each chapter comes from.**

| File | Source | Edits |
|---|---|---|
| `introduction.md`, `ch01-00-getting-started.md`, `ch01-01-installation.md`, `ch01-02-first-project.md`, `ch02-00-packs.md`, `appendix-00.md` | new | content below |
| `ch02-01-core.md`, `ch02-02-sdlc.md`, `ch02-03-journal.md`, `ch02-04-guideline-rust.md`, `ch02-05-guideline-python.md` | new | each exactly one line, `{{#include ../../packs/<pack>/README.md}}` (P42). The README's own `# <pack>` title is the page title |
| `ch03-00-migration.md` | new | §7.3 |
| `ch04-00-cli.md` | `git mv docs/cli.md` | docs link map |
| `ch05-00-plan-and-apply.md` | `git mv docs/plan-and-apply.md` | docs link map |
| `ch06-00-authoring.md` | `git mv docs/authoring.md` | docs link map, and the raw URLs below |
| `ch06-01-extensions.md` | `git mv docs/extensions.md` | docs link map, including `#values` (P40) |
| `ch06-02-hooks.md` | `git mv docs/hooks.md` | docs link map |
| `ch07-00-architecture.md` | `git mv docs/architecture.md` | docs link map |
| `ch08-*` … `ch12-*` (21 files), `appendix-01-claudevs-cli.md` | claudestacks `book/src/` at `e099300` (P46) | claudevs map |

`docs/README.md` is deleted; its last paragraph moves to `CLAUDE.md` (§7.5). After the move
`docs/` does not exist.

**Docs link map.** One pass over each moved file, these replacements and the title line below,
and nothing else:

```text
](cli.md              → ](ch04-00-cli.md
](plan-and-apply.md   → ](ch05-00-plan-and-apply.md
](authoring.md        → ](ch06-00-authoring.md
](extensions.md       → ](ch06-01-extensions.md
](hooks.md            → ](ch06-02-hooks.md
](architecture.md     → ](ch07-00-architecture.md
(#missing-values)     → (#values)
```

Line 1 of each moved file, its title, becomes its `SUMMARY.md` entry: `# CLI` → `# The Command
Line`; `# Plan, apply, doctor and update` → `# Plan, Apply and the Lock`; `# Build an agentstacks
tool extension` → `# Writing a Tool Extension`; `# Writing a tool extension` → `# The Extension
API`; `# Architecture` → `# Under the Hood`; `# Hooks` already matches. (`probes/book-results.md:52-64`
is the map that ran; `:149-155` the titles it produced.)

The anchors after `#` stay; they resolve under mdBook as they are (P39). In
`ch06-00-authoring.md` the three raw URLs (P44) also change:
`https://raw.githubusercontent.com/airsstack/agentstacks/main/docs/authoring.md` →
`…/main/book/src/ch06-00-authoring.md`; `[extensions.md](…/main/docs/extensions.md)` →
`[ch06-01-extensions.md](…/main/book/src/ch06-01-extensions.md)`; `[hooks.md](…/main/docs/hooks.md)`
→ `[ch06-02-hooks.md](…/main/book/src/ch06-02-hooks.md)`.

**claudevs map.** Each file is renamed by its claudestacks chapter number, one rule per file and
never chained: `ch05-` → `ch08-`, `ch06-` → `ch09-`, `ch07-` → `ch10-`, `ch08-` → `ch11-`,
`ch09-` → `ch12-`; `appendix-01-claudevs-cli.md` keeps its name. The same full-name map rewrites
every chapter link inside the moved files (applied in one pass, so a renamed name is never renamed
again; old `ch08-00-embedding.md` → `ch11-00-embedding.md`, old `ch05-00-…` → `ch08-00-…`). Then:

```text
https://airsstack.github.io/claudestacks/install.sh → https://airsstack.github.io/agentstacks/claudevs.sh
https://github.com/airsstack/claudestacks/          → https://github.com/airsstack/agentstacks/
claudevs 0.1.1  (the output line in ch08-01-installation.md) → claudevs 0.1.2
```

Every example the rewritten URLs name exists here (P39, `probes/book-results.md:163-181`), and no
`claudestacks` mention is left in the moved chapters (P46). Everything else is byte-identical to
claudestacks `e099300` (P57).

**Pack README links** (P42, P47): in the five `packs/*/README.md`, every relative link becomes
absolute. `../../LICENSE` → `https://github.com/airsstack/agentstacks/blob/main/LICENSE`; a link
to a directory (ending `/`) → `https://github.com/airsstack/agentstacks/tree/main/packs/<pack>/<path>`;
a link to a file → `https://github.com/airsstack/agentstacks/blob/main/packs/<pack>/<path>`.
Absolute links stay. That rewrites 12 links: `../../LICENSE` in five READMEs, and two each in
`guideline-python`, `guideline-rust`, `journal` and `sdlc`. With the 4 that were already absolute,
the READMEs hold 16 links, listed at `probes/book-results.md:12-27`.

**New chapters.** Each starts with a `#` title equal to its `SUMMARY.md` entry. Every command and
every output line shown is a spec text with its proof (§5.5, P5, P26, P32, P35, P48); nothing else
is shown as output. Every `init` line that names `--pack sdlc` names `--pack core` (P32).

- `introduction.md`: what agentstacks is, in the README's terms (packs, tool extensions, plan and
  apply, the lock); its status (pre-release 0.1.0, Claude Code only, extension API `api = 0`);
  what claudevs is (claudestacks `book/src/introduction.md:45-51`, adapted) and that it moved here
  from claudestacks; the book's three parts; and "Coming from the claudestacks plugins? Start with
  [Moving from the claudestacks Plugins](ch03-00-migration.md)."
- `ch01-00-getting-started.md`: install agentstacks, then set up a first project; links to both.
- `ch01-01-installation.md`: `## A prebuilt binary` with the two §5.3 usage lines; what the script
  prints about airsl (M8 or M9, verbatim, `<path>` as written in §5.5); binaries go to
  `~/.local/bin` unless `AGENTSTACKS_CLI_INSTALL_DIR` is set (P35); macOS and Linux on x86_64 and
  aarch64, no Windows (§4.1). `## From crates.io` with `cargo install agentstacks-cli`, the §2
  airsl note, and `curl -LsSf https://airsstack.github.io/airsl/install.sh | sh` for airsl.
  `## Check it works` with `agentstacks --version` → `agentstacks 0.1.0`, and `airsl --version`
  at 0.1.5 or newer (P5). claudevs: a link to [its installation](ch08-01-installation.md).
- `ch01-02-first-project.md`: in a project directory, `agentstacks init --tool claude --pack core
  --pack sdlc --pack guideline-rust` prints the plan, one line per file, ending `76 to write, 0 to
  delete, 0 unchanged, 1 skipped`, and asks before applying. Answering yes, or passing `--yes`,
  applies at once and prints `applied: 76 written, 0 deleted` (P48). The chapter walks that path.
  It then names the other one: with no terminal and no `--yes`, `init` prints the two P48 lines,
  writes `agentstacks.toml` and applies nothing, and `agentstacks apply --yes` then prints
  `applied: 76 written, 0 deleted`. The `agentstacks.toml` it writes, exactly as in P48. The skip
  line, quoted, and what it means: an item the tool cannot express is shown, never dropped. Then
  `agentstacks plan` → `0 to write, 0 to delete, 76 unchanged, 1 skipped`; `agentstacks doctor` →
  `healthy`; `agentstacks update` → `nothing to update` while `packs-v0.1.0` is the newest tag.
  Commit `agentstacks.toml`, `agentstacks.lock` and `.claude/`. The counts are those of
  `packs-v0.1.0`. Links to [The Command Line](ch04-00-cli.md) (the prompt, `--yes`, exit codes)
  and [Plan, Apply and the Lock](ch05-00-plan-and-apply.md).
- `ch02-00-packs.md`: a pack is what the agent gets, rendered for each tool by its extension. A
  table of the five packs: name, one line taken from the first paragraph of its README, and what
  it needs: `core` → airsl ≥0.1.5; `sdlc`, `journal` → `core` and airsl ≥0.1.5;
  `guideline-rust`, `guideline-python` → nothing (P5, P32, P58). The sentence that `sdlc` or
  `journal` without `core` is refused, with the `sdlc` refusal quoted (P32). They are released together as `packs-v*` tags, and `update` moves a
  project to the newest (§4.3). One page per pack follows.
- `appendix-00.md`: one sentence: the appendix holds the claudevs command-line reference.

### 7.7 The book build step

The `ci.yml` `book` job and the `pages.yml` build job run this exact `run:` body, in a step named
`Build the book`. mdBook exits 0 when an include target is missing (P42), so the step also fails on
any `[ERROR]` line. Proven under `bash -e` (P43), GitHub's default for `run:` (P56). `RUNNER_TEMP` is
GitHub's per-job temporary directory (P53):

```bash
mdbook build book > "$RUNNER_TEMP/mdbook.log" 2>&1 || { cat "$RUNNER_TEMP/mdbook.log"; exit 1; }
cat "$RUNNER_TEMP/mdbook.log"
if grep -q '\[ERROR\]' "$RUNNER_TEMP/mdbook.log"; then
  echo "mdbook build book logged an error; see the [ERROR] lines above" >&2
  exit 1
fi
```

## 8. Outside this repository

### 8.1 Org docs (files in `/Users/hiraq/Projects/airsstack`, a local repo, and its `.github` repo)

A plan edits these files in place; they are plain files outside this worktree, with no branch to
make. They state that claudestacks is archived, so the author commits them only after the archive
(§9 step 11).

`.github/profile/README.md`:

- Line 25: `SDLC["agentstacks<br/>agent tool setup: packs + extensions"]`. Line 46: `style SDLC
  fill:#4a3f2a,stroke:#c9a35c` (amber = pre-release; the key is at line 57).
- Projects table: insert above the claudestacks row
  `| [**agentstacks**](https://github.com/airsstack/agentstacks) | Run the factory | A CLI that sets up agentic coding tools in a project. **Packs** hold tool-neutral content (skills, agents, commands, hooks, rules); **tool extensions** in Lua, sandboxed by airsl, render them in each tool's own format, as a plan you review before it is applied. Claude Code first, OpenCode next. Hosts [**claudevs**](https://crates.io/crates/claudevs), a deterministic test engine for Claude Code setups. | Pre-release |`.
  The claudestacks row's text after its link becomes `Archived. Its Claude Code plugins became the agentstacks packs and claudevs moved to agentstacks; it still hosts airsclaude until airsdk.`
  and its status becomes `Archived`.
- The airsl row: `Powers buildl and the claudestacks hooks.` → `Powers buildl and the agentstacks hooks.`
- "Where we are heading": `Nothing below has moved yet.` → `agentstacks has moved; airsdk has not yet.`
  The `claudestacks` → **agentstacks** row stays as the record of the move.
- Line 103: ``Every change moves through `claudestacks-sdlc`:`` → ``Every change moves through the agentstacks `sdlc` pack:``.
- Line 111: `the agentstacks and airsdk regrouping` → `the airsdk regrouping`.

`README.md` (org root):

- Line 44 (map): `│ claudestacks  sdlc + plugins    │` → `│ agentstacks   tool setup: packs │`
  (the cell keeps its 33 characters).
- Line 55: `Planned regrouping, three repos with one job each (nothing has moved yet):` →
  `Regrouping into three repos with one job each (agentstacks has moved; airsstack and airsdk have not):`.
- Lines 92-94 become exactly:

  ```text
  ├── agentstacks/      agent tool setup: packs + extensions, claudevs   (pre-release)
  ├── airsstack/        controller CLI          ┐ planned: fresh private repos,
  └── airsdk/           provider SDKs           ┘ no code yet
  ```
- Projects table, the agentstacks row: name `[**agentstacks**](https://github.com/airsstack/agentstacks) (from claudestacks)`,
  status `Pre-release`, Docs `[Book](https://airsstack.github.io/agentstacks/)` (the column's shape
  for projects with a book, P50), text
  unchanged except `claudevs moves here.` → `Hosts claudevs.`.
- The claudestacks row: text `Archived: its plugins became the agentstacks packs and claudevs moved to agentstacks; it still hosts airsclaude until airsdk.`,
  status `Archived`, Docs unchanged.
- The airsl row: `Powers buildl and the claudestacks hooks.` → `Powers buildl and the agentstacks hooks.`
- Line 116: `claudestacks hooks ► airsl` → `agentstacks hooks ─► airsl` (same width).
- Line 156: ``Every change moves through `claudestacks-sdlc`:`` → ``Every change moves through the agentstacks `sdlc` pack:``.

`CLAUDE.md` (org root). Its map states agentstacks' status ("no code yet"), which is the same
status fact the intent's org-docs outcome covers:

- Line 24: `claudestacks/   airsclaude SDK, claudevs plugin test engine, Claude Code plugin suite` →
  `claudestacks/   archived; still hosts the airsclaude SDK until airsdk`.
- Lines 26-28 become exactly:

  ```text
  agentstacks/    agent tool setup, packs + extensions, claudevs   pre-release (0.1.0)
  airsstack/      controller CLI (planned, kubectl-style)  ┐ fresh private repos, created
  airsdk/         provider SDKs (AI first, cloud later)    ┘ 2026-11-20, no code yet
  ```

### 8.2 claudestacks (applied by the author, or a session opened in the claudestacks repository, on a branch)

This work is not in this chain's plans: it lands in another public repository through its own
pull request.

- `README.md`: a banner first, under the title: `> **Archived.** The Claude Code plugins are now
  agentstacks packs and claudevs lives in agentstacks: https://github.com/airsstack/agentstacks.
  Migrating: https://airsstack.github.io/agentstacks/ch03-00-migration.html. airsclaude stays
  here until it moves to airsdk.`
- The book's claudevs part points to its new home. This blockquote goes under the `# Getting
  Started` title of `book/src/ch05-00-claudevs-getting-started.md`, and above line 53
  (`Start with [claudevs Getting Started](…)`) of `book/src/introduction.md`, each followed by a
  blank line:
  `> **claudevs moved to agentstacks.** Its book is now
  > https://airsstack.github.io/agentstacks/ch08-00-claudevs-getting-started.html. The claudevs
  > chapters here describe claudevs 0.1.1 and are no longer updated.`
  claudevs 0.1.1 is the last claudestacks release (P7). Check before merging: `mdbook build book`
  exits 0 in claudestacks.
- `install/install.sh` becomes a forwarder, with the same prefix (`claudevs-install`) and helpers:
  - `CLAUDEVS_VERSION` unset, empty or `latest`: say `claudevs moved to https://github.com/airsstack/agentstacks; running https://airsstack.github.io/agentstacks/claudevs.sh`,
    download that script to the temp directory and run it with `sh <file> "$@"`. The environment,
    `CLAUDEVS_VERSION` included, passes through.
  - an explicit version: the current claudestacks download (`claudevs-cli-v<version>` from
    claudestacks releases, which has 0.1.0 and 0.1.1). If that download fails, forward as above.
  - a bad version: today's text, unchanged:
    `CLAUDEVS_VERSION='<value>' is not a version (expected something like 0.1.0)`
    (`/Users/hiraq/Projects/airsstack/claudestacks/install/install.sh:54`).
  - the forwarded script fails to download:
    `claudevs-install: error: could not download https://airsstack.github.io/agentstacks/claudevs.sh`, exit 1.
- Check before merging: `sh -n install/install.sh`. After its Pages deploy: §9 step 9's run.
  claudestacks' `pages.yml` deploys on `book/**` and `install/**` (P38), so the one merge
  republishes both.
- It is merged and its Pages deploy has finished **before** the archive. An archived repository is
  read-only (P19), so nothing can redeploy its Pages afterwards.

### 8.3 Archive

At §9 step 10: Settings → Archive, or `gh repo archive airsstack/claudestacks`. Never delete it.
Its releases, the marketplace and the Pages site stay reachable (P19). airsclaude is frozen until
airsdk; a fix needs an unarchive, a release and a re-archive (decided with the author 2026-11-24).

## 9. Release runbook (the author's steps, in order)

Nothing here is a plan task; the plans deliver the files. RELEASING.md holds this list. Binaries
release before crates.io, because dist builds from the repository and every step before step 6 can
be undone.

1. Release pull request (this chain's work) green: `cargo make dod`, `cargo make claudevs-check`,
   `cargo deny check`, the `installers` and `book` jobs, and the release workflow's plan job. In the pull
   request, replace every `Unreleased` in the five CHANGELOGs with the day you will run steps 3-6.
   Merge.
2. Settings → Pages → Source: GitHub Actions. Run `pages.yml` (workflow_dispatch). Check that
   `https://airsstack.github.io/agentstacks/`, `…/ch03-00-migration.html`, `…/install.sh` and
   `…/claudevs.sh` serve 200. The crate READMEs published at step 6 link into the book, so it is
   up before then.
3. On the merged commit, push `packs-v0.1.0`.
4. Push `agentstacks-cli-v0.1.0` and wait for its release workflow to finish green; then the same
   for `claudevs-cli-v0.1.2`. **If one fails**, nothing was published (`host` waits for smoke,
   P13). Delete any GitHub Release it left, then delete the tag (`git tag -d <tag>`,
   `git push origin :refs/tags/<tag>`), fix it in a pull request, and tag the same version on the
   new merged commit. The version is still free, because crates.io comes later.
5. Verify, in one subshell with a throwaway home and a minimal `PATH`:

   ```sh
   (
     export HOME="$(mktemp -d)" PATH=/usr/bin:/bin
     curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh    # prints M9: no airsl here
     curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh
     export PATH="$HOME/.local/bin:$PATH"
     agentstacks --version    # agentstacks 0.1.0
     claudevs --version       # claudevs 0.1.2
     airsl --version          # airsl 0.1.5 or newer
     cd "$(mktemp -d)" && agentstacks init --tool claude --pack core --yes \
       && grep 'tag = "packs-v0.1.0"' agentstacks.toml && agentstacks doctor    # healthy
   )
   ```

   A failure here is fixed like step 4's: nothing is on crates.io yet.
6. Publish to crates.io, one crate at a time, from a clean, up-to-date `main`:
   `cargo publish --workspace --dry-run`, then ✋ `cargo publish -p agentstacks`,
   `cargo publish -p agentstacks-cli`, `cargo publish -p claudevs`, `cargo publish -p claudevs-cli`.
   Each command polls until its crate is in the index. On a timeout, confirm the version on
   crates.io before the next command (P36). **If a publish fails**, the crates before it stay
   published, and you continue from the failed crate once the cause is fixed. A fix that changes
   that crate's code needs a new version: bump it, and for a `-cli` crate release that version
   again through step 4.
7. Push `agentstacks-v0.1.0` and `claudevs-v0.1.1`, each alone.
8. claudestacks: apply §8.2 (README banner, book pointer, forwarder) on a branch, merge, and wait
   for its Pages deploy.
9. Check the forwarder: `( export HOME="$(mktemp -d)" PATH=/usr/bin:/bin; curl -LsSf https://airsstack.github.io/claudestacks/install.sh | sh && "$HOME/.local/bin/claudevs" --version )`
   prints `claudevs 0.1.2`.
10. Archive claudestacks (§8.3).
11. Commit the org docs (§8.1) in the org root and push the `.github` repository.
12. `/claudestacks-sdlc:milestone foundation/01` to roll the milestone up to done.

## 10. Edge cases (settled)

| Case | Behaviour |
|---|---|
| `<VAR>` empty, or `latest` | latest lookup (`${VAR:-latest}`, as airsl does, P33) |
| `<VAR>=v0.1.0` | one leading `v` stripped → `0.1.0` (P33) |
| `<VAR>` with a space, `/`, or only `v` | M2 (P33) |
| `<VAR>` is not a released version | M7 |
| Pre-release tags (`…-v0.2.0-rc.1`) | never picked as latest (the grep wants plain `X.Y.Z`, P18); installable by naming them (P33) |
| No curl and no wget | M1 only (§5.2 step 1; checked by the `installers` job) |
| API rate limit hit, or the API unreachable | M4, which names `<VAR>` as the way around it (P17) |
| No release of that app yet | M5 |
| More than 100 releases | only the first page is read, and the API states no order (P37), so the newest could be missed. Accepted at today's scale; RELEASING.md says to revisit at 100 releases |
| airsl on `PATH`, or at `~/.local/bin/airsl` | left alone (M8), even if older than 0.1.5; `plan` names the version it needs (P4) |
| airsl only in a directory off `PATH` other than `~/.local/bin` | not found; airsl's installer puts a second copy in its default directory (M9) |
| `AGENTSTACKS_CLI_INSTALL_DIR` set | agentstacks goes there (P35). The airsl check is unchanged, and airsl's installer uses its own default unless `AIRSL_CLI_INSTALL_DIR` is set too |
| Running install.sh again | agentstacks reinstalled by dist's installer; airsl found (M8) |
| agentstacks installer fails | the script exits with its status; airsl is not attempted |
| `cargo install agentstacks-cli` | no airsl. README and the crate README say so (§2) |
| A binary tag whose version differs from its manifest | dist's plan fails (`The provided announcement tag (agentstacks-cli-v0.1.9) claims we're releasing agentstacks-cli 0.1.9, but that …`, exit 255; `probes/spec-examples-results.md` §1) and nothing is published. Recover as in §9 step 4 |
| Release workflow or smoke fails on a binary tag | nothing is published (`host` needs smoke, P13); §9 step 4's recovery, before crates.io |
| `cargo publish` fails or times out partway | §9 step 6: published crates stay; continue from the failed crate |
| `packs-v*`, `agentstacks-v*`, `claudevs-v*` pushed | no release workflow run (§4.2 trigger, P10) |
| More than 3 tags in one push | no events (P16), so the runbook pushes one at a time |
| Pull request | the release workflow runs plan only and passes (P12, P13). Builds and smoke run on the tag |
| Windows | no installer or archive (§4.1). Not claimed to build |
| claudestacks installer with an explicit old version | the claudestacks release (0.1.0, 0.1.1); newer versions forward to agentstacks (§8.2) |
| Plugins and packs both active | duplicate skills and hooks. The migration doc removes the plugins first (§7.3 step 3) |
| README example without `--pack core` | refused, because `sdlc` requires `core` (P32). Every example in §7.2, §7.3 and §7.6 lists `core` with `sdlc` |
| A chapter in `SUMMARY.md` without its file | the build fails, exit 101 (P41); the `book` job and Pages fail |
| A pack chapter's include target missing (a pack renamed or its README removed) | mdBook exits 0 with an `[ERROR]` line (P42); the §7.7 step fails on it (P43) |
| A pack README changes | Pages redeploys (`packs/*/README.md` trigger, P49) |
| A relative link added to a pack README later | it breaks in the book only; no CI check catches it (§13). `CLAUDE.md` states the absolute-link rule (§7.5) |
| A broken internal link in a chapter | mdBook does not report it (P54). Execute checks every relative href of the built book once (§12); there is no permanent link checker (§13) |
| The old raw URL `…/main/docs/authoring.md` | 404 once the move merges; README, CLAUDE.md and the chapter itself name `book/src/ch06-00-authoring.md` |
| The old claudestacks book's claudevs pages | still served after the archive (P19), with the §8.2 pointer at their entry page and in the introduction |
| `pages.yml` runs on the merge before the Pages source is set | that run fails at deploy; §9 step 2's `workflow_dispatch` run replaces it |

## 11. Architecture (approved by the author on 2026-11-24)

No Rust module is added, moved or changed. The crate layering is untouched; its contract moves
from `docs/architecture.md` to `book/src/ch07-00-architecture.md` with only its links renamed
(§7.6). The rust-guidelines references were read for this spec. The rules applied come from
`workspace.md`: members inherit `[workspace.package]`; publishable members carry a `description`
and their own `README.md`; workspace dependencies carry `version` and `path`; publish dependency
first. `modularity.md`, `mod-rs-export-only.md`, `static-dispatch.md` and `unit-test-mandate.md`
have nothing to apply, since no source file changes. `doc-comment-discipline.md` keeps planning
paths out of the book chapters as it does out of source (§7.3, §7.6).

```text
agentstacks/  (this repo)
├─ Cargo.toml                    publish = true; workspace dep versions (§2)
├─ crates/agentstacks{,-cli}/    + README.md + CHANGELOG.md; readme/keywords/categories (§2, §3)
├─ crates/claudevs{,-cli}/       version bump + CHANGELOG entry; README install line and book links (§2)
│  └─ claudevs/docs/             deleted, with its exclude line (§2)
├─ packs/CHANGELOG.md            packs-v releases (§3)
├─ packs/*/README.md             relative links made absolute (§7.6)
├─ dist-workspace.toml           §4.1
├─ .github/workflows/
│  ├─ release.yml               dist-generated + the trigger edit (§4.2)
│  ├─ smoke.yml                 §6
│  ├─ pages.yml                 book + installers (§5.1, §7.7)
│  └─ ci.yml                    + installers job, + book job (§7.1, §7.7)
├─ install/install.sh, claudevs.sh    §5
├─ book/                         §7.6
│  ├─ book.toml, mermaid.min.js, mermaid-init.js
│  └─ src/  SUMMARY.md + 40 chapters
│     ├─ introduction, ch01-ch03, appendix-00     new (ch03 = §7.3)
│     ├─ ch02-01..05                             {{#include}} of packs/<pack>/README.md
│     ├─ ch04-ch07                               ◄ docs/*.md (git mv, links renamed)
│     └─ ch08-ch12, appendix-01                  ◄ claudestacks book claudevs part
├─ docs/                         deleted (§7.6)
├─ Makefile.toml                 + book-mdbook, book-mermaid, book, book-build (§7.6)
├─ .gitignore                    + book/book/ (§7.6)
├─ RELEASING.md                  §7.4, §9
└─ README.md, CLAUDE.md          §7.2, §7.5

outside: org root docs (§8.1: edited by a plan, committed by the author after the archive) ·
         claudestacks README banner, book pointer, install.sh forwarder (§8.2, author) ·
         archive (§8.3, author)
```

Dependency direction is unchanged: `agentstacks-cli → agentstacks`, `claudevs-cli → claudevs`,
and both libraries → `airsl` from crates.io. The book depends on `packs/*/README.md` through its
includes, and nothing depends on the book.

## 12. Testing

| What | How |
|---|---|
| Workspace still green | `cargo make dod` (includes `cargo make lua`, which runs the pack text gates over `packs/CHANGELOG.md`), `cargo make claudevs-check`, `cargo deny check` |
| Manifests publishable | `cargo publish --workspace --dry-run` exits 0 (P8; rerun at §9 step 6) |
| dist config | the release workflow's plan job on the pull request (P12) |
| Archives | smoke.yml on every binary tag, before publishing (§6) |
| Installer scripts | the `installers` job (§7.1: parse, M1 first, M2); end to end at §9 step 5 |
| Book builds | the `book` job and `pages.yml` (§7.7 step: mdBook exit 0 and no `[ERROR]` line, P43). `cargo make book-build` is for local iteration only; it does not fail on an `[ERROR]` line |
| Book links | once at execute: build the book, then for every page except `print.html`, `toc.html` and `404.html`, every `href` that is not `http:`, `https:`, `mailto:` or `//` must name an existing file or directory under `book/book/`, and its anchor must be an `id` in that file, as `probes/scripts/book-probe.sh` does (843 checked, 0 broken, P39); 0 broken. Plus `grep -c -i claudestacks` over the moved claudevs chapters prints 0 (P46) |
| Moves are moves | the six `docs/` files by `git mv`: `git diff -M --stat` shows each as a rename, and its diff is only the docs link map, the title line and (authoring) the raw URLs. The 22 claudevs files: each `git -C <claudestacks> show e099300:book/src/<old name>`, piped through the §7.6 claudevs map, equals the new file (`cmp`), as `probes/scripts/book-probe6.sh` section f does (P57) |

The design-time probes of this spec's exact texts (the §4.1 file and §4.2 trigger with `dist plan`,
the §2 metadata with the publish dry run, and `packs/CHANGELOG.md` with both pack text gates) are
in `probes/spec-examples-results.md`; the book's are in `probes/book-results.md` and
`probes/book-structure-results.txt`.

## 13. Non-goals

- Windows archives or a PowerShell installer.
- Installing, pinning or upgrading airsl beyond §5.3; that belongs to `airsstack tool install`.
- An automated release command or a `/release` skill; the author runs §9.
- A permanent link checker for the book (mdBook reports no broken link, P54; the org's other
  books run none, P55).
- Rewriting the moved reference chapters (ch04-ch07) or the claudevs chapters beyond the §7.6
  edits. A tutorial rewrite was declined on 2026-11-24.
- Changing the claudestacks book beyond the §8.2 pointer, or redirecting its old URLs.
- Migration tooling beyond the documented steps (milestone non-goal).
- Moving airsclaude or openrouter-rs (airsdk).
- Changing `TAG_PREFIX` or any Rust code.
- Converting `claudestacks-cmux`.
