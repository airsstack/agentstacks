# Installation

## A prebuilt binary

No Rust toolchain is needed:

```sh
curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh
curl -LsSf https://airsstack.github.io/agentstacks/install.sh | AGENTSTACKS_VERSION=0.1.0 sh
```

The first line installs the latest release. The second installs the version you name.

agentstacks needs `airsl` on `PATH`, so the script checks for it. When it finds airsl, it leaves it
as it is and prints:

```text
agentstacks-install: airsl found at <path>; leaving it as it is
```

When airsl is missing, it installs the latest airsl and prints:

```text
agentstacks-install: agentstacks needs airsl, which is not installed; installing the latest airsl with https://airsstack.github.io/airsl/install.sh
```

An airsl that is found is never replaced or upgraded, even when it is too old. `agentstacks plan`
then names the version it needs.

The binaries go to `~/.local/bin`, unless `AGENTSTACKS_CLI_INSTALL_DIR` is set. Archives exist for
macOS and Linux, on x86_64 and aarch64. There is no Windows build.

## From crates.io

```sh
cargo install agentstacks-cli
```

Note: this route does not install airsl, which `plan` needs (≥0.1.5). Install it with:

```sh
curl -LsSf https://airsstack.github.io/airsl/install.sh | sh
```

## Check it works

```sh
agentstacks --version
```

prints `agentstacks 0.1.0`. `airsl --version` reports 0.1.5 or newer.

Next, set up [a first project](ch01-02-first-project.md).

claudevs installs separately: see [its installation](ch08-01-installation.md).
