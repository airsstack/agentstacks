---
status: approved
created: 2026-11-24
built-on: d3463785d0cb
---

# Release versions Implementation Plan

**Goal:** Make the four crates and the packs tag ready to publish at their release versions.

**Architecture:** Manifests, crate READMEs and changelogs only; no source file changes. The root
`Cargo.toml` turns publishing on and pins the workspace dependency versions; each member carries
its version and crates.io metadata. Crate READMEs link to the agentstacks book by absolute URL,
because crates.io renders them away from the repository. Changelogs live beside what they
describe: one per crate, one for the packs and extensions under `packs/`. Supersedes plan 01.

**Tech Stack:** Cargo manifests (cargo 1.94.1), Markdown.

**Content authority:** spec §2 (versions, metadata, READMEs, the claudevs book links, the
`crates/claudevs/docs/` deletion), §3 (CHANGELOGs, `packs/CHANGELOG.md` text and its text rule),
§7.6 (the claudevs map, for the README links), §10 (`cargo install agentstacks-cli` row), §12
(publish dry run, pack text gates).

---

## Files that change

```
crates/agentstacks/README.md        — create: crates.io page of the library (spec §2)
crates/agentstacks-cli/README.md    — create: crates.io page of the CLI, install + airsl note (spec §2)
crates/claudevs/README.md           — modify: book links → agentstacks book (spec §2, P52)
crates/claudevs-cli/README.md       — modify: line 12 install line; book links → agentstacks book
crates/claudevs/docs/               — delete: nine pointers nothing links to (spec §2, P45)
Cargo.toml                          — modify: publish = true; agentstacks 0.1.0, claudevs 0.1.1 dep versions
crates/agentstacks/Cargo.toml       — modify: version 0.1.0; readme, keywords, categories
crates/agentstacks-cli/Cargo.toml   — modify: version 0.1.0; readme, keywords, categories
crates/claudevs/Cargo.toml          — modify: version 0.1.1; drop the `exclude = ["docs/"]` line
crates/claudevs-cli/Cargo.toml      — modify: version 0.1.2
Cargo.lock                          — modify: the four new versions (cargo rewrites it)
crates/agentstacks/CHANGELOG.md     — create: [0.1.0] - Unreleased
crates/agentstacks-cli/CHANGELOG.md — create: [0.1.0] - Unreleased
crates/claudevs/CHANGELOG.md        — modify: add [0.1.1] - Unreleased
crates/claudevs-cli/CHANGELOG.md    — modify: add [0.1.2] - Unreleased
packs/CHANGELOG.md                  — create: the spec §3 text, exactly
```

## Order of work

### Task 1 — Crate READMEs

**Files:**
- Create `crates/agentstacks/README.md` — what the library is, that it backs the `agentstacks`
  CLI, that its API is unstable during `0.x`, and a link to the book's Under the Hood chapter,
  `https://airsstack.github.io/agentstacks/ch07-00-architecture.html`.
- Create `crates/agentstacks-cli/README.md` — install with
  `curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh` (it also installs airsl when
  missing), or with `cargo install agentstacks-cli`, followed by the note "this route does not
  install airsl, which `plan` needs (≥0.1.5)"; usage (`init --tool claude --pack core`, `plan`,
  `apply`, `doctor`, `update`); links to the repository README
  (`https://github.com/airsstack/agentstacks#readme`), the book
  (`https://airsstack.github.io/agentstacks/`) and its migration chapter
  (`https://airsstack.github.io/agentstacks/ch03-00-migration.html`).
- Modify `crates/claudevs-cli/README.md` — line 12 becomes exactly
  `curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh   # prebuilt binary, no Rust needed`.
- Modify `crates/claudevs/README.md` (lines 47, 49, 51, 53, 55, 57) and
  `crates/claudevs-cli/README.md` (lines 36-42) — each book link's prefix
  `https://airsstack.github.io/claudestacks/` becomes `https://airsstack.github.io/agentstacks/`,
  and its chapter file is renamed by the spec §7.6 claudevs map (`ch05-` → `ch08-`, `ch06-` →
  `ch09-`, `ch07-` → `ch10-`, `ch08-` → `ch11-`, `ch09-` → `ch12-`, one rule per link, never
  chained; `appendix-01-claudevs-cli.html` keeps its name). Link texts stay.

**Change:** Each published crate gets its crates.io page, and the claudevs pages point at the book
that now holds claudevs' documentation. Every link in the two new READMEs is an absolute URL.

**Proof:** checks run first and seen failing before the files exist or change:
- `readme-links-absolute` — `grep -noE '\]\([^)]*\)' crates/agentstacks/README.md crates/agentstacks-cli/README.md | grep -v '](https://'`
  prints nothing.
- `cli-readme-airsl-note` — `crates/agentstacks-cli/README.md` contains both the install.sh
  one-liner and the sentence saying `cargo install` does not install airsl.
- `claudevs-readme-install` — `sed -n 12p crates/claudevs-cli/README.md` prints the spec §2 line
  exactly.
- `claudevs-book-links` — `grep -c 'airsstack.github.io/claudestacks' crates/claudevs/README.md crates/claudevs-cli/README.md`
  prints 0 for both. The rewritten links are exactly: `ch08-00-claudevs-getting-started.html`,
  `ch09-00-writing-cases.html`, `ch10-00-check-gate.html`, `ch11-00-embedding.html`,
  `ch12-00-claudevs-under-the-hood.html`, `appendix-01-claudevs-cli.html` in `claudevs`, and
  `ch08-01-installation.html`, `ch08-00-claudevs-getting-started.html`,
  `ch09-00-writing-cases.html`, `ch10-00-check-gate.html`, `ch10-03-ci.html`,
  `appendix-01-claudevs-cli.html` in `claudevs-cli`, each under
  `https://airsstack.github.io/agentstacks/`; each names a file of the spec §7.6 `SUMMARY.md`.

**Done when:** the four checks pass and `cargo make dod` is green. Commit
`docs(repo): crates.io READMEs, claudevs links to the agentstacks book`.

### Task 2 — Versions, publishing and crates.io metadata

**Files:**
- Modify `Cargo.toml` — `[workspace.package] publish = true`; in `[workspace.dependencies]`,
  `agentstacks = { path = "crates/agentstacks", version = "0.1.0" }` and
  `claudevs = { path = "crates/claudevs", version = "0.1.1" }`.
- Modify `crates/agentstacks/Cargo.toml` — `version = "0.1.0"`; add `readme = "README.md"`,
  `keywords = ["agents", "claude-code", "cli", "lua", "setup"]`,
  `categories = ["development-tools"]`.
- Modify `crates/agentstacks-cli/Cargo.toml` — `version = "0.1.0"`; add `readme = "README.md"`,
  `keywords = ["agents", "claude-code", "cli", "lua", "setup"]`,
  `categories = ["command-line-utilities", "development-tools"]`.
- Modify `crates/claudevs/Cargo.toml` — `version = "0.1.1"`; remove the line
  `exclude    = ["docs/"]` (line 13); nothing else.
- Delete `crates/claudevs/docs/` (all nine files, P45) with `git rm -r`.
- Modify `crates/claudevs-cli/Cargo.toml` — `version = "0.1.2"`; nothing else.
- Modify `Cargo.lock` — regenerated by cargo for the four versions; no dependency changes.

**Change:** The workspace becomes publishable at the release versions of spec §2, with the
metadata copied from the spec §2 table exactly. `agentstacks --version` reads `CARGO_PKG_VERSION`
(`crates/agentstacks-cli/tests/version.rs:21`), so it prints `agentstacks 0.1.0` with no source
change. The claudevs package loses an `exclude` that would match nothing once `docs/` is gone.

**Proof:** checks run first and seen failing (publish is `false`, versions are old, `docs/` exists):
- `publish-dry-run` — `cargo publish --workspace --dry-run --allow-dirty` exits 0 and packages
  `agentstacks v0.1.0`, `agentstacks-cli v0.1.0`, `claudevs v0.1.1`, `claudevs-cli v0.1.2`,
  with no warning except `aborting upload due to dry run` (spec P8).
- `version-output` — `cargo run -q -p agentstacks-cli -- --version` prints `agentstacks 0.1.0`, and
  `cargo run -q -p claudevs-cli -- --version` prints `claudevs 0.1.2`.
- `claudevs-package-no-docs` — `test ! -e crates/claudevs/docs`, `grep -c 'docs/' crates/claudevs/Cargo.toml`
  prints 0, and `cargo package -p claudevs --list --allow-dirty | grep -c '^docs/'` prints 0.
- `workspace-graph` — `cargo tree -d` is saved before the version bump and again after;
  `cargo metadata --format-version 1 > /dev/null` exits 0, and the two `cargo tree -d` outputs
  differ only in the four bumped versions (rust-guidelines `workspace.md` DoD additions).

**Done when:** the four checks pass and `cargo make dod` is green. Commit
`build(repo): release versions and crates.io metadata`.

### Task 3 — Crate changelogs

**Files:**
- Create `crates/agentstacks/CHANGELOG.md` — header in the shape of
  `crates/claudevs/CHANGELOG.md:1-5` naming `agentstacks`; `## [0.1.0] - Unreleased`: initial
  release, minimum supported Rust 1.94; `### Added`: the pack, extension, project and lock formats;
  the plan, apply, doctor and update flows; the extension host and the hook runtime.
- Create `crates/agentstacks-cli/CHANGELOG.md` — header naming `agentstacks-cli` (the
  `agentstacks` binary); `## [0.1.0] - Unreleased`: initial release; `### Added`: `init`, `plan`,
  `apply`, `doctor`, `update`; official names `--tool claude` and
  `--pack core|sdlc|journal|guideline-rust|guideline-python`, resolved to the newest `packs-v*`
  tag; prebuilt archives for the six targets and the installer
  `curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh`, which also installs airsl
  when it is missing.
- Modify `crates/claudevs/CHANGELOG.md` — new top entry `## [0.1.1] - Unreleased`;
  `### Changed`: the source moved to https://github.com/airsstack/agentstacks, and the
  documentation to the agentstacks book,
  https://airsstack.github.io/agentstacks/ch08-00-claudevs-getting-started.html; behaviour is
  unchanged. No citation in the entry.
- Modify `crates/claudevs-cli/CHANGELOG.md` — new top entry `## [0.1.2] - Unreleased`, built on
  `claudevs` 0.1.1; `### Changed`: built and released from https://github.com/airsstack/agentstacks;
  the prebuilt installer is `curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh`.
  The 0.1.2 entry names no claudestacks URL (spec §3 entry text).

**Change:** Every released crate carries its release notes in Keep a Changelog 1.1.0, dated
`Unreleased`; the author dates them in the release pull request (spec §9 step 1). If a crate's
`CHANGELOG.md` is missing from its package (`changelog-packaged`), its manifest gets an `include`
that adds it, reported as a deviation.

**Proof:** checks run first and seen failing:
- `changelog-entries` — `grep -n '^## \[0.1.0\] - Unreleased$'` matches in both agentstacks
  changelogs, `'^## \[0.1.1\] - Unreleased$'` in claudevs', `'^## \[0.1.2\] - Unreleased$'` in
  claudevs-cli's, each as the first `## [` heading of its file.
- `changelog-packaged` — `cargo package -p agentstacks --list --allow-dirty` lists `CHANGELOG.md`
  (and the same for `agentstacks-cli`).
- `no-claudestacks-url` — the 0.1.2 entry of `crates/claudevs-cli/CHANGELOG.md` contains no
  `claudestacks`, and the 0.1.1 entry of `crates/claudevs/CHANGELOG.md` names
  `https://airsstack.github.io/agentstacks/ch08-00-claudevs-getting-started.html`.

**Done when:** the three checks pass and `cargo make dod` is green. Commit
`docs(repo): changelogs for the first agentstacks release`.

### Task 4 — Packs changelog

**Files:**
- Create `packs/CHANGELOG.md` — exactly the `markdown` block of spec §3, byte for byte.

**Change:** The `packs-v0.1.0` tag gets its release notes, linking the book's migration chapter.
The file sits beside the pack directories, so no pack installs it. It names no skill, agent,
command or plugin (spec §3 text rule), so the pack text gates (`Makefile.toml:173,181`) keep
passing (spec P51).

**Proof:** checks run first and seen failing (the file is absent):
- `packs-changelog-exact` — the first column-0 ```` ```markdown ```` block of
  `.claudestacks/sdlc/2026-11-20-release-and-migration/spec.md` (the spec §3 block), extracted
  with the `block markdown` awk function of `probes/scripts/spec-probe.sh`, compares equal to
  `packs/CHANGELOG.md` with `cmp`.
- `pack-text-gates` — `cargo make lua` exits 0 with the file present.

**Done when:** both checks pass and `cargo make dod` is green. Commit
`docs(packs): changelog for packs-v0.1.0`.

---

## Risks

- Task 2 rewrites `Cargo.lock`. If cargo resolves anything beyond the four version bumps, the
  `cargo tree -d` check shows it; nothing may be upgraded here.
- The `readme` fields (Task 2) name files Task 1 creates. Task order matters: a manifest naming a
  missing README fails `cargo package`.
- The README and CHANGELOG book links answer 404 until the book (plan 10) is merged and the author
  sets the Pages source (spec §9 step 2), which comes before the crates.io publish (step 6).
- Nothing here publishes. `cargo publish` without `--dry-run` is the author's step (spec §9 step 6).

## Proof

- `cargo make dod` green (includes the pack text gates).
- `cargo publish --workspace --dry-run --allow-dirty` exits 0 with the four versions.
- `cargo make claudevs-check` and `cargo deny check` green.
