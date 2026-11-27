---
status: done
created: 2026-11-27
depends-on: [01, 02]
built-on: 543ad491d3e5
---

# Init Dry-Run Implementation Plan

**Goal:** Preview `init` (rendered TOML plus plan) while writing nothing to the project

**Architecture:** `app/init` splits resolve+render from the write; the CLI plans against a scratch project

**Tech Stack:** Rust

## Files that change

```
crates/agentstacks/src/app/init.rs        — modify render split
crates/agentstacks-cli/src/cli.rs         — modify --dry-run flag
crates/agentstacks-cli/src/command/init.rs — modify dry-run path
crates/agentstacks/src/app/plan/ (scratch project planning via existing ports)
```

## Order of work

### Task 1 — Resolve+render split in app::init

**Files:**
- Modify `crates/agentstacks/src/app/init.rs` — extract `render(workspace, git, tools, packs) -> Result<String, Diagnostics>`; `init` calls it then writes

**Change:** Existing-file check stays in `init` before resolving; `render` only resolves and renders. Tests keep passing; add a render-without-write test.

**Proof:**
- `render_returns_text_without_writing` — fake workspace records no write

**Done when:** `cargo test -p agentstacks app::init` passes and the Definition of Done is green. Commit `feat(agentstacks): init render split`.

### Task 2 — Dry-run command

**Files:**
- Modify `crates/agentstacks-cli/src/cli.rs` — `Init { --dry-run }` conflicts with `--yes`
- Modify `crates/agentstacks-cli/src/command/init.rs` — dry-run branch

**Change:** On `--dry-run`: render, create a scratch project holding the rendered TOML, plan against it through a workspace opened on the scratch directory, print header + TOML + Summary plan, write nothing to the project. Scratch is removed on drop. Refusals print as init's, exit refused.

**Proof:**
- `dry_run_prints_toml_and_plan_and_writes_nothing` — command test on fakes/scratch
- `dry_run_and_yes_conflict` — clap exit 2

**Done when:** `cargo test -p agentstacks-cli` passes and the Definition of Done is green. Commit `feat(agentstacks-cli): init --dry-run`.

---

## Risks

- Planning a scratch project needs a Workspace over the scratch dir; reuse the way `ext test` builds its temporary project rather than inventing a second mechanism.

## Proof

- `cargo make dod-crate agentstacks-cli` and `cargo make dod-crate agentstacks` green.

---

## Review findings

- Self-review (inline): Implemented via app::init::render split plus app::plan::plan_rendered over the real workspace. Verified: dry_run command and clap conflict tests pass; local path sources plan correctly.

## Deviations

- Executed inline in this session instead of parallel coder/reviewer subagent spawns; verification is the plan's own tests plus the DoD sub-gates (fmt, clippy -D warnings, rustdoc, lua, book-build all green; the sandbox socket test remains environmental as recorded in plans 01–04).

## Deviations

- Mechanism differs from the spec's scratch project (which cannot resolve the project's local path sources, the main documented init form): `plan_rendered` parses the rendered text and plans through a `Fresh` workspace view that sees no lock and no target files, giving the specified fresh-project preview while path sources resolve against the real project. An independent review flagged the earlier reconciled preview; this closes it.
