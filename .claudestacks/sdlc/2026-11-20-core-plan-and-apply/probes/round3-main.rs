//! Round-3 probe: airsl =0.1.4 dispatch, Lua->JSON conversion, serde_json =1.0.154
//! (arbitrary_precision + preserve_order) pretty printing and edits, std::fs rename/mode.
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Mutex};

use airsl::mlua::{self, LuaSerdeExt};
use airsl::modules::{Ext, Json};
use airsl::{Engine, EventName, HostModule, InstallContext, ModuleName, ModuleSet, Policy, Script};

struct Probe {
    name: ModuleName,
    seen: Arc<Mutex<Vec<String>>>,
}

impl HostModule for Probe {
    fn name(&self) -> &ModuleName {
        &self.name
    }
    fn install(&self, lua: &mlua::Lua, table: &mlua::Table, _cx: &InstallContext<'_>) -> airsl::Result<()> {
        let fail = |e: mlua::Error| airsl::Error::lua("probe", e);
        // convert(v): Lua value -> serde_json::Value via mlua serde, record result + caller location
        let seen = Arc::clone(&self.seen);
        let convert = lua
            .create_function(move |lua, v: mlua::Value| {
                let at = lua
                    .inspect_stack(1, |d| format!("{}:{}", d.source().short_src.unwrap_or_default(), d.current_line().unwrap_or(0)))
                    .unwrap_or_default();
                let r: Result<serde_json::Value, _> = lua.from_value(v);
                seen.lock().expect("lock").push(match r {
                    Ok(j) => format!("convert at {at}: OK {}", serde_json::to_string(&j).expect("s")),
                    Err(e) => format!("convert at {at}: ERR {e}"),
                });
                Ok(())
            })
            .map_err(fail)?;
        table.set("convert", convert).map_err(fail)?;
        let refuse = lua
            .create_function(|lua, ()| {
                let at = lua
                    .inspect_stack(1, |d| format!("{}:{}", d.source().short_src.unwrap_or_default(), d.current_line().unwrap_or(0)))
                    .unwrap_or_default();
                Err::<(), _>(mlua::Error::external(format!("{at}: path must be relative")))
            })
            .map_err(fail)?;
        table.set("refuse", refuse).map_err(fail)?;
        Ok(())
    }
}

fn show<T: std::fmt::Debug>(label: &str, r: airsl::Result<T>) {
    match r {
        Ok(v) => println!("{label}: OK {v:?}"),
        Err(e) => println!("{label}: ERR <<{e}>>"),
    }
}

fn airsl_probes() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let mut set = ModuleSet::new();
    set.insert(Box::new(Json::new())).expect("json");
    set.insert(Box::new(Ext::with_events([EventName::new("values").expect("e"), EventName::new("plan").expect("e")])))
        .expect("ext");
    set.insert(Box::new(Probe { name: ModuleName::new("probe").expect("n"), seen: Arc::clone(&seen) })).expect("probe");
    let e = Engine::builder().policy(Policy::pure()).stdlib(set).build().expect("engine");
    let ev = |s: &str| EventName::new(s).expect("ev");

    // A1: same event registered twice
    show(
        "A1 eval register values twice",
        e.eval(&Script::from_source(
            "airsstack.ext.on('values', function() return {which='first'} end)\nairsstack.ext.on('values', function() return {which='second'} end)",
            "adapter.lua",
        ).expect("s")),
    );
    show("A1 dispatch values", e.dispatch(&ev("values"), &serde_json::json!({})));

    // A2: input numbers under arbitrary_precision reach Lua; handler returns mixed table
    show(
        "A2 eval",
        e.eval(&Script::from_source(
            "airsstack.ext.on('plan', function(p)\n  return { n = p.n, f = p.f, big = p.big, len = #p.list, first = p.list[1], s = p.s, empty = {}, seq = {1,2} }\nend)",
            "adapter.lua",
        ).expect("s")),
    );
    show(
        "A2 dispatch plan",
        e.dispatch(&ev("plan"), &serde_json::json!({"n": 1, "f": 1.5, "big": 12345678901234567890u64, "list": ["a","b"], "s": "x"})),
    );

    // A3: handler returning nil
    show("A3 eval", e.eval(&Script::from_source("airsstack.ext.on('plan', function() return nil end)", "adapter.lua").expect("s")));
    show("A3 dispatch plan returning nil", e.dispatch(&ev("plan"), &serde_json::json!({})));

    // A4: Lua value -> serde_json via mlua from_value, under arbitrary_precision + preserve_order
    show(
        "A4 eval",
        e.eval(&Script::from_source(
            "local P = airsstack.probe\nP.convert({})\nP.convert({1,2})\nP.convert({a=1})\nP.convert({1, a=2})\nP.convert(9007199254740993)\nP.convert(1.5)\nP.convert({z=1, a={b=true}})\nP.convert(airsstack.json.decode('{\"x\":null,\"arr\":[]}'))\nP.convert(function() end)",
            "adapter.lua",
        ).expect("s")),
    );

    // A5: error raised by a host fn inside a dispatched handler, location from inspect_stack
    show(
        "A5 eval",
        e.eval(&Script::from_source("airsstack.ext.on('plan', function()\n  local x = 1\n  airsstack.probe.refuse()\nend)", "adapter.lua").expect("s")),
    );
    show("A5 dispatch plan", e.dispatch(&ev("plan"), &serde_json::json!({})));

    for l in seen.lock().expect("lock").drain(..) {
        println!("{l}");
    }
}

fn json_probes() {
    use serde::Serialize;
    let src = "{\n    \"permissions\": {\"allow\": [\"Bash(x:*)\"]},\n    \"zeta\": 1,\n    \"alpha\": 12345678901234567890123,\n    \"d\": 0.10,\n    \"hooks\": {}\n}\n";
    let mut v: serde_json::Value = serde_json::from_str(src).expect("parse");
    for (label, indent) in [("2sp", &b"  "[..]), ("4sp", &b"    "[..]), ("tab", &b"\t"[..])] {
        let mut out = Vec::new();
        let fmt = serde_json::ser::PrettyFormatter::with_indent(indent);
        let mut ser = serde_json::Serializer::with_formatter(&mut out, fmt);
        v.serialize(&mut ser).expect("ser");
        println!("J1 {label}:\n{}", String::from_utf8(out).expect("utf8"));
    }
    // J3: replace an existing key keeps its position; a new key is appended
    let obj = v.as_object_mut().expect("obj");
    obj.insert("zeta".to_owned(), serde_json::json!(2));
    obj.insert("new".to_owned(), serde_json::json!([]));
    println!("J3 after insert existing+new: {}", serde_json::to_string(&v).expect("s"));
    // J4: pointer_mut into nested + create path
    if let Some(h) = v.pointer_mut("/hooks") {
        h.as_object_mut().expect("o").insert("PreToolUse".to_owned(), serde_json::json!([{"matcher": "Read"}]));
    }
    println!("J4 pointer_mut /hooks: {}", serde_json::to_string(&v).expect("s"));
    println!("J4 pointer_mut /missing/x is_none: {}", v.pointer_mut("/missing/x").is_none());
    // J5: empty object/array pretty
    println!("J5 empty pretty: {}", serde_json::to_string_pretty(&serde_json::json!({"o": {}, "a": []})).expect("s"));
}

fn std_probes() {
    let base = std::env::temp_dir().join(format!("probe-r3-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("base");
    // S1: rename dir onto existing non-empty dir / empty dir
    let a = base.join("a");
    let b = base.join("b");
    let c = base.join("c");
    std::fs::create_dir_all(&a).expect("a");
    std::fs::write(a.join("f"), "1").expect("f");
    std::fs::create_dir_all(&b).expect("b");
    std::fs::write(b.join("g"), "2").expect("g");
    match std::fs::rename(&a, &b) {
        Ok(()) => println!("S1 rename dir onto non-empty dir: OK"),
        Err(e) => println!("S1 rename dir onto non-empty dir: ERR kind={:?} msg={e}", e.kind()),
    }
    std::fs::create_dir_all(&c).expect("c");
    match std::fs::rename(&a, &c) {
        Ok(()) => println!("S1 rename dir onto empty dir: OK (c/f exists={})", c.join("f").exists()),
        Err(e) => println!("S1 rename dir onto empty dir: ERR kind={:?} msg={e}", e.kind()),
    }
    // S2: new file mode from std::fs::write
    let n = base.join("new.txt");
    std::fs::write(&n, "x").expect("w");
    println!("S2 std::fs::write new file mode: {:o}", std::fs::metadata(&n).expect("m").permissions().mode() & 0o777);
    // S3: rename temp over an existing 755 file -> mode of result
    let target = base.join("run.sh");
    std::fs::write(&target, "old").expect("w");
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).expect("p");
    let tmp = base.join(".run.sh.tmp");
    std::fs::write(&tmp, "new").expect("w");
    std::fs::rename(&tmp, &target).expect("r");
    println!("S3 mode after rename over 755 file: {:o}", std::fs::metadata(&target).expect("m").permissions().mode() & 0o777);
    // S4: rename over a symlink replaces the link; renaming onto the canonical target keeps it
    let real = base.join("real.json");
    std::fs::write(&real, "{}").expect("w");
    let link = base.join("link.json");
    std::os::unix::fs::symlink(&real, &link).expect("ln");
    let tmp2 = base.join(".tmp2");
    std::fs::write(&tmp2, "A").expect("w");
    std::fs::rename(&tmp2, &link).expect("r");
    println!(
        "S4 rename onto link: link is_symlink={} real content={:?}",
        std::fs::symlink_metadata(&link).expect("m").file_type().is_symlink(),
        std::fs::read_to_string(&real).expect("r")
    );
    let link2 = base.join("link2.json");
    std::os::unix::fs::symlink(&real, &link2).expect("ln");
    let canon = std::fs::canonicalize(&link2).expect("c");
    let tmp3 = base.join(".tmp3");
    std::fs::write(&tmp3, "B").expect("w");
    std::fs::rename(&tmp3, &canon).expect("r");
    println!(
        "S4 rename onto canonical target: link2 is_symlink={} real content={:?}",
        std::fs::symlink_metadata(&link2).expect("m").file_type().is_symlink(),
        std::fs::read_to_string(&real).expect("r")
    );
    // S5: canonicalize a path that does not exist yet
    match std::fs::canonicalize(base.join("nope/x.md")) {
        Ok(p) => println!("S5 canonicalize missing: OK {}", p.display()),
        Err(e) => println!("S5 canonicalize missing: ERR kind={:?} msg={e}", e.kind()),
    }
    let _ = std::fs::remove_dir_all(&base);
}

fn main() {
    airsl_probes();
    json_probes();
    std_probes();
}
