# Architecture

`crates/agentstacks` follows a ports-and-adapters (hexagonal) layout. All logic sits inside, behind
ports, and is tested with fakes. Only the adapters touch the disk, `git`, the `airsl` binary or Lua.

```text
                 ┌──────────────── inside (no OS, no airsl) ───────────────┐
                 │                                                         │
 fs/      ──impl──►  port/   ◄──uses──  app/   ──uses──►  types/           │
 process/ ──impl──►  traits             use cases          domain:         │
 host/    ──impl──►  + fakes            plan, apply,       data + pure     │
                 │                      doctor, update     rules           │
                 └─────────────────────────────────────────────────────────┘
 outside: adapters                       ▲
                                         └── agentstacks-cli wires the real adapters in
 process/ ──uses──► fs/cache   (the one edge from one adapter to another)
 types/   ──uses──► port::{FileTree, EntryKind}   (the read-only tree the format loaders take)
```

## Modules

| Module | Holds | State |
|---|---|---|
| `types/` | the domain: the pack, extension, project and lock formats, and pure rules (operations, source resolution, plan steps, JSON text edit, the hook protocol) | built |
| `port/` | traits only, each with a fake for tests: `FileTree`, `Workspace`, `Git`, `Binary`, `Adapter` | built |
| `app/` | use cases, generic over ports: `plan`, `apply`, `doctor`, `update` | built |
| `fs/` | adapter: `FileTree` and `Workspace` on the real disk | built |
| `process/` | adapter: `Git` on the `git` CLI, `Binary` on `airsl --version` | built |
| `host/` | adapter: `Adapter` on the airsl sandbox, where extension Lua runs | built |

The command-line crate `agentstacks-cli` is the composition root: it builds the real adapters and
passes them to `app/`.

## Dependency rule

```text
types             → port::{FileTree, EntryKind} only
port              → types
app               → port, types
fs, process, host → port, types;  process → fs::cache only
```

Inside (`types`, `port`, `app`) never imports an adapter and never calls `std::fs`, `std::process`
or `std::net`. `crates/agentstacks/tests/isolation.rs` enforces this.

- `types/` may name only `FileTree` and `EntryKind` from `crate::port`, the read-only tree port the
  pack, extension and project loaders take. `tests/isolation.rs` allows exactly those two names
  inside `src/types`, plus `crate::port::FakeFileTree` in `#[cfg(test)]` code.
- `process/` may use `fs::cache`, the cache-directory helper, and nothing else from `fs/`. It is the
  only edge from one adapter to another.

## Dependency policy

`deny.toml` exempts airsl's subtree from the duplicate-version ban with `skip-tree`, because
airsl 0.1.4 pins `sha2` 0.10 and `toml` 0.8 while agentstacks uses `sha2` 0.11 and `toml` 1. The
exemption can go once airsl moves to those versions. `Zlib` is in the allowed licences, for
`foldhash`, which `serde_json`'s `preserve_order` feature pulls in.

## Where logic lives

| Layer | Holds | Example |
|---|---|---|
| `types/` | **rules**: pure decisions, data in, answer out | "the file's hash differs from the lock, so the step is refused" |
| `app/` | **flows**: the order of steps; it calls a port, hands the result to a rule, then calls the next port | read the lock, resolve sources, run the extension, decide each file's step |
| `port/` | **abstraction**: traits and fakes, no logic | `Workspace::resolve(path)` |
| adapters | **translation**: no decisions | bytes on disk, `git` output, Lua values ↔ `Operation` |

One rule, traced through the layers:

```text
"a file changed since the last apply is refused"

types   decide_locked(lock_hash, state, new) → no action, so the step is StepError::Changed:
          "<path>: changed since the last apply"
          pure: three values in, one answer out
app     plan asks port::Workspace::resolve(path) for the disk state
          flow: no rule of its own
fs      StdWorkspace::resolve returns the real disk state
          translation
```

## Testing

| What | How | Needs disk, git or airsl |
|---|---|---|
| every rule (`types/`) | unit tests on plain values | no |
| every flow (`app/`): plan, apply order, conflicts, stale lock, doctor | unit tests with `FakeWorkspace`, `FakeGit`, `FakeBinary`, `FakeAdapter` | no |
| each adapter | one contract test: does the real adapter behave like its fake? | yes, only these |

Logic is never tested only through an end-to-end run. If a rule can only be reached end to end, it
is in the wrong layer.

## Rust rules applied

| Rule | Applied as |
|---|---|
| one job per module, no God object | each module above has one job; `app/` holds flows, never rules |
| one type per concept | `Operation` is defined once, in `types/`; `host/` returns it, never a copy |
| static dispatch | ports are generic parameters (`W: Workspace`); the one `dyn` is airsl's `ModuleSet::insert(Box<dyn HostModule>)`, which airsl's API requires |
| `mod.rs` is export-only | every `mod.rs` holds only its docs and `mod` / `pub use` lines |
| naming | one-word folder and file names, no `_` |
