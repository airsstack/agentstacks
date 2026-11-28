---
status: approved
created: 2026-11-28
depends-on: [01]
built-on: a7f1874ed5fe
---

# Discovery Flows Implementation Plan
**Goal:** Discover per-stream releases through init, update, ext add, and doctor.
**Architecture:** Flows stay generic over ports in `app`, rules stay in `types`; CLI `show` modules render new notes without deciding anything; chapters ship with the behavior they describe.
**Tech Stack:** Rust (agentstacks and agentstacks-cli crates), Markdown (two chapters).

---

## Files that change

```
crates/agentstacks/src/app/newest.rs         — modify per-stream newest resolution
crates/agentstacks/src/app/init.rs           — modify official shorthand per artifact
crates/agentstacks/src/app/ext/add.rs        — modify official tool resolution per stream
crates/agentstacks/src/app/update.rs         — modify per-source stream listing
crates/agentstacks/src/types/resolve/update.rs — modify frozen-stream note
crates/agentstacks/src/app/doctor.rs         — modify freshness half
crates/agentstacks-cli/src/show/update.rs    — modify render new entries and notes
crates/agentstacks-cli/src/show/doctor.rs    — modify render advisories
book/src/ch04-00-cli.md                      — modify update and doctor semantics
book/src/ch05-00-plan-and-apply.md           — modify prefix rules and migration path
```

## Order of work

### Task 1 — Resolve official names per stream

**Files:** Modify `crates/agentstacks/src/app/newest.rs` — stream-aware newest lookup; Modify `crates/agentstacks/src/app/init.rs` — per-pack stream pins; Modify `crates/agentstacks/src/app/ext/add.rs` — per-tool stream pins.

**Change:** `newest_release` takes the stream prefix plus the major rule from plan 01; official pack and tool shorthands pin the newest tag of their own stream.

**Proof:** written first, seen failing, then passing — `init_pins_each_pack_to_its_own_stream` — FakeGit with `packs-core-v1.0.2` and `packs-sdlc-v1.1.0` pins each source to its own tag; `empty_stream_refuses_with_no_tag_note` — a stream with no tags yields the no-release-tag shape; `prerelease_never_wins_a_stream` — a `v2.0.0-rc.1` tag loses to stable `v1.9.0`; `third_party_skips_major_filter` — a `.git`-suffixed URL listing official-looking tags resolves without the major filter; `lookalike_prefix_does_not_match` — prefix `packs-core-v` never lists a `packs-core-v-extra-v1.0.0` tag.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): resolve official artifacts per stream`.

### Task 2 — List per-source streams in update

**Files:** Modify `crates/agentstacks/src/app/update.rs` — per-source prefix derivation; Modify `crates/agentstacks/src/types/resolve/update.rs` — frozen-stream note type with doctests pinning its exact text.

**Change:** Each git source lists its own stream; a legacy `packs-v*` pin yields the frozen note naming its artifact stream and never rewrites.

**Proof:** written first, seen failing, then passing — `mixed_pins_resolve_independently` — one bundle pin and one stream pin produce a frozen note and a newer-tag entry respectively; `frozen_note_text_is_pinned` — doctest asserts the exact prescribed note.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): per-stream update with frozen-bundle notes`.

### Task 3 — Add the advisory freshness half to doctor

**Files:** Modify `crates/agentstacks/src/app/doctor.rs` — read-only freshness over locked git entries.

**Change:** Split each locked tag to prefix and version, list the prefix, report newer tags as advisories; missing network, git, or remote degrades to `could not check <name>`; exit status unchanged by advisories.

**Proof:** written first, seen failing, then passing — `stale_stream_reports_advisory_at_exit_zero` — FakeGit with a newer stream tag yields the note and no finding; `offline_doctor_behaves_as_today` — git-missing FakeGit yields only `could not check` notes; `drift_still_fails` — a changed file still exits 1; `path_source_reports_nothing` — a path-sourced entry yields no freshness output; `rev_pin_reports_pinned_note` — a rev-pinned entry yields the pinned note, not a split; `versionless_tag_reports_no_version_note` — a tag with no parseable version yields the no-version note; `empty_stream_reports_no_tag_advisory` — a listed stream with zero tags yields the no-release-tag shape as advisory; `missing_pinned_tag_reports_not_found` — a pinned tag absent upstream yields the not-found advisory.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): advisory freshness in doctor`.

### Task 4 — Render the new output and document it

**Files:** Modify `crates/agentstacks-cli/src/show/update.rs` — render stream entries and frozen notes; Modify `crates/agentstacks-cli/src/show/doctor.rs` — render advisories; Modify `book/src/ch04-00-cli.md` — update and doctor semantics plus exit codes; Modify `book/src/ch05-00-plan-and-apply.md` — prefix rules, frozen warning, migration path.

**Change:** CLI prints the new entries, notes, and advisories with no logic of its own; the two chapters describe exactly the shipped behavior.

**Proof:** written first, seen failing, then passing — `update_shows_frozen_note_verbatim` — CLI test prints the prescribed frozen note; `doctor_shows_advisory_without_finding` — CLI test prints the advisory and exits 0; `book-build-has-no-error-lines` — `cargo make book-build` exits 0.

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit `feat(cli): render per-stream discovery output`.

---

## Risks

- The freshness half touches `doctor`'s offline contract; the riskier step is Task 3, mitigated by the degraded-mode tests running before any network code.
- New note texts proliferate across `show` modules; the doctest-pinning in Task 2 is the single source they render.

## Proof

- `cargo make dod` green across both crates.
- `cargo make book-build` green with updated chapters.
