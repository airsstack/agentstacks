//! A tool extension directory, loaded and checked.
//!
//! It is separate from [`ExtensionManifest`] because it is the only unit that reads a directory:
//! it walks the tree, reads the manifest and hashes the whole, where the manifest only parses text.
//!
//! # Responsibilities
//!
//! - [`Extension::load`] turns the directory behind a [`FileTree`] into a checked [`Extension`], or
//!   every refusal it contains.
//! - It walks the directory once under the shared layout rules: dot entries are skipped first,
//!   empty directories are ignored, and symlinks, special files and non-UTF-8 paths are refused,
//!   with the symlink refusal worded for an extension. A walk refusal does not stop the load: it is
//!   reported together with every other refusal.
//! - It reads every walked file exactly once, hashing it as it goes; the manifest's text comes from
//!   that same read. A file that cannot be read is an I/O refusal at its own path, reported with
//!   every other refusal, and leaves the extension without a tree hash.
//! - It requires `extension.toml` (a manifest the walk refused is not reported as absent), parses
//!   it, and checks that `entry`, without its leading `./` segments, is one of the walked regular
//!   files. An `entry` the walk refused is not reported as missing. Every refusal of those checks
//!   is reported together. A manifest that is not UTF-8 is refused as `<file>: not valid UTF-8`.
//! - When nothing is refused it records the directory's tree hash and the dot entries it ignored.
//!
//! # Non-responsibilities
//!
//! What the adapter exports and how it runs belongs to the extension host. Comparing the tree hash
//! with the lock belongs to the plan and apply stage that reconciles a lock with the extensions on
//! disk, not to the lock's own types.

use std::path::{Path, PathBuf};

use crate::types::extension::error::ExtensionError;
use crate::types::extension::manifest::ExtensionManifest;
use crate::types::port::FileTree;
use crate::types::port::utf8_text;
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::DoesNotExist;
use crate::types::report::FormatError;
use crate::types::report::Location;
use crate::types::tree::ContentHash;
use crate::types::tree::WalkError;
use crate::types::tree::hash_walked;
use crate::types::tree::walk;

/// The manifest every extension directory holds at its root.
const MANIFEST_FILE: &str = "extension.toml";

/// A tool extension directory that passed every check.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::Extension;
///
/// let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/extensions/valid/minimal");
/// let extension = Extension::load(&StdFileTree::new(dir))
///     .map_err(|refusals| format!("{refusals:?}"))?;
/// assert_eq!(extension.manifest().entry().as_str(), "adapter.lua");
/// assert!(extension.dir().ends_with("valid/minimal"));
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone)]
pub struct Extension {
    dir: PathBuf,
    manifest: ExtensionManifest,
    tree: ContentHash,
    ignored: Vec<PathBuf>,
}

impl Extension {
    /// Loads the extension in the directory behind `tree`.
    ///
    /// # Errors
    ///
    /// Returns every refusal the directory holds. A refused `api` therefore never yields an
    /// [`Extension`].
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::fs::StdFileTree;
    /// use agentstacks::types::Extension;
    ///
    /// # fn main() -> Result<(), String> {
    /// let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/extensions");
    ///
    /// let extension = Extension::load(&StdFileTree::new(format!("{fixtures}/valid/minimal")))
    ///     .map_err(|refusals| format!("{refusals:?}"))?;
    /// assert_eq!(extension.manifest().name().as_str(), "claude");
    ///
    /// let refused = Extension::load(&StdFileTree::new(format!("{fixtures}/invalid/api-one")))
    ///     .err()
    ///     .ok_or("api 1 is not supported")?;
    /// let shown: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
    /// assert!(shown[0].ends_with("extension.api: 1 not supported (this agentstacks speaks 0)"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn load<T: FileTree>(tree: &T) -> Result<Self, Diagnostics> {
        let (walked, walk_refusals) = walk(tree);
        let mut diagnostics = Diagnostics::new();
        diagnostics.extend(walk_refusals.into_iter().map(reword_walk_diagnostic));
        let mut manifest_bytes = None;
        let hash = hash_walked(tree, &walked, &mut diagnostics, |path, bytes| {
            if path.as_str() == MANIFEST_FILE {
                manifest_bytes = Some(bytes);
            }
        });
        let manifest_path = tree.root().join(MANIFEST_FILE);
        let listed = walked
            .files()
            .iter()
            .any(|path| path.as_str() == MANIFEST_FILE);
        if !listed {
            if !walked.covers(Path::new(MANIFEST_FILE)) {
                diagnostics.push(Diagnostic::new(
                    Location::file(tree.root()),
                    ExtensionError::NotAnExtension,
                ));
            }
            return Err(diagnostics);
        }
        // A listed manifest without bytes failed to read, and that refusal is already reported.
        let Some(bytes) = manifest_bytes else {
            return Err(diagnostics);
        };
        let text = match utf8_text(bytes) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(Diagnostic::new(Location::file(&manifest_path), error));
                return Err(diagnostics);
            }
        };
        let (parsed, entry) = ExtensionManifest::parse_with_entry(&manifest_path, &text);
        let manifest = match parsed {
            Ok(manifest) => Some(manifest),
            Err(refusals) => {
                diagnostics.extend(refusals);
                None
            }
        };
        if let Some(entry) = entry {
            let wanted = entry.normalized();
            let walked_file = walked.files().iter().any(|path| path.as_str() == wanted);
            if !walked_file && !walked.covers(Path::new(wanted)) {
                diagnostics.push(Diagnostic::new(
                    Location::at_key(&manifest_path, "extension.entry"),
                    ExtensionError::from(DoesNotExist::new(entry.as_str())),
                ));
            }
        }
        let (Some(manifest), Some(hash)) = (manifest.filter(|_| diagnostics.is_empty()), hash)
        else {
            return Err(diagnostics);
        };
        Ok(Self {
            dir: tree.root().to_path_buf(),
            manifest,
            tree: hash,
            ignored: walked.ignored().to_vec(),
        })
    }

    /// The directory the extension was loaded from, as the tree names it in messages.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The extension's checked manifest.
    #[must_use]
    pub const fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    /// The hash of the whole directory.
    #[must_use]
    pub const fn tree(&self) -> &ContentHash {
        &self.tree
    }

    /// The dot entries the walk ignored, relative to the directory and sorted by path bytes.
    #[must_use]
    pub fn ignored(&self) -> &[PathBuf] {
        &self.ignored
    }
}

/// Words the walker's symlink refusal for an extension and passes every other diagnostic through.
fn reword_walk_diagnostic(diagnostic: Diagnostic) -> Diagnostic {
    if matches!(diagnostic.error(), FormatError::Walk(WalkError::Symlink)) {
        Diagnostic::new(diagnostic.location().clone(), ExtensionError::Symlink)
    } else {
        diagnostic
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io;

    use super::*;
    use crate::types::port::FakeFileTree;
    use crate::types::tree::tree_hash;

    type TestResult = Result<(), Box<dyn Error>>;

    const ROOT: &str = "extensions/claude";
    const MANIFEST: &str = "extensions/claude/extension.toml";

    const EXAMPLE: &str = "[extension]\n\
        name        = \"claude\"\n\
        version     = \"0.1.0\"\n\
        api         = 0\n\
        description = \"Renders packs into Claude Code project config\"\n\
        entry       = \"adapter.lua\"\n";

    fn minimal() -> FakeFileTree {
        FakeFileTree::new(ROOT)
            .file("extension.toml", EXAMPLE.as_bytes())
            .file("adapter.lua", b"return {}\n")
    }

    fn manifest_with(old: &str, new: &str) -> String {
        EXAMPLE.replace(old, new)
    }

    fn refusals(result: Result<Extension, Diagnostics>) -> Vec<String> {
        result
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.to_string())
            .collect()
    }

    fn sorted(mut shown: Vec<String>) -> Vec<String> {
        shown.sort();
        shown
    }

    #[test]
    fn extension_load_accepts_a_minimal_extension() -> TestResult {
        let extension = Extension::load(&minimal()).map_err(|d| format!("{d:?}"))?;
        assert_eq!(extension.manifest().name().as_str(), "claude");
        assert_eq!(extension.manifest().entry().as_str(), "adapter.lua");
        assert_eq!(extension.dir(), Path::new(ROOT));
        assert!(extension.ignored().is_empty());
        Ok(())
    }

    #[test]
    fn extension_load_refuses_a_directory_without_a_manifest() {
        let tree = FakeFileTree::new(ROOT).file("adapter.lua", b"");
        assert_eq!(
            refusals(Extension::load(&tree)),
            ["extensions/claude: not an extension (no extension.toml)"]
        );
    }

    #[test]
    fn extension_load_refuses_a_missing_entry() {
        let tree = FakeFileTree::new(ROOT).file("extension.toml", EXAMPLE.as_bytes());
        assert_eq!(
            refusals(Extension::load(&tree)),
            [format!(
                "{MANIFEST}: extension.entry: \"adapter.lua\" does not exist"
            )]
        );
    }

    #[test]
    fn extension_load_does_not_find_an_entry_inside_a_dot_directory() {
        let manifest = manifest_with("adapter.lua", ".hidden/adapter.lua");
        let tree = FakeFileTree::new(ROOT)
            .file("extension.toml", manifest.as_bytes())
            .file(".hidden/adapter.lua", b"");
        assert_eq!(
            refusals(Extension::load(&tree)),
            [format!(
                "{MANIFEST}: extension.entry: \".hidden/adapter.lua\" does not exist"
            )]
        );
    }

    #[test]
    fn extension_load_does_not_take_a_directory_for_an_entry() {
        let manifest = manifest_with("adapter.lua", "lib.lua");
        let tree = FakeFileTree::new(ROOT)
            .file("extension.toml", manifest.as_bytes())
            .file("lib.lua/inner.txt", b"");
        assert_eq!(
            refusals(Extension::load(&tree)),
            [format!(
                "{MANIFEST}: extension.entry: \"lib.lua\" does not exist"
            )]
        );
    }

    #[test]
    fn extension_load_passes_walk_refusals_through() {
        assert_eq!(
            refusals(Extension::load(&minimal().symlink("link"))),
            ["extensions/claude/link: symlinks are not allowed in an extension"]
        );
        assert_eq!(
            refusals(Extension::load(&minimal().special("sock"))),
            ["extensions/claude/sock: only regular files and directories are allowed"]
        );
    }

    #[test]
    fn extension_load_does_not_call_a_manifest_the_walk_refused_absent() {
        let tree = FakeFileTree::new(ROOT)
            .symlink("extension.toml")
            .file("adapter.lua", b"");
        assert_eq!(
            refusals(Extension::load(&tree)),
            ["extensions/claude/extension.toml: symlinks are not allowed in an extension"]
        );
    }

    #[test]
    fn extension_load_reports_a_walk_refusal_with_the_manifest_refusals() {
        let manifest = manifest_with("\"claude\"", "\"Claude\"");
        let tree = FakeFileTree::new(ROOT)
            .file("extension.toml", manifest.as_bytes())
            .file("adapter.lua", b"")
            .symlink("link")
            .special("sock");
        assert_eq!(
            refusals(Extension::load(&tree)),
            [
                format!("{MANIFEST}: extension.name: \"Claude\" is not a valid name"),
                "extensions/claude/link: symlinks are not allowed in an extension".to_owned(),
                "extensions/claude/sock: only regular files and directories are allowed".to_owned(),
            ]
        );
    }

    #[test]
    fn extension_load_reports_a_walk_refusal_with_a_missing_entry() {
        let tree = FakeFileTree::new(ROOT)
            .file("extension.toml", EXAMPLE.as_bytes())
            .symlink("link");
        assert_eq!(
            refusals(Extension::load(&tree)),
            [
                format!("{MANIFEST}: extension.entry: \"adapter.lua\" does not exist"),
                "extensions/claude/link: symlinks are not allowed in an extension".to_owned(),
            ]
        );
    }

    #[test]
    fn extension_load_does_not_call_an_entry_the_walk_refused_missing() {
        let tree = FakeFileTree::new(ROOT)
            .file("extension.toml", EXAMPLE.as_bytes())
            .symlink("adapter.lua");
        assert_eq!(
            refusals(Extension::load(&tree)),
            ["extensions/claude/adapter.lua: symlinks are not allowed in an extension"]
        );
    }

    #[test]
    fn extension_load_finds_an_entry_written_with_a_leading_current_directory() -> TestResult {
        for written in ["./adapter.lua", "././adapter.lua", ".//adapter.lua"] {
            let manifest = manifest_with("adapter.lua", written);
            let tree = minimal().file("extension.toml", manifest.as_bytes());
            let extension = Extension::load(&tree).map_err(|d| format!("{d:?}"))?;
            assert_eq!(extension.manifest().entry().as_str(), written);
            assert_eq!(extension.manifest().entry().normalized(), "adapter.lua");
        }
        Ok(())
    }

    #[test]
    fn extension_load_names_a_missing_entry_as_written() {
        let manifest = manifest_with("adapter.lua", "./lib/adapter.lua");
        let tree = FakeFileTree::new(ROOT)
            .file("extension.toml", manifest.as_bytes())
            .file("adapter.lua", b"");
        assert_eq!(
            refusals(Extension::load(&tree)),
            [format!(
                "{MANIFEST}: extension.entry: \"./lib/adapter.lua\" does not exist"
            )]
        );
    }

    #[cfg(unix)]
    #[test]
    fn extension_load_passes_a_non_utf8_path_refusal_through() -> TestResult {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let name = OsStr::from_bytes(&[0x66, 0xFF]);
        let tree = minimal().file(name, b"");
        let shown = refusals(Extension::load(&tree));
        assert_eq!(shown.len(), 1);
        let diagnostic = shown.first().ok_or("one refusal")?;
        assert!(
            diagnostic.starts_with("extensions/claude/f"),
            "{diagnostic}"
        );
        assert!(
            diagnostic.ends_with(": path is not valid UTF-8"),
            "{diagnostic}"
        );
        Ok(())
    }

    #[test]
    fn extension_load_ignores_dot_entries_and_lists_them() -> TestResult {
        let plain = Extension::load(&minimal()).map_err(|d| format!("{d:?}"))?;
        let tree = minimal().file(".DS_Store", b"x").file(".git/config", b"y");
        let extension = Extension::load(&tree).map_err(|d| format!("{d:?}"))?;
        assert_eq!(
            extension.ignored(),
            [PathBuf::from(".DS_Store"), PathBuf::from(".git")]
        );
        assert_eq!(extension.tree(), plain.tree());
        Ok(())
    }

    #[test]
    fn extension_load_ignores_empty_directories() -> TestResult {
        let plain = Extension::load(&minimal()).map_err(|d| format!("{d:?}"))?;
        let extension = Extension::load(&minimal().dir("lib")).map_err(|d| format!("{d:?}"))?;
        assert_eq!(extension.tree(), plain.tree());
        Ok(())
    }

    #[test]
    fn extension_load_records_the_tree_hash() -> TestResult {
        let tree = minimal();
        let extension = Extension::load(&tree).map_err(|d| format!("{d:?}"))?;
        let expected = tree_hash(&tree).map_err(|d| format!("{d:?}"))?;
        assert_eq!(extension.tree(), &expected);

        let changed = minimal().file("adapter.lua", b"return {} \n");
        let other = Extension::load(&changed).map_err(|d| format!("{d:?}"))?;
        assert_ne!(other.tree(), extension.tree());
        Ok(())
    }

    #[test]
    fn extension_load_refuses_api_one_without_returning_an_extension() {
        let tree = FakeFileTree::new(ROOT)
            .file(
                "extension.toml",
                manifest_with("api         = 0", "api         = 1").as_bytes(),
            )
            .file("adapter.lua", b"");
        assert_eq!(
            refusals(Extension::load(&tree)),
            [format!(
                "{MANIFEST}: extension.api: 1 not supported (this agentstacks speaks 0)"
            )]
        );
    }

    #[test]
    fn extension_load_reports_an_unreadable_manifest_as_io() -> TestResult {
        let unreadable = minimal().unreadable("extension.toml");
        let not_utf8 = FakeFileTree::new(ROOT)
            .file("extension.toml", &[0xFF, 0xFE])
            .file("adapter.lua", b"");
        for (tree, message) in [
            (unreadable, FakeFileTree::denied().to_string()),
            (not_utf8, "not valid UTF-8".to_owned()),
        ] {
            let diagnostics = Extension::load(&tree)
                .err()
                .ok_or("expected a refusal")?
                .into_sorted();
            assert_eq!(diagnostics.len(), 1);
            let diagnostic = diagnostics.first().ok_or("one refusal")?;
            assert_eq!(diagnostic.to_string(), format!("{MANIFEST}: {message}"));
            assert!(matches!(diagnostic.error(), FormatError::Io(_)));
        }
        Ok(())
    }

    #[test]
    fn extension_load_reports_an_unreadable_file_while_hashing() {
        let tree = minimal().unreadable("adapter.lua");
        assert_eq!(
            refusals(Extension::load(&tree)),
            [format!("{ROOT}/adapter.lua: {}", FakeFileTree::denied())]
        );
    }

    #[test]
    fn extension_load_reports_an_unreadable_file_with_the_manifest_refusals() {
        let manifest = manifest_with("\"claude\"", "\"Claude\"");
        let tree = FakeFileTree::new(ROOT)
            .file("extension.toml", manifest.as_bytes())
            .unreadable("adapter.lua");
        assert_eq!(
            refusals(Extension::load(&tree)),
            [
                format!("{ROOT}/adapter.lua: {}", FakeFileTree::denied()),
                format!("{MANIFEST}: extension.name: \"Claude\" is not a valid name"),
            ]
        );
    }

    #[test]
    fn extension_load_reports_an_unreadable_file_without_a_manifest() {
        let tree = FakeFileTree::new(ROOT).unreadable("adapter.lua");
        assert_eq!(
            refusals(Extension::load(&tree)),
            [
                format!("{ROOT}: not an extension (no extension.toml)"),
                format!("{ROOT}/adapter.lua: {}", FakeFileTree::denied()),
            ]
        );
    }

    #[test]
    fn extension_load_reports_every_refusal_together() {
        let manifest = manifest_with("\"claude\"", "\"Claude\"") + "foo = 1\n";
        let tree = FakeFileTree::new(ROOT).file("extension.toml", manifest.as_bytes());
        assert_eq!(
            sorted(refusals(Extension::load(&tree))),
            sorted(vec![
                format!("{MANIFEST}: extension.name: \"Claude\" is not a valid name"),
                format!("{MANIFEST}: extension.foo: unknown key"),
                format!("{MANIFEST}: extension.entry: \"adapter.lua\" does not exist"),
            ])
        );
    }

    #[test]
    fn reword_walk_diagnostic_rewords_only_the_symlink_refusal() {
        let at = || Location::file("extensions/claude/x");
        let reworded = reword_walk_diagnostic(Diagnostic::new(at(), WalkError::Symlink));
        assert_eq!(
            reworded.to_string(),
            "extensions/claude/x: symlinks are not allowed in an extension"
        );
        for error in [WalkError::SpecialFile, WalkError::NotUtf8] {
            let kept = reword_walk_diagnostic(Diagnostic::new(at(), error));
            assert_eq!(kept.to_string(), format!("extensions/claude/x: {error}"));
        }
        let io = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let kept = reword_walk_diagnostic(Diagnostic::new(at(), io));
        assert_eq!(kept.to_string(), "extensions/claude/x: denied");
    }
}
