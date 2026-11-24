# agentstacks and airsl on PATH (agentstacks 0.0.0, worktree HEAD 0fe1f61)

Project: extensions/claude copied to ext/claude, packs/core to packs/core.

```text
$ PATH=/usr/bin:/bin agentstacks plan   # no airsl
agentstacks.toml: airsl not found on PATH (needed by core)
exit=1
$ PATH=<dir with an airsl that prints "airsl 0.1.4">:/usr/bin:/bin agentstacks plan
agentstacks.toml: airsl 0.1.4 installed, packs need >=0.1.5
exit=1
$ agentstacks plan | tail -3   # airsl 0.1.5 at ~/.local/bin/airsl
  airsl    0.1.5 (new pin)

39 to write, 0 to delete, 0 unchanged, 1 skipped
```

## Negative: claudevs never runs the airsl binary

```text
$ grep -rn "\"airsl\"" crates/claudevs/src crates/claudevs-cli/src
exit=1
$ grep -rn "\"airsl\"" crates/agentstacks/src/process   # control
crates/agentstacks/src/process/binary.rs:26:const AIRSL: &str = "airsl";
crates/agentstacks/src/process/binary.rs:224:        let path = put(dir.path(), "airsl", 0o755)?;
crates/agentstacks/src/process/binary.rs:235:        let path = put(dir.path(), "airsl", 0o755)?;
```
