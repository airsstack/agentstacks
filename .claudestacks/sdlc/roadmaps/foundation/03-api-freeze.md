---
status: done
created: 2026-11-20
---

# Milestone: Freeze api = 1 and open it to authors

## Goal

`agentstacks 1.0` is released. Its stability promise covers the extension API, frozen as `api = 1`,
and the pack format: packs and extensions written against `1.0` keep working across every `1.x`
release. Both official extensions run on `api = 1` unchanged. A third party can write a pack or a
tool extension from the book alone, install an extension with `agentstacks ext add`, and check it
with `agentstacks ext test` before using it in a project. Every release before this one is `0.x`.

## Non-goals

- A marketplace, registry or discovery service for community packs and extensions.
- Official support for more tools.
- Stability guarantees for the Rust crates' library API beyond what the CLI and the extension API need.
