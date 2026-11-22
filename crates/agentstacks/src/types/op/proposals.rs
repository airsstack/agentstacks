//! The record of the proposals a run accepted, in the order they arrived.
//!
//! Tools run one after another, and a check made when a proposal arrives sees every earlier
//! one. So the wording of a clash depends on who was first, and the order the flow feeds this
//! record in is part of what it guarantees.
//!
//! # Responsibilities
//!
//! - [`Proposals::check_path`] refuses a path already proposed, and words the refusal by who
//!   proposed it first: a line of the same extension, another tool, a root file of a pack the
//!   core writes, or the hook runtime the core writes.
//! - [`Proposals::propose`] records one extension operation. A write or copy runs the path check
//!   and then refuses an id the tool already skipped; a skip refuses an item the tool already
//!   covered. One call raises at most one refusal, and a refused proposal is not recorded.
//! - [`Proposals::record_core`] records a write the core makes for a tool, and refuses it at the
//!   tool's entry in the project file when a tool already proposed the path.
//! - It accepts the same item skipped twice (the first reason is kept), one file copied to two
//!   paths, two writes covering one item, a repeated id in one `covers`, and the same required
//!   binary more than once.
//!
//! # Non-responsibilities
//!
//! A delete is a step decision from the lock and never reaches this record. A removed tool's
//! proposals are kept out by the flow.

use std::path::Path;

use crate::types::name::{BinaryName, ToolName};
use crate::types::op::{
    ApiFunction, ItemId, OpError, OpPath, Operation, Origin, Proposal, SkipReason,
};
use crate::types::report::{Diagnostic, Location, Quoted};

/// One accepted proposal and the tool it was made for.
#[derive(Debug, Clone)]
struct Record {
    proposal: Proposal,
    /// The tool the proposal was made for: the extension that proposed it, or the tool the core
    /// wrote it for. `None` for a proposal whose origin names no tool.
    tool: Option<ToolName>,
}

impl Record {
    const fn path(&self) -> Option<&OpPath> {
        self.proposal.op.path()
    }

    /// Whether the proposal is an extension's call for `tool`.
    fn extension_of(&self, tool: &ToolName) -> bool {
        self.tool.as_ref() == Some(tool) && matches!(self.proposal.origin, Origin::Extension { .. })
    }
}

/// The proposals of a run, in the order received.
///
/// # Examples
///
/// ```
/// use agentstacks::types::op::Proposals;
///
/// assert_eq!(Proposals::new().iter().count(), 0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Proposals {
    records: Vec<Record>,
}

impl Proposals {
    /// An empty record.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Refuses `path` when an earlier proposal holds it. `function` and `origin` are those of
    /// the call being checked.
    ///
    /// # Errors
    ///
    /// The refusal worded by the earlier proposer: [`OpError::ProposedAt`] for the same tool's
    /// extension, [`OpError::ProposedByTool`] for another tool's, [`OpError::ProposedByCore`] for
    /// a root file of a pack and [`OpError::ProposedByCoreRuntime`] for the hook runtime.
    pub fn check_path(
        &self,
        path: &OpPath,
        function: ApiFunction,
        origin: &Origin,
    ) -> Result<(), OpError> {
        let Some(earlier) = self
            .records
            .iter()
            .find(|record| record.path() == Some(path))
        else {
            return Ok(());
        };
        let path = Quoted::new(path.as_str());
        Err(match &earlier.proposal.origin {
            Origin::Extension { tool, file, line } if origin.tool() == Some(tool) => {
                OpError::ProposedAt {
                    function,
                    path,
                    earlier: Location::at_line(file.as_str(), *line),
                }
            }
            Origin::Extension { tool, .. } => OpError::ProposedByTool {
                function,
                path,
                tool: tool.clone(),
            },
            Origin::Core { pack } => OpError::ProposedByCore {
                function,
                path,
                pack: pack.clone(),
            },
            Origin::Tool { tool } => OpError::ProposedByCoreRuntime {
                function,
                path,
                tool: tool.clone(),
            },
        })
    }

    /// Records one extension operation, the extension named by its origin.
    ///
    /// The checks run in this order and the first that fails is the refusal: the path of a
    /// write or copy, then an id a write or copy covers that the tool skipped (itself, or its
    /// skill), then, for a skip, an item the tool already covered. A delete is never proposed by
    /// an extension and does not belong here.
    ///
    /// # Errors
    ///
    /// The refusal of the first failing check; the proposal is not recorded.
    pub fn propose(&mut self, proposal: Proposal) -> Result<(), OpError> {
        let tool = proposal.origin.tool().cloned();
        if let Some(function) = function_of(&proposal.op) {
            if let Some(path) = proposal.op.path() {
                self.check_path(path, function, &proposal.origin)?;
            }
            if let Some(tool) = &tool {
                self.check_covers(tool, function, &proposal.op)?;
            }
        } else if let (Operation::Skip { item, .. }, Some(tool)) = (&proposal.op, &tool)
            && let Some(earlier) = self.cover_site(tool, item)
        {
            return Err(OpError::SkipAfterCover {
                item: Quoted::new(item.as_str()),
                earlier,
            });
        }
        self.records.push(Record { proposal, tool });
        Ok(())
    }

    /// Records a write the core makes for `tool`.
    ///
    /// # Errors
    ///
    /// A diagnostic at the entry of `tool` in `project_file` when an earlier proposal already
    /// holds the path, naming the tool that proposed it. The write is not recorded.
    pub fn record_core(
        &mut self,
        tool: &ToolName,
        proposal: Proposal,
        project_file: &Path,
    ) -> Result<(), Diagnostic> {
        let clash = proposal.op.path().and_then(|path| {
            let earlier = self
                .records
                .iter()
                .find(|record| record.path() == Some(path))?;
            Some((path, earlier.tool.clone()?))
        });
        if let Some((path, earlier)) = clash {
            let location = Location::set_level(project_file, Some(format!("tools.{tool}")));
            let error = OpError::CoreProposedByTool {
                path: Quoted::new(path.as_str()),
                tool: earlier,
            };
            return Err(Diagnostic::new(location, error));
        }
        self.records.push(Record {
            proposal,
            tool: Some(tool.clone()),
        });
        Ok(())
    }

    /// Refuses an id `op` covers that `tool` already skipped, itself or by its skill.
    fn check_covers(
        &self,
        tool: &ToolName,
        function: ApiFunction,
        op: &Operation,
    ) -> Result<(), OpError> {
        for id in op.covered() {
            if let Some(earlier) = self.skip_site(tool, id) {
                return Err(OpError::CoverAfterSkip {
                    function,
                    id: Quoted::new(id.as_str()),
                    earlier,
                });
            }
        }
        Ok(())
    }

    /// The calls `tool`'s extension made, with the place of each.
    fn calls<'a>(
        &'a self,
        tool: &ToolName,
    ) -> impl Iterator<Item = (&'a Operation, Location)> + use<'a> {
        let tool = tool.clone();
        self.records
            .iter()
            .filter(move |record| record.extension_of(&tool))
            .filter_map(|record| {
                record
                    .proposal
                    .origin
                    .site()
                    .map(|site| (&record.proposal.op, site))
            })
    }

    /// Where `tool` skipped `id`, or the skill `id` is a file of.
    fn skip_site(&self, tool: &ToolName, id: &ItemId) -> Option<Location> {
        self.calls(tool).find_map(|(op, site)| match op {
            Operation::Skip { item, .. } if item == id || id.group().as_ref() == Some(item) => {
                Some(site)
            }
            _ => None,
        })
    }

    /// Where `tool` first covered `item`, or a file of the skill `item`.
    fn cover_site(&self, tool: &ToolName, item: &ItemId) -> Option<Location> {
        self.calls(tool).find_map(|(op, site)| {
            let covers = matches!(op, Operation::Write { .. } | Operation::Copy { .. })
                && op
                    .covered()
                    .iter()
                    .any(|id| id == item || id.group().as_ref() == Some(item));
            covers.then_some(site)
        })
    }

    /// Every proposal, in the order received.
    pub fn iter(&self) -> impl Iterator<Item = &Proposal> {
        self.records.iter().map(|record| &record.proposal)
    }

    /// The proposals `tool`'s extension made.
    pub fn for_tool<'a>(&'a self, tool: &ToolName) -> impl Iterator<Item = &'a Proposal> + use<'a> {
        let tool = tool.clone();
        self.records
            .iter()
            .filter(move |record| record.extension_of(&tool))
            .map(|record| &record.proposal)
    }

    /// The first reason `tool` gave for skipping `item`.
    #[must_use]
    pub fn skip_reason(&self, tool: &ToolName, item: &ItemId) -> Option<&SkipReason> {
        self.for_tool(tool).find_map(|proposal| match &proposal.op {
            Operation::Skip {
                item: skipped,
                reason,
            } if skipped == item => Some(reason),
            _ => None,
        })
    }

    /// The binaries `tool`'s extension requires, each once, with the origin of the first call.
    #[must_use]
    pub fn binaries(&self, tool: &ToolName) -> Vec<(&BinaryName, &Origin)> {
        let mut listed: Vec<(&BinaryName, &Origin)> = Vec::new();
        for proposal in self.for_tool(tool) {
            if let Operation::RequireBinary { name } = &proposal.op
                && !listed.iter().any(|(seen, _)| *seen == name)
            {
                listed.push((name, &proposal.origin));
            }
        }
        listed
    }

    /// Whether `tool`'s extension proposed a write to `path`.
    #[must_use]
    pub fn wrote(&self, tool: &ToolName, path: &OpPath) -> bool {
        self.for_tool(tool).any(|proposal| {
            matches!(&proposal.op, Operation::Write { path: written, .. } if written == path)
        })
    }

    /// Whether the core's write of the hook runtime for `tool` was recorded.
    #[must_use]
    pub fn has_runtime(&self, tool: &ToolName) -> bool {
        self.records.iter().any(|record| {
            matches!(&record.proposal.origin, Origin::Tool { tool: runtime } if runtime == tool)
                && matches!(record.proposal.op, Operation::Write { .. })
        })
    }
}

/// The function an extension called to propose `op`, for a write or a copy.
const fn function_of(op: &Operation) -> Option<ApiFunction> {
    match op {
        Operation::Write { .. } => Some(ApiFunction::Write),
        Operation::Copy { .. } => Some(ApiFunction::Copy),
        Operation::Skip { .. } | Operation::RequireBinary { .. } | Operation::Delete { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::num::NonZeroUsize;
    use std::path::Path;

    use super::*;
    use crate::types::name::{ItemName, PackName};
    use crate::types::tree::TreePath;

    type TestResult = Result<(), Box<dyn Error>>;

    fn tool(text: &str) -> Result<ToolName, Box<dyn Error>> {
        Ok(ToolName::new(text)?)
    }

    fn at(tool_name: &str, line: usize) -> Result<Origin, Box<dyn Error>> {
        Ok(Origin::Extension {
            tool: tool(tool_name)?,
            file: TreePath::from_parts(&["adapter.lua"]),
            line: NonZeroUsize::new(line).ok_or("a line starts at 1")?,
        })
    }

    fn agent(name: &str) -> Result<ItemId, Box<dyn Error>> {
        Ok(ItemId::agent(
            &PackName::new("core")?,
            &ItemName::new(name)?,
        ))
    }

    fn group() -> Result<ItemId, Box<dyn Error>> {
        Ok(ItemId::skill(
            &PackName::new("core")?,
            &ItemName::new("status")?,
        ))
    }

    fn file() -> Result<ItemId, Box<dyn Error>> {
        Ok(ItemId::skill_file(
            &PackName::new("core")?,
            &ItemName::new("status")?,
            &TreePath::from_parts(&["SKILL.md"]),
        ))
    }

    fn write(path: &str, covers: Vec<ItemId>, origin: Origin) -> Result<Proposal, Box<dyn Error>> {
        Ok(Proposal {
            op: Operation::Write {
                path: OpPath::new(path)?,
                content: Vec::new(),
                covers,
            },
            origin,
        })
    }

    fn copy(path: &str, file: ItemId, origin: Origin) -> Result<Proposal, Box<dyn Error>> {
        Ok(Proposal {
            op: Operation::Copy {
                path: OpPath::new(path)?,
                file,
            },
            origin,
        })
    }

    fn skip(item: ItemId, reason: &str, origin: Origin) -> Result<Proposal, Box<dyn Error>> {
        Ok(Proposal {
            op: Operation::Skip {
                item,
                reason: SkipReason::new(reason)?,
            },
            origin,
        })
    }

    fn require(name: &str, origin: Origin) -> Result<Proposal, Box<dyn Error>> {
        Ok(Proposal {
            op: Operation::require_binary(name)?,
            origin,
        })
    }

    fn refusal(result: Result<(), OpError>) -> Result<String, Box<dyn Error>> {
        Ok(result.err().ok_or("a refusal")?.to_string())
    }

    #[test]
    fn same_tool_duplicate_names_the_earlier_site() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(write(".claude/x", Vec::new(), at("claude", 9)?)?)?;
        let second = copy("./.claude//x", file()?, at("claude", 12)?)?;
        assert_eq!(
            refusal(proposals.propose(second))?,
            "copy: path \".claude/x\" is already proposed at adapter.lua:9"
        );
        assert_eq!(proposals.iter().count(), 1);
        Ok(())
    }

    #[test]
    fn other_tool_duplicate_names_the_tool() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(write(".claude/x", Vec::new(), at("claude", 9)?)?)?;
        let second = write(".claude/x", Vec::new(), at("opencode", 3)?)?;
        assert_eq!(
            refusal(proposals.propose(second))?,
            "write: path \".claude/x\" is already proposed by claude"
        );
        Ok(())
    }

    #[test]
    fn extension_op_on_another_tools_root_file_names_the_pack() -> TestResult {
        let mut proposals = Proposals::new();
        let root = Origin::Core {
            pack: PackName::new("core")?,
        };
        proposals
            .record_core(
                &tool("claude")?,
                write(".claude/agentstacks/core/x.md", Vec::new(), root)?,
                Path::new("agentstacks.toml"),
            )
            .map_err(|found| found.to_string())?;
        let second = write(
            ".claude/agentstacks/core/x.md",
            Vec::new(),
            at("opencode", 3)?,
        )?;
        assert_eq!(
            refusal(proposals.propose(second))?,
            "write: path \".claude/agentstacks/core/x.md\" is already proposed by core (root file of core)"
        );
        Ok(())
    }

    #[test]
    fn proposals_new_is_empty_and_default() {
        assert_eq!(Proposals::new().iter().count(), 0);
        assert_eq!(Proposals::default().iter().count(), 0);
    }

    #[test]
    fn core_root_file_duplicate_names_the_pack() -> TestResult {
        let mut proposals = Proposals::new();
        let root = Origin::Core {
            pack: PackName::new("core")?,
        };
        let path = ".claude/agentstacks/core/hooks/enforce.lua";
        proposals
            .record_core(
                &tool("claude")?,
                write(path, Vec::new(), root)?,
                Path::new("agentstacks.toml"),
            )
            .map_err(|found| found.to_string())?;
        let second = write(path, Vec::new(), at("claude", 4)?)?;
        assert_eq!(
            refusal(proposals.propose(second))?,
            "write: path \".claude/agentstacks/core/hooks/enforce.lua\" is already proposed by core (root file of core)"
        );
        Ok(())
    }

    #[test]
    fn runtime_duplicate_names_the_tool_runtime() -> TestResult {
        let mut proposals = Proposals::new();
        let runtime = Origin::Tool {
            tool: tool("claude")?,
        };
        let path = ".claude/agentstacks/runtime.lua";
        proposals
            .record_core(
                &tool("claude")?,
                write(path, Vec::new(), runtime)?,
                Path::new("agentstacks.toml"),
            )
            .map_err(|found| found.to_string())?;
        let second = write(path, Vec::new(), at("claude", 4)?)?;
        assert_eq!(
            refusal(proposals.propose(second))?,
            "write: path \".claude/agentstacks/runtime.lua\" is already proposed by core (runtime.lua of claude)"
        );
        Ok(())
    }

    #[test]
    fn core_write_meeting_an_earlier_tool_is_refused_at_the_tool_entry() -> TestResult {
        let project = Path::new("agentstacks.toml");
        let runtime = |name: &str| -> Result<Proposal, Box<dyn Error>> {
            write(
                ".shared/runtime.lua",
                Vec::new(),
                Origin::Tool { tool: tool(name)? },
            )
        };

        let mut proposals = Proposals::new();
        proposals.propose(write(".shared/runtime.lua", Vec::new(), at("claude", 2)?)?)?;
        let refused = proposals
            .record_core(&tool("opencode")?, runtime("opencode")?, project)
            .err()
            .ok_or("a refusal")?;
        assert_eq!(
            refused.to_string(),
            "agentstacks.toml: tools.opencode: path \".shared/runtime.lua\" is already proposed by claude"
        );

        let mut proposals = Proposals::new();
        proposals
            .record_core(&tool("claude")?, runtime("claude")?, project)
            .map_err(|found| found.to_string())?;
        let refused = proposals
            .record_core(&tool("opencode")?, runtime("opencode")?, project)
            .err()
            .ok_or("a refusal")?;
        assert_eq!(
            refused.to_string(),
            "agentstacks.toml: tools.opencode: path \".shared/runtime.lua\" is already proposed by claude"
        );
        assert_eq!(proposals.iter().count(), 1);
        Ok(())
    }

    #[test]
    fn skip_after_copy_is_refused_at_the_skip() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(copy(".claude/s/SKILL.md", file()?, at("claude", 9)?)?)?;
        let second = skip(group()?, "no", at("claude", 14)?)?;
        assert_eq!(
            refusal(proposals.propose(second))?,
            "skip: item \"core/skill/status\" is already covered at adapter.lua:9"
        );
        Ok(())
    }

    #[test]
    fn skip_after_a_write_covering_the_item_is_refused() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(write(
            ".claude/a.md",
            vec![agent("coder")?],
            at("claude", 5)?,
        )?)?;
        let second = skip(agent("coder")?, "no", at("claude", 14)?)?;
        assert_eq!(
            refusal(proposals.propose(second))?,
            "skip: item \"core/agent/coder\" is already covered at adapter.lua:5"
        );
        Ok(())
    }

    #[test]
    fn write_after_skip_is_refused_at_the_write() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(skip(agent("coder")?, "no", at("claude", 9)?)?)?;
        let second = write(".claude/a.md", vec![agent("coder")?], at("claude", 12)?)?;
        assert_eq!(
            refusal(proposals.propose(second))?,
            "write: covers \"core/agent/coder\", which adapter.lua:9 skipped"
        );
        Ok(())
    }

    #[test]
    fn copy_after_group_skip_is_refused() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(skip(group()?, "no", at("claude", 9)?)?)?;
        let second = copy(".claude/s/SKILL.md", file()?, at("claude", 12)?)?;
        assert_eq!(
            refusal(proposals.propose(second))?,
            "copy: covers \"core/skill/status/SKILL.md\", which adapter.lua:9 skipped"
        );
        Ok(())
    }

    #[test]
    fn duplicate_path_wins_over_a_conflict() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(skip(agent("coder")?, "no", at("claude", 9)?)?)?;
        proposals.propose(write(".claude/a.md", Vec::new(), at("claude", 10)?)?)?;
        let second = write(".claude/a.md", vec![agent("coder")?], at("claude", 12)?)?;
        assert_eq!(
            refusal(proposals.propose(second))?,
            "write: path \".claude/a.md\" is already proposed at adapter.lua:10"
        );
        Ok(())
    }

    #[test]
    fn skipping_twice_keeps_the_first_reason() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(skip(agent("coder")?, "a", at("claude", 9)?)?)?;
        proposals.propose(skip(agent("coder")?, "b", at("claude", 10)?)?)?;
        let reason = proposals.skip_reason(&tool("claude")?, &agent("coder")?);
        assert_eq!(reason.map(SkipReason::as_str), Some("a"));
        assert_eq!(
            proposals.skip_reason(&tool("opencode")?, &agent("coder")?),
            None
        );
        Ok(())
    }

    #[test]
    fn copying_one_file_to_two_paths_is_accepted() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(copy(".claude/a.md", file()?, at("claude", 9)?)?)?;
        proposals.propose(copy(".claude/b.md", file()?, at("claude", 10)?)?)?;
        assert_eq!(proposals.iter().count(), 2);
        Ok(())
    }

    #[test]
    fn two_writes_covering_one_item_are_accepted() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(write(
            ".claude/a.md",
            vec![agent("coder")?],
            at("claude", 9)?,
        )?)?;
        let twice = vec![agent("coder")?, agent("coder")?];
        proposals.propose(write(".claude/b.md", twice, at("claude", 10)?)?)?;
        assert_eq!(proposals.iter().count(), 2);
        Ok(())
    }

    #[test]
    fn require_binary_twice_is_checked_once() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(require("node", at("claude", 3)?)?)?;
        proposals.propose(require("node", at("claude", 8)?)?)?;
        proposals.propose(require("node", at("opencode", 2)?)?)?;
        let claude = tool("claude")?;
        let listed: Vec<(String, Option<String>)> = proposals
            .binaries(&claude)
            .into_iter()
            .map(|(name, origin)| (name.to_string(), origin.site().map(|site| site.to_string())))
            .collect();
        assert_eq!(
            listed,
            [("node".to_owned(), Some("adapter.lua:3".to_owned()))]
        );
        assert_eq!(proposals.binaries(&tool("opencode")?).len(), 1);
        Ok(())
    }

    #[test]
    fn conflicts_are_per_tool() -> TestResult {
        let mut proposals = Proposals::new();
        proposals.propose(skip(agent("coder")?, "no", at("claude", 9)?)?)?;
        proposals.propose(write(
            ".opencode/a.md",
            vec![agent("coder")?],
            at("opencode", 2)?,
        )?)?;
        assert_eq!(proposals.iter().count(), 2);
        Ok(())
    }

    #[test]
    fn queries_tell_one_tool_from_another() -> TestResult {
        let mut proposals = Proposals::new();
        let claude = tool("claude")?;
        proposals.propose(write(".claude/x", Vec::new(), at("claude", 1)?)?)?;
        proposals.propose(write(".opencode/x", Vec::new(), at("opencode", 1)?)?)?;
        let runtime = Origin::Tool {
            tool: claude.clone(),
        };
        proposals
            .record_core(
                &claude,
                write(".claude/agentstacks/runtime.lua", Vec::new(), runtime)?,
                Path::new("agentstacks.toml"),
            )
            .map_err(|found| found.to_string())?;
        assert_eq!(proposals.for_tool(&claude).count(), 1);
        assert!(proposals.wrote(&claude, &OpPath::new(".claude/x")?));
        assert!(!proposals.wrote(&claude, &OpPath::new(".opencode/x")?));
        assert!(!proposals.wrote(&claude, &OpPath::new(".claude/agentstacks/runtime.lua")?));
        assert!(proposals.has_runtime(&claude));
        assert!(!proposals.has_runtime(&tool("opencode")?));
        Ok(())
    }
}
