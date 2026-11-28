# Versions and Release Streams

Five numbers describe what an agentstacks project runs. Each answers a
different question, and only one of them tells you whether a newer release
exists.

| Number | Where it lives | Question it answers |
|---|---|---|
| Stream tag | One per `packs.*` / `tools.*` source in `agentstacks.toml` | What release this source follows |
| Manifest version | `pack.toml` / `extension.toml` | What code this artifact ships |
| `api` | Every manifest and project file | Whether this binary can read it |
| Lock entry | `agentstacks.lock` (tag, commit, tree hash) | What exact code was installed |
| Bundle tag | Legacy `packs-v*` pins | Nothing new; frozen (below) |

## Streams

Each official pack and extension owns one release stream. Pack streams are
tagged `packs-<name>-v<semver>` (`packs-sdlc-v1.1.0`); extension streams are
tagged `ext-<name>-v<semver>` (`ext-claude-v1.2.0`). A stream tag moves only
when that artifact's shipped content changed. The bare `packs-v` and `ext-v`
prefixes are reserved and never minted.

A manifest version declares the release: at every release the manifest version
equals the stream tag being minted, and the release gate enforces it. Where
historical data disagrees — anything recorded before the gate existed — the
tag wins for resolution. The first tag of a stream is its seed; gates go live
at the seed.

## Frozen bundle tags

Releases before streams shipped under one bundle tag, `packs-v*`, covering
every pack and extension at once. No new bundle tags are minted. Published
bundle tags resolve forever, so an existing pin keeps working; `update`
reports a bundle pin as frozen and points at the artifact's stream instead of
rewriting it.

## Discovering a newer release

`agentstacks update` lists each git source under its own stream prefix and
offers the newest tag of the `api` major this binary speaks. `agentstacks
doctor` reports a newer stream tag as advice without failing: staleness is
news, not damage, and an offline `doctor` behaves exactly as it always has.
