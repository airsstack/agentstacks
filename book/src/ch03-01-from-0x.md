# Moving a Project from 0.x

This page is for you if a project already runs on agentstacks 0.x. 1.0 reads the same
`agentstacks.toml` and `agentstacks.lock`. The one change is the `api` number: 0.x wrote `api = 0`
and 1.0 speaks `api = 1`. `agentstacks update` moves a project from one to the other.

## Install 1.0 first

Install the new binary before anything else, so it replaces the 0.x one:

```sh
curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh
```

See [Installation](ch01-01-installation.md) for the other routes. Check which one you have with
`agentstacks --version`.

## Run update

In the project:

```sh
agentstacks update
```

`update` sees `api = 0` and prints that first, then each official pack and extension that has a
newer release. The 1.0 binary only takes releases of its own major version, so a project at
`packs-v0.2.0` moves to the newest `packs-v1.*` tag:

```text
  api: 0 → 1
  packs.core: packs-v0.2.0 → packs-v1.0.0
  tools.claude: packs-v0.2.0 → packs-v1.0.0
agentstacks.toml updated
```

It then rewrites `agentstacks.toml` with the new tags and `api = 1`, keeping your comments, and
plans, asks and applies like `apply` does. [The Command Line](ch04-00-cli.md) lists the steps and
the exit codes.

A 0.x project is moved even when no tag is newer: it prints `  api: 0 → 1` first, then a `  note`
row for each git source with nothing newer, and never `nothing to update`.

The first apply after the move shows some `update` steps for files the packs wrote before. Those
are the hook runtime, the tool's translator and, for OpenCode, its plugin, whose header names the
`api`. They say `api 1`
now, and the lock holds the old text, so they are not edits of yours.

## Until you run update

`plan` and `apply` refuse a project that still says `api = 0`, and `ext add` does too. Each prints:

```text
agentstacks.toml: api 0 is from agentstacks 0.x (run agentstacks update)
```

`doctor` is unaffected. It reads only `agentstacks.lock`, which 1.x reads as it is at either api,
so on a project that has not moved it reports drift as before. An apply that has nothing to write
leaves a lock at `api = 0` as it was, and that is harmless; the next apply that writes anything
writes `api = 1`.

## Path and third-party sources

`update` moves your `agentstacks.toml`. It does not change a pack or an extension you wrote or took
from somewhere else. Each of those carries its own `api` in `pack.toml` or `extension.toml`, and 1.0
refuses one that is still at `api = 0`. The plan at the end of `update` shows it:

```text
packs/demo/pack.toml: demo: api 0 is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)
ext/claude/extension.toml: extension.api: 0 is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)
```

When the pack's name is missing or invalid, the first starts `packs/demo/pack.toml: pack.api: 0 is
from agentstacks 0.x` instead. Because `agentstacks.toml` was already rewritten, `update` ends with
`agentstacks.toml updated; fix the above and run apply` and exits 1.

What to do:

- **A source you own** (a `path` source): change `api = 0` to `api = 1` in its `pack.toml` or
  `extension.toml`. Check what else 1.0 expects in
  [The Extension API](ch06-01-extensions.md) and [Hooks](ch06-02-hooks.md), then run
  `agentstacks apply`.
- **A third-party git source:** its author has to publish a release at `api = 1`. Move to that tag
  with `agentstacks update`, or edit the `tag` in `agentstacks.toml` by hand. Until then, 1.0
  cannot install it.

The official repository is the one at `https://github.com/airsstack/agentstacks`, byte for byte.
Any other URL, a mirror or one with a `.git` suffix included, keeps the plain latest-tag rule and
may offer a tag of any major version.

## If a 0.x binary is still around

A 0.x binary does not know `api = 1`. If one runs against a project after the move, or runs
`update` once `packs-v1.0.0` exists, it refuses what it fetches or reads, for example:

```text
agentstacks.toml: api 1 not supported (this agentstacks speaks 0)
```

or the same shape for the lock (`agentstacks.lock: api 1 not supported (this agentstacks speaks 0)`),
a pack (`<pack>: api 1 not supported (this agentstacks speaks 0)`) and an extension
(`extension.api: 1 not supported (this agentstacks speaks 0)`). Install 1.0 and the
refusal goes. If a 0.x `update` already moved the tags to `packs-v1.0.0`, that is fine: the 1.0
`update` sees `api = 0` still in the file, prints `  api: 0 → 1`, rewrites only that and plans.

Once the project is on `api = 1`, [the stability promise](ch06-06-stability.md) applies to it.
