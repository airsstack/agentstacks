---
status: done
created: 2026-11-23
depends-on: [03, 05]
built-on: 0f89fdb3ebb3
---

# Guideline packs Implementation Plan

**Goal:** Ship the two guideline plugins as the packs `guideline-rust` and `guideline-python`.

**Architecture:**
- Each pack holds `pack.toml`, `README.md`, a root `enforcement.json` and one skill, `guidelines`
  (spec §6.1). There is no Lua and no hook, so the packs need no airsl (spec §3 "Work order": only
  packs holding `.lua` files take `requires.airsl`).
- Both packs ship a skill named `guidelines`. That needs plan 03 (no cross-pack name clash rule)
  and plan 05 (rendered as `/guideline-rust-guidelines` and `/guideline-python-guidelines`, two
  paths).
- `enforcement.json` is a `render` file, so `{{skill guidelines}}` in it becomes
  `/guideline-<stack>-guidelines` at install (spec §1: `render` replaces `{{skill …}}` in a root
  JSON file, `probes/crosspack-results.md` Q2).

**Tech Stack:** pack format (TOML, Markdown, JSON), the `agentstacks` binary.

**Content authority:** spec §6.1 (layout, mapping, renamed items), §6.2 (guideline manifest),
§6.4 (text conversion, frontmatter), §9 (rename rows), §14.

---

## Files that change

```
packs/guideline-rust/pack.toml                     — create: §6.2 text
packs/guideline-rust/README.md                     — create: the plugin README with the install block replaced
packs/guideline-rust/enforcement.json              — create: the plugin file, skill = "{{skill guidelines}}"
packs/guideline-rust/skills/guidelines/SKILL.md    — create: from skills/rust-guidelines/SKILL.md, frontmatter name guidelines
packs/guideline-rust/skills/guidelines/references/*.md      — create: the 9 reference files, unchanged
packs/guideline-python/pack.toml                   — create: §6.2 text with the python name and description
packs/guideline-python/README.md                   — create: as for rust
packs/guideline-python/enforcement.json            — create: skill = "{{skill guidelines}}"
packs/guideline-python/skills/guidelines/SKILL.md  — create: from skills/python-guidelines/SKILL.md, name guidelines
packs/guideline-python/skills/guidelines/references/*.md    — create: the 13 reference files, unchanged
```

## Order of work

### Task 1 — `guideline-rust`

**Files:**
- Create the `packs/guideline-rust/` files listed above from
  `../claudestacks/plugins/claudestacks-guideline-rust/`. `.claude-plugin/` and `LICENSE` are not
  carried (spec §6.1).

**Change:**
- `pack.toml` is the §6.2 text byte for byte:
  `render = ["enforcement.json"]`, `exclude = ["README.md"]`, `[pack]` with `name =
  "guideline-rust"`, `version = "0.1.0"`, `api = 0`, `description = "Rust engineering rules and the
  Definition of Done gate"`.
- `enforcement.json` keeps `stack`, `detect`, `match` and `phase` as they are; `skill` becomes
  `"{{skill guidelines}}"`.
- `SKILL.md`: frontmatter `name: guidelines`; `description` unchanged; the body is converted by the
  §6.4 table (no plugin-root or namespaced text is in it today).
- `README.md`: the plugin README, except the `## Install` block: its two `/plugin` lines become
  `agentstacks init --tool claude --pack guideline-rust`, and "Install `claudestacks` as well"
  becomes "Add the `core` pack as well". Other text, including old plugin names, stays (spec §6.4:
  READMEs describe the old names on purpose).

**Proof:**
- In a temp git project holding copies of `packs/guideline-rust` and `extensions/claude`,
  `agentstacks init --tool ./extensions/claude --pack ./packs/guideline-rust --yes` exits 0 and
  writes `.claude/skills/guideline-rust-guidelines/SKILL.md` (frontmatter `name:
  "guideline-rust-guidelines"`), its 9 references, and
  `.claude/agentstacks/guideline-rust/enforcement.json` whose `skill` is
  `"/guideline-rust-guidelines"`. `README.md` is not written.
- `grep -rnE 'CLAUDE_PLUGIN_(ROOT|OPTION)|user_config|claudestacks(-[a-z]+)*:[a-z]' packs/guideline-rust --exclude=README.md`
  prints nothing.

**Done when:** both checks pass and `cargo make dod` is green. Commit
`feat(repo): guideline-rust pack`.

### Task 2 — `guideline-python`

**Files:**
- Create the `packs/guideline-python/` files listed above from
  `../claudestacks/plugins/claudestacks-guideline-python/`.

**Change:** The same as Task 1, with `name = "guideline-python"`, `description = "Python
engineering rules and the Definition of Done gate"`, skill `python-guidelines` → `guidelines`, and
the README install line `agentstacks init --tool claude --pack guideline-python`.

**Proof:**
- The Task 1 check, for this pack: `.claude/skills/guideline-python-guidelines/SKILL.md`, 13
  references, `enforcement.json` `skill` = `"/guideline-python-guidelines"`.
- Both packs together: `agentstacks init --tool ./extensions/claude --pack ./packs/guideline-rust
  --pack ./packs/guideline-python --yes` exits 0 (two packs shipping skill `guidelines`, plan 03).
- The grep of Task 1 over `packs/guideline-python` prints nothing.

**Done when:** both checks pass and `cargo make dod` is green. Commit
`feat(repo): guideline-python pack`.

---

## Risks

- A guideline reference file that holds `{{` would be read as a token. The plugin references hold
  none (spec §6.4 row "A literal `{{` (none today)"); a load error names the line if one appears.
- These packs are checked by hand here; `tests/packs.rs` (plan 15) makes it a test.

## Proof

- `cargo make dod` green; the two-pack `init` in a temp project exits 0.

## Review findings

- 🔵 fixed (follow-up) — `packs/guideline-rust/README.md:26`, `packs/guideline-python/README.md:27`: `See [LICENSE](./LICENSE)` is broken (LICENSE not carried, spec §6.1); the README is excluded from install.
- Verification: in temp git projects holding copies of `packs/` and `extensions/claude`, `agentstacks init --tool ./extensions/claude --pack ./packs/<p> --yes` exit 0 for `guideline-rust`, `guideline-python` and both together; SKILL.md `name: "guideline-rust-guidelines"` / `"guideline-python-guidelines"`; references 9 / 13; `enforcement.json` `"skill": "/guideline-rust-guidelines"` / `"/guideline-python-guidelines"`; no README.md written outside `packs/`/`extensions/`. Plan grep over `packs` (`--exclude=README.md`) exit 1 (no match). `diff -r` of references against the plugin sources prints nothing. Reviewer re-ran the two-pack init: exit 0, 26 files.

## Deviations

- 2026-11-23 — The pack files were created in parallel with plans 05–07 (no cargo, `packs/` only); the `init` proof ran by hand once plan 05 landed.
- 2026-11-23 — Not committed (execute does not commit unless told).

## Follow-up fixes

- 2026-11-23 — Every open finding above was fixed in one follow-up pass (three parallel coders, disjoint files). Workspace `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` exit 0: 23 `test result: ok` blocks, 2474 Rust tests passed, lua `71 passed, 0 failed (5 files)`, `Build Done in 55.65 seconds`.
- Pack READMEs link the root `../../LICENSE` (Apache-2.0).
