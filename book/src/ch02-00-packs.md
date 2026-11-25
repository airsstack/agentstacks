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

The packs are released together as `packs-v*` tags. `agentstacks update` moves a project to the
newest release of the api it speaks (`packs-v1.*` for api 1). One page per pack follows.
