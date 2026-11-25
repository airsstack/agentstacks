# agentstacks

The core library of agentstacks, a tool that sets up agentic coding tools (Claude Code first,
OpenCode second) from one tool-neutral copy of skills, agents, commands, hooks and rules.

This crate holds the logic: the pack, extension, project and lock formats, the plan, apply, doctor
and update flows, the extension host that runs a tool extension under airsl, and the hook runtime.
It backs the [`agentstacks`](https://crates.io/crates/agentstacks-cli) command-line tool, which is
what most people want. Use this crate to drive the same core from your own code or tests.

## Stability

The Rust API of this crate is the engine of the `agentstacks` command-line tool. It is not covered
by the 1.x promise: it can change in any release. What is frozen (the pack format, the extension API
(`api = 1`), the hook runtime, `agentstacks.toml`, `agentstacks.lock` and the command line) is on
[the stability promise](https://airsstack.github.io/agentstacks/ch06-06-stability.html).

## Documentation

- [Under the Hood](https://airsstack.github.io/agentstacks/ch07-00-architecture.html) explains how
  the layers fit together.
- The API reference is the rustdoc on [docs.rs](https://docs.rs/agentstacks).

## License

Apache-2.0.
