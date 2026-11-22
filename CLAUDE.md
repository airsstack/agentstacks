# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Status: core built, CLI commands not yet

The pack, extension, project and lock formats, plan, apply, doctor, update, the extension host and
the hook runtime are built and tested. The CLI commands are not.

```text
crates/agentstacks/       library; types · port · app · fs · process · host, all built
crates/agentstacks-cli/   the `agentstacks` binary; answers --version and --help only
docs/                     contributor docs; docs/architecture.md is the layering contract
Makefile.toml, deny.toml  the cargo-make gate and the cargo-deny policy
.github/workflows/ci.yml  CI: `cargo make dod` and `cargo deny check`
```

Everything below the Status section except Commands describes the agreed design. Treat it as the
contract to build toward, not a description of existing files. `extensions/`, `packs/`, `book/` and the claudevs move
have not landed, and the code that will move here still lives in
[claudestacks](https://github.com/airsstack/claudestacks) (`../claudestacks` in the org workspace).
`/claudestacks-sdlc:status` shows progress.

## What agentstacks is

A CLI that sets up agentic coding tools (Claude Code first, OpenCode second) from one tool-neutral
copy of skills, agents, commands, hooks and rules.

```text
agentstacks.toml (committed; human intent: tools = [...], packs = [...])
   │
packs ──► tool extension (adapter.lua under airsl, zero grants) ──► proposed operations
                                                                     │
           Rust core: validate ──► plan (shows every operation) ──► apply (the ONLY writer)
                                                                     │
                                     agentstacks.lock (pinned pack/extension versions + file hashes)
                                                                     │
                                                    doctor: drift check against the lock
```

| Term | Meaning |
|---|---|
| **Pack** | *What* the agent gets: versioned, tool-neutral content (skills, agents, commands, Lua hooks, rules, plus `native/<tool>/` for tool-specific files). No code beyond hooks. |
| **Tool extension** | *How* one tool gets it: a Lua adapter that turns packs into that tool's artifacts. |
| **Operation** | What an extension proposes: write file, copy file, require binary, or skip-with-reason; the core adds delete for removed packs. There is no merge or run-command operation: a config file shared with the user is read (declared in `extension.toml` `reads`) and rewritten whole by the extension. |
| **Lock** | Pinned versions plus a hash of every written file. `apply` never silently overwrites user edits. |

CLI surface: `init --tool <t> --pack <p>` · `plan` · `apply` · `doctor` · `update`. There is no plugin
marketplace. For Claude Code, the CLI writes native `.claude/` project config.

## Invariants (do not break)

- **Extensions only propose.** An extension has no file, network or process access. The core
  validates every operation, `plan` shows it, and the executor alone touches disk. A third-party
  command never runs unannounced.
- **Nothing is ambient.** An extension gets only what it declared, and a refusal names what was
  missing.
- **Unsupported is visible.** An item a tool can't express becomes a skip operation with a reason,
  never a silent drop.
- **Lua is the only scripting language**, always run under airsl. When a tool demands another
  language (OpenCode plugins are TypeScript), generate a thin shim that only forwards events to airsl.
- **The official extensions use the public API** that community extensions use, with no private
  hooks. The API stays `api = 0` (unstable) until both Claude and OpenCode run on it. Freeze it as
  `api = 1` in M3.
- `airsl` comes from crates.io, never as a path dependency. Never use the `aisdk-*` crate names,
  because that family belongs to another project.

## Architecture (read before writing code)

**[`docs/architecture.md`](docs/architecture.md) is the contract for `crates/agentstacks`. Read it
before adding or moving any module.** In short:

```text
types/    RULES: domain data + pure decisions (data in → answer out)       → depends on nothing
port/     traits + a fake for each (FileTree, Workspace, Git, Binary, Adapter) → types
app/      FLOWS: use cases generic over ports (plan, apply, doctor, update)  → port, types
fs/ process/ host/   ADAPTERS: translation only (disk, git, airsl binary, Lua) → port, types
```

- Ports and adapters (hexagonal). A rule goes in `types/`, a sequence of steps in `app/`, never
  the reverse. Adapters make no decisions.
- Inside (`types`, `port`, `app`) never imports an adapter or calls `std::fs`, `std::process` or
  `std::net`; `tests/isolation.rs` enforces it.
- Every rule and flow is unit-tested with plain values or fakes. Adapters get one contract test
  each against the real thing. Logic reachable only end to end is in the wrong layer.
- The core is tool-agnostic: no tool's file shapes (e.g. Claude's `settings.json`) in `types/`,
  `app/` or `host/`; that knowledge lives only in the tool's extension.

## Planned layout

```text
crates/agentstacks/      types · port · app · fs · process · host (see docs/architecture.md)
crates/agentstacks-cli/  init · plan · apply · doctor · update
crates/claudevs{,-cli}/  deterministic Claude Code setup test engine, moved from claudestacks (keep the crate names)
extensions/{claude,opencode}/   official tool extensions (Lua)
packs/                   official packs: core, sdlc, journal, guideline-rust, guideline-python, cmux
docs/                    contributor docs until the book exists
book/                    mdBook, published to airsstack.github.io/agentstacks/
```

Roadmap: **M1** claude extension + the claudestacks plugins converted to packs · **M2** opencode
extension on the same API · **M3** freeze `api = 1`, authoring docs, `ext add` / `ext test`.

## Porting claudestacks plugins into packs

The plugins in `../claudestacks/plugins/` don't map 1:1 onto the pack content model. Settle these in
the pack format spec before you convert anything:

| Issue | Where it shows up today |
|---|---|
| Namespaced cross-references (`claudestacks-sdlc:status`, `claudestacks:coder`) | ~60 references across skills and agents. Project-level `.claude/` items have no plugin namespace, so packs need a tool-neutral reference syntax that the extension renders. |
| `${CLAUDE_PLUGIN_ROOT}` paths | ~42 files. No plugin root exists once content is written into `.claude/`, so packs need an install-root abstraction. |
| Content types beyond the README list | `scripts/*.lua` (invoked by skills, not hooks), `references/`, `enforcement.json` (guideline packs) |
| Hook runtime | Hooks are `sh` wrappers that locate an **unpinned** `airsl` binary. Reconcile that with the lock's determinism, for example with a require-binary operation. |

When the move lands, archive claudestacks (never delete it: releases, `install.sh` and the old
marketplace still point there).

## Commands

These run today. The workspace uses the airsstack gate shared by every Rust project
(`cargo install --locked cargo-make cargo-deny`):

```bash
cargo make dod                 # Definition of Done: fmt-check, clippy, rustdoc, tests, doctests (warnings = errors)
cargo make dod-crate <crate>   # the same steps scoped with -p
cargo make fmt                 # rewrite formatting (clippy-fix is the other task that edits sources)
cargo make deny                # cargo-deny; deliberately not part of dod
cargo test -p <crate> <path::to::test> [-- --exact]   # one test
```

CI runs `cargo make dod` and `cargo deny check` on push to `main`, on pull requests and on manual
dispatch.

A run short of `cargo make dod` counts as iteration, not a green result. The source of truth for the
gate is the `claudestacks-guideline-rust:rust-guidelines` skill. Workspace lints: `unsafe_code =
"forbid"`, `unwrap_used` denied, pedantic + nursery clippy. When dist is added, set
`aarch64-apple-darwin = "macos-15"` under `[dist.github-custom-runners]` and use the same runner in
the smoke workflow.

## How work is done

- Work goes through the `claudestacks-sdlc` chain (intent → design/spec → plan → execute), with
  files under `.claudestacks/sdlc/`. `/claudestacks-sdlc:status` shows the board. Read progress from
  the board, not from this file.
- Conventional Commits, `type(scope): summary`. The scope is the crate name, or `repo` for
  cross-cutting changes. Work on a branch or worktree, never on `main`.
- If this project's status, scope or lane changes, update `.github/profile/README.md` and the
  workspace `README.md` in the org root in the same change.
