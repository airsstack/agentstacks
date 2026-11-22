//! The check at the end of a tool's plan that every item got an operation.
//!
//! # Responsibilities
//!
//! - [`check_coverage`] refuses each item of the plan payload that the tool neither copied,
//!   covered with a write, nor skipped. A skip of a skill covers all its files. Only this tool's
//!   proposals count, and every uncovered item is reported, in id order.
//!
//! # Non-responsibilities
//!
//! Items the core already skipped are not in the payload, so they are not asked for here.

use std::collections::HashSet;
use std::path::Path;

use crate::types::name::ToolName;
use crate::types::op::payload::PlanInput;
use crate::types::op::{OpError, Proposals};
use crate::types::report::{Diagnostic, Diagnostics, Location};

/// The refusals of the items of `payload` that `tool`'s proposals leave without an operation,
/// each located at the extension entry `entry`.
#[must_use]
pub fn check_coverage(
    tool: &ToolName,
    entry: &Path,
    payload: &PlanInput,
    proposals: &Proposals,
) -> Diagnostics {
    let accounted: HashSet<_> = proposals
        .for_tool(tool)
        .flat_map(|proposal| proposal.op.covered())
        .collect();
    let mut found = Diagnostics::new();
    for item in payload.items() {
        let by_skill = item.group().is_some_and(|skill| accounted.contains(&skill));
        if accounted.contains(&item) || by_skill {
            continue;
        }
        found.push(Diagnostic::new(
            Location::file(entry),
            OpError::Uncovered {
                id: item.to_string(),
            },
        ));
    }
    found
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::Path;

    use super::*;
    use crate::types::name::{ItemName, PackName};
    use crate::types::op::ItemId;
    use crate::types::op::payload::PlanFile;
    use crate::types::op::testing::{cover, plan_input, rel, skip_all};

    type TestResult = Result<(), Box<dyn Error>>;

    const ENTRY: &str = "claude/adapter.lua";
    const ITEMS: [&str; 7] = [
        "core/agent/coder",
        "core/hook/0",
        "core/hook/1",
        "core/native-hook/0",
        "core/native/claude/x.md",
        "core/skill/status/SKILL.md",
        "core/skill/status/logo.png",
    ];

    /// The payload with one more file in the skill: `references/x.md`.
    fn input_with_reference() -> Result<PlanInput, Box<dyn Error>> {
        let mut input = plan_input()?;
        let id = ItemId::skill_file(
            &PackName::new("core")?,
            &ItemName::new("status")?,
            &rel(&["references", "x.md"]),
        );
        let skill = input
            .packs
            .first_mut()
            .and_then(|pack| pack.skills.first_mut())
            .ok_or("a skill")?;
        skill.files.push(PlanFile {
            id,
            rel: rel(&["references", "x.md"]),
            text: Some("x".to_owned()),
        });
        Ok(input)
    }

    fn tool() -> Result<ToolName, Box<dyn Error>> {
        Ok(ToolName::new("claude")?)
    }

    fn lines(input: &PlanInput, proposals: &Proposals) -> Result<Vec<String>, Box<dyn Error>> {
        let found = check_coverage(&tool()?, Path::new(ENTRY), input, proposals);
        Ok(found
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect())
    }

    #[test]
    fn every_item_covered_passes() -> TestResult {
        let input = plan_input()?;
        let mut proposals = Proposals::new();
        cover(&mut proposals, &input, "claude", &ITEMS)?;
        assert!(lines(&input, &proposals)?.is_empty());
        Ok(())
    }

    #[test]
    fn uncovered_items_are_refused_one_each_in_id_order() -> TestResult {
        let input = input_with_reference()?;
        let mut proposals = Proposals::new();
        let covered: Vec<&str> = ITEMS
            .iter()
            .copied()
            .filter(|id| *id != "core/agent/coder")
            .collect();
        cover(&mut proposals, &input, "claude", &covered)?;
        assert_eq!(
            lines(&input, &proposals)?,
            [
                "claude/adapter.lua: core/agent/coder got no operation (copy it, write it, or skip it with a reason)",
                "claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a reason)",
            ]
        );
        Ok(())
    }

    #[test]
    fn skill_group_skip_covers_its_files() -> TestResult {
        let input = input_with_reference()?;
        let mut proposals = Proposals::new();
        let rest = [
            "core/agent/coder",
            "core/hook/0",
            "core/hook/1",
            "core/native-hook/0",
            "core/native/claude/x.md",
        ];
        cover(&mut proposals, &input, "claude", &rest)?;
        skip_all(&mut proposals, &input, "claude", &["core/skill/status"])?;
        assert!(lines(&input, &proposals)?.is_empty());
        Ok(())
    }

    #[test]
    fn skipped_by_the_core_needs_no_cover() -> TestResult {
        let mut input = plan_input()?;
        input.packs.first_mut().ok_or("a pack")?.agents.clear();
        let mut proposals = Proposals::new();
        let rest: Vec<&str> = ITEMS
            .iter()
            .copied()
            .filter(|id| *id != "core/agent/coder")
            .collect();
        cover(&mut proposals, &input, "claude", &rest)?;
        assert!(lines(&input, &proposals)?.is_empty());
        Ok(())
    }

    #[test]
    fn another_tools_ops_do_not_cover() -> TestResult {
        let input = plan_input()?;
        let mut proposals = Proposals::new();
        let rest: Vec<&str> = ITEMS
            .iter()
            .copied()
            .filter(|id| *id != "core/agent/coder")
            .collect();
        cover(&mut proposals, &input, "claude", &rest)?;
        cover(&mut proposals, &input, "opencode", &["core/agent/coder"])?;
        assert_eq!(
            lines(&input, &proposals)?,
            [
                "claude/adapter.lua: core/agent/coder got no operation (copy it, write it, or skip it with a reason)"
            ]
        );
        Ok(())
    }
}
