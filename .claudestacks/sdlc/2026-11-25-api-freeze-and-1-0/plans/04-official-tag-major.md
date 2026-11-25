---
status: done
created: 2026-11-25
depends-on: [03]
built-on: c187d693fbea
---

# Official tags keep the api major Implementation Plan

**Goal:** The official `packs-v` tag chosen by `init` and `update` always has the major version `API`.

**Architecture:** `types/resolve/tag.rs` `latest_tag` gains an optional required major. The rule
"official" (D19, an exact match with `OFFICIAL_REPOSITORY` and the prefix `packs-v`) lives in
`types/resolve/update.rs` for update queries, and the shorthand of `init` (`app/init.rs`) passes
`API` as the major. `ext add <official>` reuses the shorthand in plan 05. Third-party sources are
unchanged.

**Tech Stack:** Rust (agentstacks crate).

**Content authority:** spec D16, D19, §5.4 last row ("no packs-v1 release tag", also for `init`), E25, E27, §12 rows `official_packs_tag_keeps_api_major`, `third_party_tags_unchanged`, `dot_git_url_is_third_party`.

---

## Files that change

```
crates/agentstacks/src/types/resolve/tag.rs      — modify: latest_tag takes an optional required major
crates/agentstacks/src/types/resolve/update.rs   — modify: an UpdateQuery for an official packs-v source asks for major API
crates/agentstacks/src/types/project/error.rs    — modify: NoReleaseTag is given the prefix packs-v<API> by init
crates/agentstacks/src/app/init.rs               — modify: the official shorthand asks for major API
crates/agentstacks/src/app/update.rs             — modify: passes the source URL into the query
```

## Order of work

### Task 1 — latest_tag with a required major

**Files:**
- Modify `crates/agentstacks/src/types/resolve/tag.rs` — `latest_tag(tags, prefix, major: Option<u64>)`; with `Some(m)`, tags whose version major is not `m` are ignored; doc and doctest updated

**Change:** The tag rule can keep to one major. With `None` it is today's rule.

**Proof:**
- `major_filter_ignores_other_majors` — tags `packs-v1.4.0`, `packs-v2.0.0`, `packs-v0.2.0` with `Some(1)` give `packs-v1.4.0`.
- `no_major_is_todays_rule` — the same tags with `None` give `packs-v2.0.0`.
- `no_tag_of_that_major_is_none` — only `packs-v0.2.0` with `Some(1)` gives `None`.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): latest_tag can keep to one major version`.

### Task 2 — Official sources keep the api major

**Files:**
- Modify `crates/agentstacks/src/types/resolve/update.rs` — an `UpdateQuery` carries the source URL; when the URL equals `OFFICIAL_REPOSITORY` byte for byte and the prefix is `packs-v`, the latest tag is asked with `Some(API as major)`
- Modify `crates/agentstacks/src/app/update.rs` — builds each query with its URL
- Modify `crates/agentstacks/src/app/init.rs` — the official shorthand asks for major `API`
- Modify `crates/agentstacks/src/app/init.rs` and `crates/agentstacks/src/types/project/error.rs` — `ProjectError::NoReleaseTag` (today built at `app/init.rs` `official`) is given the prefix `packs-v1` (`TAG_PREFIX` plus `API`), so it reads `no packs-v1 release tag in https://github.com/airsstack/agentstacks`

**Change:** D16 and D19. A 1.x binary never moves to `packs-v2.*`, and 1.0 moves `packs-v0.2.0` to the newest `packs-v1.*`.

**Proof:**
- `official_packs_tag_keeps_api_major` — an official source at `packs-v0.2.0` with remote tags up to `packs-v2.0.0` updates to the newest `packs-v1.*`.
- `third_party_tags_unchanged` — a non-official URL with `packs-v*` tags takes the newest overall.
- `dot_git_url_is_third_party` — `https://github.com/airsstack/agentstacks.git` is third-party (E27).
- `init_official_picks_api_major` — `init --pack core` against `FakeGit` tags `packs-v1.0.0` and `packs-v2.0.0` writes `tag = "packs-v1.0.0"`.
- `init_no_major_tag_refusal` — only `packs-v0.2.0` remote: `--pack "core": no packs-v1 release tag in https://github.com/airsstack/agentstacks`.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): official packs-v tags keep the api major`.

---

## Risks

- `latest_tag`'s signature changes, and every caller and its doctest must pass `None` or a major. The compiler finds them all.
- Existing init tests with only `packs-v0.*` remote tags now hit the no-tag refusal. They must move to `packs-v1.*` tags or assert the new refusal.

## Proof

- `cargo make dod` is green.

## Review findings

One `claudestacks:reviewer` over tasks 1–2 (`cargo make dod` exit 0): 0🔴 1🟡 9🔵, none blocking. Every
code finding went to one fix coder; `cargo make dod` after the fix: exit 0 (651 passed, 0 failed).

- fixed 🟡 `book/src/ch02-00-packs.md:21`: `update` now said to move to the newest release of the api it speaks (`packs-v1.*`).
- fixed 🔵 `book/src/ch01-02-first-project.md:96`: "a newer `packs-v1.*` tag".
- fixed 🔵 `book/src/ch05-00-plan-and-apply.md:332`: `latest_tag(tags, prefix, major)`.
- fixed 🔵 `crates/agentstacks/src/app/init.rs`: module and `init` docs rewrapped to 100.
- fixed 🔵 `crates/agentstacks/src/types/project/error.rs:62`: `NoReleaseTag` doc names prefix and major.
- fixed 🔵 `crates/agentstacks/src/types/shorthand.rs`: module doc names the official source; choosing among tags is `types/resolve`.
- fixed 🔵 `crates/agentstacks/src/types/shorthand.rs:72`: `API.unsigned_abs()` replaced by `types/api.rs` `API_MAJOR` (compile-time `API >= 0`), test `api_major_is_the_api`.
- recorded 🔵 plan drift (official rule placement, untouched files, book edits): see Deviations.

## Deviations

- 2026-11-25: The rule "official" is `types/shorthand.rs` `official_major` (next to `OFFICIAL_REPOSITORY` and `TAG_PREFIX`), not `types/resolve/update.rs`. Both `init` (app) and `update` (types/resolve) need it; in `resolve/update.rs` `app/init.rs` would have imported from the update rule or copied it. Layering holds (a rule in `types/`). The reviewer judged the placement correct.
- 2026-11-25: `app/update.rs` and `types/project/error.rs` did not need code changes: `UpdateQuery` already carried `url`, and `NoReleaseTag.prefix` is a `String` given `release_prefix()` = `packs-v1`. `error.rs` changed only in a doc line (review fix).
- 2026-11-25: The book lines on how the official tag is chosen (`ch04-00-cli.md`, `ch05-00-plan-and-apply.md`, `ch01-02`, `ch02-00`) were updated in this plan, because CLAUDE.md requires a chapter to change with the code. Plan 07 builds on them.
- 2026-11-25: Task 1 coder ran `cargo make dod` itself (exit 0) and the reviewer re-ran it (exit 0); the orchestrator did not run a third copy for the batch.
