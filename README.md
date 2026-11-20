# agentstacks

**Set up any agentic coding tool from one CLI: write the content once, and agentstacks renders it in
each tool's own format.**

> **Status: early development.** Nothing here is usable yet. The commands and formats below are
> the design, and they will change. Today's Claude Code plugins live in
> [claudestacks](https://github.com/airsstack/claudestacks) until they move here.

Every agentic coding tool wants its setup in its own shape: Claude Code reads `.claude/`, OpenCode
reads `.opencode/` and `opencode.json`, and the next tool will want something else. agentstacks
keeps one tool-neutral copy of that setup (skills, agents, commands, hooks, rules) and writes it
into each tool you use. Before anything is written, you see exactly what will change.

```text
 packs (tool-neutral content)
        │
        ▼
 tool extension ── claude · opencode · yours      Lua, sandboxed by airsl, zero authority
        │
        ▼
 plan  ── every file write, config merge, and command, shown before it happens
        │
        ▼
 apply ── the only step that touches disk ──► agentstacks.lock (pinned versions + file hashes)
```

## Concepts

| Term | What it is |
|---|---|
| **Pack** | *What* the agent gets. A named, versioned bundle of tool-neutral content: skills, agents, commands, hooks (Lua), and rules. No code beyond hooks. |
| **Tool extension** | *How* one tool gets it. A Lua adapter that turns packs into that tool's artifacts and steps. |
| **Plan** | The list of operations an apply would perform: write a file, merge a config key, run a declared command, require a binary, or skip an item the tool can't support, with the reason. |
| **Lock** | `agentstacks.lock`: pinned pack and extension versions plus a hash of every written file, so `doctor` detects drift and `apply` never silently overwrites your edits. |

## Usage (planned)

```sh
cd my-project
agentstacks init --tool claude --tool opencode --pack sdlc --pack guideline-rust
agentstacks plan      # show every operation; nothing is written
agentstacks apply     # perform exactly what plan showed
agentstacks doctor    # check each tool still matches the lock
agentstacks update    # newer pack versions → plan → apply
```

`init` writes the manifest that the project commits:

```toml
# agentstacks.toml
tools = ["claude", "opencode"]
packs = ["sdlc", "guideline-rust"]
```

A teammate runs `agentstacks apply` and gets the same packs in whichever supported tool they use.

## Tools

| Tool | Status |
|---|---|
| Claude Code | In development |
| OpenCode | Planned |
| Your own | Planned: write a tool extension against the public API |

There is no plugin marketplace. Every tool is set up through the CLI, in that tool's native
project configuration.

## Packs

The first packs come from today's claudestacks plugins:

| Pack | Contents |
|---|---|
| `core` | coder, reviewer, and explorer agents; orchestration; process guidelines |
| `sdlc` | the intent → spec → plan → execute chain, with roadmaps and ADRs |
| `journal` | a vault of session and topic notes |
| `guideline-rust` | the Rust Definition of Done and engineering rules |
| `guideline-python` | the Python Definition of Done and engineering rules |
| `cmux` | skills for driving cmux |

## Extending agentstacks

Anyone can write a pack (markdown, plus Lua for hooks) or a tool extension (Lua). The official
Claude Code and OpenCode extensions are built on the same public API, so the API has to be good
enough for real tools.

Extensions run under [airsl](https://github.com/airsstack/airsl) with no file, network, or process
access. An extension only *proposes* operations; the Rust core validates them, `plan` shows them,
and the executor applies them. A third-party command never runs unannounced. The extension API is
unstable (`api = 0`) until both official tools run on it.

## Design rules

- **Nothing is ambient.** An extension gets nothing it didn't declare. Every change appears in the
  plan, and a refusal names what was missing.
- **The human owns the intent.** `agentstacks.toml` is the committed record of what a project uses.
- **Lua is the only scripting language.** Hook logic is Lua run by airsl. Where a tool demands
  another language (OpenCode plugins are TypeScript), the extension generates a thin shim that only
  forwards events to airsl.
- **Unsupported is visible.** An item a tool can't express is reported as skipped, never dropped
  silently.

## Layout (planned)

```text
agentstacks/
├─ crates/
│  ├─ agentstacks/       model · validation · plan · executor · lock · extension host
│  ├─ agentstacks-cli/   init · plan · apply · doctor · update
│  ├─ claudevs/          deterministic test engine for Claude Code setups (moving from claudestacks)
│  └─ claudevs-cli/
├─ extensions/           official tool extensions: claude/ · opencode/
├─ packs/                official packs
└─ book/
```

## Roadmap

```text
M1  claude extension (api = 0); the claudestacks plugins become packs
M2  opencode extension on the same API
M3  freeze api = 1; docs for writing packs and extensions; ext add / ext test
```

## Part of airsstack

agentstacks belongs to the [airsstack](https://github.com/airsstack) organization's "run the
factory" lane. The `airsstack` CLI will hand `airsstack agent …` off to it, and its hooks run on
[airsl](https://github.com/airsstack/airsl).

## License

Apache-2.0. Contributions are accepted under the DCO.
