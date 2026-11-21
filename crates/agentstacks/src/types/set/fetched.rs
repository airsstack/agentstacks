//! The rule that a project key equals the name inside its source.
//!
//! It is its own file because it needs the project file's keys and the loaded names together, which
//! neither the pack loader nor the extension loader has.
//!
//! # Responsibilities
//!
//! - `check_fetched_names` refuses each loaded pack whose `pack.name` differs from its
//!   `[packs.<key>]` key, and each loaded extension whose `extension.name` differs from its
//!   `[tools.<key>]` key. The refusal is located at that key in the project file. An entry that
//!   failed to load is not checked.
//!
//! # Non-responsibilities
//!
//! Loading the entries belongs to [`Pack::load`](crate::types::Pack::load) and
//! [`Extension::load`](crate::types::Extension::load).

use std::collections::BTreeMap;
use std::path::Path;

use crate::types::extension::Extension;
use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::pack::KeyPath;
use crate::types::pack::Pack;
use crate::types::report::Diagnostic;
use crate::types::report::Location;
use crate::types::set::error::SetError;

/// Refuses each loaded pack and extension whose own name differs from its key in the project file
/// at `project_file`.
pub(super) fn check_fetched_names(
    project_file: &Path,
    packs: &BTreeMap<PackName, Pack>,
    extensions: &BTreeMap<ToolName, Extension>,
) -> Vec<Diagnostic> {
    let mut refusals = Vec::new();
    for (key, pack) in packs {
        let name = pack.manifest().name();
        if name != key {
            refusals.push(Diagnostic::new(
                Location::set_level(
                    project_file,
                    Some(KeyPath::new("packs").key(key.as_str()).into()),
                ),
                SetError::SourcePack { name: name.clone() },
            ));
        }
    }
    for (key, extension) in extensions {
        let name = extension.manifest().name();
        if name != key {
            refusals.push(Diagnostic::new(
                Location::set_level(
                    project_file,
                    Some(KeyPath::new("tools").key(key.as_str()).into()),
                ),
                SetError::SourceExtension { name: name.clone() },
            ));
        }
    }
    refusals
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::set::testing::{PROJECT_FILE, PackSpec, extension, loaded, name, shown};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    fn check(
        packs: &BTreeMap<PackName, Pack>,
        extensions: &BTreeMap<ToolName, Extension>,
    ) -> Vec<String> {
        shown(&check_fetched_names(
            Path::new(PROJECT_FILE),
            packs,
            extensions,
        ))
    }

    #[test]
    fn pack_key_must_equal_fetched_name() -> TestResult {
        let wrong = BTreeMap::from([(name("core")?, PackSpec::new("kore").load()?)]);
        assert_eq!(
            check(&wrong, &BTreeMap::new()),
            ["agentstacks.toml: packs.core: source contains pack \"kore\""]
        );
        let right = loaded(&[PackSpec::new("core")])?;
        assert!(check(&right, &BTreeMap::new()).is_empty());
        Ok(())
    }

    #[test]
    fn tool_key_must_equal_fetched_name() -> TestResult {
        let wrong = BTreeMap::from([(ToolName::new("claude")?, extension("claud")?)]);
        assert_eq!(
            check(&BTreeMap::new(), &wrong),
            ["agentstacks.toml: tools.claude: source contains extension \"claud\""]
        );
        let right = BTreeMap::from([(ToolName::new("claude")?, extension("claude")?)]);
        assert!(check(&BTreeMap::new(), &right).is_empty());
        Ok(())
    }
}
