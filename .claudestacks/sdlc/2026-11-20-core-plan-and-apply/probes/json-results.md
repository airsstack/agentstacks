# Probe: serde_json merge behaviour + atomic write (tempfile / std::fs)

Date: 2026-11-21. Platform: macOS (Darwin 25.3.0, APFS under /var/folders tempdir).
Toolchain: `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0 (5f94df478 2026-08-27)`.
Crate: `probe-json` (cargo new --bin, edition 2024). One run per feature set.

## Version lookup (raw)

```
$ cargo search serde_json --limit 1
serde_json = "1.0.154"    # A JSON serialization file format
... and 16401 crates more (use --limit N to see more)
note: to learn more about a package, run `cargo info <name>`
$ cargo search tempfile --limit 1
tempfile = "3.27.0"    # A library for managing temporary files and directories.
... and 746 crates more (use --limit N to see more)
note: to learn more about a package, run `cargo info <name>`
```

## Cargo.lock versions (after both runs)

serde_json 1.0.154, tempfile 3.27.0, serde/serde_core 1.0.229, indexmap 2.14.2 (pulled by
`preserve_order`), hashbrown 0.17.1, itoa 1.0.19, memchr 2.8.3, zmij 1.0.23, rustix 1.1.6,
libc 0.2.191, fastrand 2.5.0, getrandom 0.4.3, errno 0.3.15, bitflags 2.13.2, once_cell 1.21.4.

## Commands

```
cargo run -q                 > out-default.txt   # serde_json default features
cargo run -q --features po   > out-po.txt        # po = ["serde_json/preserve_order"]
```

J8-J10 (filesystem) run only in the default build (`return` before them under `po`); they do not
depend on serde_json features. In J2/J3/J6 lines, `Display=` prints `Debug|Display` of the error.

## Summary

| J | Result |
|---|---|
| J1 | default: keys sorted (`a, hooks, permissions, z`); preserve_order: source order kept, new key appended last (`z, permissions, a, hooks`) |
| J2 | comments and trailing commas rejected. `// c` / `/* c */` after value: `expected `,` or `}` at line 1 column 8`; leading `// c`: `expected value at line 1 column 1`; `{"a":1,}`: `trailing comma at line 1 column 8`; multiline `{\n  "a": 1,\n}\n`: `trailing comma at line 3 column 1`. Category Syntax |
| J3 | duplicate keys accepted silently, last wins (`{"a":1,"a":2}` -> `{"a":2}`; nested object replaced wholesale, not merged). Same both feature sets |
| J4 | `to_string_pretty` = 2-space indent, `": "` separator, NO trailing newline; empty `{}`/`[]` compact. A 4-space file with trailing `\n` round-trips to 2-space without `\n` (not identical) |
| J5 | `Map::remove` / `Vec::remove` work; emptied object kept as `{}`, emptied array kept as `[]` (no pruning) |
| J6 | `[1]`, `"x"`, `null` parse OK as Value (caller must check is_object); `""` -> `EOF while parsing a value at line 1 column 0`; `" \n\t "` -> `EOF while parsing a value at line 2 column 2` (category Eof); `{} x` -> `trailing characters at line 1 column 4`; into `Map<String,Value>` from `[1]` -> `invalid type: sequence, expected a map at line 1 column 0` (Data) |
| J7 | NOT byte-identical in general: `18446744073709551616` -> `1.8446744073709552e+19`; `-9223372036854775809` -> `-9.223372036854776e+18`; `1.50` -> `1.5`; `1e3` -> `1000.0`; `1E400` -> `number out of range at line 1 column 5`. Identical: u64::MAX, i64::MIN, `1.0`, `0.1` |
| J8 | `NamedTempFile::new_in(dir)` + `persist(target)` over existing file: OK. Tempfile is created mode 600 and the replaced target takes the tempfile's mode (target mode is NOT preserved; 600 before -> 600 after only because tempfile is 600). Persist onto a symlink REPLACES the link with a regular file (mode 600); link's real target untouched (still 640, old content). `persist_noclobber` over existing: `failed to persist temporary file: File exists (os error 17)`, AlreadyExists |
| J9 | `fs::rename(file, dir)` (empty or non-empty): `Is a directory (os error 21)`, kind IsADirectory. `persist` over dir: `failed to persist temporary file: Is a directory (os error 21)` |
| J10 | `create_dir_all(file/sub/deeper)`: `Not a directory (os error 20)`, kind NotADirectory (Display has no path). `create_dir_all(file)`: `File exists (os error 17)`, AlreadyExists. `NamedTempFile::new_in(file/sub)`: `Not a directory (os error 20) at path "<...>/blocker/sub/.tmpKdQ7K2"`, NotADirectory (tempfile adds path) |

## Raw output: default features (byte-for-byte)

```
== features: default ==
J1: to_string_pretty =
{
  "a": 2,
  "hooks": {
    "PreToolUse": []
  },
  "permissions": {
    "allow": [
      "Bash"
    ]
  },
  "z": 1
}
J1: to_string = {"a":2,"hooks":{"PreToolUse":[]},"permissions":{"allow":["Bash"]},"z":1}
J2:
  line comment: ERR Display=Error("expected `,` or `}`", line: 1, column: 8)|expected `,` or `}` at line 1 column 8 line=1 col=8 class=Syntax
  block comment: ERR Display=Error("expected `,` or `}`", line: 1, column: 8)|expected `,` or `}` at line 1 column 8 line=1 col=8 class=Syntax
  leading comment: ERR Display=Error("expected value", line: 1, column: 1)|expected value at line 1 column 1 line=1 col=1 class=Syntax
  trailing comma obj: ERR Display=Error("trailing comma", line: 1, column: 8)|trailing comma at line 1 column 8 line=1 col=8 class=Syntax
  trailing comma arr: ERR Display=Error("trailing comma", line: 1, column: 11)|trailing comma at line 1 column 11 line=1 col=11 class=Syntax
  trailing comma multiline: ERR Display=Error("trailing comma", line: 3, column: 1)|trailing comma at line 3 column 1 line=3 col=1 class=Syntax
J3:
  dup: OK Object {"a": Number(2)} -> {"a":2}
  dup nested: OK Object {"o": Object {"y": Number(2)}} -> {"o":{"y":2}}
J4: pretty="{\n  \"a\": 1,\n  \"b\": [\n    1,\n    2\n  ],\n  \"c\": {},\n  \"d\": [],\n  \"e\": {\n    \"f\": \"g\"\n  }\n}"
J4: ends_with_newline=false
J4: input(4-space)="{\n    \"a\": 1,\n    \"b\": [\n        1\n    ]\n}\n"
J4: roundtrip      ="{\n  \"a\": 1,\n  \"b\": [\n    1\n  ]\n}"
J4: identical=false
J4: roundtrip bytes=[123, 10, 32, 32, 34, 97, 34, 58, 32, 49, 44, 10, 32, 32, 34, 98, 34, 58, 32, 91, 10, 32, 32, 32, 32, 49, 10, 32, 32, 93, 10, 125]
J5: removed_key=Some(Array [String("x")]) removed_el=String("Bash")
J5: after = {"hooks":{},"perm":{"allow":["Read"]}}
J5: pretty = "{\n  \"hooks\": {},\n  \"perm\": {\n    \"allow\": [\n      \"Read\"\n    ]\n  }\n}"
J5: after emptying array = {"hooks":{},"perm":{"allow":[]}}
J5: pretty = "{\n  \"hooks\": {},\n  \"perm\": {\n    \"allow\": []\n  }\n}"
J6:
  array: OK Array [Number(1)] -> [1]
  string: OK String("x") -> "x"
  empty: ERR Display=Error("EOF while parsing a value", line: 1, column: 0)|EOF while parsing a value at line 1 column 0 line=1 col=0 class=Eof
  whitespace: ERR Display=Error("EOF while parsing a value", line: 2, column: 2)|EOF while parsing a value at line 2 column 2 line=2 col=2 class=Eof
  null: OK Null -> null
  trailing garbage: ERR Display=Error("trailing characters", line: 1, column: 4)|trailing characters at line 1 column 4 line=1 col=4 class=Syntax
  Map<String,Value> from [1]: ERR invalid type: sequence, expected a map at line 1 column 0 class=Data
J7:
  "18446744073709551616" -> "1.8446744073709552e+19" identical=false Number(1.8446744073709552e+19)
  "18446744073709551615" -> "18446744073709551615" identical=true Number(18446744073709551615)
  "-9223372036854775808" -> "-9223372036854775808" identical=true Number(-9223372036854775808)
  "-9223372036854775809" -> "-9.223372036854776e+18" identical=false Number(-9.223372036854776e+18)
  "1.0" -> "1.0" identical=true Number(1.0)
  "1.50" -> "1.5" identical=false Number(1.5)
  "1e3" -> "1000.0" identical=false Number(1000.0)
  "0.1" -> "0.1" identical=true Number(0.1)
  "1E400" -> ERR number out of range at line 1 column 5
  "{\"n\":18446744073709551616,\"f\":1.0}" -> "{\"f\":1.0,\"n\":1.8446744073709552e+19}" identical=false Object {"f": Number(1.0), "n": Number(1.8446744073709552e+19)}
J8: before: file mode=600 content="old"
J8: tempfile mode=600
J8: persist over existing: OK
J8: after: file mode=600 content="new"
J8: symlink before: link=symlink -> "/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/.tmp5sUVoJ/real.json" mode=755 content="" | real=file mode=640 content="real-old"
J8: persist onto symlink: OK
J8: symlink after: link=file mode=600 content="via-link" | real=file mode=640 content="real-old"
J8: persist_noclobber over existing: ERR failed to persist temporary file: File exists (os error 17) kind=AlreadyExists
J9: rename file over empty dir: ERR Display=Is a directory (os error 21) kind=IsADirectory raw=Some(21)
J9: rename file over non-empty dir: ERR Display=Is a directory (os error 21) kind=IsADirectory raw=Some(21)
J9: NamedTempFile::persist over dir: ERR Display=failed to persist temporary file: Is a directory (os error 21) kind=IsADirectory
J10: create_dir_all through file: ERR Display=Not a directory (os error 20) kind=NotADirectory raw=Some(20)
J10: create_dir_all on the file itself: ERR Display=File exists (os error 17) kind=AlreadyExists raw=Some(17)
J10: NamedTempFile::new_in under file: ERR Display=Not a directory (os error 20) at path "/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/.tmp5sUVoJ/blocker/sub/.tmpKdQ7K2" kind=NotADirectory
```

## Raw output: preserve_order (byte-for-byte)

```
== features: preserve_order ==
J1: to_string_pretty =
{
  "z": 1,
  "permissions": {
    "allow": [
      "Bash"
    ]
  },
  "a": 2,
  "hooks": {
    "PreToolUse": []
  }
}
J1: to_string = {"z":1,"permissions":{"allow":["Bash"]},"a":2,"hooks":{"PreToolUse":[]}}
J2:
  line comment: ERR Display=Error("expected `,` or `}`", line: 1, column: 8)|expected `,` or `}` at line 1 column 8 line=1 col=8 class=Syntax
  block comment: ERR Display=Error("expected `,` or `}`", line: 1, column: 8)|expected `,` or `}` at line 1 column 8 line=1 col=8 class=Syntax
  leading comment: ERR Display=Error("expected value", line: 1, column: 1)|expected value at line 1 column 1 line=1 col=1 class=Syntax
  trailing comma obj: ERR Display=Error("trailing comma", line: 1, column: 8)|trailing comma at line 1 column 8 line=1 col=8 class=Syntax
  trailing comma arr: ERR Display=Error("trailing comma", line: 1, column: 11)|trailing comma at line 1 column 11 line=1 col=11 class=Syntax
  trailing comma multiline: ERR Display=Error("trailing comma", line: 3, column: 1)|trailing comma at line 3 column 1 line=3 col=1 class=Syntax
J3:
  dup: OK Object {"a": Number(2)} -> {"a":2}
  dup nested: OK Object {"o": Object {"y": Number(2)}} -> {"o":{"y":2}}
J4: pretty="{\n  \"a\": 1,\n  \"b\": [\n    1,\n    2\n  ],\n  \"c\": {},\n  \"d\": [],\n  \"e\": {\n    \"f\": \"g\"\n  }\n}"
J4: ends_with_newline=false
J4: input(4-space)="{\n    \"a\": 1,\n    \"b\": [\n        1\n    ]\n}\n"
J4: roundtrip      ="{\n  \"a\": 1,\n  \"b\": [\n    1\n  ]\n}"
J4: identical=false
J4: roundtrip bytes=[123, 10, 32, 32, 34, 97, 34, 58, 32, 49, 44, 10, 32, 32, 34, 98, 34, 58, 32, 91, 10, 32, 32, 32, 32, 49, 10, 32, 32, 93, 10, 125]
J5: removed_key=Some(Array [String("x")]) removed_el=String("Bash")
J5: after = {"hooks":{},"perm":{"allow":["Read"]}}
J5: pretty = "{\n  \"hooks\": {},\n  \"perm\": {\n    \"allow\": [\n      \"Read\"\n    ]\n  }\n}"
J5: after emptying array = {"hooks":{},"perm":{"allow":[]}}
J5: pretty = "{\n  \"hooks\": {},\n  \"perm\": {\n    \"allow\": []\n  }\n}"
J6:
  array: OK Array [Number(1)] -> [1]
  string: OK String("x") -> "x"
  empty: ERR Display=Error("EOF while parsing a value", line: 1, column: 0)|EOF while parsing a value at line 1 column 0 line=1 col=0 class=Eof
  whitespace: ERR Display=Error("EOF while parsing a value", line: 2, column: 2)|EOF while parsing a value at line 2 column 2 line=2 col=2 class=Eof
  null: OK Null -> null
  trailing garbage: ERR Display=Error("trailing characters", line: 1, column: 4)|trailing characters at line 1 column 4 line=1 col=4 class=Syntax
  Map<String,Value> from [1]: ERR invalid type: sequence, expected a map at line 1 column 0 class=Data
J7:
  "18446744073709551616" -> "1.8446744073709552e+19" identical=false Number(1.8446744073709552e+19)
  "18446744073709551615" -> "18446744073709551615" identical=true Number(18446744073709551615)
  "-9223372036854775808" -> "-9223372036854775808" identical=true Number(-9223372036854775808)
  "-9223372036854775809" -> "-9.223372036854776e+18" identical=false Number(-9.223372036854776e+18)
  "1.0" -> "1.0" identical=true Number(1.0)
  "1.50" -> "1.5" identical=false Number(1.5)
  "1e3" -> "1000.0" identical=false Number(1000.0)
  "0.1" -> "0.1" identical=true Number(0.1)
  "1E400" -> ERR number out of range at line 1 column 5
  "{\"n\":18446744073709551616,\"f\":1.0}" -> "{\"n\":1.8446744073709552e+19,\"f\":1.0}" identical=false Object {"n": Number(1.8446744073709552e+19), "f": Number(1.0)}
```

## Source: Cargo.toml

```toml
[package]
name = "probe-json"
version = "0.1.0"
edition = "2024"

[features]
po = ["serde_json/preserve_order"]

[dependencies]
serde_json = "=1.0.154"
tempfile = "=3.27.0"
```

## Source: src/main.rs

```rust
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn parse(label: &str, s: &str) {
    match serde_json::from_str::<Value>(s) {
        Ok(v) => println!("  {label}: OK {v:?} -> {}", serde_json::to_string(&v).unwrap()),
        Err(e) => println!(
            "  {label}: ERR Display={e:?}|{e} line={} col={} class={:?}",
            e.line(),
            e.column(),
            e.classify()
        ),
    }
}

fn describe(p: &Path) -> String {
    match fs::symlink_metadata(p) {
        Ok(m) => {
            let ft = m.file_type();
            let kind = if ft.is_symlink() {
                format!("symlink -> {:?}", fs::read_link(p).unwrap())
            } else if ft.is_dir() {
                "dir".to_string()
            } else {
                "file".to_string()
            };
            format!(
                "{kind} mode={:o} content={:?}",
                m.permissions().mode() & 0o7777,
                if ft.is_file() { fs::read_to_string(p).unwrap_or_default() } else { String::new() }
            )
        }
        Err(e) => format!("missing ({e})"),
    }
}

fn main() {
    let feat = if cfg!(feature = "po") { "preserve_order" } else { "default" };
    println!("== features: {feat} ==");

    // J1
    let mut v: Value = serde_json::from_str(r#"{"z":1,"permissions":{"allow":["Bash"]},"a":2}"#).unwrap();
    v.as_object_mut().unwrap().insert("hooks".into(), json!({"PreToolUse": []}));
    println!("J1: to_string_pretty =\n{}", serde_json::to_string_pretty(&v).unwrap());
    println!("J1: to_string = {}", serde_json::to_string(&v).unwrap());

    // J2
    println!("J2:");
    parse("line comment", "{\"a\":1 // c\n}");
    parse("block comment", "{\"a\":1 /* c */}");
    parse("leading comment", "// c\n{\"a\":1}");
    parse("trailing comma obj", "{\"a\":1,}");
    parse("trailing comma arr", "{\"a\":[1,2,]}");
    parse("trailing comma multiline", "{\n  \"a\": 1,\n}\n");

    // J3
    println!("J3:");
    parse("dup", r#"{"a":1,"a":2}"#);
    parse("dup nested", r#"{"o":{"x":1},"o":{"y":2}}"#);

    // J4
    let v4 = json!({"a": 1, "b": [1, 2], "c": {}, "d": [], "e": {"f": "g"}});
    let s4 = serde_json::to_string_pretty(&v4).unwrap();
    println!("J4: pretty={s4:?}");
    println!("J4: ends_with_newline={}", s4.ends_with('\n'));
    let four = "{\n    \"a\": 1,\n    \"b\": [\n        1\n    ]\n}\n";
    let rt = serde_json::to_string_pretty(&serde_json::from_str::<Value>(four).unwrap()).unwrap();
    println!("J4: input(4-space)={four:?}");
    println!("J4: roundtrip      ={rt:?}");
    println!("J4: identical={}", rt == four);
    println!("J4: roundtrip bytes={:?}", rt.as_bytes());

    // J5
    let mut v5: Value = serde_json::from_str(r#"{"hooks":{"PreToolUse":["x"]},"perm":{"allow":["Bash","Read"]}}"#).unwrap();
    let removed_key = v5["hooks"].as_object_mut().unwrap().remove("PreToolUse");
    let arr = v5["perm"]["allow"].as_array_mut().unwrap();
    let idx = arr.iter().position(|x| x == "Bash").unwrap();
    let removed_el = arr.remove(idx);
    println!("J5: removed_key={removed_key:?} removed_el={removed_el:?}");
    println!("J5: after = {}", serde_json::to_string(&v5).unwrap());
    println!("J5: pretty = {:?}", serde_json::to_string_pretty(&v5).unwrap());
    v5["perm"]["allow"].as_array_mut().unwrap().clear();
    println!("J5: after emptying array = {}", serde_json::to_string(&v5).unwrap());
    println!("J5: pretty = {:?}", serde_json::to_string_pretty(&v5).unwrap());

    // J6
    println!("J6:");
    parse("array", "[1]");
    parse("string", "\"x\"");
    parse("empty", "");
    parse("whitespace", " \n\t ");
    parse("null", "null");
    parse("trailing garbage", "{} x");
    match serde_json::from_str::<serde_json::Map<String, Value>>("[1]") {
        Ok(m) => println!("  Map<String,Value> from [1]: OK {m:?}"),
        Err(e) => println!("  Map<String,Value> from [1]: ERR {e} class={:?}", e.classify()),
    }

    // J7
    println!("J7:");
    for s in [
        "18446744073709551616",
        "18446744073709551615",
        "-9223372036854775808",
        "-9223372036854775809",
        "1.0",
        "1.50",
        "1e3",
        "0.1",
        "1E400",
        "{\"n\":18446744073709551616,\"f\":1.0}",
    ] {
        match serde_json::from_str::<Value>(s) {
            Ok(v) => {
                let out = serde_json::to_string(&v).unwrap();
                println!("  {s:?} -> {out:?} identical={} {v:?}", out == s);
            }
            Err(e) => println!("  {s:?} -> ERR {e}"),
        }
    }

    if cfg!(feature = "po") {
        return; // filesystem probes are feature-independent; run once
    }

    // J8
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("settings.json");
    fs::write(&target, "old").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
    println!("J8: before: {}", describe(&target));
    let mut tmp = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    println!("J8: tempfile mode={:o}", tmp.as_file().metadata().unwrap().permissions().mode() & 0o7777);
    tmp.write_all(b"new").unwrap();
    match tmp.persist(&target) {
        Ok(_) => println!("J8: persist over existing: OK"),
        Err(e) => println!("J8: persist over existing: ERR {e}"),
    }
    println!("J8: after: {}", describe(&target));

    // symlink target
    let real = dir.path().join("real.json");
    fs::write(&real, "real-old").unwrap();
    fs::set_permissions(&real, fs::Permissions::from_mode(0o640)).unwrap();
    let link = dir.path().join("link.json");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    println!("J8: symlink before: link={} | real={}", describe(&link), describe(&real));
    let mut tmp2 = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    tmp2.write_all(b"via-link").unwrap();
    match tmp2.persist(&link) {
        Ok(_) => println!("J8: persist onto symlink: OK"),
        Err(e) => println!("J8: persist onto symlink: ERR {e}"),
    }
    println!("J8: symlink after: link={} | real={}", describe(&link), describe(&real));

    // persist_noclobber over existing, for reference
    let mut tmp3 = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    tmp3.write_all(b"x").unwrap();
    match tmp3.persist_noclobber(&target) {
        Ok(_) => println!("J8: persist_noclobber over existing: OK"),
        Err(e) => println!("J8: persist_noclobber over existing: ERR {e} kind={:?}", e.error.kind()),
    }

    // J9
    let d9 = dir.path().join("adir");
    fs::create_dir(&d9).unwrap();
    let f9 = dir.path().join("afile");
    fs::write(&f9, "x").unwrap();
    match fs::rename(&f9, &d9) {
        Ok(()) => println!("J9: rename file over empty dir: OK"),
        Err(e) => println!("J9: rename file over empty dir: ERR Display={e} kind={:?} raw={:?}", e.kind(), e.raw_os_error()),
    }
    fs::write(d9.join("child"), "c").unwrap();
    match fs::rename(&f9, &d9) {
        Ok(()) => println!("J9: rename file over non-empty dir: OK"),
        Err(e) => println!("J9: rename file over non-empty dir: ERR Display={e} kind={:?} raw={:?}", e.kind(), e.raw_os_error()),
    }
    let mut tmp9 = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    tmp9.write_all(b"x").unwrap();
    match tmp9.persist(&d9) {
        Ok(_) => println!("J9: NamedTempFile::persist over dir: OK"),
        Err(e) => println!("J9: NamedTempFile::persist over dir: ERR Display={e} kind={:?}", e.error.kind()),
    }

    // J10
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, "x").unwrap();
    match fs::create_dir_all(blocker.join("sub/deeper")) {
        Ok(()) => println!("J10: create_dir_all through file: OK"),
        Err(e) => println!("J10: create_dir_all through file: ERR Display={e} kind={:?} raw={:?}", e.kind(), e.raw_os_error()),
    }
    match fs::create_dir_all(&blocker) {
        Ok(()) => println!("J10: create_dir_all on the file itself: OK"),
        Err(e) => println!("J10: create_dir_all on the file itself: ERR Display={e} kind={:?} raw={:?}", e.kind(), e.raw_os_error()),
    }
    match tempfile::NamedTempFile::new_in(blocker.join("sub")) {
        Ok(_) => println!("J10: NamedTempFile::new_in under file: OK"),
        Err(e) => println!("J10: NamedTempFile::new_in under file: ERR Display={e} kind={:?}", e.kind()),
    }
}
```

## Cargo.lock (full)

```toml
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "bitflags"
version = "2.13.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3ded4057c258ba199e2d26386d3af3780957ecaee6c4ef4041c6b4b8b97c0b06"

[[package]]
name = "cfg-if"
version = "1.0.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4e7648175b45a9a48536d676f68d918270699102aa8dab5496df06904c914600"

[[package]]
name = "equivalent"
version = "1.0.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "00d174d5400e5e8fd687ad1049e2f578285fa914201b1af7e8b112a4546bd826"

[[package]]
name = "errno"
version = "0.3.15"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e35d6e78f4cebb90e51d543db8fe41528851d6deeef6ef21ebd4bfdc1e784776"
dependencies = [
 "libc",
 "windows-sys",
]

[[package]]
name = "fastrand"
version = "2.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "da7c62ceae207dd37ea5b845da6a0696c799f85e97da1ab5b7910be3c1c80223"

[[package]]
name = "foldhash"
version = "0.2.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "77ce24cb58228fbb8aa041425bb1050850ac19177686ea6e0f41a70416f56fdb"

[[package]]
name = "getrandom"
version = "0.4.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "300e883d756b2e4ec94e02791f39b04b522276138852cfc41d9fb7e904106099"
dependencies = [
 "cfg-if",
 "libc",
 "r-efi",
]

[[package]]
name = "hashbrown"
version = "0.17.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ed5909b6e89a2db4456e54cd5f673791d7eca6732202bbf2a9cc504fe2f9b84a"

[[package]]
name = "indexmap"
version = "2.14.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cc4e190f5d26ca7051642629da2c52fc03bde85a03197c99408dcd291734c855"
dependencies = [
 "equivalent",
 "hashbrown",
]

[[package]]
name = "itoa"
version = "1.0.19"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3bd1574e617a9a6c1f8f8181bd7e841e1f20e4277607ef28945765188d6284d4"

[[package]]
name = "libc"
version = "0.2.191"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "65cd87c9a6d60536580b316ad985726104730a5151c99845f2490428ef91240a"

[[package]]
name = "linux-raw-sys"
version = "0.12.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "32a66949e030da00e8c7d4434b251670a91556f4144941d37452769c25d58a53"

[[package]]
name = "memchr"
version = "2.8.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cf8baf1c55e62ffcace7a9f06f4bd9cd3f0c4beb022d3b367256b91b87513d98"

[[package]]
name = "once_cell"
version = "1.21.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9f7c3e4beb33f85d45ae3e3a1792185706c8e16d043238c593331cc7cd313b50"

[[package]]
name = "probe-json"
version = "0.1.0"
dependencies = [
 "serde_json",
 "tempfile",
]

[[package]]
name = "proc-macro2"
version = "1.0.107"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "985e7ec9bb745e6ce6535b544d84d6cd6f7ad8bd711c398938ae983b91a766d9"
dependencies = [
 "unicode-ident",
]

[[package]]
name = "quote"
version = "1.0.47"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1fbf4db142a473a8d80c26bbf18454ed458bf8d26c8219c331daecfdbd079001"
dependencies = [
 "proc-macro2",
]

[[package]]
name = "r-efi"
version = "6.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f8dcc9c7d52a811697d2151c701e0d08956f92b0e24136cf4cf27b57a6a0d9bf"

[[package]]
name = "rustix"
version = "1.1.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e5e76742e5294153d4dde1207dbec5bdd81eef6fabdc0d2118babadabfd858aa"
dependencies = [
 "bitflags",
 "errno",
 "libc",
 "linux-raw-sys",
 "windows-sys",
]

[[package]]
name = "serde"
version = "1.0.229"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4148590afebada386688f18773da617792bf2ef03ffc1e4cbd2b1d45b023e0ba"
dependencies = [
 "serde_core",
]

[[package]]
name = "serde_core"
version = "1.0.229"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "67dca2c9c51e58a4791a4b1ed58308b39c64224d349a935ab5039aa360942a48"
dependencies = [
 "serde_derive",
]

[[package]]
name = "serde_derive"
version = "1.0.229"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e7a5d71263a5a7d47b41f6b3f06ba276f10cc18b0931f1799f710578e2309348"
dependencies = [
 "proc-macro2",
 "quote",
 "syn",
]

[[package]]
name = "serde_json"
version = "1.0.154"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e7e9cc8b1b85264074fbcc02a88680c4096b1e47df8f739dceb03bf482f04bd6"
dependencies = [
 "foldhash",
 "indexmap",
 "itoa",
 "memchr",
 "serde",
 "serde_core",
 "zmij",
]

[[package]]
name = "syn"
version = "3.0.10"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4ae9ccbea3a7b0b09443036490c8f34a8b1b760fd345e5c0dae0aac718faea79"
dependencies = [
 "proc-macro2",
 "quote",
 "unicode-ident",
]

[[package]]
name = "tempfile"
version = "3.27.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "32497e9a4c7b38532efcdebeef879707aa9f794296a4f0244f6f69e9bc8574bd"
dependencies = [
 "fastrand",
 "getrandom",
 "once_cell",
 "rustix",
 "windows-sys",
]

[[package]]
name = "unicode-ident"
version = "1.0.27"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a2c754d6c33795a1c324727428e5a7dedb5b06195f9890bdbcba760d3e246563"

[[package]]
name = "windows-link"
version = "0.2.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f0805222e57f7521d6a62e36fa9163bc891acd422f971defe97d64e70d0a4fe5"

[[package]]
name = "windows-sys"
version = "0.61.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ae137229bcbd6cdf0f7b80a31df61766145077ddf49416a728b02cb3921ff3fc"
dependencies = [
 "windows-link",
]

[[package]]
name = "zmij"
version = "1.0.23"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "29666d0abbfad1e3dc4dcf6144730dd3a3ab225bbbdac83319345b1b44ccfc1b"
```
