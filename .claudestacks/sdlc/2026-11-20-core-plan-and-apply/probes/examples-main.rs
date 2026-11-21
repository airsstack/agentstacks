//! Runs the code examples of core-plan-and-apply/spec.md against the real crates.
//! Blocks are extracted from the spec file itself, so the text checked is the text written.
use airsl::mlua::{self, LuaSerdeExt};
use airsl::modules::{Ext, Json};
use airsl::{Engine, EventName, HostModule, InstallContext, ModuleName, ModuleSet, Policy, Script};
use serde::Serialize;

fn blocks(spec: &str, lang: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Option<String> = None;
    for line in spec.lines() {
        match &mut cur {
            None if line == format!("```{lang}") => cur = Some(String::new()),
            Some(b) if line == "```" => {
                out.push(std::mem::take(b));
                cur = None;
            }
            Some(b) => {
                b.push_str(line);
                b.push('\n');
            }
            None => {}
        }
    }
    out
}

/// Lua value -> JSON per spec §5 conversion table (sequence -> array, string keys -> sorted object).
fn to_json(v: &mlua::Value) -> Result<serde_json::Value, String> {
    Ok(match v {
        mlua::Value::Nil => serde_json::Value::Null,
        mlua::Value::Boolean(b) => serde_json::Value::Bool(*b),
        mlua::Value::Integer(i) => serde_json::Value::from(*i),
        mlua::Value::Number(n) => serde_json::Number::from_f64(*n).map(serde_json::Value::Number).ok_or("NaN")?,
        mlua::Value::String(s) => serde_json::Value::String(s.to_str().map_err(|e| e.to_string())?.to_string()),
        mlua::Value::Table(t) => {
            let n = t.raw_len();
            let mut count = 0usize;
            let mut keys = Vec::new();
            for p in t.pairs::<mlua::Value, mlua::Value>() {
                let (k, val) = p.map_err(|e| e.to_string())?;
                count += 1;
                if let mlua::Value::String(s) = &k {
                    keys.push((s.to_str().map_err(|e| e.to_string())?.to_string(), val));
                }
            }
            if count == 0 {
                serde_json::Value::Object(serde_json::Map::new())
            } else if keys.is_empty() && count == n {
                let mut a = Vec::new();
                for i in 1..=n {
                    a.push(to_json(&t.raw_get::<mlua::Value>(i).map_err(|e| e.to_string())?)?);
                }
                serde_json::Value::Array(a)
            } else if keys.len() == count {
                keys.sort_by(|a, b| a.0.cmp(&b.0));
                let mut m = serde_json::Map::new();
                for (k, val) in keys {
                    m.insert(k, to_json(&val)?);
                }
                serde_json::Value::Object(m)
            } else {
                return Err("table mixes list and keyed entries".into());
            }
        }
        other => return Err(format!("{} cannot be JSON", other.type_name())),
    })
}

/// json.edit per spec §5 (only the paths the example exercises).
fn edit(text: &str, path: &[String], value: &mlua::Value) -> Result<String, String> {
    let src = if text.trim().is_empty() { "{}" } else { text };
    let mut root: serde_json::Value = serde_json::from_str(src).map_err(|e| e.to_string())?;
    let indent: String = text
        .lines()
        .skip(1)
        .find(|l| l.starts_with(' ') || l.starts_with('\t'))
        .map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').collect::<String>())
        .filter(|w| w.chars().all(|c| c == ' ') || w == "\t")
        .unwrap_or_else(|| "  ".to_owned());
    let mut cur = root.as_object_mut().ok_or("top level must be an object")?;
    for key in &path[..path.len() - 1] {
        cur = cur
            .entry(key.clone())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or(format!("\"{key}\" is not an object"))?;
    }
    let last = path.last().ok_or("path: empty")?;
    if matches!(value, mlua::Value::Nil) {
        cur.shift_remove(last);
    } else {
        cur.insert(last.clone(), to_json(value)?);
    }
    let mut out = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes());
    let mut ser = serde_json::Serializer::with_formatter(&mut out, fmt);
    root.serialize(&mut ser).map_err(|e| e.to_string())?;
    let mut s = String::from_utf8(out).map_err(|e| e.to_string())?;
    if text.ends_with('\n') {
        s.push('\n');
    }
    Ok(s)
}

struct Stub {
    name: ModuleName,
    out: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl HostModule for Stub {
    fn name(&self) -> &ModuleName {
        &self.name
    }
    fn install(&self, lua: &mlua::Lua, _t: &mlua::Table, _cx: &InstallContext<'_>) -> airsl::Result<()> {
        let fail = |e: mlua::Error| airsl::Error::lua("stub", e);
        let g = lua.globals();
        let ag = lua.create_table().map_err(fail)?;
        println!("install: global airsstack is {}", g.get::<mlua::Value>("airsstack").map_err(fail)?.type_name());
        // forward at call time: airsstack.ext.on is looked up when agentstacks.on runs
        let on = lua
            .create_function(|lua, (event, f): (mlua::Value, mlua::Value)| {
                let airsstack: mlua::Table = lua.globals().get("airsstack")?;
                let ext: mlua::Table = airsstack.get("ext")?;
                let on: mlua::Function = ext.get("on")?;
                on.call::<()>((event, f))
            })
            .map_err(fail)?;
        ag.set("on", on).map_err(fail)?;
        let json = lua.create_table().map_err(fail)?;
        let out = std::sync::Arc::clone(&self.out);
        let f = lua
            .create_function(move |_, (text, path, value): (String, Vec<String>, mlua::Value)| {
                let r = edit(&text, &path, &value).map_err(mlua::Error::external)?;
                *out.lock().expect("lock") = Some(r.clone());
                Ok(r)
            })
            .map_err(fail)?;
        json.set("edit", f).map_err(fail)?;
        // where(): caller location via inspect_stack(1); req(): host-side load of a module chunk
        let w = lua
            .create_function(|lua, ()| {
                Ok(lua
                    .inspect_stack(1, |d| format!("{}:{}", d.source().short_src.unwrap_or_default(), d.current_line().unwrap_or(0)))
                    .unwrap_or_default())
            })
            .map_err(fail)?;
        ag.set("where", w).map_err(fail)?;
        let req = lua
            .create_function(|lua, ()| {
                lua.load("local M = {}\nfunction M.go()\n  return agentstacks.where()\nend\nreturn M\n")
                    .set_name("@lib/settings.lua")
                    .call::<mlua::Value>(())
            })
            .map_err(fail)?;
        ag.set("req", req).map_err(fail)?;
        ag.set("json", json).map_err(fail)?;
        g.set("agentstacks", ag).map_err(fail)?;
        g.set("print", mlua::Value::Nil).map_err(fail)?;
        Ok(())
    }
}

fn main() {
    let spec = std::fs::read_to_string(std::env::args().nth(1).expect("spec path")).expect("read spec");
    let json = blocks(&spec, "json");
    let lua = blocks(&spec, "lua");
    println!("found {} json blocks, {} lua blocks", json.len(), lua.len());
    for (i, b) in json.iter().enumerate() {
        match serde_json::from_str::<serde_json::Value>(b) {
            Ok(_) => println!("json[{i}]: parses OK"),
            Err(e) => println!("json[{i}]: ERR {e}"),
        }
    }

    let out = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut set = ModuleSet::new();
    set.insert(Box::new(Json::new())).expect("json");
    set.insert(Box::new(Ext::with_events([EventName::new("values").expect("e"), EventName::new("plan").expect("e")])))
        .expect("ext");
    set.insert(Box::new(Stub { name: ModuleName::new("stub").expect("n"), out: std::sync::Arc::clone(&out) })).expect("stub");
    let e = Engine::builder().policy(Policy::pure()).stdlib(set).build().expect("engine");

    // lua[0] = values example, dispatched with json[0] (the values payload)
    match e.eval(&Script::from_source(&lua[0], "adapter.lua").expect("s")) {
        Ok(()) => println!("lua[0] values example: eval OK"),
        Err(err) => println!("lua[0] values example: eval ERR {err}"),
    }
    let payload: serde_json::Value = serde_json::from_str(&json[0]).expect("payload");
    match e.dispatch(&EventName::new("values").expect("e"), &payload) {
        Ok(v) => println!("lua[0] dispatch values: OK {}", serde_json::to_string(&v).expect("s")),
        Err(err) => println!("lua[0] dispatch values: ERR {err}"),
    }

    // lua[1] = json.edit example; its output must equal json[2] byte for byte
    match e.eval(&Script::from_source(&lua[1], "adapter.lua").expect("s")) {
        Ok(()) => println!("lua[1] json.edit example: eval OK"),
        Err(err) => println!("lua[1] json.edit example: eval ERR {err}"),
    }
    let got = out.lock().expect("lock").clone().unwrap_or_default();
    println!("lua[1] output:\n{got}<<end>>");
    println!("lua[1] output == json[2] block: {}", got == json[2]);

    // lua[2] = runtime.lua; must equal the file that ran on the airsl binary
    let ran = std::fs::read_to_string(std::env::args().nth(2).expect("runtime path")).expect("read runtime");
    println!("lua[2] runtime.lua == probes/runtime-runtime.lua: {}", lua[2] == ran);
    // caller location for an op called from a required module, inside a dispatched handler
    match e.eval(&Script::from_source(
        "local s = agentstacks.req()\nagentstacks.on('plan', function()\n  return { at = s.go(), here = agentstacks.where() }\nend)",
        "adapter.lua",
    ).expect("s")) {
        Ok(()) => println!("R eval: OK"),
        Err(err) => println!("R eval: ERR {err}"),
    }
    match e.dispatch(&EventName::new("plan").expect("e"), &serde_json::json!({})) {
        Ok(v) => println!("R dispatch plan (location from module / from adapter): OK {}", serde_json::to_string(&v).expect("s")),
        Err(err) => println!("R dispatch plan: ERR {err}"),
    }
    // P: a limit breach caught by pcall inside a dispatched handler
    match e.eval(&Script::from_source(
        "agentstacks.on('plan', function()\n  local ok, err = pcall(function() while true do end end)\n  return { caught = ok, err = tostring(err) }\nend)",
        "adapter.lua",
    ).expect("s")) {
        Ok(()) => println!("P eval: OK"),
        Err(err) => println!("P eval: ERR {err}"),
    }
    match e.dispatch(&EventName::new("plan").expect("e"), &serde_json::json!({})) {
        Ok(v) => println!("P dispatch plan (pcall around infinite loop): OK {}", serde_json::to_string(&v).expect("s")),
        Err(err) => println!("P dispatch plan (pcall around infinite loop): ERR {err} | exhausted_limit={:?}", err.exhausted_limit()),
    }
    let _ = mlua::Lua::new().to_value(&()); // keep LuaSerdeExt import used
}
