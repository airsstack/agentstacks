---
status: draft
created: 2026-11-22
depends-on: [12, 13, 14, 15, 16]
built-on: ea104af7451e
---

# Contributor docs Implementation Plan

**Goal:** Document the built extension API, hook runtime, plan, apply, doctor and update for contributors.

**Architecture:** Three new Markdown documents under `docs/`, one per audience, plus the state
columns of the two existing ones. `docs/extensions.md` is for extension authors (the `api = 0` Lua
surface and its duties). `docs/hooks.md` is for pack and extension authors (the neutral hook
protocol and the shared runtime). `docs/plan-and-apply.md` is for core contributors (operations,
steps, apply, lock, doctor, sources, binaries). Each document describes what the code does at the
end of plans 12-16 (`docs/README.md`: "A document describes what the code does"); where code and
spec disagree, the docs task stops and reports it instead of choosing. No Rust source changes.

**Tech Stack:** Markdown only. The gate is still `cargo make dod`, which must stay green.

**Content authority:** spec §3-§11 (the `api = 0` contract, including §11.2 working-directory duty,
§11.4 neutral protocol and translator contract, §11.5 shims), §12 (format changes an author meets),
§13 (locations), §15 docs row, §17 (what is not done).

**Writing rules for every document in this plan:**

- Every refusal or output message is copied byte for byte from the spec (backticks, quotes and
  `…` included); none is retyped from memory or invented.
- Every example from a tool is labelled "example: claude extension" (or the tool it is from); the
  core is described in tool-neutral terms (spec §17 "Any tool-specific code in the core").
- No reference to spec sections, decision numbers (`D1`…), probe files, plans, chain names or
  agents. The documents cite code locations (`crates/agentstacks/src/types/op/`, …) and each
  other instead.
- Fenced `json` and `lua` blocks are copied byte for byte from the spec blocks named in each task.

---

## Files that change

```
docs/extensions.md       — create: the api 0 extension contract (load, sandbox, agentstacks table, events, operations, require, json.edit, errors, duties)
docs/hooks.md            — create: hook layout, command line, runtime.lua, neutral protocol, translator contract, may_block, shims
docs/plan-and-apply.md   — create: operations, plan steps, inside-the-project rule, removed tool, apply, lock, doctor, sources, update, binaries, locations
docs/README.md           — modify: link the three documents and set every row's state
docs/architecture.md     — modify: the "State" column, and the two examples that must match the built code
```

## Order of work

### Task 1 — Write the extension guide

**Files:**
- Create `docs/extensions.md` — the `api = 0` contract an extension author codes against.

**Change:** the document holds these sections, in this order, each with the spec content named:

1. **What an extension is and is not** — it only proposes operations; the core validates, `plan`
   shows, apply alone writes; no merge or run-command operation; everything is `api = 0`,
   unstable (spec §1 D1, D3; §17 last bullet).
2. **`extension.toml`** — the new optional `reads` list and its five refusals, verbatim (spec §12
   row 1).
3. **Load** — source resolved, `Extension::load`, `api = 0`, every `.lua` file read into memory,
   Lua never touches disk (spec §4.1).
4. **Sandbox** — the stdlib set (`Json`, `Ext` with events `values` and `plan`, `agentstacks`);
   what is removed (`print`, `warn`), absent (`require`, `load`, `dofile`, `os`, `io`,
   `coroutine`, other airsl modules) and present; limits (10,000,000 instructions per evaluation
   or dispatch, 16 MiB, reset per dispatch); both breach messages verbatim; a breach caught by the
   extension's own `pcall` is its own business (spec §4.2).
5. **The `agentstacks` table** — the function table (spec §4.3) with phases and returns, and the
   four `read` rules including the outside-project message.
6. **Events** — the four-step flow; the three `dispatch` outcomes; a second registration replaces
   the first; the `values` payload `json` block (spec lines 268-270); the `values` result `lua`
   block, labelled "example: claude extension" (spec lines 276-286); the token table; missing
   values (whole-skill skip, the two skip reasons verbatim, item-level skip for the other kinds,
   core skip of a root file); JSON type words (object → `table`, array → `list`, …, an empty Lua
   table arrives as an object); the `values` refusal table verbatim; the `plan` payload `json`
   block (spec lines 321-332) and its three notes; the return value of `plan` is ignored; `registers
   no plan handler`; tools run in name order and that order decides `… already proposed by <tool>`
   (spec §4.4).
7. **Operations** — the kinds table (spec §3.1), marking which the extension proposes and which
   the core makes (root files to `<root>/<pack>/…`, `<root>/runtime.lua`, `require_binary` for
   packs and airsl, `delete`); how `by` is derived; a shared file has no `by` (spec §3.1, D7).
8. **Item ids and coverage** — the id table and the case table with every message verbatim; ids
   are copied, never built; root files and core-skipped items need no cover (spec §3.2).
9. **Validation** — checked at the call, located at the caller's file and line (a call from a
   required module names that module); the rule table with every refusal verbatim; normalisation
   of op paths, `reads` and `values` paths; accepted without refusal; type words; strictness (no
   coercion, unknown keys refused, `content` keeps raw bytes); refusals stick under `pcall` and the
   refused op is not recorded (spec §3.3).
10. **`agentstacks.require`** — `require` stays absent; the name rule; name → path mapping; loaded
    once with chunk name `@<path>`, cached; errors carry the module's file and line; the refusal
    table verbatim; a module that raised is not cached (spec §4.5).
11. **`agentstacks.json.edit`** — both tables (rules; Lua value → JSON) with every refusal
    verbatim; the checked example `lua` block (spec lines 408-409) and its output `json` block
    (spec lines 413-426) with the sentence after it; numbers re-printed in serde_json's normal
    form, the plan's diff shows the change; the core knows no config format, a shared file is read
    (declared in `reads`), edited and rewritten whole (spec §5, §1 D2, D9).
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
- Spec sections covered, each with its section number above: §1 D1-D3, D7, D9; §3.1; §3.2 (both
  tables); §3.3 (table, normalisation, accepted list, type words, strictness, sticky); §4.1; §4.2;
  §4.3 (table and `read` rules); §4.4 (flow, dispatch outcomes, both payloads, values result,
  token table, missing values, type words, values table, plan notes, name order); §4.5 (rules and
  table); §4.6; §5 (both tables, checked example); §10 row 2 and the "once for each" line; §11.2
  working-directory and quoting duties; §11.4 translator contract, filtering, message hooks;
  §11.5; §12 row 1; §13 rows 1-3; §17 last bullet.
- Every message in the file is found verbatim in the spec: for each, `grep -F '<message text>'
  .claudestacks/sdlc/2026-11-20-core-plan-and-apply/spec.md` prints a line (messages holding a
  `<placeholder>` are checked with the placeholder exactly as the spec prints it).
- The five fenced blocks equal the spec lines named in sections 6 and 11, compared with `diff`
  (for example `sed -n '276,286p' .claudestacks/sdlc/2026-11-20-core-plan-and-apply/spec.md` against
  the block's lines).
- `grep -nE '§|probes/|plans/' docs/extensions.md` and `grep -nwE 'spec|D[0-9]+' docs/extensions.md` print nothing.

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit
`docs(repo): extension api 0 guide`.

### Task 2 — Write the hook guide

**Files:**
- Create `docs/hooks.md` — how a pack hook runs, from the tool's command to the hook's result.

**Change:** the document holds these sections, in this order:

1. **Layout** — the three files under `<root>` (spec §11.1 `text` block); who writes each
   (`runtime.lua` the core, `translate.lua` the extension, `<root>/<pack>/hooks/…` the core as a
   root file); airsl's `require` root is the script's directory; how a `run` of
   `{{root core}}/hooks/enforce.lua` becomes the module `core.hooks.enforce` (spec §11.1).
2. **Rules for a pack hook** — `grants.policy = "pure"` refused in `api = 0`, `confined` the only
   allowed value; each `run` segment must be require-able (also in `native/<tool>/hooks.toml`);
   the three refusals verbatim (spec §12 rows 3-4, §1 D14).
3. **Command line** — the argv shape (spec §11.2 `text` block, copied); the core builds it with
   project-relative paths and grants map one to one; the extension's duty to run it with the
   project directory as working directory (example: claude extension, `cd "$CLAUDE_PROJECT_DIR"
   && …`); a project-relative grant resolves against that directory, and without one a read is
   denied with `fs.read denied: … is outside the granted read roots: none are granted`;
   `--fail-open` is fixed, so a missing module exits `0` silently, while without it airsl prints
   `module \`core.hooks.missing\` not found under …` and exits 1; `{{var}}` values may be shell
   expressions, and quoting is the extension's job (spec §11.2).
4. **`runtime.lua`** — the full text as the only `lua` block in the file, byte-identical to
   `crates/agentstacks/assets/runtime.lua`; when the core writes it (a pack of this tool has a
   `lua` hook or a native hook that is not skipped, spec §3.1); the behaviour table (context,
   block without `may_block`, block with `may_block`, native, context plus refused block,
   translator `nil`) with each output verbatim (spec §11.3); `airsstack.stdio` and
   `airsstack.hook.payload` exist under `confined`.
5. **Neutral protocol** — the event field table; no raw payload on a neutral event, a native hook
   gets `{ payload = <raw>, args = … }`; the result shapes (spec §11.4).
6. **Translator contract** — `M.event(payload)` and `M.result(result or nil)`, one
   `translate.lua` per tool written by the extension; lua hooks without a write to
   `<root>/translate.lua` are refused with the spec §3.2 message verbatim (spec §11.4, §3.2).
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
  filtering, message hooks); §11.5; §12 rows 3-4.
- `awk '/^```lua$/{f=1;next} /^```$/{f=0} f' docs/hooks.md | diff - crates/agentstacks/assets/runtime.lua`
  prints nothing.
- Every message in the file is found verbatim in the spec with `grep -F` as in Task 1.
- `grep -nE '§|probes/|plans/' docs/hooks.md` and `grep -nwE 'spec|D[0-9]+' docs/hooks.md` print nothing.

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit
`docs(repo): hook runtime and neutral protocol guide`.

### Task 3 — Write the plan and apply guide

**Files:**
- Create `docs/plan-and-apply.md` — how the core turns a project into a plan, applies it and
  checks it later.

**Change:** the document holds these sections, in this order:

1. **The flow** — the ten plan steps, each with the port or module that does it (spec §6.1 table,
   naming `app/plan`, `types/resolve`, `types/op`, `types/plan` and the ports); what a `Plan`
   holds; refusals collected and sorted; a plan with any refusal applies nothing (spec §6.1, §13
   last line).
2. **Operations** — the kinds table and `by` rules in short, linking to `extensions.md` for
   validation (spec §3.1).
3. **Steps** — the three disk states; the definition of shared; both step tables with every
   message verbatim; the checked-examples table (spec §6.2).
4. **Inside the project** — the three-step rule; a symlink inside is written through and the lock
   records the normalised proposed path; the `path` source rule and its refusal verbatim (spec
   §6.3).
5. **Removed pack and removed tool** — a removed pack's files get delete steps; a removed tool's
   locked extension runs once with zero packs, only its shared writes are kept, its locked files
   are deleted, nothing of it is locked (spec §9.1 rows 4-5, §6.4, §1 D8).
6. **Apply** — the five steps (spec §7 `text` block, copied); it never re-plans; modes; the
   failure table verbatim; directories left in place; lock content (spec §7).
7. **The lock** — what it records (tools, packs, non-shared files with `by`, `[airsl]`), shared
   files never recorded, no `[[merge]]` and its refusal `agentstacks.lock: merge: unknown key`
   (spec §7 lock content, §6.2, §12 row 2).
8. **Doctor** — offline; the findings table verbatim; no extension needed (spec §8).
9. **Sources** — the reconciliation table with its messages verbatim and the moved-tag rule (spec
   §9.1); git commands, environment, URL after `--`, the `FileTree` mode mapping, the failure
   table verbatim (spec §9.2); the cache layout, filling, `HOME` refusal, temps never cleaned
   (spec §9.3, §1 D10, D11).
10. **Update** — `latest_tag` steps, the split rule with both examples, the case table verbatim,
    the `app/update` output, and what the CLI owns (writing the tag, shorthand expansion) (spec
    §9.4).
11. **Binaries and the airsl pin** — the requirement table verbatim, "once for each", the pin and
    `airsl 0.1.4 → 0.1.5` in the plan, how a missing airsl after install is caught (spec §10).
12. **Where a refusal is located** — the locations table (spec §13).
13. **Not done** — no merge or run-command operation, no tool-specific core code, no writing of
    `agentstacks.toml`, no network in `doctor`, no cache lock or temp clean-up, no exact number
    spelling or inner layout in shared JSON, no pure-policy hooks, empty directories kept, paths
    compared by bytes (spec §17).

**Proof:** written first as a checklist, then ticked against the finished file.
- Spec sections covered: §1 D1, D2, D8, D10, D11; §3.1; §6.1; §6.2 (both tables, shared,
  examples); §6.3 (rule, write-through, path source); §6.4; §7 (steps, modes, failures, lock
  content); §8 (every row); §9.1-§9.4 (every table); §10; §12 row 2; §13; §17.
- Every message in the file is found verbatim in the spec with `grep -F` as in Task 1.
- The apply `text` block equals spec lines 528-535 (`diff` as in Task 1).
- `grep -nE '§|probes/|plans/' docs/plan-and-apply.md` and `grep -nwE 'spec|D[0-9]+' docs/plan-and-apply.md` print nothing.

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit
`docs(repo): plan, apply, doctor and update guide`.

### Task 4 — Mark the documents built

**Files:**
- Modify `docs/README.md` — the three new rows link to their files; every row's state reads
  `built` (the documents now describe built code; `architecture.md` describes the layout the
  crate now has).
- Modify `docs/architecture.md` — the "State" column of the module table reads `built` for
  `types/`, `port/`, `app/`, `fs/`, `process/` and `host/`, and the `port/` row drops "(today
  under `types/port/`)"; the `port/` example in "Where logic lives" names a `Workspace` method that
  exists in `crates/agentstacks/src/port/workspace.rs` instead of `Workspace::hash(path)` if that
  name was not built; the traced rule's refusal text becomes the built message `<path>: changed
  since the last apply` (spec §6.2) instead of `<path> changed since last apply`.

**Change:** documents only; the README's closing paragraph about code and docs agreeing stays.

**Proof:** written first as a checklist, then ticked against the finished files.
- `docs/README.md`: four rows, each a link, each state `built`; spec §15 docs row met.
- `docs/architecture.md`: no `planned` left in the module table; every method or message it names
  is found in `crates/agentstacks/src` with `grep -rF`.

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit
`docs(repo): mark the core documents built`.

---

## Risks

- The documents restate rules that live in code. If a plan's built message differs from the spec
  (a later fix, a typo), the docs would quietly disagree with the code. The `grep -F` checks catch
  differences against the spec only; the writer also greps each fixed message in
  `crates/agentstacks/src` and stops on a mismatch instead of picking one.
- The riskiest step is Task 1: it is the largest document and quotes the most messages. Keep it to
  the spec's tables rather than paraphrasing them.
- Not done here: `CLAUDE.md`, the root `README.md`, the org profile and the mdBook (`book/` does
  not exist). They are outside this plan's files.

## Proof

- Spec coverage for documentation: §3 (all) Tasks 1, 3; §4 (all) Task 1; §5 Task 1; §6 Task 3;
  §7 Task 3; §8 Tasks 3, 2 (link); §9 Task 3; §10 Tasks 1, 2, 3; §11.1-§11.3 Task 2; §11.2
  working-directory duty Tasks 1, 2; §11.4 Tasks 1, 2; §11.5 Tasks 1, 2; §12 Tasks 1, 2, 3; §13
  Tasks 1, 3; §15 docs row Task 4; §17 Tasks 1, 3.
- The runtime.lua `diff` in Task 2 prints nothing.
- `cargo make dod` is green after every task and at the end of the plan.
