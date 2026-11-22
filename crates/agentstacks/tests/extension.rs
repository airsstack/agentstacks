//! Loads the extension fixtures through the public API over the real disk.

use std::error::Error;
use std::path::PathBuf;

use agentstacks::fs::StdFileTree;
use agentstacks::types::{Diagnostics, Extension, SubPath, tree_hash};

type TestResult = Result<(), Box<dyn Error>>;

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/extensions")
        .join(relative)
}

fn refusals(result: Result<Extension, Diagnostics>) -> Vec<String> {
    result
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|diagnostic| diagnostic.to_string())
        .collect()
}

#[test]
fn minimal_extension_fixture_loads() -> TestResult {
    let tree = StdFileTree::new(fixture("valid/minimal"));
    let extension = Extension::load(&tree).map_err(|d| format!("{d:?}"))?;
    assert_eq!(extension.manifest().name().as_str(), "claude");
    assert_eq!(extension.manifest().entry().as_str(), "adapter.lua");
    let expected = tree_hash(&tree).map_err(|d| format!("{d:?}"))?;
    assert_eq!(extension.tree(), &expected);
    Ok(())
}

#[test]
fn entry_written_with_a_leading_current_directory_loads() -> TestResult {
    let tree = StdFileTree::new(fixture("valid/entry-current-dir"));
    let extension = Extension::load(&tree).map_err(|d| format!("{d:?}"))?;
    assert_eq!(extension.manifest().entry().as_str(), "./adapter.lua");
    assert_eq!(extension.manifest().entry().normalized(), "adapter.lua");
    Ok(())
}

#[test]
fn extension_fixture_with_reads_loads() -> TestResult {
    let tree = StdFileTree::new(fixture("valid/reads"));
    let extension = Extension::load(&tree).map_err(|d| format!("{d:?}"))?;
    let normalized: Vec<String> = extension.reads().iter().map(SubPath::normalized).collect();
    assert_eq!(normalized, [".claude/settings.json", "opencode.json"]);
    Ok(())
}

#[test]
fn each_invalid_extension_fixture_reports_its_refusal() {
    // (fixture, whether the refusal is located at the directory rather than the manifest, rest)
    let cases = [
        (
            "no-manifest",
            true,
            ": not an extension (no extension.toml)",
        ),
        ("missing-name", false, ": extension.name: required"),
        (
            "bad-name",
            false,
            ": extension.name: \"Claude\" is not a valid name",
        ),
        (
            "bad-version",
            false,
            ": extension.version: \"1.2\" is not semver",
        ),
        ("empty-description", false, ": extension.description: empty"),
        (
            "api-one",
            false,
            ": extension.api: 1 not supported (this agentstacks speaks 0)",
        ),
        (
            "entry-outside",
            false,
            ": extension.entry: \"../a.lua\" must stay inside the extension",
        ),
        (
            "entry-absolute",
            false,
            ": extension.entry: \"/a.lua\" must stay inside the extension",
        ),
        (
            "entry-not-lua",
            false,
            ": extension.entry: \"adapter.js\" is not a .lua file",
        ),
        (
            "entry-missing",
            false,
            ": extension.entry: \"adapter.lua\" does not exist",
        ),
        (
            "reads-absolute",
            false,
            ": extension.reads[0]: \"/x\" must be relative",
        ),
        ("unknown-key", false, ": extension.foo: unknown key"),
        ("syntax-error", false, ":3: unclosed array, expected `]`"),
        ("manifest-not-utf8", false, ": not valid UTF-8"),
    ];
    for (name, at_directory, rest) in cases {
        let dir = fixture(&format!("invalid/{name}"));
        let located = if at_directory {
            dir.clone()
        } else {
            dir.join("extension.toml")
        };
        let expected = format!("{}{rest}", located.display());
        assert_eq!(
            refusals(Extension::load(&StdFileTree::new(dir))),
            [expected],
            "{name}"
        );
    }
}
