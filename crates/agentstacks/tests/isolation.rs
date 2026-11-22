//! Guards the rule that the inside, `types`, `port` and `app`, reaches the disk, processes, the
//! network and the adapters only through ports, and that `types` reaches only the read-only tree
//! port.
//!
//! Logic under `src/types` takes a `FileTree` instead of calling `std::fs`, so a rule runs the same
//! over the real disk and over an in-memory tree. Nothing else enforces that: a stray `std::fs::read`
//! compiles, passes clippy and passes every test. These tests read every `.rs` file under
//! `src/types`, `src/port` and `src/app` and fail when code, as opposed to a comment, names one of
//! the forbidden paths, whether written whole (`std::fs::read`, `::std::fs::read`), in a single
//! `use`, or in a grouped `use std::{..}` that lists `fs`, `process` or `net` as a member. They
//! also fail when the inside names an adapter module (`crate::fs`, `crate::process`,
//! `crate::host`), and when `src/types` names a `crate::port` path other than `FileTree` and
//! `EntryKind`, or `FakeFileTree` in test code. The scans are lexical: a relative path such as
//! `super::super::fs` is not seen, so the crate writes `crate::` paths.

use std::error::Error;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The directories of the inside, relative to the crate root.
const INSIDE: [&str; 3] = ["src/types", "src/port", "src/app"];

/// The adapter modules the inside must never name, each with the path it is refused as.
const ADAPTERS: [(&str, &str); 3] = [
    ("fs", "crate::fs"),
    ("process", "crate::process"),
    ("host", "crate::host"),
];

/// The items of `crate::port` that `types` may name anywhere.
const TREE_PORT: [&str; 2] = ["FileTree", "EntryKind"];

/// The in-memory tree, which `types` may name in test code only.
const TEST_FAKE: &str = "FakeFileTree";

/// The paths the inside must never name in code.
const FORBIDDEN: [&str; 4] = ["std::fs", "std::process", "std::net", "File::open"];

/// The `std` modules `types` must never reach through a grouped `use std::{..}`, each with the
/// path it stands for in [`FORBIDDEN`].
const FORBIDDEN_IN_GROUP: [(&str, &str); 3] = [
    ("fs", "std::fs"),
    ("process", "std::process"),
    ("net", "std::net"),
];

/// `source` with every comment replaced by spaces, so line numbers and string contents survive.
///
/// Handles line comments (`//`, `///`, `//!`), nested block comments, string and raw string
/// literals (a `//` inside one is not a comment) and character literals (so `'"'` does not open a
/// string). A lifetime is told from a character literal by the closing quote.
fn without_comments(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i = skip_block_comment(&chars, i, &mut out);
        } else if c == '"' {
            i = copy_string(&chars, i, &mut out);
        } else if let Some(hashes) = raw_string_hashes(&chars, i) {
            i = copy_raw_string(&chars, i, hashes, &mut out);
        } else if c == '\'' {
            i = copy_char_literal(&chars, i, &mut out);
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Blanks the block comment that starts at `start` and returns the index after it.
fn skip_block_comment(chars: &[char], start: usize, out: &mut String) -> usize {
    let mut depth = 0;
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
            depth += 1;
            out.push_str("  ");
            i += 2;
        } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
            depth -= 1;
            out.push_str("  ");
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            out.push(if chars[i] == '\n' { '\n' } else { ' ' });
            i += 1;
        }
    }
    i
}

/// Copies the string literal that starts at `start` and returns the index after it.
fn copy_string(chars: &[char], start: usize, out: &mut String) -> usize {
    out.push('"');
    let mut i = start + 1;
    while i < chars.len() {
        out.push(chars[i]);
        if chars[i] == '\\' {
            if let Some(escaped) = chars.get(i + 1) {
                out.push(*escaped);
            }
            i += 2;
        } else if chars[i] == '"' {
            return i + 1;
        } else {
            i += 1;
        }
    }
    i
}

/// The number of `#` of a raw string literal that starts at `i`, or `None` when none does.
fn raw_string_hashes(chars: &[char], i: usize) -> Option<usize> {
    let starts_word = i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
    if chars[i] != 'r' || !starts_word {
        return None;
    }
    let hashes = chars[i + 1..].iter().take_while(|c| **c == '#').count();
    (chars.get(i + 1 + hashes) == Some(&'"')).then_some(hashes)
}

/// Copies the raw string literal that starts at `start` and returns the index after it.
fn copy_raw_string(chars: &[char], start: usize, hashes: usize, out: &mut String) -> usize {
    let closing: Vec<char> = std::iter::once('"')
        .chain(std::iter::repeat_n('#', hashes))
        .collect();
    let body_start = start + 2 + hashes;
    out.extend(&chars[start..body_start]);
    let mut i = body_start;
    while i < chars.len() {
        if chars[i..].starts_with(&closing) {
            out.extend(&closing);
            return i + closing.len();
        }
        out.push(chars[i]);
        i += 1;
    }
    i
}

/// Copies a character literal, or the lone quote of a lifetime, and returns the index after it.
fn copy_char_literal(chars: &[char], start: usize, out: &mut String) -> usize {
    let end = if chars.get(start + 1) == Some(&'\\') {
        chars
            .get(start + 3..)
            .and_then(|rest| rest.iter().position(|c| *c == '\''))
            .map(|offset| start + 3 + offset)
    } else {
        (chars.get(start + 2) == Some(&'\'')).then_some(start + 2)
    };
    if let Some(end) = end {
        out.extend(&chars[start..=end]);
        end + 1
    } else {
        out.push('\'');
        start + 1
    }
}

/// The 1-based line of byte `index` in `text`.
fn line_of(text: &str, index: usize) -> usize {
    text[..index].bytes().filter(|byte| *byte == b'\n').count() + 1
}

/// Whether `byte` can be part of an identifier.
const fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Every direct member of a grouped `use <root>::{..}` in `stripped` (comments already removed),
/// as the byte index and the leading identifier of the member.
///
/// A member is the first identifier after the opening brace or after a comma of the group's own
/// level, so `std::{io, fs::File}` lists `io` and `fs` while `std::{io::{self, fs}}` lists only
/// `io`: the latter is not `std::fs`.
fn grouped_members<'a>(stripped: &'a str, root: &str) -> Vec<(usize, &'a str)> {
    let bytes = stripped.as_bytes();
    let mut found = Vec::new();
    for (start, _) in stripped.match_indices(root) {
        let own_word = start == 0 || !is_ident(bytes[start - 1]);
        let Some(group) = stripped[start + root.len()..]
            .trim_start()
            .strip_prefix("::")
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix('{'))
        else {
            continue;
        };
        if !own_word {
            continue;
        }
        let mut index = stripped.len() - group.len();
        let mut depth = 1_usize;
        let mut member_start = true;
        while index < bytes.len() && depth > 0 {
            let byte = bytes[index];
            if byte.is_ascii_whitespace() {
                index += 1;
            } else if member_start && depth == 1 && is_ident(byte) {
                let end = (index..bytes.len())
                    .find(|at| !is_ident(bytes[*at]))
                    .unwrap_or(bytes.len());
                found.push((index, &stripped[index..end]));
                member_start = false;
                index = end;
            } else {
                match byte {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                member_start = byte == b',' && depth == 1;
                index += 1;
            }
        }
    }
    found
}

/// Every forbidden module that a grouped `use std::{..}` in `stripped` lists as a direct member,
/// with the 1-based line of the member.
fn grouped_imports(stripped: &str) -> Vec<(usize, &'static str)> {
    grouped_members(stripped, "std")
        .into_iter()
        .flat_map(|(index, word)| {
            FORBIDDEN_IN_GROUP
                .iter()
                .filter(move |(module, _)| *module == word)
                .map(move |(_, path)| (line_of(stripped, index), *path))
        })
        .collect()
}

/// Every path into an adapter module that `source` names in code, with its 1-based line.
///
/// A path counts whole (`crate::fs::StdFileTree`) or as a direct member of a grouped
/// `use crate::{..}`. It must end at a word boundary, so `crate::fsx` is not `crate::fs`.
fn adapter_violations(source: &str) -> Vec<(usize, &'static str)> {
    let stripped = without_comments(source);
    let bytes = stripped.as_bytes();
    let mut found = Vec::new();
    for (module, path) in ADAPTERS {
        for (start, _) in stripped.match_indices(path) {
            let own_word = start == 0 || !is_ident(bytes[start - 1]);
            let ends = bytes
                .get(start + path.len())
                .is_none_or(|byte| !is_ident(*byte));
            if own_word && ends {
                found.push((line_of(&stripped, start), path));
            }
        }
        found.extend(
            grouped_members(&stripped, "crate")
                .into_iter()
                .filter(|(_, word)| *word == module)
                .map(|(index, _)| (line_of(&stripped, index), path)),
        );
    }
    found.sort_unstable();
    found
}

/// `code` with the contents of every string, raw string and character literal replaced by spaces,
/// each character by as many spaces as it has bytes, so a byte index means the same in both texts.
fn without_literals(code: &str) -> String {
    let chars: Vec<char> = code.chars().collect();
    let mut sink = String::new();
    let mut out = String::with_capacity(code.len());
    let mut i = 0;
    while i < chars.len() {
        let end = if chars[i] == '"' {
            copy_string(&chars, i, &mut sink)
        } else if let Some(hashes) = raw_string_hashes(&chars, i) {
            copy_raw_string(&chars, i, hashes, &mut sink)
        } else if chars[i] == '\'' {
            copy_char_literal(&chars, i, &mut sink)
        } else {
            out.push(chars[i]);
            i += 1;
            continue;
        };
        for c in &chars[i..end.min(chars.len())] {
            if *c == '\n' {
                out.push('\n');
            } else {
                out.extend(std::iter::repeat_n(' ', c.len_utf8()));
            }
        }
        i = end;
    }
    out
}

/// The byte ranges of `code` (comments already removed) that are test code: the text from each
/// `#[cfg(test)]` to the end of the item it marks, a brace-matched body or the closing `;`.
fn test_ranges(code: &str) -> Vec<Range<usize>> {
    let blanked = without_literals(code);
    let bytes = blanked.as_bytes();
    let mut ranges = Vec::new();
    for (at, marker) in blanked.match_indices("#[cfg(test)]") {
        let start = at + marker.len();
        let Some(open) = blanked[start..]
            .find(['{', ';'])
            .map(|offset| start + offset)
        else {
            continue;
        };
        if bytes[open] == b';' {
            ranges.push(start..open + 1);
            continue;
        }
        let mut depth = 0_usize;
        let mut end = bytes.len();
        for (offset, byte) in bytes[open..].iter().enumerate() {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + offset + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        ranges.push(start..end);
    }
    ranges
}

/// Every `crate::port` path `source` names in code that is not the read-only tree port, with its
/// 1-based line and the path that is refused.
///
/// `crate::port::FileTree` and `crate::port::EntryKind` are allowed anywhere, whole or as members
/// of a grouped `use crate::port::{..}`; `crate::port::FakeFileTree` is allowed in test code only,
/// which is `whole_file_is_test` or the body of an item marked `#[cfg(test)]`. A bare
/// `crate::port`, whole or as a member of a grouped `use crate::{..}`, is refused.
fn port_violations(source: &str, whole_file_is_test: bool) -> Vec<(usize, String)> {
    let stripped = without_comments(source);
    let bytes = stripped.as_bytes();
    let tests = test_ranges(&stripped);
    let in_test = |at: usize| whole_file_is_test || tests.iter().any(|range| range.contains(&at));
    let mut named: Vec<(usize, &str)> = Vec::new();
    for (start, _) in stripped.match_indices("crate::port") {
        let end = start + "crate::port".len();
        let own_word = start == 0 || !is_ident(bytes[start - 1]);
        if !own_word || bytes.get(end).is_some_and(|byte| is_ident(*byte)) {
            continue;
        }
        match stripped[end..].trim_start().strip_prefix("::") {
            Some(after) => {
                let after = after.trim_start();
                if !after.starts_with('{') {
                    let word_end = after.find(|c: char| !(c.is_alphanumeric() || c == '_'));
                    named.push((start, &after[..word_end.unwrap_or(after.len())]));
                }
            }
            None => named.push((start, "")),
        }
    }
    named.extend(grouped_members(&stripped, "crate::port"));
    named.extend(
        grouped_members(&stripped, "crate")
            .into_iter()
            .filter(|(_, word)| *word == "port")
            .map(|(index, _)| (index, "")),
    );
    let mut found: Vec<(usize, String)> = named
        .into_iter()
        .filter(|(index, word)| {
            !(TREE_PORT.contains(word) || (*word == TEST_FAKE && in_test(*index)))
        })
        .map(|(index, word)| {
            let path = if word.is_empty() {
                "crate::port".to_owned()
            } else {
                format!("crate::port::{word}")
            };
            (line_of(&stripped, index), path)
        })
        .collect();
    found.sort_unstable();
    found
}

/// Whether `parent_source`, the text of a module file, declares the module `name` under
/// `#[cfg(test)]`, as `#[cfg(test)] mod testing;` does.
fn declares_test_module(parent_source: &str, name: &str) -> bool {
    let stripped = without_comments(parent_source);
    stripped.match_indices("#[cfg(test)]").any(|(at, marker)| {
        let after = stripped[at + marker.len()..].trim_start();
        let after = after.strip_prefix("pub").map_or(after, |rest| {
            let rest = rest.trim_start();
            if rest.starts_with('(') {
                rest.split_once(')')
                    .map_or(rest, |(_, tail)| tail.trim_start())
            } else {
                rest
            }
        });
        after
            .strip_prefix("mod")
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix(name))
            .is_some_and(|rest| rest.trim_start().starts_with(';'))
    })
}

/// Whether the whole of `file` is test code because its parent declares it under `#[cfg(test)]`,
/// as `set/mod.rs` does for `set/testing.rs`.
fn is_test_file(file: &Path) -> Result<bool, Box<dyn Error>> {
    let (Some(dir), Some(stem)) = (
        file.parent(),
        file.file_stem().and_then(|stem| stem.to_str()),
    ) else {
        return Ok(false);
    };
    let (declarer, name) = if stem == "mod" {
        let name = dir.file_name().and_then(|name| name.to_str());
        (dir.parent().map(|grand| grand.join("mod.rs")), name)
    } else {
        (Some(dir.join("mod.rs")), Some(stem))
    };
    match (declarer, name) {
        (Some(declarer), Some(name)) if declarer.is_file() => {
            Ok(declares_test_module(&fs::read_to_string(declarer)?, name))
        }
        _ => Ok(false),
    }
}

/// Every forbidden path `source` names in code, with its 1-based line.
fn violations(source: &str) -> Vec<(usize, &'static str)> {
    let stripped = without_comments(source);
    let mut found = Vec::new();
    for (index, line) in stripped.lines().enumerate() {
        for forbidden in FORBIDDEN {
            if line.contains(forbidden) {
                found.push((index + 1, forbidden));
            }
        }
    }
    found.extend(grouped_imports(&stripped));
    found.sort_unstable();
    found
}

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(rust_files(&path)?);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Runs `scan` over every `.rs` file under `dir` (relative to the crate root), which must hold at
/// least `min_files` of them, and returns one `file:line: path` offence for each hit.
fn scan_dir<F>(dir: &str, min_files: usize, scan: F) -> Result<Vec<String>, Box<dyn Error>>
where
    F: Fn(&Path, &str) -> Result<Vec<(usize, String)>, Box<dyn Error>>,
{
    let files = rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join(dir))?;
    assert!(
        files.len() >= min_files,
        "found only {} files under {dir}",
        files.len()
    );
    let mut offences = Vec::new();
    for file in &files {
        let source = fs::read_to_string(file)?;
        for (line, path) in scan(file, &source)? {
            offences.push(format!("{}:{line}: {path}", file.display()));
        }
    }
    Ok(offences)
}

#[test]
fn types_never_names_the_disk_a_process_or_the_network() -> TestResult {
    let offences = scan_dir("src/types", 51, |_, source| Ok(owned(violations(source))))?;
    assert!(
        offences.is_empty(),
        "types must reach the outside only through its ports:\n{}",
        offences.join("\n")
    );
    Ok(())
}

#[test]
fn the_scan_flags_code_that_names_a_forbidden_path() {
    assert_eq!(violations("let _ = std::fs::read(p);\n"), [(1, "std::fs")]);
    assert_eq!(
        violations("use std::process::Command;\nuse std::net::TcpStream;\n"),
        [(1, "std::process"), (2, "std::net")]
    );
    assert_eq!(violations("let f = File::open(p);\n"), [(1, "File::open")]);
    assert_eq!(
        violations("fn f() {}\nstd::fs::write(a, b);\n"),
        [(2, "std::fs")]
    );
}

#[test]
fn the_scan_ignores_comments() {
    let source = "// std::fs is forbidden\n\
        /// reads through std::fs::read\n\
        //! std::process\n\
        /* File::open */\n\
        /* a /* std::net */ std::fs */\n\
        fn f() {}\n";
    assert!(violations(source).is_empty());
}

#[test]
fn the_scan_does_not_mistake_literals_for_comments() {
    assert_eq!(
        violations("let a = \"//\"; std::fs::read(p);\n"),
        [(1, "std::fs")]
    );
    assert_eq!(
        violations("let a = '\"'; std::fs::read(p);\n"),
        [(1, "std::fs")]
    );
    assert_eq!(
        violations("let a = r#\"// \"quoted\"\"#; std::net::x();\n"),
        [(1, "std::net")]
    );
    assert_eq!(
        violations("fn f<'a>(x: &'a str) { std::fs::read(x); }\n"),
        [(1, "std::fs")]
    );
    assert_eq!(
        violations("let a = '\\\\'; std::fs::read(p);\n"),
        [(1, "std::fs")]
    );
}

#[test]
fn the_scan_flags_grouped_imports_of_a_forbidden_module() {
    assert_eq!(violations("use std::{fs, path::Path};\n"), [(1, "std::fs")]);
    assert_eq!(
        violations("use std::{process::Command};\n"),
        [(1, "std::process")]
    );
    assert_eq!(violations("use std::{io, fs::File};\n"), [(1, "std::fs")]);
    assert_eq!(
        violations("use std::{\n    path::Path,\n    net::{TcpStream, UdpSocket},\n};\n"),
        [(3, "std::net")]
    );
    assert_eq!(violations("use ::std::{ fs };\n"), [(1, "std::fs")]);
}

#[test]
fn the_scan_does_not_flag_grouped_imports_of_other_modules() {
    assert!(violations("use std::{path::Path, fmt};\n").is_empty());
    assert!(violations("use std::{io::{self, Read}, collections::BTreeMap};\n").is_empty());
    assert!(violations("use std::{io::{fs}};\n").is_empty());
    assert!(violations("use mystd::{fs};\n").is_empty());
    assert!(violations("use other::{fs, process};\n").is_empty());
}

#[test]
fn the_scan_flags_fully_qualified_paths() {
    assert_eq!(
        violations("let _ = ::std::fs::read(p);\n"),
        [(1, "std::fs")]
    );
    assert_eq!(
        violations("let _ = std::process::id();\n"),
        [(1, "std::process")]
    );
}

/// The scan results as `(line, path)` pairs, so a test compares plain text.
fn paths(found: &[(usize, String)]) -> Vec<(usize, &str)> {
    found
        .iter()
        .map(|(line, path)| (*line, path.as_str()))
        .collect()
}

/// Owned `(line, path)` pairs from a scan whose paths are fixed words.
fn owned(found: Vec<(usize, &'static str)>) -> Vec<(usize, String)> {
    found
        .into_iter()
        .map(|(line, path)| (line, path.to_owned()))
        .collect()
}

#[test]
fn port_never_names_the_disk_a_process_or_the_network() -> TestResult {
    let offences = scan_dir("src/port", 1, |_, source| Ok(owned(violations(source))))?;
    assert!(
        offences.is_empty(),
        "port must reach the outside only through adapters:\n{}",
        offences.join("\n")
    );
    Ok(())
}

#[test]
fn app_never_names_the_disk_a_process_or_the_network() -> TestResult {
    let offences = scan_dir("src/app", 1, |_, source| Ok(owned(violations(source))))?;
    assert!(
        offences.is_empty(),
        "app must reach the outside only through ports:\n{}",
        offences.join("\n")
    );
    Ok(())
}

#[test]
fn inside_never_names_an_adapter_module() -> TestResult {
    let mut offences = Vec::new();
    for dir in INSIDE {
        offences.extend(scan_dir(dir, 1, |_, source| {
            Ok(owned(adapter_violations(source)))
        })?);
    }
    assert!(
        offences.is_empty(),
        "types, port and app must not name an adapter module:\n{}",
        offences.join("\n")
    );
    Ok(())
}

#[test]
fn types_names_only_the_tree_port() -> TestResult {
    let offences = scan_dir("src/types", 50, |file, source| {
        Ok(port_violations(source, is_test_file(file)?))
    })?;
    assert!(
        offences.is_empty(),
        "types may name only the read-only tree port:\n{}",
        offences.join("\n")
    );
    Ok(())
}

#[test]
fn the_scan_flags_a_path_into_an_adapter_module() {
    assert_eq!(
        adapter_violations("use crate::fs::StdFileTree;\n"),
        [(1, "crate::fs")]
    );
    assert_eq!(
        adapter_violations("fn f() {}\ncrate::process::GitCli::new();\n"),
        [(2, "crate::process")]
    );
    assert_eq!(
        adapter_violations("use crate::host::AirslAdapter;\n"),
        [(1, "crate::host")]
    );
}

#[test]
fn the_scan_flags_grouped_imports_of_an_adapter_module() {
    assert_eq!(
        adapter_violations("use crate::{fs, types::Location};\n"),
        [(1, "crate::fs")]
    );
    assert_eq!(
        adapter_violations("use crate::{\n    types,\n    host::AirslAdapter,\n};\n"),
        [(3, "crate::host")]
    );
}

#[test]
fn the_scan_does_not_flag_other_crate_paths() {
    assert!(adapter_violations("use crate::types::fs_like;\n").is_empty());
    assert!(adapter_violations("crate::fsx::a();\n").is_empty());
    assert!(adapter_violations("crate::hosts::b();\n").is_empty());
    assert!(adapter_violations("// use crate::fs\n").is_empty());
}

#[test]
fn the_port_scan_allows_only_the_tree_port() {
    for source in [
        "use crate::port::FileTree;\n",
        "use crate::port::EntryKind;\n",
        "use crate::port::{EntryKind, FileTree};\n",
    ] {
        assert!(port_violations(source, false).is_empty(), "{source}");
    }
    assert_eq!(
        paths(&port_violations("use crate::port::FakeFileTree;\n", false)),
        [(1, "crate::port::FakeFileTree")]
    );
    assert_eq!(
        paths(&port_violations(
            "use crate::port::{FileTree, Workspace};\n",
            false
        )),
        [(1, "crate::port::Workspace")]
    );
    assert_eq!(
        paths(&port_violations("crate::port::tree::FileTree\n", false)),
        [(1, "crate::port::tree")]
    );
    assert_eq!(
        paths(&port_violations("use crate::{port, types};\n", false)),
        [(1, "crate::port")]
    );
}

#[test]
fn the_port_scan_allows_the_fake_in_test_code_only() {
    let in_tests = "#[cfg(test)]\nmod tests {\n    use crate::port::FakeFileTree;\n    fn f() -> &'static str { \"{\" }\n}\n";
    assert!(port_violations(in_tests, false).is_empty());
    assert_eq!(
        paths(&port_violations("use crate::port::FakeFileTree;\n", false)),
        [(1, "crate::port::FakeFileTree")]
    );
    let after_tests = "#[cfg(test)]\nmod tests {}\nuse crate::port::FakeFileTree;\n";
    assert_eq!(
        paths(&port_violations(after_tests, false)),
        [(3, "crate::port::FakeFileTree")]
    );
    assert!(port_violations("use crate::port::FakeFileTree;\n", true).is_empty());
}

#[test]
fn a_parent_marks_a_module_file_as_test_code() {
    assert!(declares_test_module(
        "#[cfg(test)]\nmod testing;\n",
        "testing"
    ));
    assert!(declares_test_module(
        "#[cfg(test)]\npub(crate) mod testing;\n",
        "testing"
    ));
    assert!(!declares_test_module("mod testing;\n", "testing"));
    assert!(!declares_test_module(
        "#[cfg(test)]\nmod testing;\n",
        "other"
    ));
    assert!(!declares_test_module(
        "// #[cfg(test)]\nmod testing;\n",
        "testing"
    ));
}
