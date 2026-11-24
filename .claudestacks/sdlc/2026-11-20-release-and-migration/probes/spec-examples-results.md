# The spec's exact texts, probed (2026-11-24)

Script `spec-probe.sh` (scratch) extracts the §4.1 `toml`, §4.2 `yaml` and §3 `markdown` blocks and
the §2 metadata rows from spec.md, never retyping them, and runs them on the scratch copy of
the workspace (agentstacks 0.1.0, agentstacks-cli 0.1.0, claudevs 0.1.1, claudevs-cli 0.1.2).
In §2 the publish output is filtered to `Packaging|Packaged|warning|error` lines, minus
`aborting upload due to dry run`; in §1 each `dist plan` is summarised by the script's printer.

```text
== 1. dist (§4.1, §4.2)
cargo-dist 0.33.1
$ dist generate (without allow-dirty) exit=0
$ cmp dist-workspace.toml <(§4.1 block)
identical
$ sed -n 36,52p .github/workflows/release.yml
# will hard limit this to 3 tags per commit, as it will assume more tags is a
# mistake.
#
# If there's a prerelease-style suffix to the version, then the release(s)
# will be marked as a prerelease.
# Edited by hand, kept by allow-dirty = ["ci"] in dist-workspace.toml: only agentstacks-cli and
# claudevs-cli tags start a release. Library tags (agentstacks-v*, claudevs-v*) and packs-v* tags
# do not; dist's plan step fails on a packs-v* tag. To regenerate this file, remove allow-dirty,
# run `dist generate`, then reapply this edit and allow-dirty.
on:
  pull_request:
  push:
    tags:
      - 'agentstacks-cli-v[0-9]+.[0-9]+.[0-9]+*'
      - 'claudevs-cli-v[0-9]+.[0-9]+.[0-9]+*'

jobs:
$ dist plan --tag=agentstacks-cli-v0.1.0
exit=0
agentstacks-cli-v0.1.0 [('agentstacks-cli', '0.1.0')] ['agentstacks-cli-installer.sh'] 6 archives
$ dist plan --tag=claudevs-cli-v0.1.2
exit=0
claudevs-cli-v0.1.2 [('claudevs-cli', '0.1.2')] ['claudevs-cli-installer.sh'] 6 archives
$ dist plan   # untagged, as on a pull request
exit=0
announcing v0.1.0
announcing v0.1.2
$ dist plan --tag=agentstacks-cli-v0.1.9   # version not in the manifest
exit=255
  × The provided announcement tag (agentstacks-cli-v0.1.9) claims we're releasing agentstacks-cli 0.1.9, but that
$ dist generate; trigger kept?
exit=0
    tags:
      - 'agentstacks-cli-v[0-9]+.[0-9]+.[0-9]+*'
      - 'claudevs-cli-v[0-9]+.[0-9]+.[0-9]+*'

== 2. crate metadata (§2) + publish dry run
agentstacks ['readme = "README.md"', 'keywords = ["agents", "claude-code", "cli", "lua", "setup"]', 'categories = ["development-tools"]']
agentstacks-cli ['readme = "README.md"', 'keywords = ["agents", "claude-code", "cli", "lua", "setup"]', 'categories = ["command-line-utilities", "development-tools"]']
version                = "0.1.0"
readme = "README.md"
keywords = ["agents", "claude-code", "cli", "lua", "setup"]
categories = ["development-tools"]
version                = "0.1.0"
readme = "README.md"
keywords = ["agents", "claude-code", "cli", "lua", "setup"]
categories = ["command-line-utilities", "development-tools"]
14:agentstacks = { path = "crates/agentstacks", version = "0.1.0" }
17:claudevs = { path = "crates/claudevs", version = "0.1.1" }
11:publish       = true
$ cargo publish --workspace --dry-run --allow-dirty exit=0
   Packaging agentstacks v0.1.0 (<scratch>/crates/agentstacks)
    Packaged 849 files, 2.3MiB (507.1KiB compressed)
   Packaging agentstacks-cli v0.1.0 (<scratch>/crates/agentstacks-cli)
    Packaged 33 files, 164.5KiB (39.6KiB compressed)
   Packaging claudevs v0.1.1 (<scratch>/crates/claudevs)
    Packaged 165 files, 554.1KiB (141.2KiB compressed)
   Packaging claudevs-cli v0.1.2 (<scratch>/crates/claudevs-cli)
    Packaged 10 files, 61.1KiB (18.6KiB compressed)
== 3. packs/CHANGELOG.md through the pack text gates
$ head -3 packs/CHANGELOG.md
# Changelog

All notable changes to the official packs and tool extensions are recorded here. They are released
$ Makefile.toml:173
grep exit=1 (1 = no hit)
$ Makefile.toml:181
grep exit=1 (1 = no hit)
```
