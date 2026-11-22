# Symlinked temp path probe (2026-11-22)

Probe: `probes/symlink-main.rs`, a throwaway crate built with `cargo +1.94.1 new` and run with `cargo +1.94.1 run -q`.

## S1 — macOS

```text
$ cargo +1.94.1 run -q
1 fs::write via link -> victim = "tools = []\n"
2 create_new on link->existing: Err("File exists (os error 17)") kind=Some(AlreadyExists) victim="VICTIM\n"
3 create_new on dangling link: Err("File exists (os error 17)") kind=Some(AlreadyExists) created_outside=false
4 remove_file(link): link_exists=false victim_exists=true
5 create_new after remove: is_symlink=false victim="VICTIM\n"
6 create_new on leftover regular file: Err("File exists (os error 17)") kind=Some(AlreadyExists)
7 remove_file(dir): Err(("Operation not permitted (os error 1)", PermissionDenied))
$ cargo +1.94.1 --version; rustc +1.94.1 --version; uname -sr
cargo 1.94.1 (29ea6fb6a 2026-03-24)
rustc 1.94.1 (e408947bf 2026-03-25)
Darwin 25.3.0
```

## S2 — Linux (not run)

The same probe in `rust:1.94.1-slim` could not run. Docker is backed by OrbStack, which failed to start:

```text
$ orb start
wait for sconrpc ready event: [-32603] Post "http://vmrpc": EOF
$ docker run --rm -v "$S":/p -w /p rust:1.94.1-slim sh -c 'cargo run -q 2>&1; rustc --version; uname -sr'
Cannot connect to the Docker daemon at unix:///Users/hiraq/.orbstack/run/docker.sock. Is the docker daemon running?
```

The author chose to cite macOS only (2026-11-22). Linux is covered by the regression tests, which CI runs on ubuntu-latest (`.github/workflows/ci.yml:28`).
