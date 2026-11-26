---
status: done
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

## Review findings

One reviewer pass over Tasks 1-3 (compliant-with-amendments; `blocking:` D1, D2, D9, D10); one fix
coder for D1-D9 and P2.

- 🔴 fixed (D1) — the page said skills only the opencode extension writes show in OpenCode; it now
  says skills only the musecode extension writes (in `.agents/skills/`) do.
- 🟡 fixed (D2) — only `--workspace <root>` from a subdirectory runs hooks there; a plain start in a
  subdirectory loads no project config.
- 🟡 fixed (D9) — OpenCode already shipped in `packs-v0.2.0`; ch01-02, ch04-00, introduction and README
  now say only Muse Code ships with the next `packs-v*` release.
- 🔵 fixed — D3 YAML `invocation: user`; D4 dropped context from neutral `agent.stop`/`session.end`
  stated; D5 invented "J1" label removed; D6 prose reflowed to ≤100 columns; D7 ch06-03 limit matches
  "Several tools"; D8 ch06-01/02 say unstable until frozen as `api = 1` in M3; P2 README M2 line
  restored to match CLAUDE.md; CLAUDE.md `cargo make lua` line names the two check packs.
- 🟡 fixed (D10) — the branch is rebased onto `main` `9ce5239` (`git rebase main`: 1edaa50, 1586178,
  b738e28, c8cf6fe applied clean; the plan 04-06 work conflicted in CLAUDE.md, Makefile.toml,
  README.md, book/src/{SUMMARY,ch01-02-first-project,ch04-00-cli,ch06-01-extensions,ch06-02-hooks,
  introduction}.md and the three CHANGELOGs, each resolved keeping main's 1.0 text plus the musecode
  additions). musecode is ported to `api = 1` as main ported claude/opencode (see Deviations); the
  page is `book/src/ch06-07-musecode.md`, listed after `The 1.x Stability Promise`; the official tool
  lists in ch04-00 (`--tool`, `ext add`), ch06-05 (`native/<tool>/`), introduction, README and
  CLAUDE.md name musecode and keep the "ships with the next `packs-v*` release (`packs-v1.0.0` does
  not include it)" / `--tool ext/musecode` note; the changelog entries sit under `## [Unreleased]`
  above main's 1.0.0 entries. `packs.rs` `every_official_pack_and_extension_is_at_version_one_or_later`
  counts `8` ("five packs and three extensions"). Proof: `grep -rn "api 0\|api = 0" extensions/musecode
  crates/agentstacks/tests crates/agentstacks-cli/tests/musecode-check book/src README.md CLAUDE.md
  *.md crates/*/CHANGELOG.md packs/CHANGELOG.md | grep -i muse` → no line (the other hits are main's
  legacy-0 text); `cargo make dod` → rc 0, `api-0 gate: passed`, `651 passed, 0 failed (25 files)`,
  `Build Done in 35.18 seconds`; `agentstacks ext test extensions/musecode` → `ext test musecode
  1.0.0`, `lua tests: 85 passed, 0 failed (5 files)`, `10 to write, 0 to delete, 0 unchanged, 1
  skipped`, `applied: 10 written, 0 deleted`, `second plan: nothing to change`, `passed`, exit 0;
  `cargo make book-build` → rc 0, 0 `[ERROR]`, `toc.js` lists `6.7. Muse Code`.
- 🔵 fixed — main's 1.0 text replaced the README/introduction `pre-release (0.1.0)` / `packs-v0.1.0`
  lines and the OpenCode "ships with the next packs release" line; the remaining stale lines, in
  `book/src/ch01-01-installation.md` (`AGENTSTACKS_VERSION=0.1.0`, ``prints `agentstacks 0.1.0` ``),
  now say `1.0.0`. `grep -rnE "pre-release|packs-v0\.[12]|ships with the next" README.md
  book/src/introduction.md book/src/ch01-0*.md CLAUDE.md` → only the Muse Code lines and the README
  roadmap history (`M1 … released (0.1.0)`, `M2 … released (packs-v0.2.0)`), which main keeps.
- Verified (before the D10 rebase): `cargo make book-build` → 0 `[ERROR]` lines, `ch06-04-musecode.html` in `toc.js` as 6.4;
  all 14 skip reasons on the page match `extensions/musecode/lib/*.lua` (`grep -F`); no 5-/6-word
  shingle shared with `probes/docs/*.txt` beyond event names; `grep -rn -i "both official\|both
  extensions\|both are built\|both tools" book/src README.md CLAUDE.md` → no line; `cargo make dod`
  exit 0.

### Cross-check (plans 04-06, after all coders finished)

One crate-wide consistency + guideline review (verdict: compliant, `blocking: none`; 1🟡 9🔵), fixed
by the orchestrator:

- 🟡 fixed — the committed MIT Muse Code SDK guide copies under `probes/docs/` carried no licence
  notice; `probes/docs/LICENSE-muse-code-sdk` (upstream `LICENSE`, `gh api
  repos/meta-models/muse-code-sdk/license` → `MIT`) added and named in `SOURCES.md`.
- 🔵 fixed — ch06-04 says a blank `.muse/hooks.json` counts as `{}` and lists H2 among the reasons
  worded as claude's; check.mjs names `museEnvProblem` and prints `FAIL S<n> Keychain:` (no spec id);
  the `removed` fixture's unknown top-level key is a neutral `editor` key, not a Claude shape;
  musecode.rs doc says stdout and stderr are compared; translator/mod.rs `//!` lines ≤100 columns.
- 🔵 kept by design (no change) — `translator::run::Run.kind` is `pub(crate)` and claude.rs keeps its own `native()`
  builder, so the musecode binary has no dead builder; over-100-column lines in the `.mjs` files follow
  opencode-check's existing pattern.
- Final gates (orchestrator, once each): `cargo make dod` → rc 0, Lua `651 passed, 0 failed (25
  files)`, fingerprint gate 8 cases (both hashers), pack text gates 4 passed, `Build Done in 36.23
  seconds`; `cargo test -p agentstacks --test musecode --test claude` → `28 passed` / `23 passed`;
  `cargo make book-build` → rc 0, 0 `[ERROR]`; `cargo make musecode-check` → `ok S1` … `ok S6`,
  `Build Done in 14.06 seconds`, no Keychain text.

## Deviations

- 2026-11-26 — No commit made: the execute instruction did not ask for one.
- 2026-11-26 — Plan 06 ran while plan 05's review was in flight (docs files disjoint from plan 05's).
- 2026-11-26 — The page has a section 9 "The check" (musecode-check) beyond the plan; ch06-03 §7's
  "Both tools" bullet renamed "Several tools"; the journal README states that on Muse Code `capture`
  and `note` run without the transcript and key the note to the first eight characters of the session
  id (verified in `packs/journal/skills/{capture,note}/SKILL.md`); README intro names `.agents/` and
  `.muse/`.
- 2026-11-26 — `.github/profile/README.md` and the org-root workspace `README.md` (CLAUDE.md "How work
  is done") live outside this repository and were not updated.
- 2026-11-26 — The spec's `api = 0` (and its "unstable until `api = 1` in M3" wording, D8 above) is
  superseded by main's api-1 chain, `2026-11-25-api-freeze-and-1-0` (commit `482e429`, its
  `spec.md`): the extension API is frozen at `api = 1`. The musecode extension was ported the same way
  as claude and opencode: `extension.toml` `version = "1.0.0"`, `api = 1`; the translator header (and
  its test and the expected fixture copies) says `api 1`; `values_test.lua` payload `api = 1`; the
  musecode fixtures and the musecode-check pack say `api = 1`; `musecode.rs` asserts version `1.0.0`.
  The spec is not edited.
- 2026-11-26 — The page is `book/src/ch06-07-musecode.md`, not the spec's `ch06-04-musecode.md`:
  main took `ch06-04` … `ch06-06` for the pack authoring, pack format and stability pages.
