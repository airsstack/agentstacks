# dist 0.33.1: tags, trigger, releases, archives, runners

Scratch copy of this workspace (HEAD 0fe1f61), all four crates set to 0.2.0, publish = true.
Script: `dist-probe.sh` (scratch). Run 2026-11-24.

```text
$ dist --version
cargo-dist 0.33.1
$ grep -h ^version crates/*/Cargo.toml
version                = "0.2.0"
version                = "0.2.0"
version     = "0.2.0"
version     = "0.2.0"
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
tag-namespace = "v"
github-attestations = true
global-artifacts-jobs = ["./smoke"]
pr-run-mode = "upload"

[dist.github-custom-runners]
aarch64-apple-darwin = "macos-15"
$ ls .github/workflows
ci.yml
v-release.yml
$ sed -n 41,46p .github/workflows/v-release.yml
on:
  pull_request:
  push:
    tags:
      - 'v**[0-9]+.[0-9]+.[0-9]+*'

$ grep -n "uses: ./.github/workflows/smoke.yml" .github/workflows/v-release.yml
231:    uses: ./.github/workflows/smoke.yml
$ dist plan --tag=v0.2.0 --output-format=json (releases, archives, runners)
releases: [('agentstacks-cli', '0.2.0'), ('claudevs-cli', '0.2.0')]
executable-zip agentstacks-cli-aarch64-apple-darwin.tar.xz
executable-zip agentstacks-cli-aarch64-unknown-linux-gnu.tar.xz
executable-zip agentstacks-cli-aarch64-unknown-linux-musl.tar.xz
installer agentstacks-cli-installer.sh
executable-zip agentstacks-cli-x86_64-apple-darwin.tar.xz
executable-zip agentstacks-cli-x86_64-unknown-linux-gnu.tar.xz
executable-zip agentstacks-cli-x86_64-unknown-linux-musl.tar.xz
executable-zip claudevs-cli-aarch64-apple-darwin.tar.xz
executable-zip claudevs-cli-aarch64-unknown-linux-gnu.tar.xz
executable-zip claudevs-cli-aarch64-unknown-linux-musl.tar.xz
installer claudevs-cli-installer.sh
executable-zip claudevs-cli-x86_64-apple-darwin.tar.xz
executable-zip claudevs-cli-x86_64-unknown-linux-gnu.tar.xz
executable-zip claudevs-cli-x86_64-unknown-linux-musl.tar.xz
runner macos-15 ['aarch64-apple-darwin']
runner ubuntu-22.04-arm ['aarch64-unknown-linux-gnu']
runner ubuntu-22.04-arm ['aarch64-unknown-linux-musl']
runner macos-15-intel ['x86_64-apple-darwin']
runner ubuntu-22.04 ['x86_64-unknown-linux-gnu']
runner ubuntu-22.04 ['x86_64-unknown-linux-musl']
$ dist plan --tag=packs-v0.1.0
  ╰─▶ unexpected character 'p' while parsing major version number

$ dist plan --tag=agentstacks-cli-v0.2.0 (releases)
releases: [('agentstacks-cli', '0.2.0')]
--- without tag-namespace:
$ sed -n 41,46p .github/workflows/release.yml
on:
  pull_request:
  push:
    tags:
      - '**[0-9]+.[0-9]+.[0-9]+*'

```
