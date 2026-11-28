---
status: approved
created: 2026-11-28
depends-on: [02, 03]
built-on: a7f1874ed5fe
---

# Prose and History Implementation Plan
**Goal:** Give every artifact its own changelog and bring the book and runbook onto streams.
**Architecture:** Colocated per-artifact changelogs replace the shared bundle file; chapters change only where behavior changed; the runbook drops its bundle placeholder.
**Tech Stack:** Markdown.

---

## Files that change

```
packs/core/CHANGELOG.md                       — create with 1.0.1 baseline entry
packs/sdlc/CHANGELOG.md                       — create with 1.1.0 seed entry
packs/journal/CHANGELOG.md                    — create with 1.0.0 baseline entry
packs/guideline-rust/CHANGELOG.md             — create with 1.0.0 baseline entry
packs/guideline-python/CHANGELOG.md           — create with 1.0.0 baseline entry
extensions/claude/CHANGELOG.md                — create with 1.1.0 baseline entry
extensions/musecode/CHANGELOG.md              — create with 1.1.0 baseline entry
extensions/opencode/CHANGELOG.md              — create with 1.1.0 baseline entry
packs/CHANGELOG.md                            — modify freeze note pointing at successors
book/src/ch02-00-packs.md                     — modify streams replace the bundle tag
book/src/ch06-01-extensions.md                — modify extension version rules
book/src/ch06-04-pack-authoring.md            — modify version and changelog rules
book/src/ch06-05-pack-format.md               — modify version field meaning
book/src/ch01-02-first-project.md             — modify tag examples to streams
book/src/ch03-01-from-0x.md                   — modify migration path to streams
book/src/introduction.md                      — modify release pointer
RELEASING.md                                  — modify per-artifact placeholders and solo-push discipline
```

## Order of work

### Task 1 — Seed the eight changelogs and freeze the bundle file

**Files:** Create the eight `CHANGELOG.md` files — baseline or seed entries; Modify `packs/CHANGELOG.md` — freeze note.

**Change:** Seven baselines carry current numbers forward; `packs/sdlc/CHANGELOG.md` honestly describes `1.1.0` (chain-root content, first per-stream release); the shared file gains a top note pointing at the eight successors with no history rewritten.

**Proof:** written first, seen failing, then passing — `every_artifact_has_a_changelog` — all eight paths exist with a top heading matching the stipulated seed version; `bundle_file_names_successors` — grep finds all eight successor paths in the freeze note.

**Done when:** `cargo make book-build` passes and the Definition of Done is green. Commit `docs(packs): per-artifact changelogs`.

### Task 2 — Move the remaining chapters onto streams

**Files:** Modify `book/src/ch02-00-packs.md` — streams replace the bundle tag; Modify `book/src/ch06-01-extensions.md` — extension version rules; Modify `book/src/ch06-04-pack-authoring.md` — version-means-change plus changelog rules; Modify `book/src/ch06-05-pack-format.md` — version field meaning; Modify `book/src/ch01-02-first-project.md` — stream tag examples; Modify `book/src/ch03-01-from-0x.md` — bundle-to-stream migration; Modify `book/src/introduction.md` — release pointer.

**Change:** No chapter claims a bundle release anymore; the extension and pack authoring chapters plus the pack format reference state bump-iff-changed with the shipped-scope definition; the migration chapter gives the retag path.

**Proof:** written first, seen failing, then passing — `no_chapter_claims_bundle_releases` — grep finds no "released together as `packs-v*`" outside the frozen-stream explanation; `book-build-has-no-error-lines` — `cargo make book-build` exits 0.

**Done when:** `cargo make book-build` passes and the Definition of Done is green. Commit `docs(book): chapters onto per-artifact streams`.

### Task 3 — Rewrite the runbook placeholders

**Files:** Modify `RELEASING.md` — per-artifact versions, solo-push discipline per stream, dispatch workflow in the steps.

**Change:** `<packs-version>` becomes per-artifact versions with the seed enumeration; the tag-push rule is stated per stream; the verification subshell stays, pointed at streams.

**Proof:** written first, seen failing, then passing — `no_bundle_placeholder_remains` — grep finds no `<packs-version>`; `runbook_names_dispatch_inputs` — grep finds the `artifact` and `version` inputs.

**Done when:** the greps pass and the Definition of Done is green. Commit `docs(repo): runbook for per-artifact releases`.

---

## Risks

- The sdlc `1.1.0` seed entry must describe versioning and distribution only, never re-document #27's migration; the task's entry text is reviewed against that line.
- A stale `packs-v*` reference hiding in an unlisted chapter breaks the migration story; the Task 2 grep runs over the whole `book/src` tree, not just the listed files.

## Proof

- `cargo make book-build` green.
- Every grep assertion green.
