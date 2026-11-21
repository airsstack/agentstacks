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
```

## Modules

| Module | Holds | State |
|---|---|---|
| `types/` | the domain: the pack, extension, project and lock formats, and pure rules (operations, source resolution, plan steps, JSON text edit, the hook protocol) | formats built; rules planned |
| `port/` | traits only, each with a fake for tests: `FileTree`, `Workspace`, `Git`, `Binary`, `Adapter` | `FileTree` built (today under `types/port/`); the rest planned |
| `app/` | use cases, generic over ports: `plan`, `apply`, `doctor`, `update` | planned |
| `fs/` | adapter: `FileTree` and `Workspace` on the real disk | `FileTree` built |
| `process/` | adapter: `Git` on the `git` CLI, `Binary` on `airsl --version` | planned |
| `host/` | adapter: `Adapter` on the airsl sandbox, where extension Lua runs | planned |

The command-line crate `agentstacks-cli` is the composition root: it builds the real adapters and
passes them to `app/`.

## Dependency rule

```text
types            → nothing
port             → types
app              → port, types
fs, process, host → port, types
```

Inside (`types`, `port`, `app`) never imports an adapter and never calls `std::fs`, `std::process`
or `std::net`. `crates/agentstacks/tests/isolation.rs` enforces this.

## Where logic lives

| Layer | Holds | Example |
|---|---|---|
| `types/` | **rules**: pure decisions, data in, answer out | "the file's hash differs from the lock, so the step is refused" |
| `app/` | **flows**: the order of steps; it calls a port, hands the result to a rule, then calls the next port | read the lock, resolve sources, run the extension, decide each file's step |
| `port/` | **abstraction**: traits and fakes, no logic | `Workspace::hash(path)` |
| adapters | **translation**: no decisions | bytes on disk, `git` output, Lua values ↔ `Operation` |

One rule, traced through the layers:

```text
"a file changed since the last apply is refused"

types   plan step(op, lock_hash, disk_hash) → Refused("<path> changed since last apply")
          pure: three values in, one answer out
app     plan reads disk_hash through port::Workspace
          flow: no rule of its own
fs      StdWorkspace returns the real hash
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
