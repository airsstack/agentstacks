---
status: done
created: 2026-11-27
built-on: 543ad491d3e5
---

# Cache Home Implementation Plan

**Goal:** Honour `AIRSSTACK_HOME` for the git cache with a discoverable remedy on failure

**Architecture:** Root resolved once in `types/resolve/cache.rs`; `url_dir` composes from the root; adapters pass env values in

**Tech Stack:** Rust

## Files that change

```
crates/agentstacks/src/types/resolve/cache.rs — modify root rule and url_dir
crates/agentstacks/src/types/resolve/error.rs — modify HomeUnset text, CacheIo remedy
crates/agentstacks/src/fs/cache.rs            — modify Cache holds resolved root
crates/agentstacks/src/process/git/cli.rs     — modify from_env reads both vars
```

## Order of work

### Task 1 — Root rule and error texts

**Files:**
- Modify `crates/agentstacks/src/types/resolve/cache.rs` — root resolution from `AIRSSTACK_HOME`/`HOME`
- Modify `crates/agentstacks/src/types/resolve/error.rs` — messages

**Change:** `cache_home(airsstack_home, home)` returns the root: non-empty `AIRSSTACK_HOME`, else `<HOME>/.airsstack`, else `HomeUnset` with text `AIRSSTACK_HOME and HOME are not set, so the cache (~/.airsstack/cache) has no home`. `url_dir(root, url) = <root>/cache/git/<hex>`. `CacheIo` display gains ` (set AIRSSTACK_HOME to a writable directory to move the cache)`.

**Proof:**
- `cache_home_prefers_airsstack_home` — override wins, empty treated unset
- `cache_home_falls_back_to_home_dot_airsstack` — HOME branch composes root
- `cache_io_display_names_the_remedy` — suffix exact

**Done when:** `cargo test -p agentstacks types::resolve` passes and the Definition of Done is green. Commit `feat(agentstacks): AIRSSTACK_HOME cache root`.

### Task 2 — Adapter wiring

**Files:**
- Modify `crates/agentstacks/src/fs/cache.rs` — `Cache::from_env(airsstack, home)` holds root
- Modify `crates/agentstacks/src/process/git/cli.rs` — `from_env` reads both vars, tests updated to root layout

**Change:** Thread both env values; cached layout becomes `<root>/cache/git/<hex>/<rev>.git` (same bytes location as before under HOME).

**Proof:**
- existing `fs::cache` and `process::git` tests updated to the root layout and passing

**Done when:** `cargo test -p agentstacks` passes (excluding the known sandbox socket failure) and the Definition of Done is green. Commit `feat(agentstacks): git cache under resolved root`.

---

## Risks

- Existing caches under the old composition must resolve to the same path under HOME; the HOME-branch test pins this.

## Proof

- `cargo make dod-crate agentstacks` green except the pre-existing sandbox socket failure recorded in the spec.

---

## Review findings

- Self-review (inline): Resolved root (AIRSSTACK_HOME wins, empty=unset), url_dir from root, HomeUnset reworded, CacheIo remedy suffix. Verified: cargo test -p agentstacks --lib 1863 passed (1 known sandbox failure). Fixed on review: fmt, clippy `map_or_else`/`String::len`, test `panic!` removal.

## Deviations

- Executed inline in this session instead of parallel coder/reviewer subagent spawns; verification is the plan's own tests plus `cargo make dod` sub-gates.
- `cargo make dod` test stage failed in the sandbox only on `fs::disk::tests::a_unix_socket_is_other`: the sandbox blocks `AF_UNIX bind` everywhere (proven with a Python probe in three directories). Re-run outside the sandbox on 2026-11-27 at the author's direction: the test passes and the full core suite is 1864 passed, 0 failed — environmental, no code fix needed. fmt, clippy -D warnings, rustdoc and doctests (312+1) passed.
- Plans 05–07 of the spec (activation hint, init --dry-run, book) were not drafted or executed in this run; the chain stays open.
