# agentstacks

**Set up any agentic coding tool from one CLI: write the content once, and agentstacks renders it in
each tool's own format.**

> **Status: pre-release (0.1.0).** The CLI and the Claude Code extension are released, with the
> five official packs (`packs-v0.1.0`). The OpenCode extension is built and ships with the next
> `packs-v*` release. The extension API is `api = 0` and will change. Coming from the claudestacks
> plugins? See
> [the migration guide](https://airsstack.github.io/agentstacks/ch03-00-migration.html).

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
 plan  ── every file write, copy and delete, and every required binary, shown before it happens
        │
        ▼
 apply ── the only step that touches disk ──► agentstacks.lock (pinned versions + file hashes)
```

## Concepts

| Term | What it is |
|---|---|
| **Pack** | *What* the agent gets. A named, versioned bundle of tool-neutral content: skills, agents, commands, hooks (Lua), and rules. No code beyond hooks. |
| **Tool extension** | *How* one tool gets it. A Lua adapter that turns packs into that tool's artifacts and steps. |
| **Plan** | The list of operations an apply would perform: write, copy or delete a file, require a binary, or skip an item the tool can't support, with the reason. A config file you share with a tool (such as `.claude/settings.json`) is rewritten by its extension, and the plan shows the diff. |
| **Lock** | `agentstacks.lock`: pinned pack and extension versions plus a hash of every written file, so `doctor` detects drift and `apply` never silently overwrites your edits. |

## Install

```sh
curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh
```

The script also installs airsl when it is missing. Or build from crates.io:

```sh
cargo install agentstacks-cli
```

This route does not install airsl, which `plan` needs (0.1.5 or newer).

## Usage

```sh
cd my-project
agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust
agentstacks plan      # show every operation; nothing is written
agentstacks apply     # perform exactly what plan showed
agentstacks doctor    # check each tool still matches the lock
agentstacks update    # newer pack versions → plan → apply
```

Local extensions and packs work too:

```sh
cd my-project
agentstacks init --tool ./ext/claude --pack ./packs/mine   # writes agentstacks.toml, shows the plan, asks to apply
agentstacks plan                                            # what apply would change now
agentstacks doctor                                          # healthy, or what changed since apply
```

The official-names `init` above writes the manifest that the project commits:

```toml
# agentstacks.toml
api = 0

[tools.claude]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "extensions/claude"

[packs.core]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "packs/core"

[packs.guideline-rust]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "packs/guideline-rust"

[packs.sdlc]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "packs/sdlc"
```

A teammate runs `agentstacks apply` and gets the same packs in whichever supported tool they use.

## Tools

| Tool | Status |
|---|---|
| Claude Code | Released (`extensions/claude`, `packs-v0.1.0`) |
| OpenCode | Built (`extensions/opencode`), ships with the next `packs-v*` release |
| Your own | Write a tool extension against the public API: [Writing a Tool Extension](https://airsstack.github.io/agentstacks/ch06-00-authoring.html) |

There is no plugin marketplace. Every tool is set up through the CLI, in that tool's native
project configuration.

## Packs

The five packs come from the claudestacks plugins and are released as `packs-v0.1.0`:

| Pack | Contents |
|---|---|
| `core` | coder, reviewer, and explorer agents; orchestration; process guidelines |
| `sdlc` | the intent → spec → plan → execute chain, with roadmaps and ADRs |
| `journal` | a vault of session and topic notes |
| `guideline-rust` | the Rust Definition of Done and engineering rules |
| `guideline-python` | the Python Definition of Done and engineering rules |

## Extending agentstacks

Anyone can write a pack (markdown, plus Lua for hooks) or a tool extension (Lua). The official
Claude Code and OpenCode extensions are built on the same public API, so the API has to be good
enough for real tools.

Extensions run under [airsl](https://github.com/airsstack/airsl) with no file, network, or process
access. An extension only *proposes* operations; the Rust core validates them, `plan` shows them,
and the executor applies them. A third-party command never runs unannounced. The extension API is
unstable (`api = 0`); both official tools run on it, and it stays `api = 0` until it is frozen as
`api = 1` in M3.

To write a tool extension, give your coding agent the guide
[Writing a Tool Extension](https://airsstack.github.io/agentstacks/ch06-00-authoring.html), or its raw link:

```text
Build an agentstacks extension for <tool>.
Follow https://raw.githubusercontent.com/airsstack/agentstacks/main/book/src/ch06-00-authoring.md
```

## Design rules

- **Nothing is ambient.** An extension gets nothing it didn't declare. Every change appears in the
  plan, and a refusal names what was missing.
- **The human owns the intent.** `agentstacks.toml` is the committed record of what a project uses.
- **Lua is the only scripting language.** Hook logic is Lua run by airsl. Where a tool demands
  another language (OpenCode hooks are JavaScript plugins), the extension generates a thin shim
  that only forwards events to airsl.
- **Unsupported is visible.** An item a tool can't express is reported as skipped, never dropped
  silently.

## Layout

```text
agentstacks/
├─ crates/
│  ├─ agentstacks/       core library: types · port · app · fs · process · host     built
│  ├─ agentstacks-cli/   init · plan · apply · doctor · update                       built
│  ├─ claudevs/          deterministic test engine for Claude Code setups            built (moved from claudestacks)
│  └─ claudevs-cli/      the claudevs binary                                         built
├─ extensions/           official tool extensions: claude/ built · opencode/ built
├─ packs/                official packs: core · sdlc · journal · guideline-*         built
├─ install/              installer scripts, served on Pages                          built
├─ book/                 the agentstacks book (mdBook), on airsstack.github.io/agentstacks   built
└─ RELEASING.md          the maintainer's release runbook                            built
```

## Documentation

The agentstacks book, https://airsstack.github.io/agentstacks/, covers installing, the packs, moving
from the claudestacks plugins, the CLI, plan and apply, writing a tool extension, the architecture,
and claudevs. Its source is `book/src/`; contributors start with
[Under the Hood](https://airsstack.github.io/agentstacks/ch07-00-architecture.html): the layers, the
dependency rule, where logic lives, and how it is tested.

## Roadmap

```text
M1  claude extension (api = 0) ✓ released (0.1.0); the claudestacks plugins are packs
M2  opencode extension on the same API ✓ built; ships with the next packs release
M3  freeze api = 1; docs for writing packs and extensions; ext add / ext test
```

## Part of airsstack

agentstacks belongs to the [airsstack](https://github.com/airsstack) organization's "run the
factory" lane. The `airsstack` CLI will hand `airsstack agent …` off to it, and its hooks run on
[airsl](https://github.com/airsstack/airsl).

## License

Apache-2.0. Contributions are accepted under the DCO.
