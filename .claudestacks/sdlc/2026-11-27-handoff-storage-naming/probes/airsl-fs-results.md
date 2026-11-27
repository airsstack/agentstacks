# Probe: airsl 0.1.5 fs semantics the handoff rename relies on

Command: `airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" /tmp/handoff-probe`
Runtime: airsl 0.1.5 (`airsl --version`). Probe source was a throwaway
`probe_test.lua` under /tmp (since deleted); raw output below is verbatim.

## Raw output

```
/tmp/handoff-probe/probe_test.lua
fs.list file: ok=false err=list failed on `/private/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/airsl-04KdSm/f.txt`: Not a directory (os error 20)
fs.list missing: ok=false err=list failed on `/private/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/airsl-04KdSm/nope`: No such file or directory (os error 2)
  ok    list_on_a_file_and_missing_path
fs.mkdir deep: ok=true err=nil is_dir=true
  ok    mkdir_is_recursive_and_exists_checks
new in new: true
legacy in legacy: true
new in legacy (must be false, plain): false
legacy in new (must be false): false
legacy contains /handoff/: true
  ok    plain_find_distinguishes_new_and_legacy_segments
remove_dir empty leaf: ok=true exists=false
remove_dir now-empty parent: ok=true err=nil
remove_dir non-empty dir: ok=true err=nil still_exists=false
  ok    remove_dir_on_an_empty_leaf_and_nonempty_parent
remove_dir nested: ok=true err=nil exists=false
  ok    remove_dir_removes_a_session_tree_with_files
fs.remove on dir: ok=false err=remove failed on `.../d`: Operation not permitted (os error 1) exists=true
  ok    remove_on_a_directory

6 passed, 0 failed (1 files)
```

## Findings the spec relies on

- F1: `fs.remove_dir` is recursive — it removes a non-empty directory with
  files. Any legacy parent-directory cleanup must check emptiness with
  `fs.list` first; `remove_dir` alone is not a safe "remove if empty".
- F2: `fs.remove` on a directory fails (`Operation not permitted`), so
  directory removal must go through `fs.remove_dir`.
- F3: `fs.list` on a file or a missing path raises; callers must `pcall` it
  (`lib/handoff.lua` `sessions()` already does).
- F4: `fs.mkdir` creates intermediate directories.
- F5: plain (`find(..., 1, true)`) substring tests for
  `.airsstack/handoff` and `.airsstack/cc/plugins/claudestacks/handoff`
  do not overlap: neither string contains the other.
- F6: the legacy path also contains a bare `/handoff/` segment, so the
  validator's fallback-tier test would match it even without a session test;
  tier classification must check the session segments explicitly.

## Second probe run (design review follow-up)

Command: `airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" /tmp/handoff-probe2`
(throwaway probe sources under /tmp, since deleted). Verbatim key output:

```
fs members: append:function, atomic_write:function, canonicalize:function, copy:function, create_exclusive:function, exists:function, is_dir:function, is_file:function, list:function, mkdir:function, read:function, read_lines:function, remove:function, remove_dir:function, rename:function, same_content:function, stat:function, tempdir:function, tempfile:function, walk:function, write:function
remove_dir missing: ok=false err=remove_dir failed on `.../nope`: No such file or directory (os error 2)
rename non-empty dir: ok=true err=nil dst_file=true src_exists=false
rename onto existing non-empty dir: ok=false err=rename failed on `.../b`: Directory not empty (os error 66)
rename missing source: ok=false err=rename failed on `.../c`: No such file or directory (os error 2)

4 passed, 0 failed (2 files)
```

## Findings added

- F7: `fs.remove_dir` on a missing path fails (os error 2); a `pcall`
  around it swallows exactly this, which is what a second concurrent
  pruner observes.
- F8: the full `airsstack.fs` surface (list above) has no empty-only
  directory removal: `fs.remove` fails on directories (F2),
  `fs.remove_dir` is recursive (F1), and `fs.rename` moves a non-empty
  directory. A race-free "remove only if still empty" therefore cannot
  be built from this runtime; the parent cleanup's check-then-remove
  window can be documented and bounded, not eliminated.

## Negative-claim searches (design stage, re-run at review)

```
grep -rn "cc/plugins/claudestacks" crates/ --include="*.rs"   # only crates/agentstacks-cli/tests/packs.rs:551
grep -rn "cc/plugins" extensions/                             # no match; control: grep -rln "airsstack_home" extensions/ finds values.lua files
grep -rn "handoff" book/src/                                  # only book/src/ch06-07-musecode.md:200 (hook wiring)
```
