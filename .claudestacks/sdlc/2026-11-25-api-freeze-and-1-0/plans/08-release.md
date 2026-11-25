---
status: approved
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
