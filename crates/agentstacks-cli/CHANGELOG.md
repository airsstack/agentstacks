# Changelog

All notable changes to `agentstacks-cli` (the `agentstacks` binary) are recorded here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

Initial release.

### Added

- The commands `init`, `plan`, `apply`, `doctor` and `update`.
- Official names `--tool claude` and `--pack core|sdlc|journal|guideline-rust|guideline-python`,
  resolved to the newest `packs-v*` tag.
- Prebuilt archives for the six release targets, and the installer
  `curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh`, which also installs airsl
  when it is missing.
