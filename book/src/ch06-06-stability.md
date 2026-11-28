# The 1.x Stability Promise

agentstacks 1.0 freezes the pack format and the extension API as `api = 1`. This page says what
that means: what keeps working, what a 1.x release may add, what it may not, and what the promise
does not cover.

Anything that works on agentstacks 1.N works on every 1.M with M ≥ N. A pack or an extension you
write for `api = 1` keeps working until 2.0.

## What is covered

- **The pack format:** the layout, `pack.toml`, skill and agent frontmatter, tokens and vars, the
  hook schema and the event vocabulary, and `native/<tool>/`.
- **The extension API:** `extension.toml`, the sandbox and its limits, the `agentstacks` table, the
  `values` and `plan` events and their payload fields, the operations, and the coverage and
  validation rules. [The Extension API](ch06-01-extensions.md) is the contract.
- **The hook runtime:** the command line, the behaviour of `runtime.lua`, the neutral event and
  result, and the translator contract. [Hooks](ch06-02-hooks.md) is the contract.
- **`agentstacks.toml`.**
- **`agentstacks.lock`:** its format is frozen for all of 1.x. Every 1.x reads and writes the same
  lock, and reads a lock written with `api = 0`.
- **Release stream prefixes:** `packs-<name>-v` and `ext-<name>-v` are frozen for all of 1.x.
  Renaming a prefix would break every pin, so a new naming scheme needs `api = 2`.
  [Versions and Release Streams](ch06-08-versioning.md) is the contract.
- **The command line:** the commands, their flags and their exit codes.

## What a 1.x minor release may add

Additions only, and none of them can break a 1.0 pack or extension:

- new optional keys that only the core reads;
- new payload fields, which extensions ignore;
- new `agentstacks.*` functions;
- new vars, because a var an extension does not supply is already a visible skip (see
  [The Extension API](ch06-01-extensions.md));
- new commands and flags;
- allowing a refused value that creates no new item, such as `grants.policy = "pure"`.

## What needs `api = 2`

These stay frozen for all of 1.x, because each would give a working 1.0 extension an item it cannot
cover:

- new item kinds, such as `commands/` and `rules/`, which are reserved today;
- new neutral events;
- new `match`, agent `tools` or `model` words;
- any other change that would give a 1.0 extension an item it cannot cover.

## What was settled before the freeze

| # | Finding | Settled as |
|---|---|---|
| R1 | `grants.policy = "pure"` is refused while [airsl#12](https://github.com/airsstack/airsl/issues/12) is open, and its text names the api | Allowing it later is an addition. |
| R2 | `commands/` and `rules/` are reserved, and their refusal names the api | Supporting them needs `api = 2`. |
| R3 | A hook with no `match` gets every value | The vocabularies are frozen, so no value an extension does not know can appear in 1.x. |
| R4 | Item ids such as `core/hook/0` | Ids are opaque strings: copy them back, never parse them. |
| R5 | The exact text of refusals and messages | Not covered. Only the location and key shapes are. |
| R6 | A future `packs-v2.*` release would be fetched by the shorthand and by `update` | They never fetch it: the bundle stream is frozen, and each artifact moves only inside its own `*-v1.*` stream (see [Versions and Release Streams](ch06-08-versioning.md) and [The Command Line](ch04-00-cli.md)). |
| R7 | A key added in 1.N to `agentstacks.toml` is `unknown key` to 1.0 | A team that uses a 1.N key needs agentstacks 1.N everywhere. There is no gate for it. |

## What authors do

When a pack or an extension uses something a 1.N release added, declare it:

```toml
[requires]
agentstacks = ">=1.N"
```

An agentstacks that is older than the range refuses the pack or extension with
`"<range>" does not match agentstacks <version> (install a newer agentstacks)` under the key
`requires.agentstacks`, instead of failing later on a key it does not know.

## What is not covered

- The exact wording of refusals, notes and prompts.
- The layout of the plan text.
- The text of `--help`.
- The Rust API of the `agentstacks` library crate. It is the engine of the command line, and only
  the command line is covered.
- claudevs, which has its own releases.

## 2.0

2.0 is the only release that may remove or change a covered behaviour. It would ship as
`api = 2`, with its own move from 1.x.
