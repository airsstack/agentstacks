# The Packs

A pack is what the agent gets: a named, versioned bundle of tool-neutral skills, agents, commands,
hooks and rules. Each tool's extension renders it in that tool's own format. The five official packs
come with agentstacks:

| Pack | What it is | Needs |
|---|---|---|
| [`core`](ch02-01-core.md) | coder, reviewer and explorer agents, an `orchestrate` skill that runs them in order, and the hooks: reply rules, project snapshots and the dispatcher that points the agent at the right guideline | airsl ≥0.1.5 |
| [`sdlc`](ch02-02-sdlc.md) | an AI-native software development lifecycle: intent, design, plan, execute, each step leaving a committed Markdown file, with a status board | `core` and airsl ≥0.1.5 |
| [`journal`](ch02-03-journal.md) | a memory for coding agents that you can read: notes in one Obsidian-compatible vault on your machine | `core` and airsl ≥0.1.5 |
| [`guideline-rust`](ch02-04-guideline-rust.md) | Rust engineering rules and a Definition of Done in one skill | nothing |
| [`guideline-python`](ch02-05-guideline-python.md) | Python engineering rules and a Definition of Done in one skill | nothing |

`sdlc` or `journal` without `core` is refused. For `sdlc`:

```text
agentstacks.toml: packs.sdlc: sdlc requires pack "core", which agentstacks.toml does not list (add it: --pack core)
```

Each pack is released on its own stream of tags, `packs-<name>-v*`, carrying only its own
version: a pack's number moves if and only if that pack's shipped content changed, with a
changelog entry beside it. `agentstacks update` moves each pinned pack to the newest release
of its stream whose major version is the `api` this agentstacks speaks (`packs-core-v1.*` for
`api = 1`). One page per pack follows.
