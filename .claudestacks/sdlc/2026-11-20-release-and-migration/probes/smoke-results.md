# smoke commands with an empty PATH (no git, airsl or other tools)

Script: `scripts/smoke2-probe.sh` (commands and `PATH=/nonexistent` are in it).

```text
--- init --yes
exit=0

11 to write, 0 to delete, 0 unchanged, 0 skipped
applied: 11 written, 0 deleted
--- plan
agentstacks plan

0 to write, 0 to delete, 11 unchanged, 0 skipped
exit=0
--- doctor
healthy
exit=0
```

$ claudevs --version (worktree build)
```text
claudevs 0.1.1
```
