# Probe round 2 results (raw)

Date: 2026-11-21T11:12:59Z. Host: Darwin arm64.

## Toolchain

```
rustc 1.94.1 (e408947bf 2026-03-25)
cargo 1.94.1 (29ea6fb6a 2026-03-24)
```

Note: the toolchain installed here is rustc 1.94.1 (= the project MSRV), not 1.99.

## File::lock / try_lock stability (toolchain library source)

Source: /Users/hiraq/.rustup/toolchains/1.94.1-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/fs.rs

```
144-#[stable(feature = "file_lock", since = "1.89.0")]
145:pub enum TryLockError {
--
863-    #[stable(feature = "file_lock", since = "1.89.0")]
864:    pub fn lock(&self) -> io::Result<()> {
--
915-    #[stable(feature = "file_lock", since = "1.89.0")]
916:    pub fn lock_shared(&self) -> io::Result<()> {
--
979-    #[stable(feature = "file_lock", since = "1.89.0")]
980:    pub fn try_lock(&self) -> Result<(), TryLockError> {
--
1043-    #[stable(feature = "file_lock", since = "1.89.0")]
1044:    pub fn try_lock_shared(&self) -> Result<(), TryLockError> {
--
1080-    #[stable(feature = "file_lock", since = "1.89.0")]
1081:    pub fn unlock(&self) -> io::Result<()> {
```

## Crate: airsl

### Cargo.toml

```toml
[package]
name = "probe-r2-airsl"
version = "0.1.0"
edition = "2024"

[workspace]

[dependencies]
airsl = "=0.1.4"
serde_json = "1"
```

### Cargo.lock (relevant entries)

```toml
[[package]]
name = "airsl"
version = "0.1.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a47f225b4b5a151c26115ce4c9da03157f2220a164fa46a9f696246354af9c2b"
dependencies = [
 "globset",
 "jiff",
 "mlua",
 "regex",
 "serde",
 "serde_json",
 "sha1",
 "sha2",
 "tempfile",
 "thiserror",
 "toml",
 "walkdir",
]

[[package]]
name = "mlua"
version = "0.12.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7a7ae6b65bf54b7fdd4cf5b6f44f60862350ebf3fe3779b3a9cc005e7fa14f81"
dependencies = [
 "bstr",
 "either",
 "erased-serde",
 "libc",
 "mlua-sys",
 "num-traits",
 "parking_lot",
 "rustc-hash",
 "serde",
 "serde-value",
]

[[package]]
name = "mlua-sys"
version = "0.13.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1631cda17bd6486410ab751d91f5fd4484f441589278cdad879eb3afc721a285"
dependencies = [
 "cc",
 "cfg-if",
 "libc",
 "lua-src",
 "luajit-src",
 "pkg-config",
]

[[package]]
name = "serde_json"
version = "1.0.154"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e7e9cc8b1b85264074fbcc02a88680c4096b1e47df8f739dceb03bf482f04bd6"
dependencies = [
 "itoa",
 "memchr",
 "serde",
 "serde_core",
 "zmij",
]

```

### src/main.rs

```rust
//! Throwaway probe against airsl =0.1.4 (mlua via `airsl::mlua`). Prints `Rn: <result>`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use airsl::mlua;
use airsl::modules::{Ext, Json};
use airsl::{Engine, EventName, HostModule, InstallContext, ModuleName, ModuleSet, Policy, Script};

type Handlers = Arc<Mutex<HashMap<String, mlua::RegistryKey>>>;

#[derive(Clone, Default)]
struct Shared {
    lines: Arc<Mutex<Vec<String>>>,
}

struct Agentstacks {
    name: ModuleName,
    shared: Shared,
}

fn describe(v: &mlua::Value) -> String {
    match v {
        mlua::Value::Nil => "Value::Nil".to_owned(),
        mlua::Value::Boolean(b) => format!("Value::Boolean({b})"),
        mlua::Value::Integer(i) => format!("Value::Integer({i})"),
        mlua::Value::Number(n) => format!("Value::Number({n:?})"),
        mlua::Value::String(s) => format!("Value::String({:?})", s.as_bytes().to_vec()),
        mlua::Value::Table(_) => "Value::Table(..)".to_owned(),
        other => format!("other type_name={}", other.type_name()),
    }
}

fn stack_dump(lua: &mlua::Lua) -> String {
    let mut out = Vec::new();
    for level in 0..6 {
        let Some(s) = lua.inspect_stack(level, |d| {
            let src = d.source();
            format!(
                "[level {level}: source={:?} short_src={:?} what={:?} current_line={:?} name={:?}]",
                src.source,
                src.short_src,
                src.what,
                d.current_line(),
                d.names().name
            )
        }) else {
            break;
        };
        out.push(s);
    }
    out.join(" ")
}

impl HostModule for Agentstacks {
    fn name(&self) -> &ModuleName {
        &self.name
    }

    fn install(
        &self,
        lua: &mlua::Lua,
        table: &mlua::Table,
        _cx: &InstallContext<'_>,
    ) -> airsl::Result<()> {
        let fail = |e: mlua::Error| airsl::Error::lua("agentstacks", e);
        let handlers: Handlers = Arc::default();

        let h = Arc::clone(&handlers);
        let on = lua
            .create_function(move |lua, (name, f): (String, mlua::Function)| {
                let key = lua.create_registry_value(f)?;
                h.lock().expect("lock").insert(name, key);
                Ok(())
            })
            .map_err(fail)?;
        table.set("on", on).map_err(fail)?;

        let h = Arc::clone(&handlers);
        let call = lua
            .create_function(move |lua, (name, arg): (String, mlua::Value)| {
                let f: mlua::Function = {
                    let map = h.lock().expect("lock");
                    let Some(key) = map.get(&name) else {
                        return Err(mlua::Error::external(format!("no handler `{name}`")));
                    };
                    lua.registry_value(key)?
                };
                f.call::<mlua::Value>(arg)
            })
            .map_err(fail)?;
        table.set("call", call).map_err(fail)?;

        // R3: strict inspection of named fields + all keys via pairs().
        let lines = Arc::clone(&self.shared.lines);
        let inspect = lua
            .create_function(move |_, (label, t): (String, mlua::Table)| {
                let path: mlua::Value = t.raw_get("path")?;
                let mut keys = Vec::new();
                for pair in t.pairs::<mlua::Value, mlua::Value>() {
                    let (k, v) = pair?;
                    keys.push(format!("{} => {}", describe(&k), describe(&v)));
                }
                keys.sort();
                lines.lock().expect("lock").push(format!(
                    "R3 {label}: path field = {} ; pairs = [{}]",
                    describe(&path),
                    keys.join(", ")
                ));
                Ok(())
            })
            .map_err(fail)?;
        table.set("inspect", inspect).map_err(fail)?;

        // R4: raw bytes vs to_str.
        let lines = Arc::clone(&self.shared.lines);
        let bytes = lua
            .create_function(move |_, v: mlua::Value| {
                let mlua::Value::String(s) = v else {
                    return Err(mlua::Error::external("not a string"));
                };
                let raw = s.as_bytes().to_vec();
                let to_str = match s.to_str() {
                    Ok(x) => format!("Ok({:?})", &*x),
                    Err(e) => format!("Err Display=<<{e}>> Debug=<<{e:?}>>"),
                };
                lines
                    .lock()
                    .expect("lock")
                    .push(format!("R4: as_bytes={raw:?} ; to_str={to_str}"));
                Ok(())
            })
            .map_err(fail)?;
        table.set("bytes", bytes).map_err(fail)?;

        // R5: where is the caller?
        let lines = Arc::clone(&self.shared.lines);
        let refuse = lua
            .create_function(move |lua, label: String| {
                let caller = lua.inspect_stack(1, |d| {
                    (d.source().short_src.map(|s| s.into_owned()), d.current_line())
                });
                lines.lock().expect("lock").push(format!(
                    "R5 {label}: inspect_stack(1) -> {caller:?}\nR5 {label}: full stack: {}",
                    stack_dump(lua)
                ));
                let at = match caller {
                    Some((Some(src), Some(line))) => format!("{src}:{line}"),
                    other => format!("{other:?}"),
                };
                Err::<(), _>(mlua::Error::external(format!("{at}: op 1: path must be relative")))
            })
            .map_err(fail)?;
        table.set("refuse", refuse).map_err(fail)?;

        Ok(())
    }
}

fn engine(shared: Shared) -> airsl::Result<Engine> {
    let mut set = ModuleSet::new();
    set.insert(Box::new(Json::new()))?;
    set.insert(Box::new(Agentstacks {
        name: ModuleName::new("agentstacks").expect("name"),
        shared,
    }))?;
    Engine::builder().policy(Policy::pure()).stdlib(set).build()
}

fn show<T: std::fmt::Debug>(label: &str, r: airsl::Result<T>) {
    match r {
        Ok(v) => println!("{label}: OK {v:?}"),
        Err(e) => println!(
            "{label}: ERR Display=<<{e}>> exhausted_limit={:?}",
            e.exhausted_limit()
        ),
    }
}

fn drain(shared: &Shared) {
    for l in shared.lines.lock().expect("lock").drain(..) {
        println!("{l}");
    }
}

const ADAPTER: &str = r#"local A = airsstack.agentstacks
A.on("values", function(q) return { name = q.item } end)
A.on("plan", function(p) return { n = #p } end)
A.on("spin", function() while true do end end)
A.on("refuse", function(p)
  local x = 1
  A.refuse("in-handler")
end)
"#;

fn main() {
    // ---------- R1 ----------
    let shared = Shared::default();
    let e = engine(shared.clone()).expect("engine");
    let adapter = Script::from_source(ADAPTER, "adapter.lua").expect("script");
    show("R1 eval1 adapter.lua", e.eval(&adapter));

    let host_name = match Script::from_source("return 1", "<host>") {
        Ok(_) => {
            println!("R1 chunk name \"<host>\" accepted");
            "<host>"
        }
        Err(err) => {
            println!("R1 chunk name \"<host>\" refused: {err}");
            "host"
        }
    };
    let host = |src: &str| Script::from_source(src, host_name).expect("script");

    match e.eval_to::<mlua::Table>(&host(
        "return airsstack.agentstacks.call('values', {item='x'})",
    )) {
        Ok(t) => {
            let name: mlua::Value = t.raw_get("name").expect("get");
            let mut keys = Vec::new();
            for p in t.pairs::<String, mlua::Value>() {
                let (k, v) = p.expect("pair");
                keys.push(format!("{k}={}", describe(&v)));
            }
            println!("R1 eval2 call('values'): OK name={} keys=[{}]", describe(&name), keys.join(","));
        }
        Err(err) => println!("R1 eval2 call('values'): ERR {err}"),
    }
    show(
        "R1 eval3 call('plan', {1,2,3}).n",
        e.eval_to::<i64>(&host("return airsstack.agentstacks.call('plan', {1,2,3}).n")),
    );

    // ---------- R2 ----------
    show(
        "R2 eval4 call('spin') (handler runs while true do end)",
        e.eval(&host("airsstack.agentstacks.call('spin', {})")),
    );
    show(
        "R2 eval5 after spin: call('values').name",
        e.eval_to::<String>(&host("return airsstack.agentstacks.call('values', {item='y'}).name")),
    );
    // budget reset per eval: each eval burns ~60% of a loop that would exhaust in a single eval
    show(
        "R2 eval6 busy loop 3e6 iterations in one eval",
        e.eval_to::<i64>(&host("local n=0 for i=1,3000000 do n=n+1 end return n")),
    );
    show(
        "R2 eval7 busy loop 1e6 iterations",
        e.eval_to::<i64>(&host("local n=0 for i=1,1000000 do n=n+1 end return n")),
    );
    show(
        "R2 eval8 busy loop 1e6 iterations again",
        e.eval_to::<i64>(&host("local n=0 for i=1,1000000 do n=n+1 end return n")),
    );

    // ---------- R3 ----------
    let r3 = host(
        "local A = airsstack.agentstacks\n\
         A.inspect('path=5', {path = 5})\n\
         A.inspect('path=5.0', {path = 5.0})\n\
         A.inspect('path=\"5\"', {path = \"5\"})\n\
         A.inspect('path=true', {path = true})\n\
         A.inspect('path={}', {path = {}})\n\
         A.inspect('path missing, pth typo', {pth = \"a/b.md\"})\n\
         A.inspect('path=nil explicit', {path = nil, kind = \"write\"})\n\
         A.inspect('array part', {\"x\", path = 2^53})",
    );
    show("R3 eval", e.eval(&r3));
    drain(&shared);

    // ---------- R4 ----------
    show(
        "R4 eval",
        e.eval(&host("airsstack.agentstacks.bytes('a\\xffb\\x00c')")),
    );
    drain(&shared);

    // ---------- R5 ----------
    let r5 = Script::from_source(
        "-- line 1\nlocal x = 1\nairsstack.agentstacks.refuse('direct')\n",
        "adapter.lua",
    )
    .expect("script");
    show("R5 direct eval Display", e.eval(&r5));
    drain(&shared);
    show(
        "R5 via handler eval Display",
        e.eval(&host("airsstack.agentstacks.call('refuse', {})")),
    );
    drain(&shared);

    // ---------- R1b / R2b: airsl's own Ext + Engine::dispatch ----------
    let mut set = ModuleSet::new();
    set.insert(Box::new(Json::new())).expect("json");
    set.insert(Box::new(Ext::with_events([
        EventName::new("values").expect("ev"),
        EventName::new("spin").expect("ev"),
    ])))
    .expect("ext");
    let e2 = Engine::builder()
        .policy(Policy::pure())
        .stdlib(set)
        .build()
        .expect("engine");
    show(
        "R1b eval adapter using airsstack.ext.on",
        e2.eval(
            &Script::from_source(
                "airsstack.ext.on('values', function(q) return {name=q.item} end)\n\
                 airsstack.ext.on('spin', function() while true do end end)",
                "adapter.lua",
            )
            .expect("script"),
        ),
    );
    show(
        "R1b dispatch values {item:x}",
        e2.dispatch(
            &EventName::new("values").expect("ev"),
            &serde_json::json!({"item": "x"}),
        ),
    );
    show(
        "R1b dispatch unregistered-but-valid name `plan`",
        e2.dispatch(&EventName::new("plan").expect("ev"), &serde_json::json!({})),
    );
    show(
        "R2b dispatch spin",
        e2.dispatch(&EventName::new("spin").expect("ev"), &serde_json::json!({})),
    );
    show(
        "R1b ext.on undeclared event",
        e2.eval(
            &Script::from_source("airsstack.ext.on('nope', function() end)", "adapter.lua")
                .expect("script"),
        ),
    );
}
```

### Raw output (byte for byte)

```
R1 eval1 adapter.lua: OK ()
R1 chunk name "<host>" accepted
R1 eval2 call('values'): OK name=Value::String([120]) keys=[name=Value::String([120])]
R1 eval3 call('plan', {1,2,3}).n: OK 3
R2 eval4 call('spin') (handler runs while true do end): ERR Display=<<script `<host>` exceeded its instruction limit of 10000000>> exhausted_limit=Some(Instructions)
R2 eval5 after spin: call('values').name: OK "y"
R2 eval6 busy loop 3e6 iterations in one eval: OK 3000000
R2 eval7 busy loop 1e6 iterations: OK 1000000
R2 eval8 busy loop 1e6 iterations again: OK 1000000
R3 eval: OK ()
R3 path=5: path field = Value::Integer(5) ; pairs = [Value::String([112, 97, 116, 104]) => Value::Integer(5)]
R3 path=5.0: path field = Value::Number(5.0) ; pairs = [Value::String([112, 97, 116, 104]) => Value::Number(5.0)]
R3 path="5": path field = Value::String([53]) ; pairs = [Value::String([112, 97, 116, 104]) => Value::String([53])]
R3 path=true: path field = Value::Boolean(true) ; pairs = [Value::String([112, 97, 116, 104]) => Value::Boolean(true)]
R3 path={}: path field = Value::Table(..) ; pairs = [Value::String([112, 97, 116, 104]) => Value::Table(..)]
R3 path missing, pth typo: path field = Value::Nil ; pairs = [Value::String([112, 116, 104]) => Value::String([97, 47, 98, 46, 109, 100])]
R3 path=nil explicit: path field = Value::Nil ; pairs = [Value::String([107, 105, 110, 100]) => Value::String([119, 114, 105, 116, 101])]
R3 array part: path field = Value::Number(9007199254740992.0) ; pairs = [Value::Integer(1) => Value::String([120]), Value::String([112, 97, 116, 104]) => Value::Number(9007199254740992.0)]
R4 eval: OK ()
R4: as_bytes=[97, 255, 98, 0, 99] ; to_str=Err Display=<<error converting Lua string to &str (invalid utf-8 sequence of 1 bytes from index 1)>> Debug=<<FromLuaConversionError { from: "string", to: "&str", message: Some("invalid utf-8 sequence of 1 bytes from index 1") }>>
R5 direct eval Display: ERR Display=<<lua error in adapter.lua: adapter.lua:3: op 1: path must be relative
stack traceback:
	[C]: in field 'refuse'
	adapter.lua:3: in main chunk
>> exhausted_limit=None
R5 direct: inspect_stack(1) -> Some((Some("adapter.lua"), Some(3)))
R5 direct: full stack: [level 0: source=Some("=[C]") short_src=Some("[C]") what="C" current_line=None name=Some("refuse")] [level 1: source=Some("@adapter.lua") short_src=Some("adapter.lua") what="main" current_line=Some(3) name=None]
R5 via handler eval Display: ERR Display=<<lua error in <host>: adapter.lua:7: op 1: path must be relative
stack traceback:
	[C]: in field 'refuse'
	adapter.lua:7: in function <adapter.lua:5>
	>[C]: in field 'call'
	<host>:1: in main chunk
>> exhausted_limit=None
R5 in-handler: inspect_stack(1) -> Some((Some("adapter.lua"), Some(7)))
R5 in-handler: full stack: [level 0: source=Some("=[C]") short_src=Some("[C]") what="C" current_line=None name=Some("refuse")] [level 1: source=Some("@adapter.lua") short_src=Some("adapter.lua") what="Lua" current_line=Some(7) name=None] [level 2: source=Some("=[C]") short_src=Some("[C]") what="C" current_line=None name=Some("call")] [level 3: source=Some("@<host>") short_src=Some("<host>") what="main" current_line=Some(1) name=None]
R1b eval adapter using airsstack.ext.on: OK ()
R1b dispatch values {item:x}: OK Some(Object {"name": String("x")})
R1b dispatch unregistered-but-valid name `plan`: OK None
R2b dispatch spin: ERR Display=<<script `spin` exceeded its instruction limit of 10000000>> exhausted_limit=Some(Instructions)
R1b ext.on undeclared event: ERR Display=<<lua error in adapter.lua: event `nope` is not one this host dispatches; declared events: spin, values
stack traceback:
	[C]: in field 'on'
	adapter.lua:1: in main chunk
>> exhausted_limit=None
exit=0
```

## Crate: json

### Cargo.toml

```toml
[package]
name = "probe-r2-json"
version = "0.1.0"
edition = "2024"

[workspace]

[dependencies]
serde = "1"
serde_json = { version = "=1.0.154", features = ["arbitrary_precision", "preserve_order"] }
```

### Cargo.lock (relevant entries)

```toml
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
name = "indexmap"
version = "2.14.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cc4e190f5d26ca7051642629da2c52fc03bde85a03197c99408dcd291734c855"
dependencies = [
 "equivalent",
 "hashbrown",
]

```

### src/main.rs

```rust
//! Throwaway probe against serde_json =1.0.154 (arbitrary_precision + preserve_order).
//! Prints `Rn: <result>`.

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

/// The private key serde_json uses to smuggle an arbitrary-precision number through `visit_map`.
const NUMBER_TOKEN: &str = "$serde_json::private::Number";

#[derive(Debug)]
#[allow(dead_code)]
enum Node {
    Null,
    Bool(bool),
    Num(serde_json::Number),
    Str(String),
    Seq(Vec<Node>),
    Map(Vec<(String, Node)>),
}

#[derive(Clone)]
struct NodeSeed {
    path: Rc<RefCell<Vec<String>>>,
}

impl NodeSeed {
    fn path_string(&self) -> String {
        let p = self.path.borrow();
        if p.is_empty() {
            "$".to_owned()
        } else {
            format!("$.{}", p.join("."))
        }
    }
}

impl<'de> DeserializeSeed<'de> for NodeSeed {
    type Value = Node;
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Node, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for NodeSeed {
    type Value = Node;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any JSON value")
    }
    fn visit_unit<E>(self) -> Result<Node, E> {
        Ok(Node::Null)
    }
    fn visit_bool<E>(self, b: bool) -> Result<Node, E> {
        Ok(Node::Bool(b))
    }
    fn visit_u64<E: de::Error>(self, n: u64) -> Result<Node, E> {
        println!("R7 trace: visit_u64({n}) called");
        Ok(Node::Num(n.into()))
    }
    fn visit_i64<E: de::Error>(self, n: i64) -> Result<Node, E> {
        println!("R7 trace: visit_i64({n}) called");
        Ok(Node::Num(n.into()))
    }
    fn visit_f64<E: de::Error>(self, n: f64) -> Result<Node, E> {
        println!("R7 trace: visit_f64({n}) called");
        serde_json::Number::from_f64(n)
            .map(Node::Num)
            .ok_or_else(|| de::Error::custom("non-finite float"))
    }
    fn visit_str<E: de::Error>(self, s: &str) -> Result<Node, E> {
        Ok(Node::Str(s.to_owned()))
    }
    fn visit_string<E: de::Error>(self, s: String) -> Result<Node, E> {
        Ok(Node::Str(s))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
        let mut out = Vec::new();
        loop {
            self.path.borrow_mut().push(out.len().to_string());
            let next = seq.next_element_seed(self.clone());
            self.path.borrow_mut().pop();
            match next? {
                Some(n) => out.push(n),
                None => break,
            }
        }
        Ok(Node::Seq(out))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
        let mut out: Vec<(String, Node)> = Vec::new();
        let mut seen = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if key == NUMBER_TOKEN && out.is_empty() {
                let raw: String = map.next_value()?;
                println!("R7 trace: number token map, raw={raw}");
                let n: serde_json::Number = raw.parse().map_err(de::Error::custom)?;
                return Ok(Node::Num(n));
            }
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate key `{key}` in object at {}",
                    self.path_string()
                )));
            }
            self.path.borrow_mut().push(key.clone());
            let v = map.next_value_seed(self.clone());
            self.path.borrow_mut().pop();
            out.push((key, v?));
        }
        Ok(Node::Map(out))
    }
}

fn strict_parse(s: &str) -> Result<Node, serde_json::Error> {
    let mut de = serde_json::Deserializer::from_str(s);
    let node = NodeSeed {
        path: Rc::default(),
    }
    .deserialize(&mut de)?;
    de.end()?;
    Ok(node)
}

fn main() {
    // ---------- R6 ----------
    for input in [
        "18446744073709551616",
        "-9223372036854775809",
        "1.50",
        "1e3",
        "1E400",
        "0.1",
        "-0",
        "-0.0",
        "100000000000000000000000000000.000000000000000000001",
        r#"{"a":18446744073709551616,"b":1.50,"c":1E400,"d":[0.1,1e3]}"#,
    ] {
        match serde_json::from_str::<Value>(input) {
            Ok(v) => {
                let out = serde_json::to_string(&v).expect("to_string");
                println!(
                    "R6: in={input} out={out} identical={} as_u64={:?} as_i64={:?} as_f64={:?}",
                    input == out,
                    v.as_u64(),
                    v.as_i64(),
                    v.as_f64()
                );
            }
            Err(e) => println!("R6: in={input} ERR {e}"),
        }
    }

    // ---------- R7 ----------
    for input in [
        r#"{"a":1,"a":2}"#,
        r#"{"o":{"x":1,"x":2}}"#,
        "{\n  \"o\": {\n    \"x\": 1,\n    \"x\": 2\n  }\n}",
        r#"{"l":[{"k":1},{"k":2,"k":3}]}"#,
        r#"{"n":18446744073709551616,"m":1.50,"i":-5,"e":1E400}"#,
        r#"{"a":1,"a":2,"a":3}"#,
        r#"[1,{"z":{},"z":[]}]"#,
        r#"{"a":1,"b":2}"#,
    ] {
        let shown = input.replace('\n', "\\n");
        let plain = serde_json::from_str::<Value>(input)
            .map(|v| v.to_string())
            .unwrap_or_else(|e| format!("ERR {e}"));
        match strict_parse(input) {
            Ok(n) => println!("R7: in={shown} strict=OK {n:?} | plain Value={plain}"),
            Err(e) => println!(
                "R7: in={shown} strict=ERR Display=<<{e}>> line={} column={} classify={:?} | plain Value={plain}",
                e.line(),
                e.column(),
                e.classify()
            ),
        }
    }
}
```

### Raw output (byte for byte)

Run 1 (visitor lacked visit_u64/i64/f64 -> every small integer refused; kept as evidence that
with arbitrary_precision small integers arrive via visit_u64/visit_i64, not the number-token map):

```
R6: in=18446744073709551616 out=18446744073709551616 identical=true as_u64=None as_i64=None as_f64=Some(1.8446744073709552e19)
R6: in=-9223372036854775809 out=-9223372036854775809 identical=true as_u64=None as_i64=None as_f64=Some(-9.223372036854776e18)
R6: in=1.50 out=1.50 identical=true as_u64=None as_i64=None as_f64=Some(1.5)
R6: in=1e3 out=1e+3 identical=false as_u64=None as_i64=None as_f64=Some(1000.0)
R6: in=1E400 out=1e+400 identical=false as_u64=None as_i64=None as_f64=None
R6: in=0.1 out=0.1 identical=true as_u64=None as_i64=None as_f64=Some(0.1)
R6: in=-0 out=-0 identical=true as_u64=None as_i64=Some(0) as_f64=Some(-0.0)
R6: in=-0.0 out=-0.0 identical=true as_u64=None as_i64=None as_f64=Some(-0.0)
R6: in=100000000000000000000000000000.000000000000000000001 out=100000000000000000000000000000.000000000000000000001 identical=true as_u64=None as_i64=None as_f64=Some(1e29)
R6: in={"a":18446744073709551616,"b":1.50,"c":1E400,"d":[0.1,1e3]} out={"a":18446744073709551616,"b":1.50,"c":1e+400,"d":[0.1,1e+3]} identical=false as_u64=None as_i64=None as_f64=None
R7: in={"a":1,"a":2} strict=ERR Display=<<invalid type: integer `1`, expected any JSON value at line 1 column 6>> line=1 column=6 classify=Data | plain Value={"a":2}
R7: in={"o":{"x":1,"x":2}} strict=ERR Display=<<invalid type: integer `1`, expected any JSON value at line 1 column 11>> line=1 column=11 classify=Data | plain Value={"o":{"x":2}}
R7: in={\n  "o": {\n    "x": 1,\n    "x": 2\n  }\n} strict=ERR Display=<<invalid type: integer `1`, expected any JSON value at line 3 column 10>> line=3 column=10 classify=Data | plain Value={"o":{"x":2}}
R7: in={"l":[{"k":1},{"k":2,"k":3}]} strict=ERR Display=<<invalid type: integer `1`, expected any JSON value at line 1 column 12>> line=1 column=12 classify=Data | plain Value={"l":[{"k":1},{"k":3}]}
R7: in={"n":18446744073709551616,"m":1.50} strict=OK Map([("n", Num(Number(18446744073709551616))), ("m", Num(Number(1.50)))]) | plain Value={"n":18446744073709551616,"m":1.50}
R7: in={"a":1,"b":2} strict=ERR Display=<<invalid type: integer `1`, expected any JSON value at line 1 column 6>> line=1 column=6 classify=Data | plain Value={"a":1,"b":2}
exit=0
```

Run 2 (after adding visit_u64/visit_i64/visit_f64 and trace lines; source above is run 2):

```
R6: in=18446744073709551616 out=18446744073709551616 identical=true as_u64=None as_i64=None as_f64=Some(1.8446744073709552e19)
R6: in=-9223372036854775809 out=-9223372036854775809 identical=true as_u64=None as_i64=None as_f64=Some(-9.223372036854776e18)
R6: in=1.50 out=1.50 identical=true as_u64=None as_i64=None as_f64=Some(1.5)
R6: in=1e3 out=1e+3 identical=false as_u64=None as_i64=None as_f64=Some(1000.0)
R6: in=1E400 out=1e+400 identical=false as_u64=None as_i64=None as_f64=None
R6: in=0.1 out=0.1 identical=true as_u64=None as_i64=None as_f64=Some(0.1)
R6: in=-0 out=-0 identical=true as_u64=None as_i64=Some(0) as_f64=Some(-0.0)
R6: in=-0.0 out=-0.0 identical=true as_u64=None as_i64=None as_f64=Some(-0.0)
R6: in=100000000000000000000000000000.000000000000000000001 out=100000000000000000000000000000.000000000000000000001 identical=true as_u64=None as_i64=None as_f64=Some(1e29)
R6: in={"a":18446744073709551616,"b":1.50,"c":1E400,"d":[0.1,1e3]} out={"a":18446744073709551616,"b":1.50,"c":1e+400,"d":[0.1,1e+3]} identical=false as_u64=None as_i64=None as_f64=None
R7 trace: visit_u64(1) called
R7: in={"a":1,"a":2} strict=ERR Display=<<duplicate key `a` in object at $ at line 1 column 10>> line=1 column=10 classify=Data | plain Value={"a":2}
R7 trace: visit_u64(1) called
R7: in={"o":{"x":1,"x":2}} strict=ERR Display=<<duplicate key `x` in object at $.o at line 1 column 15>> line=1 column=15 classify=Data | plain Value={"o":{"x":2}}
R7 trace: visit_u64(1) called
R7: in={\n  "o": {\n    "x": 1,\n    "x": 2\n  }\n} strict=ERR Display=<<duplicate key `x` in object at $.o at line 4 column 7>> line=4 column=7 classify=Data | plain Value={"o":{"x":2}}
R7 trace: visit_u64(1) called
R7 trace: visit_u64(2) called
R7: in={"l":[{"k":1},{"k":2,"k":3}]} strict=ERR Display=<<duplicate key `k` in object at $.l.1 at line 1 column 24>> line=1 column=24 classify=Data | plain Value={"l":[{"k":1},{"k":3}]}
R7 trace: number token map, raw=18446744073709551616
R7 trace: number token map, raw=1.50
R7 trace: visit_i64(-5) called
R7 trace: number token map, raw=1e+400
R7: in={"n":18446744073709551616,"m":1.50,"i":-5,"e":1E400} strict=OK Map([("n", Num(Number(18446744073709551616))), ("m", Num(Number(1.50))), ("i", Num(Number(-5))), ("e", Num(Number(1e+400)))]) | plain Value={"n":18446744073709551616,"m":1.50,"i":-5,"e":1e+400}
R7 trace: visit_u64(1) called
R7: in={"a":1,"a":2,"a":3} strict=ERR Display=<<duplicate key `a` in object at $ at line 1 column 10>> line=1 column=10 classify=Data | plain Value={"a":3}
R7 trace: visit_u64(1) called
R7: in=[1,{"z":{},"z":[]}] strict=ERR Display=<<duplicate key `z` in object at $.1 at line 1 column 14>> line=1 column=14 classify=Data | plain Value=[1,{"z":[]}]
R7 trace: visit_u64(1) called
R7 trace: visit_u64(2) called
R7: in={"a":1,"b":2} strict=OK Map([("a", Num(Number(1))), ("b", Num(Number(2)))]) | plain Value={"a":1,"b":2}
exit=0
```

## Crate: std

### Cargo.toml

```toml
[package]
name = "probe-r2-std"
version = "0.1.0"
edition = "2024"
rust-version = "1.94"

[workspace]

[dependencies]
```

### Cargo.lock (relevant entries)

```toml
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "probe-r2-std"
version = "0.1.0"
```

### src/main.rs

```rust
//! Throwaway probe of std::fs::File::lock / try_lock and std::fs::canonicalize.
//! Prints `Rn: <result>`. Modes: (none) = parent; `hold <file>`; `wait <file>`.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis()
}

fn open(path: &str) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .expect("open lock file")
}

fn child(mode: &str, path: &str) {
    let f = open(path);
    match mode {
        "hold" => {
            f.lock().expect("lock");
            println!("hold: acquired at {}", now_ms());
            std::io::stdout().flush().expect("flush");
            std::thread::sleep(Duration::from_secs(2));
            println!("hold: releasing (drop) at {}", now_ms());
            drop(f);
        }
        "wait" => {
            let t0 = now_ms();
            println!("wait: calling lock() at {t0}");
            f.lock().expect("lock");
            let t1 = now_ms();
            println!("wait: acquired at {t1} (blocked {} ms)", t1 - t0);
        }
        _ => unreachable!(),
    }
}

fn r8(dir: &Path) {
    let lock_path = dir.join("agentstacks.lock.lck");
    let lp = lock_path.to_str().expect("utf8").to_owned();
    let exe = std::env::current_exe().expect("exe");

    let mut hold = Command::new(&exe)
        .args(["hold", &lp])
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn hold");
    let mut hold_out = BufReader::new(hold.stdout.take().expect("stdout"));
    let mut first = String::new();
    hold_out.read_line(&mut first).expect("read");
    println!("R8: [child hold] {}", first.trim_end());

    // try_lock from the parent (a separate process) while held.
    let f = open(&lp);
    match f.try_lock() {
        Ok(()) => println!("R8: parent try_lock while held: Ok(()) (UNEXPECTED)"),
        Err(e) => {
            let variant = match &e {
                TryLockError::WouldBlock => "TryLockError::WouldBlock".to_owned(),
                TryLockError::Error(io) => format!("TryLockError::Error(kind={:?})", io.kind()),
            };
            println!(
                "R8: parent try_lock while held at {}: Err variant={variant} Display=<<{e}>> Debug=<<{e:?}>>",
                now_ms()
            );
            let io: std::io::Error = e.into();
            println!(
                "R8: TryLockError -> io::Error: kind={:?} Display=<<{io}>>",
                io.kind()
            );
        }
    }
    // try_lock_shared too, for completeness.
    match f.try_lock_shared() {
        Ok(()) => println!("R8: parent try_lock_shared while held: Ok(()) (UNEXPECTED)"),
        Err(e) => println!("R8: parent try_lock_shared while held: Err Display=<<{e}>>"),
    }

    let wait = Command::new(&exe)
        .args(["wait", &lp])
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn wait");
    let wait_out = wait.wait_with_output().expect("wait child");
    let hold_rest: Vec<String> = hold_out.lines().map(|l| l.expect("line")).collect();
    hold.wait().expect("hold exit");
    for l in hold_rest {
        println!("R8: [child hold] {l}");
    }
    for l in String::from_utf8_lossy(&wait_out.stdout).lines() {
        println!("R8: [child wait] {l}");
    }
    match f.try_lock() {
        Ok(()) => println!("R8: parent try_lock after both children exited: Ok(())"),
        Err(e) => println!("R8: parent try_lock after both children exited: Err {e}"),
    }
}

fn r9(dir: &Path) {
    let proj = dir.join("proj");
    let outside = dir.join("outside").join("real");
    std::fs::create_dir_all(&proj).expect("mkdir proj");
    std::fs::create_dir_all(outside.join("sub")).expect("mkdir outside");
    let link = proj.join("link");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink("../outside/real", &link).expect("symlink");
    let dangling = proj.join("dangling");
    let _ = std::fs::remove_file(&dangling);
    std::os::unix::fs::symlink("../outside/nothing-here", &dangling).expect("symlink");

    std::env::set_current_dir(&proj).expect("cd proj");
    println!(
        "R9: cwd={} (project root); link -> {:?}",
        std::env::current_dir().expect("cwd").display(),
        std::fs::read_link("link").expect("readlink")
    );
    for rel in ["link", "link/sub", "./link/../link/sub", ".", "missing", "missing/x", "dangling"] {
        match std::fs::canonicalize(rel) {
            Ok(p) => println!(
                "R9: canonicalize({rel:?}) = Ok({}) starts_with(project root canonical)={}",
                p.display(),
                p.starts_with(std::fs::canonicalize(".").expect("canon ."))
            ),
            Err(e) => println!(
                "R9: canonicalize({rel:?}) = Err Display=<<{e}>> kind={:?} raw_os_error={:?}",
                e.kind(),
                e.raw_os_error()
            ),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 {
        child(&args[1], &args[2]);
        return;
    }
    let dir = Path::new(&args[1]).to_path_buf();
    std::fs::create_dir_all(&dir).expect("mkdir work");
    r8(&dir);
    r9(&dir);
}
```

### Raw output (byte for byte)

```
R8: [child hold] hold: acquired at 1795259529758
R8: parent try_lock while held at 1795259529759: Err variant=TryLockError::WouldBlock Display=<<lock acquisition failed because the operation would block>> Debug=<<"WouldBlock">>
R8: TryLockError -> io::Error: kind=WouldBlock Display=<<operation would block>>
R8: parent try_lock_shared while held: Err Display=<<lock acquisition failed because the operation would block>>
R8: [child hold] hold: releasing (drop) at 1795259531760
R8: [child wait] wait: calling lock() at 1795259529764
R8: [child wait] wait: acquired at 1795259531760 (blocked 1996 ms)
R8: parent try_lock after both children exited: Ok(())
R9: cwd=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-round2/std-work/proj (project root); link -> "../outside/real"
R9: canonicalize("link") = Ok(/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-round2/std-work/outside/real) starts_with(project root canonical)=false
R9: canonicalize("link/sub") = Ok(/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-round2/std-work/outside/real/sub) starts_with(project root canonical)=false
R9: canonicalize("./link/../link/sub") = Err Display=<<No such file or directory (os error 2)>> kind=NotFound raw_os_error=Some(2)
R9: canonicalize(".") = Ok(/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-round2/std-work/proj) starts_with(project root canonical)=true
R9: canonicalize("missing") = Err Display=<<No such file or directory (os error 2)>> kind=NotFound raw_os_error=Some(2)
R9: canonicalize("missing/x") = Err Display=<<No such file or directory (os error 2)>> kind=NotFound raw_os_error=Some(2)
R9: canonicalize("dangling") = Err Display=<<No such file or directory (os error 2)>> kind=NotFound raw_os_error=Some(2)
exit=0
```

