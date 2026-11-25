# Introduction

agentstacks sets up agentic coding tools from one tool-neutral copy of skills, agents, commands,
hooks and rules. Every such tool wants its setup in its own shape: Claude Code reads `.claude/`,
and the next tool will want something else. agentstacks keeps one copy and writes it into each tool
you use. Before anything is written, you see exactly what will change.

- A **pack** is what the agent gets: a named, versioned bundle of tool-neutral content.
- A **tool extension** is how one tool gets it: a Lua adapter that turns packs into that tool's
  files. It runs under airsl with no file, network or process access, and it only proposes
  operations.
- **`plan`** shows every operation. **`apply`** is the only step that touches disk, and it records
  pinned versions and a hash of every written file in `agentstacks.lock`. `doctor` checks the
  project against that lock.

## Status

agentstacks is at 1.0. It ships official extensions for Claude Code and OpenCode and five official
packs: `core`, `sdlc`, `journal`, `guideline-rust` and `guideline-python`. The pack format and the
extension API are frozen as `api = 1`; [the stability page](ch06-06-stability.md) says what that
promises. A project written for the 0.x releases moves with `agentstacks update`, see
[Moving a Project from 0.x](ch03-01-from-0x.md). You can also add your own tool with
`agentstacks ext add` and check it with `agentstacks ext test`; see
[Writing a Tool Extension](ch06-00-authoring.md).

## claudevs

A Claude Code plugin is a folder of hooks, skills, agents and scripts. Most of it is code that runs
when Claude Code asks it to, and you rarely see it run until something goes wrong in a live session.

`claudevs` runs that code ahead of time, with no model in the loop, so the same input gives the same
answer every time. You describe a case in YAML or Lua, such as "this hook blocks this command", and
`claudevs test` checks it. `claudevs check` goes further and runs the whole gate: manifest
validation, wiring, the cases, and the cases again against the plugin as it looks once installed.

claudevs used to live in the claudestacks repository. It moved here with its book chapters, and it
is released from this repository. Start with
[claudevs Getting Started](ch08-00-claudevs-getting-started.md).

## How the book is laid out

- **agentstacks**: [Getting Started](ch01-00-getting-started.md), [The Packs](ch02-00-packs.md),
  [moving from the claudestacks plugins](ch03-00-migration.md), then the reference chapters: the
  command line, plan and apply, writing a tool extension, and the architecture.
- **claudevs**: getting started, writing cases, the `check` gate, embedding the engine, and how it
  works inside.
- **Appendix**: [the claudevs command-line reference](appendix-01-claudevs-cli.md).

Coming from the claudestacks plugins? Start with
[Moving from the claudestacks Plugins](ch03-00-migration.md).
