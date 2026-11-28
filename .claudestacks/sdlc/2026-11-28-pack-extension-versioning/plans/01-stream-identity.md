---
status: done
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
crates/agentstacks/src/types/mod.rs        — modify re-exports for the new stream items
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

---

## Review findings

One agent round against this plan only. All fixed in this plan before close:

| # | Disposition |
|---|---|
| 1 | Applied — section is `ch06-08-versioning.md`, SUMMARY wiring in Files; book task leads. |
| 2 | Applied — new Task 3 owns stream-tag split tests; Task 1 consumes them. |
| 3 | Applied — five doctor sub-case tests named in plan 02 Task 3. |
| 4 | Applied — 03 Task 1 owns both gate modes including no-base equality check. |
| 5 | Applied — exclude-edit and SKILL/adapter/hook fixtures in 03 Task 1. |
| 6 | Applied — cli contract test in 01 Task 3. |
| 7 | Applied — `runbook_corpora_green` in 05 Task 3. |
| 8 | Applied — missing-airsl pointer text prescribed; six refusal tests in 03 Task 3. |
| 9 | Applied — `ch06-01` (plus `ch06-05` under the same-change rule) in 04 Task 2. |
| 10 | Applied — 05 Task 1 owns only the sdlc manifest plus its entry. |
| 11 | Applied — no-base mode stipulation check replaces the contradiction. |
| 12 | Applied — `existing_refusals_still_hold` pins both sides in 01 Task 2. |
| 13 | Applied — book task leads 01; 03 needs no chapter (runbook covers it in 04). |
| 14 | Applied — third-party and lookalike tests in 02 Task 1. |
| 15 | Applied — concrete `python3 -c yaml.safe_load` check in 03 Task 2. |
| 16 | Acknowledged — live proofs ride the plan-05 checkpoints by design. |

`cargo make dod-crate agentstacks` green (unit suite plus doctests, zero failures), `cargo fmt --check` clean, `cargo make book-build` green with the new section wired in.

## Deviations

- 2026-11-28: board renders `Legacy chain detected under .claudestacks/sdlc`; pre-existing condition, carried on per the chain's out-of-scope rule.
- 2026-11-28: reviewer handoff ran through native subagent delivery, not `context-handoff` files; the review round itself happened as specified.
- 2026-11-28: Task 3 characterization tests (split, anchoring, contract) passed on first run — they pin existing generic machinery, so there was no red phase; the genuinely new shorthand code went red-then-green.
- 2026-11-28: closed table ships as `STREAM_PREFIXES` with `Stream` items; `stream_prefix`/`Stream::name` are the accessors plan-02 tasks consume.
