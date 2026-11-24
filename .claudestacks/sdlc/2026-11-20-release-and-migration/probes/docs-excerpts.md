# Cited documentation excerpts (fetched 2026-11-24, HTML tags stripped)

## POSIX shell, set -n (https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html, lines 5483-5487)

```text
-n

The shell shall read commands but does not execute them; this can be used to check for shell script syntax errors. An
interactive shell may ignore this option.

```

## Cargo manifest, keywords (https://doc.rust-lang.org/cargo/reference/manifest.html, lines 423-425)

```text
crates.io allows a maximum of 5 keywords. Each keyword must be
ASCII text, have at most 20 characters, start with an alphanumeric character,
and only contain letters, numbers, _, - or +.
```

## Which packs require airsl (grep -n airsl packs/*/pack.toml, worktree HEAD 0fe1f61)

```text
packs/core/pack.toml:10:airsl = ">=0.1.5"
packs/journal/pack.toml:11:airsl = ">=0.1.5"
packs/sdlc/pack.toml:11:airsl = ">=0.1.5"
```

## cargo publish (https://doc.rust-lang.org/cargo/commands/cargo-publish.html, lines 185-215, tags and blank lines stripped)

```text
                        cargo-publish(1)
NAME
cargo-publish — Upload a package to the registry
SYNOPSIS
cargo publish [options]
DESCRIPTION
This command will create a distributable, compressed .crate file with the
source code of the package in the current directory and upload it to a
registry. The default registry is https://crates.io. This performs the
following steps:
Performs a few checks, including:
Checks the package.publish key in the manifest for restrictions on
which registries you are allowed to publish to.
Create a .crate file by following the steps in cargo-package(1).
Upload the crate to the registry. The server will perform additional
checks on the crate.
The client will poll waiting for the package to appear in the index,
and may timeout. In that case, you will need to check for completion
manually. This timeout does not affect the upload.
This command requires you to be authenticated using cargo-login(1)
or environment variables of the registry.token
and registries.&lt;name&gt;.token config fields.
See the reference for more details about
```
