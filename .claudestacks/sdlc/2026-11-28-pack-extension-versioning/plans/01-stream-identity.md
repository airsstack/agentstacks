---
status: approved
created: 2026-11-28
built-on: a7f1874ed5fe
---

# Stream Identity Implementation Plan
**Goal:** Resolve every official artifact through its own tag stream.
**Architecture:** Closed prefix table in `types`, major rule generalized beside the existing byte-equality rule, vocabulary pinned in the new book section first.
**Tech Stack:** Rust (agentstacks crate), Markdown (book).

---

## Files that change

```
book/src/ch06-08-versioning.md              — create the scheme section, wired in SUMMARY.md
book/src/ch06-06-stability.md               — modify stream format frozen for 1.x
crates/agentstacks/src/types/shorthand.rs   — modify closed stream-prefix table + major rule
crates/agentstacks/src/types/resolve/tag.rs — modify stream-tag split tests
crates/agentstacks/src/process/git/cli.rs   — modify prefix-filter contract tests
```

## Order of work

### Task 1 — Pin the vocabulary in the book

**Files:** Create `book/src/ch06-08-versioning.md` — the scheme section; Modify `book/src/SUMMARY.md` — wire the section after `ch06-07`; Modify `book/src/ch06-06-stability.md` — stream format frozen for 1.x.

**Change:** The section maps stream tag, manifest version, `agentstacks.toml` pin, `api`, and lock proof with the tag-wins conflict rule, and defines *stream*, *bundle tag*, *frozen*, *seed*; the stability chapter records the frozen prefix format.

**Proof:** written first, seen failing, then passing — `book-build-has-no-error-lines` — `cargo make book-build` exits 0 with no `[ERROR]`; `versioning-section-names-all-five-terms` — grep finds *stream*, *bundle tag*, *frozen*, *seed*, and the tag-wins rule in the new section.

**Done when:** `cargo make book-build` passes and the Definition of Done is green. Commit `docs(book): per-artifact versioning scheme section`.

### Task 2 — Generalize the official prefix rule to eight streams

**Files:** Modify `crates/agentstacks/src/types/shorthand.rs` — closed `STREAM_PREFIXES` table plus the major rule beside `official_major`.

**Change:** New `StreamPrefix` type (name plus kind) listing `packs-core-v`, `packs-sdlc-v`, `packs-journal-v`, `packs-guideline-rust-v`, `packs-guideline-python-v`, `ext-claude-v`, `ext-musecode-v`, `ext-opencode-v`; `official_major`-style function returning `API_MAJOR` for the official URL with a known stream prefix, `None` otherwise; `TAG_PREFIX` untouched so legacy pins keep resolving.

**Proof:** written first, seen failing, then passing — `stream_prefixes_map_all_eight_to_api_major` — each of the eight prefixes with the official URL yields `API_MAJOR`; `unknown_prefix_maps_to_none` — `packs-evil-v` and empty prefix yield `None`; `git_suffix_is_third_party` — official URL plus `.git` yields `None` under every stream prefix; `existing_refusals_still_hold` — the malformed-version refusal and the unknown pack/tool refusal tests pass unchanged.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): per-artifact release stream prefixes`.

### Task 3 — Prove stream-tag splitting and listing

**Files:** Modify `crates/agentstacks/src/types/resolve/tag.rs` — stream-tag split unit tests; Modify `crates/agentstacks/src/process/git/cli.rs` — prefix-filter contract tests.

**Change:** Unit tests split every relied-upon shape (`packs-sdlc-v1.1.0` to prefix `packs-sdlc-v`, version `1.1.0`; legacy `packs-v0.1.0` unchanged; full-prefix anchoring so `packs-core-v` never matches a `packs-core-v-extra-v*` tag); contract tests list tags filtered by a stream prefix against real git.

**Proof:** written first, seen failing, then passing — `splits_every_stream_shape` — each of the eight stream prefixes plus the legacy prefix splits to the expected prefix and version; `prefix_anchoring_rejects_lookalikes` — a lookalike tag is not listed under the shorter prefix; `contract_lists_only_matching_prefix` — real-git listing under one stream prefix excludes the others.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `test(agentstacks): stream-tag split and listing coverage`.

---

## Risks

- The section number assumes the `ch06` part keeps its shape; if a chapter was added there first, renumber the file and its `SUMMARY.md` line together.
- Over-generalizing the prefix rule (e.g. accepting any `packs-*-v`) would admit lookalike third-party streams; the closed table is the mitigation, pinned by the unknown-prefix test.

## Proof

- `cargo make dod-crate agentstacks` green.
- `cargo make book-build` green with the new section linked from the summary.
