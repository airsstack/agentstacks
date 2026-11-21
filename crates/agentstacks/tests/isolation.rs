//! Guards the rule that `types` reaches the disk, processes and the network only through its ports.
//!
//! Logic under `src/types` takes a `FileTree` instead of calling `std::fs`, so a rule runs the same
//! over the real disk and over an in-memory tree. Nothing else enforces that: a stray `std::fs::read`
//! compiles, passes clippy and passes every test. This test reads every `.rs` file under
//! `src/types` and fails when code, as opposed to a comment, names one of the forbidden paths,
//! whether written whole (`std::fs::read`, `::std::fs::read`), in a single `use`, or in a grouped
//! `use std::{..}` that lists `fs`, `process` or `net` as a member.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The paths `types` must never name in code.
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

/// Every forbidden module that a grouped `use std::{..}` in `stripped` (comments already removed)
/// lists as one of its direct members, with the 1-based line of the member.
///
/// A member is the first identifier after the opening brace or after a comma of the group's own
/// level, so `std::{io, fs::File}` is flagged for `fs` while `std::{io::{self, fs}}` is not: the
/// latter is not `std::fs`.
fn grouped_imports(stripped: &str) -> Vec<(usize, &'static str)> {
    let bytes = stripped.as_bytes();
    let mut found = Vec::new();
    for (start, _) in stripped.match_indices("std") {
        let own_word = start == 0 || !is_ident(bytes[start - 1]);
        let Some(group) = stripped[start + 3..]
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
                let word = &stripped[index..end];
                found.extend(
                    FORBIDDEN_IN_GROUP
                        .iter()
                        .filter(|(module, _)| *module == word)
                        .map(|(_, path)| (line_of(stripped, index), *path)),
                );
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

#[test]
fn types_never_names_the_disk_a_process_or_the_network() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/types");
    let files = rust_files(&root)?;
    assert!(files.len() > 50, "found only {} files", files.len());
    let mut offences = Vec::new();
    for file in &files {
        let source = fs::read_to_string(file)?;
        for (line, forbidden) in violations(&source) {
            offences.push(format!("{}:{line}: {forbidden}", file.display()));
        }
    }
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
