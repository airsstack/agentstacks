---
status: done
created: 2026-11-25
depends-on: [07]
built-on: c187d693fbea
---

# Release 1.0 preparation Implementation Plan

**Goal:** The repository is ready for the maintainer to release 1.0.0.

**Architecture:** CHANGELOG entries and `RELEASING.md` only. The versions moved to 1.0.0 in plan 09.
Pushing tags, publishing to crates.io and committing the org docs are the maintainer's steps in
`RELEASING.md`, outside this plan, because they cannot be undone (intent constraint).

**Tech Stack:** Markdown.

**Content authority:** spec §8 (CHANGELOGs, `RELEASING.md`, no change to dist/installers/CI), D8, D14, D16, D17.

---

## Files that change

```
crates/agentstacks/CHANGELOG.md        — modify: 1.0.0 entry (Unreleased)
crates/agentstacks-cli/CHANGELOG.md    — modify: 1.0.0 entry (Unreleased)
packs/CHANGELOG.md                     — modify: 1.0.0 entry (Unreleased)
RELEASING.md                           — modify: the 0.1.0 record titled as such; a Later releases section
```

## Order of work

### Task 1 — CHANGELOGs and the release runbook

**Files:**
- Modify `crates/agentstacks/CHANGELOG.md`, `crates/agentstacks-cli/CHANGELOG.md` — `1.0.0` (Unreleased): the api 1 freeze, `requires.agentstacks`, the 0.x move by `update`, `ext add`, `ext test`, official tags keep the api major (D16)
- Modify `packs/CHANGELOG.md` — `1.0.0` (Unreleased): api 1, every pack and extension at 1.0.0, the D17 headers
- Modify `RELEASING.md` — today's steps keep their text under a heading that marks them as the 0.1.0 record; a new **Later releases** section with spec §8's six steps for `<version>`, including the step-1 gates (`opencode-check` included), the step-2 warning about 0.x binaries, the step-4 verification subshell exactly as spec §8 writes it, and step 5's two `cargo publish` commands

**Change:** The release can be run from `RELEASING.md` alone.

**Proof:**
- The Later releases section holds the six steps of spec §8 in order, and its subshell is spec §8's text with `<version>` (a review check).
- `cargo make book-build` passes (the CHANGELOGs are included in the book where they are today).
- The api-0 gate of plan 01 over the whole spec §8 path list prints only the kept lines listed there.

**Done when:** `cargo make dod` and `cargo make book-build` pass. Commit `docs(repo): 1.0.0 changelogs and a reusable release runbook`.

---

## Risks

- The tags, crates.io publishes and the org-docs commit are not done here. They are RELEASING steps 2–6 of the Later releases section, the maintainer's.

## Proof

- `cargo make dod`, `cargo make claudevs-check`, `cargo make opencode-check`, `cargo deny check` and `cargo make book-build` are green on the branch.
- The spec §8 api-0 gate prints only the kept lines of plan 01.

## Review findings

One `claudestacks:reviewer` over Task 1 (`cargo make dod` exit 0; `cargo make book-build` exit 0, no
`[ERROR]`): 0🔴 4🟡 7🔵 2❓. Every finding went to one fix coder; `cargo make book-build` after the
fix: exit 0, no `[ERROR]`. No release step was run.

- fixed 🟡 `RELEASING.md` Later releases: separate `<cli-version>`, `<packs-version>`, `<lib-version>` with "only when …" steps.
- fixed 🟡 `RELEASING.md` step 3 recovery re-tags `packs-v<packs-version>` when the fix touches `packs/` or `extensions/`; step 4 has a failure path.
- fixed 🟡 `RELEASING.md` step 5 regains the clean `main`, `cargo publish --workspace --dry-run`, crates.io poll check and partial-publish recovery gates.
- fixed 🟡 `RELEASING.md` step 6 names `<milestone>`, not `foundation/03`.
- fixed 🔵 `RELEASING.md`: precise 0.x warning; `curl -fsI` checks of the stability and from-0x pages; "step 6 onwards … step 5 onwards".
- fixed 🔵 `crates/agentstacks-cli/CHANGELOG.md`: coverage wording matches `ch06-06-stability.md`; names `ext add --tag/--rev/--subdir/--yes`, `ext test --pack`.
- fixed 🔵 `crates/agentstacks/CHANGELOG.md`: `ext add` also refuses an api 0 project.
- resolved ❓ step-4 subshell: matches spec §8 command for command; only the version placeholders differ.
- resolved ❓ api-0 gate over the four files: only plan 01's kept lines (1.0.0 move entries, 0.x history in `packs/CHANGELOG.md`).
- recorded 🔵 plan Proof text: see Deviations.

## Deviations

- 2026-11-25: The plan's Proof says the CHANGELOGs "are included in the book where they are today"; nothing under `book/src` includes a CHANGELOG, so `book-build` does not exercise them. Checked by the reviewer instead.
- 2026-11-25: Task 1 ran while plan 07's fix pass was in flight; the files are disjoint.
- 2026-11-25: Plan Proof, run once each after every plan and the chain-wide cross-check: `cargo make dod` 0, `cargo make claudevs-check` 0, `cargo make opencode-check` 0, `cargo deny check` 0, `cargo make book-build` 0 with 0 `[ERROR]` lines.
