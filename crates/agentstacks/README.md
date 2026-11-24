# agentstacks

The core library of agentstacks, a tool that sets up agentic coding tools (Claude Code first,
OpenCode second) from one tool-neutral copy of skills, agents, commands, hooks and rules.

This crate holds the logic: the pack, extension, project and lock formats, the plan, apply, doctor
and update flows, the extension host that runs a tool extension under airsl, and the hook runtime.
It backs the [`agentstacks`](https://crates.io/crates/agentstacks-cli) command-line tool, which is
what most people want. Use this crate to drive the same core from your own code or tests.

## Stability

The API is unstable during `0.x`. Expect breaking changes between minor versions.

## Documentation

- [Under the Hood](https://airsstack.github.io/agentstacks/ch07-00-architecture.html) explains how
  the layers fit together.
- The API reference is the rustdoc on [docs.rs](https://docs.rs/agentstacks).

## License

Apache-2.0.
