---
status: done
created: 2026-11-22
built-on: cefc42fbbc4c
---

# Runtime payload Implementation Plan

**Goal:** Pass the hook payload to `translate.result` in the core's `runtime.lua`.

**Architecture:**
- This is a one-line change to the shipped asset, `crates/agentstacks/assets/runtime.lua`, which is
  embedded as `RUNTIME_LUA` (`src/types/protocol/runtime.rs:27`).
- The pinned-hash test and the runtime fixture follow it.
- The translator contract in `docs/hooks.md` gains the `payload` argument.

**Tech Stack:** Lua under airsl 0.1.4; Rust tests in `crates/agentstacks`.

**Content authority:** spec D17, §8.4, §12 (runtime row), §13 (docs row: `docs/hooks.md`).

---

## Files that change

```
crates/agentstacks/assets/runtime.lua                                    — modify: translate.result(result, payload)
crates/agentstacks/src/types/protocol/runtime.rs                         — modify: the pinned hash of the new bytes
crates/agentstacks/tests/fixtures/runtime/proj/.tool/agentstacks/translate.lua — modify: M.result reads the payload's hook_event_name
crates/agentstacks/tests/runtime.rs                                      — modify: a native case that shows the event reaches result
docs/hooks.md                                                            — modify: runtime text, translator contract
```

## Order of work

### Task 1 — `runtime.lua` passes the payload

**Files:**
- Modify `crates/agentstacks/tests/fixtures/runtime/proj/.tool/agentstacks/translate.lua` —
  `M.result(r, p)` adds `event = p.hook_event_name` to its output when `p` is a table. The output of
  the existing cases changes only by that field.
- Modify `crates/agentstacks/tests/runtime.rs`:
  - add `runtime_native_result_sees_the_payload_event`;
  - update the expected output of existing cases for the new field.
- Modify `crates/agentstacks/assets/runtime.lua` — `translate.result(result)` becomes
  `translate.result(result, payload)`.
- Modify `crates/agentstacks/src/types/protocol/runtime.rs` — in `runtime_lua_hash_is_pinned`
  (`:35`), the pinned hash and the length check (`RUNTIME_LUA.len()`, `:40`) become the new bytes' values.

**Change:** A translator now knows the event for neutral and native hooks alike. The proven output
is `{"context":"ctx","event":"PreToolUse"}` (`probes/runtime-payload-results.md`).

**Proof:**
- `runtime_native_result_sees_the_payload_event` (new; airsl binary, skipped with a reason when
  absent, as the file's other tests are): a native hook returning `{context="ctx"}` on a
  `PreToolUse` payload prints `{"context":"ctx","event":"PreToolUse"}`.
- `runtime_context_with_read_grant` and the other existing `tests/runtime.rs` cases (updated):
  their output carries the payload's event.
- `runtime_lua_hash_is_pinned` (updated) and `runtime_lua_starts_and_ends_as_shipped`: they pass on
  the new bytes.

**Done when:** `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks --test runtime` and
`cargo test -p agentstacks types::protocol` pass, and `cargo make dod` is green. Commit
`feat(agentstacks): pass the hook payload to translate.result`.

### Task 2 — Document the translator contract

**Files:**
- Modify `docs/hooks.md`:
  - §4 runtime full text (`:101`): `translate.result(result, payload)`;
  - §6 translator contract (`:153`): `M.result(result or nil, payload)` returns the string to print
    or `nil`, and `payload` is the raw payload for both kinds;
  - one sentence on why: the payload can be read once.

**Change:** The docs match the shipped bytes. The §4 block stays byte for byte equal to
`assets/runtime.lua`.

**Proof:**
- `runtime_lua_starts_and_ends_as_shipped` still passes.
- A diff of the `docs/hooks.md` §4 code block against `crates/agentstacks/assets/runtime.lua` is
  empty, checked by the Done-when command.

**Done when:** `diff <(awk '/^```lua$/{f=1;next}/^```$/{f=0}f' docs/hooks.md) crates/agentstacks/assets/runtime.lua`
prints nothing, and `cargo make dod` is green. Commit
`docs(repo): translator result receives the payload`.

---

## Risks

- **Changed bytes.** Existing projects will see `runtime.lua` as an `update` step on their next
  plan, because its bytes changed. The plan shows this.
- **No airsl binary.** Without the binary, the runtime tests skip. The Done-when command sets
  `AGENTSTACKS_REQUIRE_AIRSL=1` so a skip fails.

## Proof

- `cargo make dod` green.
- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks --test runtime` green.

## Review findings

- No blocking finding.
- 🟡 open: no DoD step keeps the `docs/hooks.md` §4 copy equal to `assets/runtime.lua` (equal today: the awk diff is empty).
- 🔵 open: `tests/runtime.rs:5` counts "five hook modules".

## Deviations

- 2026-11-22 Task 1: new fixture hook `tests/fixtures/runtime/proj/.tool/agentstacks/core/hooks/native_ctx.lua` (returns `{context="ctx"}`) for `runtime_native_result_sees_the_payload_event`.
- 2026-11-22 (process): coders read their task sections straight from this plan file instead of a task-briefer brief; the ledger came from `grep '^### Task'`. Earlier batches were staged in the git index (not committed) so each review saw only its own batch.
