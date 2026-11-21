# airsl =0.1.4 probe results

Command: `cargo run --offline --manifest-path /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-airsl/Cargo.toml` (exit 0), run 2026-11-21.

## Raw stdout+stderr (byte for byte)

````text
   Compiling probe-airsl v0.1.0 (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-airsl)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.94s
     Running `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-airsl/target/debug/probe-airsl`
Q1: OK "print=function require=nil load=nil dofile=nil os=nil io=nil string=table table=table math=table utf8=table coroutine=nil airsstack=table airsstack.json=table airsstack.stdio=nil airsstack.time=nil airsstack.fs=nil airsstack.agentstacks=table | ALL_GLOBALS=_G,_VERSION,airsstack,arg,assert,collectgarbage,error,getmetatable,ipairs,math,next,pairs,pcall,print,rawequal,rawget,rawlen,rawset,select,setmetatable,string,table,tonumber,tostring,type,utf8,warn,xpcall"
Q1 airsstack keys: OK "agentstacks,json | agentstacks=fail,fail_rt,propose,propose_raw,require"
Q2: OK "nil"
Q3 ok-call: OK ()
Q3 serde records: [Op { kind: "write", path: "a/b.md", content: "x" }]
Q3 raw records: [RawRecord { kind: "write", path: "a/b.md", content: [120] }]
Q3 serde path=5: ERR Display=<<lua error in adapter.lua: deserialize error: invalid type: integer `5`, expected a string
stack traceback:
	[C]: in field 'propose'
	adapter.lua:1: in main chunk
>> exhausted_limit=None has_traceback=true
Q3 serde missing content: ERR Display=<<lua error in adapter.lua: deserialize error: missing field `content`
stack traceback:
	[C]: in field 'propose'
	adapter.lua:1: in main chunk
>> exhausted_limit=None has_traceback=true
Q3 serde unknown key: OK ()
Q3 serde non-table arg: ERR Display=<<lua error in adapter.lua: deserialize error: invalid type: string, expected table
stack traceback:
	[C]: in field 'propose'
	adapter.lua:1: in main chunk
>> exhausted_limit=None has_traceback=true
Q3 raw path=5: OK ()
Q3 raw path=true: ERR Display=<<lua error in adapter.lua: error converting Lua boolean to String (expected string or number)
stack traceback:
	[C]: in field 'propose_raw'
	adapter.lua:1: in main chunk
>> exhausted_limit=None has_traceback=true
Q3 serde records after: [Op { kind: "write", path: "a/b.md", content: "x" }, Op { kind: "write", path: "p", content: "c" }]
Q3 raw records after: [RawRecord { kind: "write", path: "a/b.md", content: [120] }, RawRecord { kind: "write", path: "5", content: [120] }]
Q4 require twice(21): OK 42
Q4 require raising module: ERR Display=<<lua error in adapter.lua: runtime error: lib/util.lua:2: boom
stack traceback:
	[C]: in ?
	[C]: in function 'error'
	lib/util.lua:2: in main chunk
	[C]: in field 'require'
	adapter.lua:1: in main chunk
stack traceback:
	[C]: in field 'require'
	adapter.lua:1: in main chunk
>> exhausted_limit=None has_traceback=true
Q4 error in function from required module: ERR Display=<<lua error in adapter.lua: runtime error: lib/late.lua:2: late
stack traceback:
	[C]: in ?
	[C]: in function 'error'
	lib/late.lua:2: in function <lib/late.lua:1>
	(...tail calls...)>> exhausted_limit=None has_traceback=true
Q4 load still withheld from script: OK "nil"
Q5a syntax: ERR Display=<<lua error in adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>>> exhausted_limit=None has_traceback=false
Q5b runtime error: ERR Display=<<lua error in adapter.lua: runtime error: adapter.lua:1: bad op
stack traceback:
	[C]: in ?
	[C]: in function 'error'
	adapter.lua:1: in main chunk>> exhausted_limit=None has_traceback=true
Q5c infinite loop: ERR Display=<<script `adapter.lua` exceeded its instruction limit of 10000000>> exhausted_limit=Some(Instructions) has_traceback=false
Q10 reuse after Q5c: OK 7
Q5d external from callback: ERR Display=<<lua error in adapter.lua: op 3: path must be relative
stack traceback:
	[C]: in field 'fail'
	adapter.lua:1: in main chunk
>> exhausted_limit=None has_traceback=true
Q5d2 RuntimeError from callback: ERR Display=<<lua error in adapter.lua: runtime error: op 3: path must be relative
stack traceback:
	[C]: in field 'fail_rt'
	adapter.lua:1: in main chunk
>> exhausted_limit=None has_traceback=true
Q6 method: OK "ABC"
Q6 function: OK "ABC"
Q6 getmetatable(''): OK "false"
Q7 lua view: OK "string #text=3 #blob=3 bytes=0,255,65"
Q7 roundtrip[0]: path="core" len=3 bytes=[0, 255, 65]
Q7 roundtrip[1]: path="t" len=3 bytes=[97, 0, 98]
Q7 serde String with 0xff: ERR Display=<<lua error in adapter.lua: deserialize error: invalid value: byte array, expected a string
stack traceback:
	[C]: in field 'propose'
	adapter.lua:1: in main chunk
>> exhausted_limit=None has_traceback=true
Q7 serde String with 0x00 only: OK ()
Q7 serde records: [Op { kind: "write", path: "p", content: "a\0b" }]
Q9a 20MiB string.rep in script: ERR Display=<<script `adapter.lua` exceeded its memory limit of 16777216 bytes>> exhausted_limit=Some(Memory) has_traceback=false
Q9a reuse after: OK 8
Q9b 20MiB set at install (build): ERR Display=<<lua error in agentstacks: memory error: not enough memory>> exhausted_limit=None has_traceback=false
Q9c 20MiB returned from callback: ERR Display=<<script `adapter.lua` exceeded its memory limit of 16777216 bytes>> exhausted_limit=Some(Memory) has_traceback=false
Q9d 8MiB returned from callback: OK 8388608
````

## Q8: source excerpts (airsl-0.1.4/src/modules/registry.rs)

````text
registry.rs:97: pub trait HostModule: Send + Sync {
registry.rs:98:     /// The key this module is installed under in the `airsstack` table.
registry.rs:99:     fn name(&self) -> &ModuleName;
registry.rs:100: 
registry.rs:101:     /// Populates `table` with this module's functions.
registry.rs:102:     ///
registry.rs:103:     /// `context` carries the policy the engine was built with. A module needing no authority — a
registry.rs:104:     /// pure computation like `path` — ignores it; one that guards an operation captures the grants
registry.rs:105:     /// it needs into the functions it creates, and checks them there rather than here.
registry.rs:106:     ///
registry.rs:107:     /// # Errors
registry.rs:108:     ///
registry.rs:109:     /// Returns an error when a function cannot be created or set, which the engine reports as
registry.rs:110:     /// [`Error::ModuleInstall`].
registry.rs:111:     fn install(
registry.rs:112:         &self,
registry.rs:113:         lua: &mlua::Lua,
registry.rs:114:         table: &mlua::Table,
registry.rs:115:         context: &InstallContext<'_>,
registry.rs:116:     ) -> Result<()>;
registry.rs:117: }

registry.rs:139:     /// Adds `module`, keeping insertion order.
registry.rs:140:     ///
registry.rs:141:     /// # Errors
registry.rs:142:     ///
registry.rs:143:     /// Returns [`Error::DuplicateModule`] when a module with the same name is already present.
registry.rs:144:     pub fn insert(&mut self, module: Box<dyn HostModule>) -> Result<()> {
registry.rs:145:         if self.contains(module.name()) {
registry.rs:146:             return Err(Error::DuplicateModule {
registry.rs:147:                 module: module.name().to_string(),
registry.rs:148:             });
registry.rs:149:         }
registry.rs:150:         self.modules.push(module);
registry.rs:151:         Ok(())
````

## Resolved versions (Cargo.lock)

````text
name = "airsl"
version = "0.1.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a47f225b4b5a151c26115ce4c9da03157f2220a164fa46a9f696246354af9c2b"
--
name = "mlua"
version = "0.12.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7a7ae6b65bf54b7fdd4cf5b6f44f60862350ebf3fe3779b3a9cc005e7fa14f81"
--
name = "mlua-sys"
version = "0.13.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1631cda17bd6486410ab751d91f5fd4484f441589278cdad879eb3afc721a285"
````

## Cargo.toml

````toml
[package]
name = "probe-airsl"
version = "0.1.0"
edition = "2024"

[workspace]

[dependencies]
airsl = "=0.1.4"
serde = { version = "1", features = ["derive"] }
````

## src/main.rs (exact probe source)

````rust
//! Throwaway probe against airsl =0.1.4. Each probe prints `Qn: <result>`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use airsl::mlua::{self, LuaSerdeExt as _};
use airsl::modules::Json;
use airsl::{Engine, HostModule, InstallContext, ModuleName, ModuleSet, Policy, Script};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Op {
    kind: String,
    path: String,
    content: String,
}

#[derive(Debug)]
#[allow(dead_code)]
struct RawRecord {
    kind: String,
    path: String,
    content: Vec<u8>,
}

/// Serializes through `serialize_bytes`, which mlua's serializer turns into a Lua string.
struct Bytes(Vec<u8>);
impl Serialize for Bytes {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}

#[derive(Serialize)]
struct Pack {
    name: String,
    text: String,
    blob: Bytes,
}

#[derive(Default, Clone)]
struct Shared {
    serde_ops: Arc<Mutex<Vec<Op>>>,
    raw_ops: Arc<Mutex<Vec<RawRecord>>>,
}

struct Agentstacks {
    name: ModuleName,
    shared: Shared,
    sources: Arc<HashMap<String, (String, String)>>,
    remove_print: bool,
    packs: Option<Arc<Vec<Pack>>>,
    big_at_install: Option<usize>,
    big_on_call: Option<usize>,
}

impl Agentstacks {
    fn new(shared: Shared) -> Self {
        let mut sources = HashMap::new();
        sources.insert(
            "lib.util".to_owned(),
            (
                "@lib/util.lua".to_owned(),
                "return { twice = function(x) return x*2 end }".to_owned(),
            ),
        );
        sources.insert(
            "lib.bad".to_owned(),
            (
                "@lib/util.lua".to_owned(),
                "local x = 1\nerror(\"boom\")\nreturn {}".to_owned(),
            ),
        );
        sources.insert(
            "lib.late".to_owned(),
            (
                "@lib/late.lua".to_owned(),
                "return { f = function()\n  error(\"late\")\nend }".to_owned(),
            ),
        );
        Self {
            name: ModuleName::new("agentstacks").expect("valid module name"),
            shared,
            sources: Arc::new(sources),
            remove_print: false,
            packs: None,
            big_at_install: None,
            big_on_call: None,
        }
    }
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

        if self.remove_print {
            lua.globals().set("print", mlua::Value::Nil).map_err(fail)?;
        }

        let ops = Arc::clone(&self.shared.serde_ops);
        let propose = lua
            .create_function(move |lua, v: mlua::Value| {
                let op: Op = lua.from_value(v)?;
                ops.lock().expect("lock").push(op);
                Ok(())
            })
            .map_err(fail)?;
        table.set("propose", propose).map_err(fail)?;

        let raw = Arc::clone(&self.shared.raw_ops);
        let propose_raw = lua
            .create_function(move |_, t: mlua::Table| {
                let rec = RawRecord {
                    kind: t.get::<String>("kind")?,
                    path: t.get::<String>("path")?,
                    content: t.get::<mlua::LuaString>("content")?.as_bytes().to_vec(),
                };
                raw.lock().expect("lock").push(rec);
                Ok(())
            })
            .map_err(fail)?;
        table.set("propose_raw", propose_raw).map_err(fail)?;

        let sources = Arc::clone(&self.sources);
        let require = lua
            .create_function(move |lua, name: String| {
                let Some((chunk, src)) = sources.get(&name) else {
                    return Err(mlua::Error::external(format!("module `{name}` not found")));
                };
                lua.load(src.as_str())
                    .set_name(chunk.clone())
                    .call::<mlua::Value>(())
            })
            .map_err(fail)?;
        table.set("require", require).map_err(fail)?;

        let fail_ext = lua
            .create_function(|_, ()| {
                Err::<(), _>(mlua::Error::external("op 3: path must be relative"))
            })
            .map_err(fail)?;
        table.set("fail", fail_ext).map_err(fail)?;

        let fail_rt = lua
            .create_function(|_, ()| {
                Err::<(), _>(mlua::Error::RuntimeError(
                    "op 3: path must be relative".to_owned(),
                ))
            })
            .map_err(fail)?;
        table.set("fail_rt", fail_rt).map_err(fail)?;

        if let Some(packs) = &self.packs {
            let v = lua.to_value(&**packs).map_err(fail)?;
            table.set("packs", v).map_err(fail)?;
        }

        if let Some(n) = self.big_at_install {
            let s = lua.create_string(vec![b'x'; n]).map_err(fail)?;
            table.set("big", s).map_err(fail)?;
        }

        if let Some(n) = self.big_on_call {
            let getbig = lua
                .create_function(move |lua, ()| lua.create_string(vec![b'x'; n]))
                .map_err(fail)?;
            table.set("getbig", getbig).map_err(fail)?;
        }

        Ok(())
    }
}

fn engine(module: Agentstacks) -> airsl::Result<Engine> {
    let mut set = ModuleSet::new();
    set.insert(Box::new(Json::new()))?;
    set.insert(Box::new(module))?;
    Engine::builder().policy(Policy::pure()).stdlib(set).build()
}

fn script(src: &str) -> Script {
    Script::from_source(src, "adapter.lua").expect("script")
}

fn show_err(label: &str, err: &airsl::Error) {
    let text = err.to_string();
    println!(
        "{label}: ERR Display=<<{text}>> exhausted_limit={:?} has_traceback={}",
        err.exhausted_limit(),
        text.contains("stack traceback")
    );
}

fn show<T: std::fmt::Debug>(label: &str, r: airsl::Result<T>) {
    match r {
        Ok(v) => println!("{label}: OK {v:?}"),
        Err(e) => show_err(label, &e),
    }
}

const Q1_SRC: &str = r#"
local function t(f)
  local ok, v = pcall(f)
  if ok then return type(v) else return "ERR:" .. tostring(v) end
end
local out = {
  "print=" .. type(print), "require=" .. type(require), "load=" .. type(load),
  "dofile=" .. type(dofile), "os=" .. type(os), "io=" .. type(io),
  "string=" .. type(string), "table=" .. type(table), "math=" .. type(math),
  "utf8=" .. type(utf8), "coroutine=" .. type(coroutine), "airsstack=" .. type(airsstack),
  "airsstack.json=" .. t(function() return airsstack.json end),
  "airsstack.stdio=" .. t(function() return airsstack.stdio end),
  "airsstack.time=" .. t(function() return airsstack.time end),
  "airsstack.fs=" .. t(function() return airsstack.fs end),
  "airsstack.agentstacks=" .. t(function() return airsstack.agentstacks end),
}
local g = {}
for k in pairs(_G) do g[#g + 1] = tostring(k) end
table.sort(g)
return table.concat(out, " ") .. " | ALL_GLOBALS=" .. table.concat(g, ",")
"#;

fn main() {
    // Q1
    let e1 = engine(Agentstacks::new(Shared::default())).expect("build q1");
    show("Q1", e1.eval_to::<String>(&script(Q1_SRC)));
    show(
        "Q1 airsstack keys",
        e1.eval_to::<String>(&script(
            "local k = {} for n in pairs(airsstack) do k[#k+1]=n end table.sort(k) \
             local a = {} for n in pairs(airsstack.agentstacks) do a[#a+1]=n end table.sort(a) \
             return table.concat(k, ',') .. ' | agentstacks=' .. table.concat(a, ',')",
        )),
    );

    // Q2
    let mut m2 = Agentstacks::new(Shared::default());
    m2.remove_print = true;
    let e2 = engine(m2).expect("build q2");
    show("Q2", e2.eval_to::<String>(&script("return type(print)")));

    // Q3
    let shared3 = Shared::default();
    let e3 = engine(Agentstacks::new(shared3.clone())).expect("build q3");
    show(
        "Q3 ok-call",
        e3.eval(&script(
            "airsstack.agentstacks.propose({kind=\"write\", path=\"a/b.md\", content=\"x\"})\n\
             airsstack.agentstacks.propose_raw({kind=\"write\", path=\"a/b.md\", content=\"x\"})",
        )),
    );
    println!("Q3 serde records: {:?}", shared3.serde_ops.lock().unwrap());
    println!("Q3 raw records: {:?}", shared3.raw_ops.lock().unwrap());
    show(
        "Q3 serde path=5",
        e3.eval(&script(
            "airsstack.agentstacks.propose({kind=\"write\", path=5, content=\"x\"})",
        )),
    );
    show(
        "Q3 serde missing content",
        e3.eval(&script(
            "airsstack.agentstacks.propose({kind=\"write\", path=\"p\"})",
        )),
    );
    show(
        "Q3 serde unknown key",
        e3.eval(&script(
            "airsstack.agentstacks.propose({kind=\"write\", path=\"p\", content=\"c\", extra=1})",
        )),
    );
    show(
        "Q3 serde non-table arg",
        e3.eval(&script("airsstack.agentstacks.propose(\"nope\")")),
    );
    show(
        "Q3 raw path=5",
        e3.eval(&script(
            "airsstack.agentstacks.propose_raw({kind=\"write\", path=5, content=\"x\"})",
        )),
    );
    show(
        "Q3 raw path=true",
        e3.eval(&script(
            "airsstack.agentstacks.propose_raw({kind=\"write\", path=true, content=\"x\"})",
        )),
    );
    println!("Q3 serde records after: {:?}", shared3.serde_ops.lock().unwrap());
    println!("Q3 raw records after: {:?}", shared3.raw_ops.lock().unwrap());

    // Q4
    let e4 = engine(Agentstacks::new(Shared::default())).expect("build q4");
    show(
        "Q4 require twice(21)",
        e4.eval_to::<i64>(&script(
            "return airsstack.agentstacks.require(\"lib.util\").twice(21)",
        )),
    );
    show(
        "Q4 require raising module",
        e4.eval(&script("airsstack.agentstacks.require(\"lib.bad\")")),
    );
    show(
        "Q4 error in function from required module",
        e4.eval(&script("airsstack.agentstacks.require(\"lib.late\").f()")),
    );
    show(
        "Q4 load still withheld from script",
        e4.eval_to::<String>(&script("return type(load)")),
    );

    // Q5 + Q10
    let e5 = engine(Agentstacks::new(Shared::default())).expect("build q5");
    show("Q5a syntax", e5.eval(&script("local function f(")));
    show("Q5b runtime error", e5.eval(&script("error(\"bad op\")")));
    show("Q5c infinite loop", e5.eval(&script("while true do end")));
    show("Q10 reuse after Q5c", e5.eval_to::<i64>(&script("return 7")));
    show(
        "Q5d external from callback",
        e5.eval(&script("airsstack.agentstacks.fail()")),
    );
    show(
        "Q5d2 RuntimeError from callback",
        e5.eval(&script("airsstack.agentstacks.fail_rt()")),
    );

    // Q6
    show(
        "Q6 method",
        e5.eval_to::<String>(&script("return (\"abc\"):upper()")),
    );
    show(
        "Q6 function",
        e5.eval_to::<String>(&script("return string.upper(\"abc\")")),
    );
    show(
        "Q6 getmetatable('')",
        e5.eval_to::<String>(&script("return tostring(getmetatable(''))")),
    );

    // Q7
    let shared7 = Shared::default();
    let mut m7 = Agentstacks::new(shared7.clone());
    m7.packs = Some(Arc::new(vec![Pack {
        name: "core".to_owned(),
        text: "a\0b".to_owned(),
        blob: Bytes(vec![0x00, 0xff, 0x41]),
    }]));
    let e7 = engine(m7).expect("build q7");
    show(
        "Q7 lua view",
        e7.eval_to::<String>(&script(
            "local p = airsstack.agentstacks.packs[1]\n\
             airsstack.agentstacks.propose_raw({kind=\"write\", path=p.name, content=p.blob})\n\
             airsstack.agentstacks.propose_raw({kind=\"write\", path=\"t\", content=p.text})\n\
             return type(p.blob) .. ' #text=' .. #p.text .. ' #blob=' .. #p.blob \
             .. ' bytes=' .. table.concat({string.byte(p.blob, 1, -1)}, ',')",
        )),
    );
    for (i, r) in shared7.raw_ops.lock().unwrap().iter().enumerate() {
        println!(
            "Q7 roundtrip[{i}]: path={:?} len={} bytes={:?}",
            r.path,
            r.content.len(),
            r.content
        );
    }
    show(
        "Q7 serde String with 0xff",
        e7.eval(&script(
            "airsstack.agentstacks.propose({kind=\"write\", path=\"p\", content=\"\\xff\\x00\"})",
        )),
    );
    show(
        "Q7 serde String with 0x00 only",
        e7.eval(&script(
            "airsstack.agentstacks.propose({kind=\"write\", path=\"p\", content=\"a\\x00b\"})",
        )),
    );
    println!("Q7 serde records: {:?}", shared7.serde_ops.lock().unwrap());

    // Q9
    let e9 = engine(Agentstacks::new(Shared::default())).expect("build q9");
    show(
        "Q9a 20MiB string.rep in script",
        e9.eval(&script("local s = string.rep(\"x\", 20*1024*1024)")),
    );
    show("Q9a reuse after", e9.eval_to::<i64>(&script("return 8")));
    let mut m9b = Agentstacks::new(Shared::default());
    m9b.big_at_install = Some(20 * 1024 * 1024);
    match engine(m9b) {
        Ok(e) => show(
            "Q9b 20MiB set at install -> #big",
            e.eval_to::<i64>(&script("return #airsstack.agentstacks.big")),
        ),
        Err(e) => show_err("Q9b 20MiB set at install (build)", &e),
    }
    let mut m9c = Agentstacks::new(Shared::default());
    m9c.big_on_call = Some(20 * 1024 * 1024);
    let e9c = engine(m9c).expect("build q9c");
    show(
        "Q9c 20MiB returned from callback",
        e9c.eval_to::<i64>(&script("return #airsstack.agentstacks.getbig()")),
    );
    let mut m9d = Agentstacks::new(Shared::default());
    m9d.big_on_call = Some(8 * 1024 * 1024);
    let e9d = engine(m9d).expect("build q9d");
    show(
        "Q9d 8MiB returned from callback",
        e9d.eval_to::<i64>(&script("return #airsstack.agentstacks.getbig()")),
    );
}
````
