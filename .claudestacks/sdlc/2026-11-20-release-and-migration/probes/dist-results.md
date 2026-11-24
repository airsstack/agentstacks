# dist 0.33.1: per-binary tags with a hand-edited release trigger

Scratch copy of this workspace (HEAD 0fe1f61): agentstacks 0.1.0, agentstacks-cli 0.1.0,
claudevs 0.1.1, claudevs-cli 0.1.2, publish = true. Script: `dirty-probe.sh` (scratch). Run 2026-11-24.

```text
$ dist --version
cargo-dist 0.33.1
$ grep -h ^version crates/*/Cargo.toml
version                = "0.1.0"
version                = "0.1.0"
version     = "0.1.2"
version     = "0.1.1"
$ cat dist-workspace.toml
[workspace]
members = ["cargo:."]

[dist]
cargo-dist-version = "0.33.1"
ci = "github"
installers = ["shell"]
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "aarch64-unknown-linux-musl", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"]
install-path = "~/.local/bin"
hosting = "github"
install-updater = false
github-attestations = true
global-artifacts-jobs = ["./smoke"]
pr-run-mode = "upload"

[dist.github-custom-runners]
aarch64-apple-darwin = "macos-15"
$ sed -n 41,47p .github/workflows/release.yml   # hand-edited trigger
on:
  pull_request:
  push:
    tags:
      - 'agentstacks-cli-v[0-9]+.[0-9]+.[0-9]+*'
      - 'claudevs-cli-v[0-9]+.[0-9]+.[0-9]+*'

$ dist plan --tag=agentstacks-cli-v0.1.0   # no allow-dirty
exit=255
  │  jobs:
  │    # Run 'dist plan' (or host) to determine what tasks we need to do
  │ 
  help: run 'dist init' to update the file
        ('allow-dirty' in Cargo.toml to ignore out of date contents)

$ (allow-dirty = ["ci"] added under [dist])
$ dist plan --tag=agentstacks-cli-v0.1.0
exit=0
agentstacks-cli-v0.1.0 [('agentstacks-cli', '0.1.0')] ['agentstacks-cli-installer.sh']
$ dist plan --tag=claudevs-cli-v0.1.2
exit=0
claudevs-cli-v0.1.2 [('claudevs-cli', '0.1.2')] ['claudevs-cli-installer.sh']
$ dist plan --tag=claudevs-v0.1.1   # library tag
exit=0
Traceback (most recent call last):
  File "<string>", line 5, in <module>
KeyError: 'artifacts'
$ dist generate; sed -n 44,47p .github/workflows/release.yml
exit=0
    tags:
      - 'agentstacks-cli-v[0-9]+.[0-9]+.[0-9]+*'
      - 'claudevs-cli-v[0-9]+.[0-9]+.[0-9]+*'

```
