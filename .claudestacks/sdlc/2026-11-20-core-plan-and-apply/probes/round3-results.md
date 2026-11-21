# Probe round 3 (2026-11-21)

Crate: probe-round3/src/main.rs. airsl 0.1.4, mlua 0.12.2, serde_json 1.0.154, rustc 1.94.1 (e408947bf 2026-03-25), macOS, git 2.39.3.

## Run 1: serde_json features = [arbitrary_precision, preserve_order]
```text
A1 eval register values twice: OK ()
A1 dispatch values: OK Some(Object {"which": String("second")})
A2 eval: OK ()
A2 dispatch plan: OK Some(Object {"big": Object {"$serde_json::private::Number": String("12345678901234567890")}, "empty": Object {}, "f": Object {"$serde_json::private::Number": String("1.5")}, "first": String("a"), "len": Number(2), "n": Object {"$serde_json::private::Number": String("1")}, "s": String("x"), "seq": Array [Number(1), Number(2)]})
A3 eval: OK ()
A3 dispatch plan returning nil: OK Some(Null)
A4 eval: OK ()
A5 eval: OK ()
A5 dispatch plan: ERR <<lua error in plan: adapter.lua:3: path must be relative
stack traceback:
	[C]: in field 'refuse'
	adapter.lua:3: in function <adapter.lua:1>
>>
convert at adapter.lua:2: OK {}
convert at adapter.lua:3: OK [1,2]
convert at adapter.lua:4: OK {"a":1}
convert at adapter.lua:5: OK [1]
convert at adapter.lua:6: OK 9007199254740993
convert at adapter.lua:7: OK 1.5
convert at adapter.lua:8: OK {"z":1,"a":{"b":true}}
convert at adapter.lua:9: OK {"x":null,"arr":[]}
convert at adapter.lua:10: ERR deserialize error: unsupported value type `function`
J1 2sp:
{
  "permissions": {
    "allow": [
      "Bash(x:*)"
    ]
  },
  "zeta": 1,
  "alpha": 12345678901234567890123,
  "d": 0.10,
  "hooks": {}
}
J1 4sp:
{
    "permissions": {
        "allow": [
            "Bash(x:*)"
        ]
    },
    "zeta": 1,
    "alpha": 12345678901234567890123,
    "d": 0.10,
    "hooks": {}
}
J1 tab:
{
	"permissions": {
		"allow": [
			"Bash(x:*)"
		]
	},
	"zeta": 1,
	"alpha": 12345678901234567890123,
	"d": 0.10,
	"hooks": {}
}
J3 after insert existing+new: {"permissions":{"allow":["Bash(x:*)"]},"zeta":2,"alpha":12345678901234567890123,"d":0.10,"hooks":{},"new":[]}
J4 pointer_mut /hooks: {"permissions":{"allow":["Bash(x:*)"]},"zeta":2,"alpha":12345678901234567890123,"d":0.10,"hooks":{"PreToolUse":[{"matcher":"Read"}]},"new":[]}
J4 pointer_mut /missing/x is_none: true
J5 empty pretty: {
  "o": {},
  "a": []
}
S1 rename dir onto non-empty dir: ERR kind=DirectoryNotEmpty msg=Directory not empty (os error 66)
S1 rename dir onto empty dir: OK (c/f exists=true)
S2 std::fs::write new file mode: 644
S3 mode after rename over 755 file: 644
S4 rename onto link: link is_symlink=false real content="{}"
S4 rename onto canonical target: link2 is_symlink=true real content="B"
S5 canonicalize missing: ERR kind=NotFound msg=No such file or directory (os error 2)
```

## Run 2: serde_json features = [preserve_order]
```text
A1 eval register values twice: OK ()
A1 dispatch values: OK Some(Object {"which": String("second")})
A2 eval: OK ()
A2 dispatch plan: OK Some(Object {"big": Number(1.2345678901234567e+19), "empty": Object {}, "f": Number(1.5), "first": String("a"), "len": Number(2), "n": Number(1), "s": String("x"), "seq": Array [Number(1), Number(2)]})
A3 eval: OK ()
A3 dispatch plan returning nil: OK Some(Null)
A4 eval: OK ()
A5 eval: OK ()
A5 dispatch plan: ERR <<lua error in plan: adapter.lua:3: path must be relative
stack traceback:
	[C]: in field 'refuse'
	adapter.lua:3: in function <adapter.lua:1>
>>
convert at adapter.lua:2: OK {}
convert at adapter.lua:3: OK [1,2]
convert at adapter.lua:4: OK {"a":1}
convert at adapter.lua:5: OK [1]
convert at adapter.lua:6: OK 9007199254740993
convert at adapter.lua:7: OK 1.5
convert at adapter.lua:8: OK {"a":{"b":true},"z":1}
convert at adapter.lua:9: OK {"arr":[],"x":null}
convert at adapter.lua:10: ERR deserialize error: unsupported value type `function`
J1 2sp:
{
  "permissions": {
    "allow": [
      "Bash(x:*)"
    ]
  },
  "zeta": 1,
  "alpha": 1.2345678901234568e+22,
  "d": 0.1,
  "hooks": {}
}
J1 4sp:
{
    "permissions": {
        "allow": [
            "Bash(x:*)"
        ]
    },
    "zeta": 1,
    "alpha": 1.2345678901234568e+22,
    "d": 0.1,
    "hooks": {}
}
J1 tab:
{
	"permissions": {
		"allow": [
			"Bash(x:*)"
		]
	},
	"zeta": 1,
	"alpha": 1.2345678901234568e+22,
	"d": 0.1,
	"hooks": {}
}
J3 after insert existing+new: {"permissions":{"allow":["Bash(x:*)"]},"zeta":2,"alpha":1.2345678901234568e+22,"d":0.1,"hooks":{},"new":[]}
J4 pointer_mut /hooks: {"permissions":{"allow":["Bash(x:*)"]},"zeta":2,"alpha":1.2345678901234568e+22,"d":0.1,"hooks":{"PreToolUse":[{"matcher":"Read"}]},"new":[]}
J4 pointer_mut /missing/x is_none: true
J5 empty pretty: {
  "o": {},
  "a": []
}
S1 rename dir onto non-empty dir: ERR kind=DirectoryNotEmpty msg=Directory not empty (os error 66)
S1 rename dir onto empty dir: OK (c/f exists=true)
S2 std::fs::write new file mode: 644
S3 mode after rename over 755 file: 644
S4 rename onto link: link is_symlink=false real content="{}"
S4 rename onto canonical target: link2 is_symlink=true real content="B"
S5 canonicalize missing: ERR kind=NotFound msg=No such file or directory (os error 2)
```

## G1: git with prompts disabled
```text
$ GIT_TERMINAL_PROMPT=0 git ls-remote --tags --refs -- https://github.com/airsstack/does-not-exist-probe 'refs/tags/packs-v*'
fatal: could not read Username for 'https://github.com': terminal prompts disabled
exit=128
```

## D1: cargo-deny 0.20.2 licenses with the project's deny.toml (copied into the probe crate)
```text
$ cargo deny check licenses   # errors only
error[rejected]: failed to satisfy license requirements
   ┌─ /Users/hiraq/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/foldhash-0.2.0/Cargo.toml:40:12
   │
40 │ license = "Zlib"
   │            ━━━━
   │            │
   │            rejected: license is not explicitly allowed
--
error[unlicensed]: probe-r3 = 0.1.0 is unlicensed
 ├ probe-r3 v0.1.0 (*)

warning[license-not-encountered]: license was not encountered
   ┌─ /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-round3/deny.toml:44:6
   │
44 │     "BSD-3-Clause",
```
(probe-r3 itself is unlicensed; the agentstacks crate is Apache-2.0, so only the foldhash error applies.)

## D2: what enables foldhash
```text
$ cargo tree -i foldhash -e features
foldhash v0.2.0
└── serde_json v1.0.154
    ├── serde_json feature "alloc"
    │   └── serde_json feature "preserve_order"
    │       └── probe-r3 v0.1.0 (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-round3)
    │           └── probe-r3 feature "default" (command-line)
    ├── serde_json feature "default"
    │   ├── airsl v0.1.4
    │   │   └── airsl feature "default"
    │   │       └── probe-r3 v0.1.0 (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-round3) (*)
    │   └── probe-r3 v0.1.0 (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-round3) (*)
    ├── serde_json feature "indexmap"
    │   └── serde_json feature "preserve_order" (*)
    ├── serde_json feature "preserve_order" (*)
    └── serde_json feature "std"
        └── serde_json feature "default" (*)
$ grep -n -B2 -A4 foldhash ~/.cargo/registry/src/*/serde_json-1.0.154/Cargo.toml
67-    "indexmap",
68-    "alloc",
69:    "dep:foldhash",
70-]
71-raw_value = []
72-std = [
73-    "memchr/std",
--
108-path = "tests/test.rs"
109-
110:[dependencies.foldhash]
111-version = "0.2"
112-optional = true
113-default-features = false
```
