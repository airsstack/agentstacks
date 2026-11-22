//! The binaries a run needs on `PATH`, and the refusal when one is missing.
//!
//! # Responsibilities
//!
//! - [`collect_binaries`] lists what to look up: each pack's `requires.binaries` in pack name
//!   order, then each tool's `require_binary` calls in tool name order, a name once per tool.
//!   A binary a pack and an extension both need is listed for each, so each is reported.
//! - [`BinaryRequirement::not_found`] is the refusal for one that is not found, located at the
//!   pack's entry in the project file or at the extension's call.
//!
//! # Non-responsibilities
//!
//! Looking a name up on `PATH` belongs to the binary port.

use std::path::Path;

use crate::types::name::{BinaryName, PackName, ToolName};
use crate::types::op::{OpError, Origin, Proposals};
use crate::types::pack::Pack;
use crate::types::report::{Diagnostic, Location};

/// A binary some part of the run needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryRequirement {
    /// Required by a pack's `requires.binaries`.
    Pack {
        /// The pack.
        pack: PackName,
        /// The binary.
        name: BinaryName,
    },
    /// Required by an extension's `require_binary` call.
    Extension {
        /// The call.
        origin: Origin,
        /// The binary.
        name: BinaryName,
    },
}

impl BinaryRequirement {
    /// The binary to look up.
    #[must_use]
    pub const fn name(&self) -> &BinaryName {
        match self {
            Self::Pack { name, .. } | Self::Extension { name, .. } => name,
        }
    }

    /// The refusal for a binary that is not on `PATH`. A pack's is located at its entry in
    /// `project_file`, an extension's at its call.
    #[must_use]
    pub fn not_found(&self, project_file: &Path) -> Diagnostic {
        match self {
            Self::Pack { pack, name } => Diagnostic::new(
                Location::set_level(project_file, Some(format!("packs.{pack}"))),
                OpError::PackBinaryNotFound { name: name.clone() },
            ),
            Self::Extension { origin, name } => Diagnostic::new(
                origin
                    .location()
                    .unwrap_or_else(|| Location::set_level(project_file, None)),
                OpError::BinaryNotFound { name: name.clone() },
            ),
        }
    }
}

/// What to look up on `PATH`: the binaries of `packs`, then those the extensions of `tools`
/// proposed in `proposals`.
#[must_use]
pub fn collect_binaries(
    packs: &[&Pack],
    proposals: &Proposals,
    tools: &[ToolName],
) -> Vec<BinaryRequirement> {
    let mut ordered: Vec<&Pack> = packs.to_vec();
    ordered.sort_by(|a, b| a.manifest().name().cmp(b.manifest().name()));
    let mut found: Vec<BinaryRequirement> = ordered
        .iter()
        .flat_map(|pack| {
            pack.manifest()
                .requires()
                .binaries()
                .iter()
                .map(|name| BinaryRequirement::Pack {
                    pack: pack.manifest().name().clone(),
                    name: name.clone(),
                })
        })
        .collect();
    let mut tools: Vec<&ToolName> = tools.iter().collect();
    tools.sort();
    for tool in tools {
        found.extend(proposals.binaries(tool).into_iter().map(|(name, origin)| {
            BinaryRequirement::Extension {
                origin: origin.clone(),
                name: name.clone(),
            }
        }));
    }
    found
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::path::Path;

    use super::*;
    use crate::types::op::testing::{doc, load};
    use crate::types::op::{Operation, Proposal};
    use crate::types::tree::TreePath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn pack_requiring(name: &str, binaries: &[&str]) -> Result<Pack, Box<dyn std::error::Error>> {
        let listed: Vec<String> = binaries.iter().map(|b| format!("\"{b}\"")).collect();
        let tables = format!("[requires]\nbinaries = [{}]\n", listed.join(", "));
        load(name, &tables, &[("skills/s/SKILL.md", &doc("s", "x"))])
    }

    fn at(tool: &str, line: usize) -> Result<Origin, Box<dyn std::error::Error>> {
        Ok(Origin::Extension {
            tool: ToolName::new(tool)?,
            file: TreePath::from_parts(&["adapter.lua"]),
            line: NonZeroUsize::new(line).ok_or("a line starts at 1")?,
        })
    }

    fn require(
        proposals: &mut Proposals,
        tool: &str,
        name: &str,
        line: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        proposals.propose(Proposal {
            op: Operation::require_binary(name)?,
            origin: at(tool, line)?,
        })?;
        Ok(())
    }

    fn shown(requirement: &BinaryRequirement) -> String {
        match requirement {
            BinaryRequirement::Pack { pack, name } => format!("{pack}/{name}"),
            BinaryRequirement::Extension { origin, name } => format!(
                "{}:{name}@{}",
                origin.tool().map(ToString::to_string).unwrap_or_default(),
                origin
                    .site()
                    .map(|site| site.to_string())
                    .unwrap_or_default()
            ),
        }
    }

    fn collected(
        packs: &[&Pack],
        proposals: &Proposals,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let tools = [ToolName::new("opencode")?, ToolName::new("claude")?];
        Ok(collect_binaries(packs, proposals, &tools)
            .iter()
            .map(shown)
            .collect())
    }

    #[test]
    fn pack_binaries_come_first_in_pack_order() -> TestResult {
        let sdlc = pack_requiring("sdlc", &["rg"])?;
        let core = pack_requiring("core", &["git", "rg"])?;
        let mut proposals = Proposals::new();
        require(&mut proposals, "claude", "node", 7)?;
        assert_eq!(
            collected(&[&sdlc, &core], &proposals)?,
            [
                "core/git",
                "core/rg",
                "sdlc/rg",
                "claude:node@adapter.lua:7"
            ]
        );
        Ok(())
    }

    #[test]
    fn extension_binaries_once_per_tool() -> TestResult {
        let mut proposals = Proposals::new();
        require(&mut proposals, "claude", "node", 3)?;
        require(&mut proposals, "claude", "node", 9)?;
        require(&mut proposals, "opencode", "node", 2)?;
        assert_eq!(
            collected(&[], &proposals)?,
            ["claude:node@adapter.lua:3", "opencode:node@adapter.lua:2"]
        );
        Ok(())
    }

    #[test]
    fn pack_and_extension_requirement_both_listed() -> TestResult {
        let core = pack_requiring("core", &["rg"])?;
        let mut proposals = Proposals::new();
        require(&mut proposals, "claude", "rg", 4)?;
        assert_eq!(
            collected(&[&core], &proposals)?,
            ["core/rg", "claude:rg@adapter.lua:4"]
        );
        Ok(())
    }

    #[test]
    fn pack_binary_not_found_text() -> TestResult {
        let requirement = BinaryRequirement::Pack {
            pack: PackName::new("core")?,
            name: BinaryName::new("rg")?,
        };
        assert_eq!(
            requirement
                .not_found(Path::new("agentstacks.toml"))
                .to_string(),
            "agentstacks.toml: packs.core: binary \"rg\" not found on PATH"
        );
        Ok(())
    }

    #[test]
    fn extension_binary_not_found_text() -> TestResult {
        let requirement = BinaryRequirement::Extension {
            origin: at("claude", 7)?,
            name: BinaryName::new("node")?,
        };
        assert_eq!(
            requirement
                .not_found(Path::new("agentstacks.toml"))
                .to_string(),
            "claude/adapter.lua:7: require_binary: \"node\" not found on PATH"
        );
        Ok(())
    }
}
