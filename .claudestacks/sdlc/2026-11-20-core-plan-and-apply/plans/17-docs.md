---
status: done
created: 2026-11-22
depends-on: [10, 11, 12, 13, 14, 15, 16]
built-on: f39b2a814319
---

# Contributor docs Implementation Plan

**Goal:** Document the built core for contributors.

**Architecture:** Three new Markdown documents under `docs/`, one per audience, plus the state of
the existing contributor docs. `docs/extensions.md` is for extension authors (the `api = 0` Lua
surface and its duties). `docs/hooks.md` is for pack and extension authors (the neutral hook
protocol and the shared runtime). `docs/plan-and-apply.md` is for core contributors (operations,
steps, apply, lock, doctor, sources, update, binaries). `docs/architecture.md`, `docs/README.md`
and the Status block of `CLAUDE.md` are brought up to the built layout. Each document describes
what the code does at the end of plans 12-16 (`docs/README.md`: "A document describes what the
code does"); where code and spec disagree, the docs task stops and reports it instead of choosing.
No Rust source changes.

**Tech Stack:** Markdown only. The gate is still `cargo make dod`, which must stay green.

**Content authority:** spec §3-§11 (the `api = 0` contract, including the §11.2 working-directory
duty, the §11.4 neutral protocol and translator contract, the §11.5 shims), §12 (format changes an
author meets), §13 (locations), §15 (dependency policy and docs row), §16 (X1-X3, R1-R28, which
override any earlier section where the two meet), §18 (what is not done).

**Writing rules for every document in this plan:**

- Every refusal or output message is copied byte for byte from the spec (backticks, quotes and
  `…` included); none is retyped from memory or invented. Where a §16 rule overrides an earlier
  row, the document uses the §16 text (for example the R24 run-segment message replaces the §12
  row 4 example; R5 says which refusal one call raises). R28 overrides texts too: the cover-after-skip
  shape (R28 (l)) replaces the §3.2 example, the R13 covers refusal carries `write: ` (R28 (m)), and
  the lock-path and `reads` rules follow R28 (e) and (n).
- Every example from a tool is labelled "example: claude extension" (or the tool it is from); the
  core is described in tool-neutral terms (spec §18 "Any tool-specific code in the core").
- No reference to spec sections, rule or decision numbers (`D1`, `R1`, `X1`…), probe files, plans,
  chain names or agents. The documents cite code locations (`crates/agentstacks/src/types/op/`, …)
  and each other instead.
- Fenced `json` and `lua` blocks are copied byte for byte from the spec lines named in each task.

---

## Files that change

```
docs/extensions.md       — create: the api 0 extension contract (load, sandbox, agentstacks table, events, operations, require, json.edit, errors, duties)
docs/hooks.md            — create: hook layout, pack hook rules, command line, runtime.lua, neutral protocol, translator contract, may_block, shims
docs/plan-and-apply.md   — create: operations, plan order, steps, inside-the-project rule, removed tool, apply, lock, doctor, sources, update, binaries, locations
docs/README.md           — modify: link the three documents and set every row's state
docs/architecture.md     — modify: the dependency rule (types → FileTree port, process → fs::cache), the dependency policy note, the "State" column, two examples
CLAUDE.md                — modify: the Status heading, its paragraph and the crates/agentstacks/ line of its text block only
```

## Order of work

### Task 1 — Write the extension guide

**Files:**
- Create `docs/extensions.md` — the `api = 0` contract an extension author codes against.

**Change:** the document holds these sections, in this order, each with the spec content named:

1. **What an extension is and is not** — it only proposes operations; the core validates, `plan`
   shows, apply alone writes; no merge or run-command operation; everything is `api = 0`,
   unstable (spec §1 D1, D3; §18 last bullet).
2. **`extension.toml`** — the optional `reads` list and its five refusals verbatim (spec §12
   row 1); a repeated entry is quoted as written (spec §16 R24); entries follow the lock path rule
   including the separator refusal `extension.toml: extension.reads[0]: "a\b" must use / as the
   separator`, and naming `agentstacks.toml` or `agentstacks.lock` there is allowed, because reading
   them is harmless (spec §16 R28 (n)).
3. **Load** — source resolved, `Extension::load`, `api = 0`, every `.lua` file read into memory,
   Lua never touches disk; a `.lua` file that is not UTF-8 is refused `<tool>/<file>: not valid
   UTF-8` (spec §4.1, §16 R3 last sentence).
4. **Sandbox** — the stdlib set (`Json`, `Ext` with events `values` and `plan`, `agentstacks`);
   what is removed (`print`, `warn`), absent (`require`, `load`, `dofile`, `os`, `io`,
   `coroutine`, other airsl modules) and present; limits (10,000,000 instructions per evaluation
   or dispatch, 16 MiB, reset per dispatch); both breach messages verbatim; a breach caught by the
   extension's own `pcall` is its own business (spec §4.2).
5. **The `agentstacks` table** — the function table with phases and returns (spec §4.3); the four
   `read` rules including the outside-project message; an I/O failure reading a `reads` path is
   `<path>: <io::Error Display>`, reported only when the extension reads that path, and it sticks
   like the other `read` refusals (spec §16 R1, R28 (b)).
6. **Events** — the four-step flow; the three `dispatch` outcomes; a second registration replaces
   the first; the `values` payload `json` block (spec lines 268-270), whose `vars` are the vars this
   tool's packs use, sorted by name (spec §16 R8); the `values` result `lua` block, labelled
   "example: claude extension" (spec lines 276-286); the token table; missing values (whole-skill
   skip, the two skip reasons verbatim, item-level skip for the other kinds, core skip of a root
   file; with several missing values the reason names the first missing token in text order, and a
   missing skill entry comes first, spec §16 R8); JSON type words (object → `table`, array →
   `list`, …; an empty Lua table arrives as an object); the `values` refusal table verbatim plus
   the §16 additions: `values.root` and each `dir` follow the path rows under their key, `.`
   normalises to empty, the managed-file rule does not apply, and the four R4 messages verbatim; a
   `values.skills` entry needs both `invoke` and `dir`, with the R12 message verbatim; the `plan`
   payload `json` block (spec lines 321-332) and its three notes, plus the native hook shape
   (`kind`, `event`, `match` as the list the formats reader holds, `may_block`, and `command` or
   `text`, spec §16 R11, R28 (d)); absent `values.skills`, `agents` or `vars` are accepted, their
   items falling under missing values (spec §16 R28 (h)); the return value of `plan` is ignored; `registers no plan handler`; order: listed
   tools in name order, then removed tools in name order, each listed tool's core writes recorded
   just before its `plan` dispatch, and that order decides `… already proposed by <tool>` (spec
   §4.4, §16 R6).
7. **Operations** — the kinds table (spec §3.1), marking which the extension proposes and which
   the core makes (root files to `<root>/<pack>/…`, `<root>/runtime.lua`, `require_binary` for
   packs and airsl, `delete`); `runtime.lua` is written when a pack of the tool has any lua hook,
   neutral or native, even if the extension later skips it (spec §16 R11); how `by` is derived; a
   shared file has no `by` (spec §3.1, §1 D7).
8. **Item ids and coverage** — the id table and the case table with every message verbatim; ids
   are copied, never built; root files and core-skipped items need no cover; the `translate.lua`
   rule applies when `runtime.lua` is written and at least one lua hook is covered by a write (spec
   §3.2, §16 R11); coverage refusals are ordered by id bytes (`core/hook/10` before `core/hook/2`),
   and "already covered at" names the first covering op (spec §16 R28 (h)); a cover after a skip
   reads `<tool>/<file>:<line>: <function>: covers "<id>", which <file>:<line> skipped`, where a
   `copy`'s `<id>` is its `file` (spec §16 R28 (l), replacing the §3.2 example); `copy.file` must
   name a file id (`copy: file "core/agent/coder" is not a file`) and `covers` may not name a skill
   group (`write: covers[1]: "core/skill/status" is a skill; list its files`) (spec §16 R13,
   R28 (m)).
9. **Validation** — checked at the call; the phase check (`only allowed inside the plan handler`)
   runs before every other check (spec §16 R28 (a)); the rule table with every refusal verbatim; the generic
   argument shapes of spec §16 R3 verbatim (`<fn>: <field>: expected <type>, got <type>`, `…
   expected a list`, `… not valid UTF-8`, `<fn>: keys must be strings, got <type>`, `require:
   expected string, got <type>`); a path that normalises to nothing is `path: empty` (spec §16
   R25); one call raises one refusal, the first failing check in table order, located at the nearest
   Lua frame on the stack, so `pcall(agentstacks.write, t)` is located at the `pcall` line (spec §16
   R5); a call from a required module names that module; the core's own `runtime.lua` as the
   earlier proposer: `path "<p>" is already proposed by core (runtime.lua of claude)` (spec §16 R6);
   normalisation of op paths, `reads` and `values` paths; accepted without refusal, with
   `require_binary` checked once per tool (spec §16 R28 (h)); type words;
   strictness (no coercion, unknown keys refused, `content` keeps raw bytes); op refusals stick under
   `pcall` and the refused op is not recorded (spec §3.3).
10. **`agentstacks.require`** — `require` stays absent; the name rule; name → path mapping; loaded
    once with chunk name `@<path>`, cached; errors carry the module's file and line; the refusal
    table verbatim (its cycle row located at `claude/lib/b.lua:1`, the call that closes the cycle);
    the `(use dots: …)` hint only when the name contains `/`, otherwise `claude/adapter.lua:1:
    require: invalid module name "x y"`; `require` refusals are ordinary Lua errors that do not
    stick, and uncaught they show their located message, not the wrapper; a module that raised is
    not cached (spec §4.5, §16 R19).
11. **`agentstacks.json.edit`** — both tables (rules; Lua value → JSON) with every refusal
    verbatim, plus the §16 R3 shapes for `json.edit` (`text: not valid UTF-8`, `path: expected a
    list`, `value: <type> cannot be JSON`, `value: -inf cannot be JSON`, `value: table contains
    itself`, `value: key is not valid UTF-8`); removing through a parent that exists but is not an
    object refuses like setting (`json.edit: "hooks" is not an object`); whitespace-only text keeps
    its indent and trailing newline; with several failing rows the first row of the table wins
    (spec §16 R14); `json.edit` refusals are ordinary Lua errors that do not stick (spec §16 R19);
    the checked example `lua` block (spec lines 408-409) and its output `json` block (spec lines
    413-426) with the sentence after it; numbers re-printed in serde_json's normal form, the plan's
    diff shows the change; the core knows no config format, a shared file is read (declared in
    `reads`), edited and rewritten whole (spec §5, §1 D2, D9).
12. **Errors and locations** — the error table verbatim (spec §4.6); any extension error refuses
    the whole plan; the three `<tool>/…` location rows of the locations table (spec §13 rows 1-3)
    and the `<extension file>` path rule.
13. **`require_binary`** — the extension row of the binaries table verbatim; a binary required by
    both a pack and an extension is reported once for each (spec §10).
14. **The extension's duties outside Lua** — make the tool run the hook argv with the project
    directory as its working directory (example: claude extension, `cd "$CLAUDE_PROJECT_DIR" &&
    …`), turning the argv into the tool's command string with its own quoting (spec §11.2);
    write `<root>/translate.lua` to the translator contract (spec §11.4, link to `hooks.md`);
    filter by `match` and `agents` in the tool's own config; render message hooks natively (spec
    §11.4); write a shim with `write` when the tool demands another hook language, example:
    OpenCode plugins are TypeScript (spec §11.5).

**Proof:** written first as a checklist, then ticked against the finished file.
- Spec sections covered, each in the section named above: §1 D1-D3, D7, D9; §3.1; §3.2 (both
  tables); §3.3 (table, normalisation, accepted list, type words, strictness, sticky); §4.1; §4.2;
  §4.3 (table and `read` rules); §4.4 (flow, dispatch outcomes, both payloads, values result,
  token table, missing values, type words, values table, plan notes, order); §4.5 (rules and
  table); §4.6; §5 (both tables, checked example); §10 row 2 and "once for each"; §11.2
  working-directory and quoting duties; §11.4 translator contract, filtering, message hooks;
  §11.5; §12 row 1; §13 rows 1-3; §16 R1 (`reads`), R3, R4, R5, R6, R8 (vars, missing token), R11,
  R12, R13, R14, R19, R24 (`reads` quoting), R25, R28 (a), (b), (d), (h), (l), (m), (n); §18 last
  bullet.
- Every message in the file is found verbatim in the spec: for each, `grep -F '<message text>'
  .claudestacks/sdlc/2026-11-20-core-plan-and-apply/spec.md` prints a line (messages holding a
  `<placeholder>` are checked with the placeholder exactly as the spec prints it).
- The five fenced blocks equal the spec lines named in sections 6 and 11, compared with `diff`
  (for example `sed -n '276,286p' .claudestacks/sdlc/2026-11-20-core-plan-and-apply/spec.md` against
  the block's lines).
- `grep -nE '§|probes/|plans/' docs/extensions.md` and `grep -nwE 'spec|[DRX][0-9]+' docs/extensions.md`
  print nothing.

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit
`docs(repo): extension api 0 guide`.

### Task 2 — Write the hook guide

**Files:**
- Create `docs/hooks.md` — how a pack hook runs, from the tool's command to the hook's result.

**Change:** the document holds these sections, in this order:

1. **Layout** — the three files under `<root>` (spec §11.1 `text` block, lines 666-668, with the trailing `(§11.3)`, `(required, §3.2)` and `(D7)` references replaced by plain words: "(below)", "(required)", nothing); who
   writes each (`runtime.lua` the core, `translate.lua` the extension, `<root>/<pack>/hooks/…` the
   core as a root file); airsl's `require` root is the script's directory; how a `run` of
   `{{root core}}/hooks/enforce.lua` becomes the module `core.hooks.enforce` (spec §11.1).
2. **Rules for a pack hook** — `grants.policy = "pure"` refused in `api = 0`, `confined` the only
   allowed value, with both §12 row 3 messages verbatim; each `run` segment must be require-able
   (also in `native/<tool>/hooks.toml`); the run-segment refusal quotes `run` as written, reports
   only the first bad segment and applies only to a `run` ending in `.lua`, with the R24 message
   verbatim (spec §12 rows 3-4, §16 R24, §1 D14).
3. **Command line** — the argv shape (spec §11.2 `text` block, lines 676-678); the core builds it
   with project-relative paths and grants map one to one; the extension's duty to run it with the
   project directory as working directory (example: claude extension, `cd "$CLAUDE_PROJECT_DIR"
   && …`); a project-relative grant resolves against that directory, and without one a read is
   denied with `fs.read denied: … is outside the granted read roots: none are granted`;
   `--fail-open` is fixed, so a missing module exits `0` silently, while without it airsl prints
   `module \`core.hooks.missing\` not found under …` and exits 1; `{{var}}` values may be shell
   expressions, and quoting is the extension's job (spec §11.2).
4. **`runtime.lua`** — the full text as the only `lua` block in the file, byte-identical to
   `crates/agentstacks/assets/runtime.lua`; when the core writes it: a pack of the tool has any lua
   hook, neutral or native, whether or not the extension later skips it (spec §16 R11); the
   behaviour table (context, block without `may_block`, block with `may_block`, native, context plus
   refused block, translator `nil`) with each output verbatim (spec §11.3); `airsstack.stdio` and
   `airsstack.hook.payload` exist under `confined`.
5. **Neutral protocol** — the event field table; no raw payload on a neutral event, a native hook
   gets `{ payload = <raw>, args = … }`; the result shapes (spec §11.4); native hooks reach the
   extension in the hook shape (`kind`, `event`, `match` as the list the formats reader holds (spec
   §16 R28 (d)), `may_block`, `command` or
   `text`, spec §16 R11).
6. **Translator contract** — `M.event(payload)` and `M.result(result or nil)`, one
   `translate.lua` per tool written by the extension; it is required when `runtime.lua` is written
   and at least one lua hook is covered by a write, refused otherwise with the spec §3.2 message
   verbatim (spec §11.4, §3.2, §16 R11).
7. **`may_block`** — a `block` from a hook not declared `may_block` is dropped with one stderr
   line, and its `context` is kept (spec §1 D12, §11.3 rows 2 and 5).
8. **Filtering and message hooks** — `match` and `agents` filtering is the extension's job in the
   tool's config; the runtime does not filter; message hooks never reach the runtime (spec §11.4).
9. **Shims for other hook languages** — the shim only runs the argv and forwards payload and
   result; it is an ordinary `write`, shown in the plan; example: OpenCode plugins are TypeScript
   (spec §11.5).
10. **airsl on the machine** — the core's `require_binary` for `airsl` when a locked pack
    declares `requires.airsl`, the `[airsl] version` pin, and `doctor` as the check after install
    (spec §10, §8; link to `plan-and-apply.md`).

**Proof:** written first as a checklist, then ticked against the finished file.
- Spec sections covered: §1 D12, D14; §3.1 core `runtime.lua` row; §3.2 translate row; §8 airsl
  rows (by link); §10 airsl row and pin paragraph; §11.1; §11.2 (every bullet); §11.3 (text,
  behaviour table, confined line); §11.4 (event table, native shape, result, translator contract,
  filtering, message hooks); §11.5; §12 rows 3-4; §16 R11, R24, R28 (d).
- `awk '/^```lua$/{f=1;next} /^```$/{f=0} f' docs/hooks.md | diff - crates/agentstacks/assets/runtime.lua`
  prints nothing.
- The command-line `text` block equals spec lines 676-678 (`diff` as in Task 1); the layout block differs from lines 666-668 only in the replaced references.
- Every message in the file is found verbatim in the spec with `grep -F` as in Task 1.
- `grep -nE '§|probes/|plans/' docs/hooks.md` and `grep -nwE 'spec|[DRX][0-9]+' docs/hooks.md`
  print nothing.

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit
`docs(repo): hook runtime and neutral protocol guide`.

### Task 3 — Write the plan and apply guide

**Files:**
- Create `docs/plan-and-apply.md` — how the core turns a project into a plan, applies it and
  checks it later.

**Change:** the document holds these sections, in this order:

1. **The flow** — the ten plan steps, each with the port or module that does it (spec §6.1 table,
   naming `app/plan`, `types/resolve`, `types/op`, `types/plan` and the ports); a refusal in steps
   1-7 skips steps 8-9, so no step is decided and no false delete appears (spec §16 R7); tool
   order: listed tools in name order, then removed tools in name order, with each listed tool's
   core writes recorded just before its `plan` dispatch, and both R6 messages verbatim (spec §16
   R6); what a `Plan` holds; plan notes, including the two R20 messages verbatim (spec §16 R20);
   refusals collected and sorted; a plan with any refusal applies nothing (spec §6.1, §13 last
   line).
2. **Operations** — the kinds table and `by` rules in short, linking to `extensions.md` for
   validation (spec §3.1).
3. **Steps** — the three disk states; the definition of shared; both step tables with every
   message verbatim, plus the two §16 R15 additions (a shared file whose disk differs from what
   `read` returned is refused `<path>: changed after it was read; run plan again`; a locked path no
   longer proposed that is now a symlink, dangling or pointing outside included, is `forget`,
   checked before any other row); the checked-examples table (spec §6.2, §16 R15).
4. **Inside the project** — the three-step rule; a symlink inside is written through and the lock
   records the normalised proposed path; the `path` source rule and its refusal verbatim (spec
   §6.3).
5. **Removed pack and removed tool** — a removed pack's files get delete steps; a removed tool's
   locked extension runs once with zero packs, its git tree is checked against its lock `tree`,
   its ops are still checked against each other, only its kept shared writes meet the other tools'
   proposals and its dropped proposals never reach the duplicate check, its locked files are
   deleted, nothing of it is locked (spec §9.1 rows 4-5, §6.4, §1 D8, §16 R18, R28 (g)).
6. **Apply** — the five steps (spec §7 `text` block, lines 528-535); it never re-plans; temps sit
   beside the canonical target, the lock temp is `.agentstacks.lock.agentstacks-tmp` and the lock
   keeps its mode, a path that resolves outside at apply time is refused `<path>: resolves outside
   the project` (spec §16 R21); modes; the failure table verbatim; the lock after a step-4 failure
   (new `[[tool]]`, `[[pack]]`, `[airsl]`; new `[[file]]` entries for files that landed, old ones
   for the rest; `forget` and every `unchanged` entry applied, spec §16 R17, R28 (f)); a mode-only
   change between plan and apply is not "changed", the check compares content hashes only (spec §16
   R28 (i)); directories left in place; lock content (spec §7).
7. **The lock** — what it records (tools, packs, non-shared files with `by`, `[airsl]`), shared
   files never recorded, no `[[merge]]` and its refusal `agentstacks.lock: merge: unknown key`, a
   `[[file]].path` with `\` refused at read with the R16 message verbatim, a lock path naming
   `agentstacks.toml` or `agentstacks.lock` refused `agentstacks.lock: file[0].path:
   "agentstacks.lock" is managed by agentstacks` (same shape for `agentstacks.toml`, spec §16
   R28 (e)), and
   `check_airsl_presence` run on the new lock contents only (spec §7 lock content, §6.2, §12 row 2,
   §16 R16, R27, R28 (e)).
8. **Doctor** — offline; the findings table verbatim, plus `outside the project: <path>` and
   `<path>: <io::Error Display>` (spec §16 R23); findings in the lock's path order, then airsl
   (spec §16 R8); no extension needed (spec §8).
9. **Sources** — the reconciliation table with its messages verbatim and the moved-tag rule;
   "same source fields" compares normalised values and messages quote a value as written (spec
   §9.1, §16 R9); git commands, environment, URL after `--`, `ls-tree` without a `subdir` lists the
   whole tree (spec §16 R26), the `FileTree` mode mapping (`100644` and `100755` file, `120000`
   symlink, any other other, spec §16 R10), a git source's tree label is the `subdir` as written or
   `.` (spec §16 R22), the failure table verbatim, unreadable git output with the R2 message
   verbatim, lossy decoding and trimming covering quoted text and `ls-remote` lines while `ls-tree
   -z` paths stay raw bytes (spec §9.2, §16 R2, R28 (j)); the cache layout, filling, a leftover cache temp of the same name
   removed and reused (spec §16 R21), a failed cache rename reported at the `<rev>.git` target
   (spec §16 R28 (b)), the `HOME` refusal (unset or empty, reported once, spec §16
   R10), temps never cleaned (spec §9.3, §1 D10, D11).
10. **Update** — `latest_tag` steps (equal precedence → the greater tag in byte order, spec §16
    R8), the split rule with both examples, the case table verbatim, the `app/update` output in key
    order (`packs.*`, then `tools.*`, spec §16 R8), and what the CLI owns (writing the tag,
    shorthand expansion) (spec §9.4).
11. **Binaries and the airsl pin** — the requirement table verbatim, "once for each", what "on
    PATH" means (a regular file with an execute bit; empty `PATH` entries skipped; unset `PATH`
    finds nothing; a symlink on `PATH` counts when it resolves to an executable regular file,
    spec §16 R10, R28 (c)), the pin and `airsl 0.1.4 → 0.1.5` in the plan, how a missing
    airsl after install is caught (spec §10).
12. **Where a refusal is located** — the locations table (spec §13), and the generic I/O messages
    (`<path>: <io::Error Display>`, the two spawn forms) verbatim (spec §16 R1).
13. **Not done** — no merge or run-command operation, no tool-specific core code, no writing of
    `agentstacks.toml`, no network in `doctor`, no cache lock or temp clean-up, no exact number
    spelling or inner layout in shared JSON, no pure-policy hooks, empty directories kept, paths
    compared by bytes (spec §18).

**Proof:** written first as a checklist, then ticked against the finished file.
- Spec sections covered: §1 D1, D2, D8, D10, D11; §3.1; §6.1; §6.2 (both tables, shared,
  examples); §6.3 (rule, write-through, path source); §6.4; §7 (steps, modes, failures, lock
  content); §8 (every row); §9.1-§9.4 (every table); §10; §12 row 2; §13; §16 R1, R2, R6, R7, R8,
  R9, R10, R15, R16, R17, R18, R20, R21, R22, R23, R26, R27, R28 (b), (c), (e), (f), (g), (i), (j);
  §18.
- Every message in the file is found verbatim in the spec with `grep -F` as in Task 1.
- The apply `text` block equals spec lines 528-535 (`diff` as in Task 1).
- `grep -nE '§|probes/|plans/' docs/plan-and-apply.md` and
  `grep -nwE 'spec|[DRX][0-9]+' docs/plan-and-apply.md` print nothing.

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit
`docs(repo): plan, apply, doctor and update guide`.

### Task 4 — Bring the architecture, index and status up to the built code

**Files:**
- Modify `docs/architecture.md`:
  - the dependency-rule block and the paragraph under it follow spec §2: `types` may import only
    `port::{FileTree, EntryKind}`, the read-only tree port the formats loaders take, and
    `tests/isolation.rs` allows exactly those two names from `crate::port` inside `src/types`,
    plus `crate::port::FakeFileTree` in `#[cfg(test)]` code (spec §16 X2, R28 (k)); `process` may use `fs::cache` and nothing else from `fs`, the only edge from one
    adapter to another (spec §16 X3); the ASCII diagram's arrows stay consistent with that;
  - a short dependency-policy note: `deny.toml` exempts airsl's subtree from the duplicate-version
    ban with `skip-tree`, because airsl 0.1.4 pins `sha2` 0.10 and `toml` 0.8 while agentstacks
    uses `sha2` 0.11 and `toml` 1, and the exemption can go once airsl moves (spec §15 skip-tree
    row, §16 X1); `Zlib` is allowed for `foldhash` (spec §15);
  - the "State" column of the module table reads `built` for `types/`, `port/`, `app/`, `fs/`,
    `process/` and `host/`, and the `port/` row drops "(today under `types/port/`)";
  - the `port/` example in "Where logic lives" names a method that exists in
    `crates/agentstacks/src/port/workspace/seam.rs` (for example `Workspace::resolve(path)`) in place
    of `Workspace::hash(path)`, which is not built; the traced rule's refusal text becomes the built
    message `<path>: changed since the last apply` (spec §6.2) in place of `<path> changed since last
    apply`.
- Modify `docs/README.md` — the three new rows link to their files; every row's state reads
  `built`. The closing paragraph about code and docs agreeing stays.
- Modify `CLAUDE.md` — only the Status block: the heading, the paragraph under it, and the
  `crates/agentstacks/` line of its `text` block. The heading and paragraph say the formats, plan,
  apply, doctor, update, the extension host and the hook runtime are built and tested, and the CLI
  commands are not; the `crates/agentstacks/` line lists `types · port · app · fs · process · host`
  as built. No other line of `CLAUDE.md` changes.

**Change:** documents only.

**Proof:** written first as a checklist, then ticked against the finished files.
- `docs/architecture.md`: the dependency rule matches spec §2's block (X2, X3); the policy note
  states X1; no `planned` left in the module table; every method or message it names is found in
  `crates/agentstacks/src` with `grep -rF`; `grep -nwE 'spec|[DRX][0-9]+' docs/architecture.md`
  prints nothing.
- `docs/README.md`: four rows, each a link, each state `built`; spec §15 docs row met.
- `CLAUDE.md`: `git diff CLAUDE.md` touches only the Status heading, its paragraph and the
  `crates/agentstacks/` line.

**Done when:** `cargo make dod` passes, and the Definition of Done is green at the end of the plan.
Commit `docs(repo): mark the core built in the contributor docs`.

---

## Risks

- The documents restate rules that live in code. If a built message differs from the spec, the docs
  would quietly disagree with the code. The `grep -F` checks catch differences against the spec
  only; the writer also greps each fixed message in `crates/agentstacks/src` and stops on a mismatch
  instead of picking one.
- The riskiest step is Task 1: it is the largest document, quotes the most messages, and must merge
  §16 overrides into the earlier tables without leaving the superseded text behind.
- `CLAUDE.md` is edited only in its Status block; the design sections below it still describe the
  contract and stay. The org root rule ("update `.github/profile/README.md` and the workspace
  `README.md` when a project's status changes") is outside this plan's files and this repository.
- Not done here: the mdBook (`book/` does not exist), the root `README.md`.

## Proof

- Spec coverage for documentation: §3 (all) Tasks 1, 3; §4 (all) Task 1; §5 Task 1; §6 Task 3;
  §7 Task 3; §8 Tasks 3, 2 (link); §9 Task 3; §10 Tasks 1, 2, 3; §11.1-§11.3 Task 2; §11.2
  working-directory duty Tasks 1, 2; §11.4 Tasks 1, 2; §11.5 Tasks 1, 2; §12 Tasks 1, 2, 3; §13
  Tasks 1, 3; §15 Task 4; §16 X1-X3 Task 4; §16 R1-R28: extension API rows (R3, R4, R5, R6, R8,
  R11, R12, R13, R14, R19, R25, R1 for `reads`, R24 for `reads`, R28 (a), (b), (d), (h), (l), (m),
  (n)) Task 1, hook rows (R11, R24, R28 (d)) Task 2, core rows (R1, R2, R6-R10, R15-R18, R20-R23,
  R26, R27, R28 (b), (c), (e), (f), (g), (i), (j)) Task 3, R28 (k) Task 4; §18 Tasks 1, 3.
- The runtime.lua `diff` in Task 2 prints nothing.
- `cargo make dod` is green after every task and at the end of the plan.

## Review findings

Reviewer, one round: docs vs code 0🔴 7🟡 9🔵, plan 0🔴 2🟡 1🔵. Reviewer `cargo make dod`: exit 0. Blocking: `extensions.md:227`, `:304`, `:417`, `:470`, `plan-and-apply.md:72`, and the missing Deviations section.

- fixed 🟡 (blocking) `docs/extensions.md:227`: the native-hook example lacked `kind` and gave `match` as a string. The spec block is kept, with a note and the built shape under it. The bullets now say `match` and `agents` are lists, left out when empty.
- fixed 🟡 (blocking) `extensions.md:304`: added the refusal `<tool>: <function> must be called from the extension's Lua code`, and noted that a refusal with no Lua frame is located at `<tool>` alone.
- fixed 🟡 (blocking) `extensions.md:417`: added `json.edit: value: nested more than 128 levels deep`.
- fixed 🟡 (blocking) `extensions.md:470`: the runtime-error example now carries the code's `runtime error: ` prefix and is labelled "example: claude extension".
- fixed 🟡 (blocking) `plan-and-apply.md:72` and `extensions.md:252`: `runtime.lua` is written for a native `lua` hook whether or not the extension later skips it.
- fixed 🟡 `plan-and-apply.md:173`: cites `types::lock::write_lock` instead of a wrong line number.
- fixed 🔵 Nits:
  - `architecture.md:72`: the traced example uses names that exist: `decide_locked`, `StepError::Changed`, `Workspace::resolve`, `StdWorkspace::resolve`.
  - `architecture.md:16`: gained the types → port edge.
  - The reads table gained `extension.reads[0]: empty`.
  - The notes list now has its third kind.
  - The apply failures gained `has no content to write`.
  - Added the doctor row for a locked path that cannot form an `OpPath`.
  - `hooks.md:111` is labelled "example: claude extension".
- Fix verification: `cargo make dod` exit 0 (287 doctests). The plan's greps (`§`, `probes/`, `plans/`, `spec`, `[DRX][0-9]+`) print nothing. Every added message was found with `grep -rF` in `crates/agentstacks/src`.

## Deviations

- 2026-11-22: The docs describe the built code where it differs from the spec. They carry these texts, which are not in spec.md, each from source:
  - `<tool>: <function> must be called from the extension's Lua code` (host/ops.rs)
  - the `<tool>`-alone location of a refusal with no Lua frame (host/ops.rs)
  - `json.edit: value: nested more than 128 levels deep` (types/json/error.rs)
  - `… lua error in plan: runtime error: …` (host/events.rs)
  - `extension.toml: extension.reads[0]: empty` (types/extension/manifest.rs)
  - `<path>: has no content to write` (app/apply/stage.rs)
  - `symlink leads nowhere` and `resolves to the project directory` (fs/workspace)
  - doctor's `not a regular file: <path>` for a locked path that cannot form an `OpPath` (app/doctor.rs)
- 2026-11-22: The spec's native-hook payload example is kept byte for byte, with a note giving the built shape under it.
- 2026-11-22: Spec §4.6 quotes `declared events: spin, values`, a probe artifact. The docs use `plan, values`, which matches `src/host/events.rs`.
- 2026-11-22: Git commands removing `GIT_DIR` and its kin is documented, from the plan 11 review fix.
- 2026-11-22: The docs were written while the plan 12 fix coder was still at work, then reconciled with the final host code in the review fix round.
