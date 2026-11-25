//! Every `.lua` file of an extension, held in memory.
//!
//! # Responsibilities
//!
//! - [`LuaFiles::read`] walks the extension directory once and reads each `.lua` file through the
//!   file port, so Lua never touches the disk. A file that is not UTF-8, or cannot be read, is
//!   refused at `<tool>/<file>`, and every such file is reported.
//! - [`LuaModule`] is a dotted module name, `lib.util` for `lib/util.lua`. It is built only from
//!   a name of letters, digits, `_` and `-` joined by single dots.
//! - [`LuaFiles::entry`], [`LuaFiles::module`] and [`LuaFiles::entry_module`] serve the entry
//!   file and the modules a script requires, by name.
//!
//! # Non-responsibilities
//!
//! Running a file, caching a loaded module and refusing a bad `require` belong to the other
//! host files.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use crate::port::FileTree;
use crate::types::extension::Extension;
use crate::types::name::ToolName;
use crate::types::report::{Diagnostic, Diagnostics, Location};
use crate::types::tree::{TreePath, utf8_text, walk};

/// The suffix of a Lua file, which a module name leaves out.
const SUFFIX: &str = ".lua";

/// A module name as `require` takes it: segments of ASCII letters, digits, `_` and `-` joined by
/// single dots.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct LuaModule(String);

impl LuaModule {
    /// The module named `name`, or `None` when `name` is not letters, digits, `_` and `-` joined
    /// by single dots.
    pub(super) fn new(name: &str) -> Option<Self> {
        let segment = |part: &str| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        };
        name.split('.').all(segment).then(|| Self(name.to_owned()))
    }

    /// The name as written, dots included.
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }

    /// The file the module lives in: dots become `/` and `.lua` follows.
    fn file(&self) -> TreePath {
        TreePath::from_parts(&[&format!("{}{SUFFIX}", self.0.replace('.', "/"))])
    }

    /// The module a Lua file at `path` is, or `None` when its name is not a module name.
    fn of_file(path: &TreePath) -> Option<Self> {
        let stem = path.as_str().strip_suffix(SUFFIX)?;
        Self::new(&stem.replace('/', "."))
    }
}

/// The `.lua` files of one extension, with their text.
#[derive(Debug)]
pub(super) struct LuaFiles {
    entry: TreePath,
    files: BTreeMap<TreePath, String>,
}

impl LuaFiles {
    /// Reads every `.lua` file under `tree`, the directory `extension` was loaded from.
    ///
    /// # Errors
    ///
    /// A refusal at `<tool>/<file>` for each file that is not UTF-8 or cannot be read, and one
    /// for an entry file the tree does not hold.
    pub(super) fn read<T: FileTree>(tree: &T, extension: &Extension) -> Result<Self, Diagnostics> {
        let tool = extension.manifest().name();
        // The extension was loaded from this tree, so its walk already passed.
        let (walked, _) = walk(tree);
        let mut files = BTreeMap::new();
        let mut refused = Diagnostics::new();
        for path in walked
            .files()
            .iter()
            .filter(|path| path.has_extension("lua"))
        {
            let read = tree.read_file(Path::new(path.as_str())).and_then(utf8_text);
            match read {
                Ok(text) => {
                    files.insert(path.clone(), text);
                }
                Err(error) => refused.push(Diagnostic::new(at(tool, path), error)),
            }
        }
        let entry = TreePath::from_parts(&[extension.manifest().entry().normalized()]);
        if refused.is_empty() && !files.contains_key(&entry) {
            let missing = io::Error::new(io::ErrorKind::NotFound, "no such file or directory");
            refused.push(Diagnostic::new(at(tool, &entry), missing));
        }
        refused.into_result(Self { entry, files })
    }

    /// The entry file's path and text.
    pub(super) fn entry(&self) -> (&TreePath, &str) {
        let text = self.files.get(&self.entry).map_or("", String::as_str);
        (&self.entry, text)
    }

    /// The module the entry file is, `adapter` for `adapter.lua` and `src.main` for
    /// `src/main.lua`, or `None` when its path is not a module name.
    pub(super) fn entry_module(&self) -> Option<LuaModule> {
        LuaModule::of_file(&self.entry)
    }

    /// The path and text of the file `name` lives in, or `None` when the extension has none.
    pub(super) fn module(&self, name: &LuaModule) -> Option<(&TreePath, &str)> {
        let (path, text) = self.files.get_key_value(&name.file())?;
        Some((path, text.as_str()))
    }
}

/// Where a refusal about the extension file `path` of `tool` is reported: `<tool>/<path>`.
fn at(tool: &ToolName, path: &TreePath) -> Location {
    Location::file(format!("{tool}/{path}"))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::host::testing::{load, tree};
    use crate::port::FakeFileTree;

    type TestResult = Result<(), Box<dyn Error>>;

    fn read(tree: &FakeFileTree) -> Result<LuaFiles, Box<dyn Error>> {
        let extension = load(tree)?;
        LuaFiles::read(tree, &extension).map_err(|refusals| {
            let shown: Vec<String> = refusals.into_iter().map(|d| d.to_string()).collect();
            shown.join("; ").into()
        })
    }

    fn module(name: &str) -> Result<LuaModule, Box<dyn Error>> {
        Ok(LuaModule::new(name).ok_or("a valid module name")?)
    }

    #[test]
    fn reads_every_lua_file_of_the_extension() -> TestResult {
        let tree = tree(
            &[],
            &[
                ("adapter.lua", b"return {}"),
                ("lib/util.lua", b"return 1"),
                ("README.md", b"# docs"),
            ],
        );
        let files = read(&tree)?;
        let found: Vec<(&str, &str)> = files
            .files
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect();
        assert_eq!(
            found,
            [("adapter.lua", "return {}"), ("lib/util.lua", "return 1")]
        );
        assert_eq!(
            files.entry(),
            (&TreePath::from_parts(&["adapter.lua"]), "return {}")
        );
        Ok(())
    }

    #[test]
    fn maps_a_dotted_module_name_to_its_file() -> TestResult {
        let files = read(&tree(
            &[],
            &[("adapter.lua", b""), ("lib/util.lua", b"return 7")],
        ))?;
        let found = files.module(&module("lib.util")?);
        assert_eq!(
            found.map(|(path, text)| (path.as_str(), text)),
            Some(("lib/util.lua", "return 7"))
        );
        assert!(files.module(&module("lib.nope")?).is_none());
        Ok(())
    }

    #[test]
    fn a_module_name_is_segments_joined_by_single_dots() {
        for good in ["a", "lib.util", "a-b_c.d9", "A.B"] {
            assert!(LuaModule::new(good).is_some(), "{good}");
        }
        for bad in ["", ".", "a.", ".a", "a..b", "lib/x", "x y", "caf\u{e9}"] {
            assert!(LuaModule::new(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn entry_module_drops_the_extension_and_uses_dots() -> TestResult {
        let adapter = read(&tree(&[], &[("adapter.lua", b"")]))?;
        assert_eq!(
            adapter.entry_module().as_ref().map(LuaModule::as_str),
            Some("adapter")
        );

        let manifest = "[extension]\nname = \"claude\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\nentry = \"src/main.lua\"\n";
        let nested = FakeFileTree::new("ext")
            .file("extension.toml", manifest.as_bytes())
            .file("src/main.lua", b"");
        let files = read(&nested)?;
        assert_eq!(
            files.entry_module().as_ref().map(LuaModule::as_str),
            Some("src.main")
        );
        Ok(())
    }

    #[test]
    fn non_utf8_lua_file_is_refused() -> TestResult {
        let tree = tree(&[], &[("adapter.lua", b""), ("lib/bad.lua", &[0xff])]);
        let extension = load(&tree)?;
        let refused = LuaFiles::read(&tree, &extension)
            .err()
            .ok_or("a refusal")?
            .into_sorted();
        assert_eq!(
            refused.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["claude/lib/bad.lua: not valid UTF-8"]
        );
        Ok(())
    }

    #[test]
    fn every_bad_file_is_reported() -> TestResult {
        let tree = tree(
            &[],
            &[("adapter.lua", b""), ("a.lua", &[0xff]), ("b.lua", &[0xfe])],
        );
        let extension = load(&tree)?;
        let refused = LuaFiles::read(&tree, &extension)
            .err()
            .ok_or("a refusal")?
            .into_sorted();
        assert_eq!(
            refused.iter().map(ToString::to_string).collect::<Vec<_>>(),
            [
                "claude/a.lua: not valid UTF-8",
                "claude/b.lua: not valid UTF-8"
            ]
        );
        Ok(())
    }

    #[test]
    fn unreadable_lua_file_is_an_io_refusal() -> TestResult {
        let extension = load(&tree(&[], &[("adapter.lua", b"")]))?;
        let failing = tree(&[], &[("adapter.lua", b"")]).unreadable("lib/x.lua");
        let refused = LuaFiles::read(&failing, &extension)
            .err()
            .ok_or("a refusal")?
            .into_sorted();
        assert_eq!(
            refused.iter().map(ToString::to_string).collect::<Vec<_>>(),
            [format!("claude/lib/x.lua: {}", FakeFileTree::denied())]
        );
        Ok(())
    }

    #[test]
    fn an_entry_the_tree_lacks_is_refused() -> TestResult {
        let extension = load(&tree(&[], &[("adapter.lua", b"")]))?;
        let other = FakeFileTree::new("ext").file("lib/x.lua", b"");
        let refused = LuaFiles::read(&other, &extension)
            .err()
            .ok_or("a refusal")?
            .into_sorted();
        assert_eq!(
            refused.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["claude/adapter.lua: no such file or directory"]
        );
        Ok(())
    }
}
