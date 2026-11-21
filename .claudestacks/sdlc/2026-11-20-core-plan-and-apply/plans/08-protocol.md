---
status: draft
created: 2026-11-22
depends-on: [01]
built-on: ea104af7451e
---

# Protocol Implementation Plan

**Goal:** build the hook command line and runtime.

**Architecture:** pure values in `types/protocol` (spec §2 table row `types/protocol`). `HookModule` turns
a hook's `run` into the module name airsl's `require` resolves from `<root>/runtime.lua` (§11.1).
`HookCommand` builds the exact argv of §11.2 from already-rendered values; rendering tokens and
quoting the argv for a tool are not done here (§11.2, §16). `RUNTIME_LUA` is the text of
`crates/agentstacks/assets/runtime.lua`, compiled in with `include_str!`, and that asset is the §11.3
block byte for byte (905 bytes, the same bytes as `probes/runtime-runtime.lua`). One integration test
runs the shipped runtime with the argv `HookCommand` builds on the real `airsl` binary when present,
as §14's `runtime.lua` row asks. §11.4 and §11.5 are a contract for extensions (documented by plan
17); nothing in them is built.

**Tech Stack:** Rust 2024, `crates/agentstacks` only; existing types `HookRun` (`src/types/hooks/run.rs`),
`PackName`, `EnvName`, `BinaryName` (`src/types/name/*`), `SubPath` (`src/types/project/path/sub.rs`),
`ContentHash`, `PackManifest` (to build hooks in tests); the `airsl` binary 0.1.4 for the
integration test only; no new dependency.

**Content authority:** spec §11.1, §11.2, §11.3 (with `probes/runtime-runtime.lua`,
`probes/runtime-results.txt`, `probes/runtime.sh`, `probes/runtime-translate.lua`), §14 row
`runtime.lua`.

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6): `mod-rs-export-only.md`
(`types/protocol/mod.rs` holds only docs, `mod` and `pub use`); `modularity.md` Rule 1 (module name,
kind, blocking, command line and runtime text each in one file); `strong-types.md` (`may_block` is
the two-variant `Blocking` enum, not a `bool` parameter; kind is `HookKind`; the root is a
`SubPath`; env and exec grants keep `EnvName` / `BinaryName`); `unit-test-mandate.md` (colocated
tests in every logic file; the integration test complements them); `doc-comment-discipline.md`;
`static-dispatch.md` (no trait objects).

---

## Files that change

```
crates/agentstacks/assets/runtime.lua                   — create: the §11.3 text, byte for byte
crates/agentstacks/src/types/protocol/mod.rs            — modify: add mod / pub use lines (docs stay)
crates/agentstacks/src/types/protocol/runtime.rs        — create: RUNTIME_FILE, RUNTIME_LUA
crates/agentstacks/src/types/protocol/kind.rs           — create: HookKind
crates/agentstacks/src/types/protocol/blocking.rs       — create: Blocking
crates/agentstacks/src/types/protocol/module.rs         — create: HookModule
crates/agentstacks/src/types/protocol/command.rs        — create: HookCommand and its argv
crates/agentstacks/tests/runtime.rs                     — create: the runtime on the airsl binary
crates/agentstacks/tests/fixtures/runtime/…             — create: a project with translate.lua and four hook modules
```

## Order of work

### Task 1 — Ship `runtime.lua`

**Files:**
- Create `crates/agentstacks/assets/runtime.lua` — a byte-for-byte copy of
  `.claudestacks/sdlc/2026-11-20-core-plan-and-apply/probes/runtime-runtime.lua` (made with `cp`, never
  retyped).
- Create `crates/agentstacks/src/types/protocol/runtime.rs` — the constants.
- Modify `crates/agentstacks/src/types/protocol/mod.rs` — `mod runtime;` plus `pub use`.

**Change:** `RUNTIME_FILE` is `runtime.lua`, the file name under `<root>` (§11.1). `RUNTIME_LUA` is
`include_str!("../../../assets/runtime.lua")`: the text the core writes to `<root>/runtime.lua`
(§3.1 core write row, D7; plan 13 makes the write). Its doc comment states what the runtime does
(§11.3: loads `translate`, reads `arg[1..3]`, builds a neutral or native event, drops a `block` from
a hook not declared `may_block` with one stderr line, prints `translate.result`) without naming the
spec.

**Proof:**
- `runtime_lua_equals_the_probed_text` — `RUNTIME_LUA` equals
  `include_str!` of `../../../../../.claudestacks/sdlc/2026-11-20-core-plan-and-apply/probes/runtime-runtime.lua`
  (the file the spec's §11.3 block was checked against, `probes/examples-results.txt`: `runtime.lua`
  equals the file that ran, `true`).
- `runtime_lua_hash_is_pinned` — `ContentHash::of_bytes(RUNTIME_LUA.as_bytes())` displays
  `sha256:4ce3f46b94f471cd15beac1d987207647b082d0399275e0485b64f1f1b2e72f4` and the length is 905
  bytes (measured on the probe file with `shasum -a 256` and `wc -c`; this test survives a move of the
  chain directory).
- `runtime_lua_starts_and_ends_as_shipped` — the text starts with `-- agentstacks hook runtime, api 0.`
  and ends with `end` and one `\n`.

**Done when:** `cmp crates/agentstacks/assets/runtime.lua .claudestacks/sdlc/2026-11-20-core-plan-and-apply/probes/runtime-runtime.lua`
prints nothing, `cargo make dod-crate agentstacks` passes, and the Definition of Done is green.
Commit `feat(agentstacks): ship the hook runtime`.

### Task 2 — Name the hook kind and the blocking flag

**Files:**
- Create `crates/agentstacks/src/types/protocol/kind.rs` — `HookKind`.
- Create `crates/agentstacks/src/types/protocol/blocking.rs` — `Blocking`.
- Modify `crates/agentstacks/src/types/protocol/mod.rs` — `mod kind; mod blocking;` plus `pub use`.

**Change:** `HookKind` is `Neutral` (a `[[hooks]]` lua hook in `pack.toml`) or `Native` (a lua hook
in `native/<tool>/hooks.toml`), with `as_arg()` giving `neutral` / `native` (§11.2 `<kind:
neutral|native>`). `Blocking` is `May` or `MayNot`, with `as_arg()` giving `1` / `0` (§11.2
`<may_block: 0|1>`) and `From<bool>` so `LuaHook::may_block()` converts at the call site.

**Proof:**
- `hook_kind_args` — `Neutral` → `neutral`, `Native` → `native`.
- `blocking_args` — `May` → `1`, `MayNot` → `0`; `Blocking::from(true)` is `May`,
  `Blocking::from(false)` is `MayNot`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): hook kind and blocking arguments`.

### Task 3 — Turn a hook's `run` into its module name

**Files:**
- Create `crates/agentstacks/src/types/protocol/module.rs` — `HookModule`.
- Modify `crates/agentstacks/src/types/protocol/mod.rs` — `mod module;` plus `pub use`.

**Change:** `HookModule::new(owner: &PackName, run: &HookRun) -> HookModule`: the pack is
`run.pack()`, or `owner` for the hook's own pack (pack-relative and `{{root}}/…` forms); the module
is that pack's name, then each segment of `run.path()` with `.lua` removed from the last, joined by
`.` (§11.1: airsl's `require` root is the script's directory, so `{{root core}}/hooks/enforce.lua`
becomes `core.hooks.enforce`; core root files land at `<root>/<pack>/<rel>`, D7). It cannot fail:
every segment is require-able because plan 03's `run` segment rule (§12 row 4) refuses the rest at
pack load. `as_str()` and `Display` give the name.

**Proof:** hooks are built by parsing a small `pack.toml` with `PackManifest::parse`, as the
`HookRun` doctest does.
- `module_from_another_packs_root` — `{{root core}}/hooks/enforce.lua` in pack `team-lint` →
  `core.hooks.enforce` (the §11.1 example).
- `module_from_own_pack_relative` — `hooks/my-file.lua` in pack `team-lint` →
  `team-lint.hooks.my-file` (the module `probes/hookreq-results.md` ran).
- `module_from_own_root_token` — `{{root}}/hooks/a/b.lua` in pack `core` → `core.hooks.a.b`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): hook module name from run`.

### Task 4 — Build the hook command line

**Files:**
- Create `crates/agentstacks/src/types/protocol/command.rs` — `HookCommand`.
- Modify `crates/agentstacks/src/types/protocol/mod.rs` — `mod command;` plus `pub use`.

**Change:** `HookCommand` holds `root: SubPath` (the extension's `values.root`, §4.4), `module:
HookModule`, `blocking: Blocking`, `kind: HookKind`, `args: Vec<String>` (rendered), `read:
Vec<String>` and `write: Vec<String>` (rendered grant paths, project-relative after rendering or
absolute, §11.2 bullet 1), `env: Vec<EnvName>` and `exec: Vec<BinaryName>`. `argv(&self) ->
Vec<String>` returns exactly, in this order (§11.2):

1. `airsl`, `run`, `--fail-open`, `--policy`, `confined` (fail-open is fixed, formats §6.3; `pure` is
   refused at pack load in api 0, D14, so the policy is always `confined`);
2. one `--allow-read`, `<p>` pair per read path, then `--allow-write` pairs, then `--allow-env`
   pairs, then `--allow-exec` pairs, each group in declared order, one flag per grant (one to one,
   formats §6.3);
3. `<normalised root>/runtime.lua` (`SubPath::normalized` joined with `RUNTIME_FILE`);
4. the module, the blocking argument, the kind argument;
5. the args, in order.

Values pass through unchanged: an element holding a `{{var}}` value such as `$TMPDIR` stays as text;
turning the argv into the tool's command string and setting the working directory are the
extension's (§11.2 bullets 2 and 5). The same argv is the plan payload's `command` (§4.4); plan 13
fills it.

**Proof:**
- `argv_without_grants` — root `.claude/agentstacks`, module `core.hooks.enforce`, `MayNot`,
  `Neutral`, no args → `airsl run --fail-open --policy confined .claude/agentstacks/runtime.lua core.hooks.enforce 0 neutral`
  as separate elements.
- `argv_grants_in_order` — two read paths, one write, two env, one exec → the pairs appear as
  read, read, write, env, env, exec, before the runtime path.
- `argv_appends_args_after_kind` — args `--manifest`, `.claude/agentstacks/core/enforcement.json`
  come last, in order.
- `argv_may_block_and_native` — `May` and `Native` give `1` and `native`.
- `argv_normalises_the_root` — root `./.tool//agentstacks` → `.tool/agentstacks/runtime.lua`.
- `argv_keeps_var_values_as_text` — a write grant `$TMPDIR` stays `$TMPDIR`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): hook command line`.

### Task 5 — Run the shipped runtime on airsl

**Files:**
- Create `crates/agentstacks/tests/fixtures/runtime/proj/.tool/agentstacks/translate.lua` — a
  byte-for-byte copy of `probes/runtime-translate.lua` (made with `cp`).
- Create `crates/agentstacks/tests/fixtures/runtime/proj/.tool/agentstacks/core/data.txt` — `hello\n`.
- Create `crates/agentstacks/tests/fixtures/runtime/proj/.tool/agentstacks/core/hooks/{ctx,blk,raw,both}.lua`
  — four hook modules, described below.
- Create `crates/agentstacks/tests/runtime.rs` — the integration test.

**Change:** the test copies the fixture project into a `tempfile` directory, writes `RUNTIME_LUA`
to `.tool/agentstacks/runtime.lua` there, builds each command with `HookCommand::argv` (root
`.tool/agentstacks`), and runs it with `std::process::Command` from the project directory (the
working-directory duty of §11.2 bullet 2), piping the payload
`{"hook_event_name":"PreToolUse","session_id":"s1","cwd":"/p","tool_name":"Edit"}` on stdin (as
`probes/runtime.sh`). When `airsl --version` cannot run, each test prints a reason and returns
without failing (§14 `runtime.lua` row: "when the binary is present"). Fixture modules, each
returning one function of the event:
- `ctx.lua` — reads `.tool/agentstacks/core/data.txt` with `airsstack.fs.read` under `pcall` and
  returns `context` = `event=<event.event> session=<event.session_id> args=<args joined by ,> read=<ok>:<data>`;
- `blk.lua` — returns `block` = `edits to <event.event> are not allowed`;
- `raw.lua` — returns `context` = `native payload tool_name=<payload.tool_name> event=<event.event>`;
- `both.lua` — returns `context` = `keep this context` and `block` = `and drop this block`.

**Proof:** each expected output is the §11.3 table's or §11.2's, from `probes/runtime-results.txt`.
- `runtime_context_with_read_grant` — `--allow-read .tool/agentstacks/core`, module `core.hooks.ctx`,
  args `--manifest x.json` → stdout is
  `{"context":"event=tool.before session=s1 args=--manifest,x.json read=true:hello\n"}` (the `\n`
  there is JSON's two-character escape of the file's line break, as the probe printed it) followed by
  one real line break from the translator; exit 0 (§11.2 bullet 3, first run; §11.3 row "context").
- `runtime_read_without_grant_is_denied` — same without the grant → the context holds
  `read=false:fs.read denied:` (§11.2 bullet 3, second run).
- `runtime_drops_block_when_not_may_block` — `core.hooks.blk`, `MayNot` → stdout empty; stderr is
  `agentstacks: hook core.hooks.blk tried to block but is not declared may_block; reason: edits to tool.before are not allowed`
  plus a line break (§11.3 row 2).
- `runtime_passes_block_when_may_block` — `May` → stdout
  `{"decision":"block","reason":"edits to tool.before are not allowed"}` (§11.3 row 3).
- `runtime_native_gets_the_raw_payload` — `core.hooks.raw`, `Native` → stdout
  `{"context":"native payload tool_name=Edit event=nil"}` (§11.3 row 4).
- `runtime_keeps_context_and_drops_block` — `core.hooks.both`, `MayNot` → stdout
  `{"context":"keep this context"}`, the block line on stderr (§11.3 row 5).
- `runtime_untranslated_event_is_silent` — payload `{"hook_event_name":"Notification","session_id":"s1","cwd":"/p"}`,
  `core.hooks.ctx` → stdout empty, exit 0 (§11.3 row 6).
- `runtime_missing_module_fails_open` — module `core.hooks.missing` → stdout empty, exit 0 (§11.2
  bullet 4, sixth run).

**Done when:** `cargo test -p agentstacks --test runtime` passes (with `airsl` 0.1.4 on `PATH`) and
`cargo make dod` passes (end of plan); the Definition of Done is green. Commit
`test(agentstacks): run the hook runtime on airsl`.

---

## Risks

- The asset must stay byte-identical. A checkout with `core.autocrlf=true` on Windows would turn its
  `\n` into `\r\n` and fail `runtime_lua_hash_is_pinned`; the repo has no `.gitattributes` today.
  Adding `crates/agentstacks/assets/*.lua -text` is a repo-level change outside this plan's files;
  execute reports it if CI on Windows is added.
- `runtime_lua_equals_the_probed_text` reads a file outside the crate. The crate has
  `publish = false`, so packaging is not affected; if the chain directory is ever archived, that one
  test moves to the hash pin alone.
- `tests/runtime.rs` is new and not listed in the plan-set map; it is the only home for §14's
  `runtime.lua` row and depends only on this plan's types.
- `tests/runtime.rs` uses `std::process` and `std::fs`; `tests/isolation.rs` checks `src/` only, so
  this is allowed. The test never touches the network.
- The test outputs depend on airsl's JSON encoder printing a one-key object as `{"key":"value"}`;
  that is what airsl 0.1.4 printed in the probe. A future airsl may change it, and claudestacks'
  history shows unpinned airsl releases break Lua CI; the skip-when-absent rule does not guard
  against a different version, so the test prints the `airsl --version` it ran.
- Option not taken: building `HookCommand` from a `LuaHook` directly. Rendering of `args` and grant
  paths happens in the flow with the `values` result (plan 13), so the builder takes rendered text.

## Proof

- `cargo make dod` is green at the end of the plan.
- `cmp` of the asset with `probes/runtime-runtime.lua` prints nothing, and
  `shasum -a 256 crates/agentstacks/assets/runtime.lua` prints
  `4ce3f46b94f471cd15beac1d987207647b082d0399275e0485b64f1f1b2e72f4`.
- Spec rows covered, each by a named test above: §11.1 layout file name and module rule with its
  example (Tasks 1, 3); §11.2 argv order, fixed flags, one-to-one grants, `may_block` and `kind`
  arguments, args, `{{var}}` values kept, relative grant resolution from the project directory, and
  fail-open on a missing module (Tasks 2, 4, 5); §11.3 text byte for byte (Task 1) and its six table
  rows (Task 5); §14 `runtime.lua` row (Task 5). §11.4 and §11.5 are extension documentation (plan
  17).
- `grep -rn "spec\|chain" crates/agentstacks/src/types/protocol` finds nothing in doc comments.
