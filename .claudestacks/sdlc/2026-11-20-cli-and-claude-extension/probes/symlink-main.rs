use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::symlink;
fn main() {
    let d = std::env::temp_dir().join("symprobe-run"); let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join("proj")).unwrap(); fs::create_dir_all(d.join("outside")).unwrap();
    let out = d.join("outside/victim"); fs::write(&out, "VICTIM\n").unwrap();
    let tmp = d.join("proj/.agentstacks.toml.agentstacks-tmp");
    // 1. today: std::fs::write through a planted link
    symlink(&out, &tmp).unwrap();
    fs::write(&tmp, "tools = []\n").unwrap();
    println!("1 fs::write via link -> victim = {:?}", fs::read_to_string(&out).unwrap());
    fs::write(&out, "VICTIM\n").unwrap();
    // 2. create_new on a link to an existing file
    let e = OpenOptions::new().write(true).create_new(true).open(&tmp);
    println!("2 create_new on link->existing: {:?} kind={:?} victim={:?}", e.as_ref().map(|_|()).map_err(|e| e.to_string()), e.as_ref().err().map(|e| e.kind()), fs::read_to_string(&out).unwrap());
    // 3. create_new on a dangling link
    fs::remove_file(&tmp).unwrap(); let gone = d.join("outside/nothere"); symlink(&gone, &tmp).unwrap();
    let e = OpenOptions::new().write(true).create_new(true).open(&tmp);
    println!("3 create_new on dangling link: {:?} kind={:?} created_outside={}", e.as_ref().map(|_|()).map_err(|e| e.to_string()), e.as_ref().err().map(|e| e.kind()), gone.exists());
    // 4. remove_file on a link removes the link, not the target
    fs::remove_file(&tmp).unwrap(); symlink(&out, &tmp).unwrap();
    fs::remove_file(&tmp).unwrap();
    println!("4 remove_file(link): link_exists={} victim_exists={}", fs::symlink_metadata(&tmp).is_ok(), out.exists());
    // 5. after removal, create_new succeeds and writes inside
    let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp).unwrap(); f.write_all(b"tools = []\n").unwrap();
    println!("5 create_new after remove: is_symlink={} victim={:?}", fs::symlink_metadata(&tmp).unwrap().file_type().is_symlink(), fs::read_to_string(&out).unwrap());
    // 6. create_new on a leftover regular temp
    let e = OpenOptions::new().write(true).create_new(true).open(&tmp);
    println!("6 create_new on leftover regular file: {:?} kind={:?}", e.as_ref().map(|_|()).map_err(|e| e.to_string()), e.as_ref().err().map(|e| e.kind()));
    // 7. a directory planted at the temp path
    fs::remove_file(&tmp).unwrap(); fs::create_dir(&tmp).unwrap();
    println!("7 remove_file(dir): {:?}", fs::remove_file(&tmp).map_err(|e| (e.to_string(), e.kind())));
}
