# cargo publish --workspace --dry-run --allow-dirty (cargo 1.94.1)

Same scratch copy as dist-results.md: agentstacks 0.1.0, agentstacks-cli 0.1.0, claudevs 0.1.1,
claudevs-cli 0.1.2, publish = true, workspace dependency versions matching. Run 2026-11-24.
Filtered: only lines matching `Packaging|Packaged|Uploading|warning|error|exit=` are kept
(the `Updating`, `Compiling`, `Verifying` and `Finished` lines are dropped).

```text
   Packaging agentstacks v0.1.0 (<scratch>/crates/agentstacks)
    Packaged 848 files, 2.3MiB (507.0KiB compressed)
   Packaging agentstacks-cli v0.1.0 (<scratch>/crates/agentstacks-cli)
    Packaged 32 files, 164.3KiB (39.5KiB compressed)
   Packaging claudevs v0.1.1 (<scratch>/crates/claudevs)
    Packaged 165 files, 554.1KiB (141.2KiB compressed)
   Packaging claudevs-cli v0.1.2 (<scratch>/crates/claudevs-cli)
    Packaged 10 files, 61.1KiB (18.6KiB compressed)
   Uploading agentstacks v0.1.0 (<scratch>/crates/agentstacks)
warning: aborting upload due to dry run
   Uploading claudevs v0.1.1 (<scratch>/crates/claudevs)
warning: aborting upload due to dry run
   Uploading agentstacks-cli v0.1.0 (<scratch>/crates/agentstacks-cli)
warning: aborting upload due to dry run
   Uploading claudevs-cli v0.1.2 (<scratch>/crates/claudevs-cli)
warning: aborting upload due to dry run
exit=0
```

## crates.io state (curl https://crates.io/api/v1/crates/<name>)

```text
agentstacks: [{'detail': 'crate `agentstacks` does not exist'}]
agentstacks-cli: [{'detail': 'crate `agentstacks-cli` does not exist'}]
claudevs: ['0.1.0']
claudevs-cli: ['0.1.1', '0.1.0']
```
