# Changelog

All notable changes to the `core` pack are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the pack follows
[Semantic Versioning](https://semver.org/spec/v2.0.0/): the version moves if
and only if the pack's shipped content changed. Each release is tagged
`packs-core-v<version>`.

## [1.0.1] - 2026-11-27

Baseline. Version 1.0.1 as last released under the `packs-v*` bundle tags
(`packs-v1.1.1`); from here this file records each release of `core`.

### Fixed

- Handoff sessions are minted under `.airsstack/handoff/` instead of the
  legacy `.airsstack/cc/plugins/claudestacks/handoff/`. The first session
  after updating prunes the legacy tree (live leases are respected) and
  removes its empty parent directories; reports written under the legacy path
  remain recognised.

[1.0.1]: https://github.com/airsstack/agentstacks/tree/packs-core-v1.0.1
