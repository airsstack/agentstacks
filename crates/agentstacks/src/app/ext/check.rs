//! The flow of `ext test`: checking a tool extension against the built-in conformance pack in a
//! temporary project.
//!
//! The flow opens the extension and loads it, runs its Lua tests, builds the temporary project
//! from the extension, the conformance pack and the packs the author named, then plans, applies
//! and plans again with the flows `plan` and `apply` use. Nothing is written outside the
//! temporary project.
//!
//! # Responsibilities
//!
//! - [`check()`] runs eight steps in order and returns a [`CheckReport`] holding what each one
//!   found:
//!   1. `airsl` is on `PATH`;
//!   2. the extension directory is opened and loaded with [`Extension::load`];
//!   3. the Lua tests run when the extension holds a test file ([`has_tests`]);
//!   4. each named pack is opened and named, and the temporary project is staged ([`stage`]) and
//!      created;
//!   5. the project is planned;
//!   6. the plan is applied;
//!   7. the project is planned again, and the second plan must change nothing
//!      ([`second_plan`]);
//!   8. the [`Verdict`].
//! - A failure of step 1 or 2 stops the run with no verdict. A failure of step 4, 5 or 6 stops it
//!   with the verdict `failed`. A failing step 3 or 7 does not stop the run and makes the verdict
//!   `failed`.
//! - [`Dir`] is a directory the author named, [`Checked`] the extension that loaded,
//!   [`LuaOutcome`] how step 3 ended and [`FirstPlan`] the plan of step 5 with whether step 6
//!   applied it.
//!
//! # Non-responsibilities
//!
//! No rule and no adapter: the file set, the Lua-test decision and the second-plan check belong
//! to `types::check`; the disk, the temporary directory and `airsl` belong to the ports. Showing
//! the report and choosing the exit code belong to the command line.

use std::io;
use std::path::Path;

use crate::app::apply::apply;
use crate::app::plan::plan;
use crate::port::{Adapter, Binary, FileTree, Git, Scratch};
use crate::types::check::{
    CheckError, GivenPack, NotRepeatable, Verdict, has_tests, pack_name, second_plan, stage,
    summary,
};
use crate::types::extension::Extension;
use crate::types::manifest::PackVersion;
use crate::types::name::{AIRSL, BinaryName, ToolName};
use crate::types::plan::Plan;
use crate::types::report::{Diagnostic, Diagnostics, Location};
use crate::types::tree::walk;

/// Where refusals of the run itself, such as a missing `airsl`, are located.
const COMMAND: &str = "ext test";

/// A directory the author named on the command line.
#[derive(Debug, Clone, Copy)]
pub struct Dir<'a> {
    /// The path to read, resolved against the project directory.
    pub resolved: &'a Path,
    /// The path as the author wrote it, which every refusal about the directory shows.
    pub label: &'a str,
}

/// The extension that loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// The tool it is for.
    pub name: ToolName,
    /// Its version.
    pub version: PackVersion,
}

/// The plan of step 5, and whether step 6 applied it.
#[derive(Debug)]
pub enum FirstPlan {
    /// The plan was made, and applying it was refused.
    Planned(Plan),
    /// The plan was applied.
    Applied(Plan),
}

impl FirstPlan {
    /// The plan itself.
    #[must_use]
    pub const fn plan(&self) -> &Plan {
        match self {
            Self::Planned(plan) | Self::Applied(plan) => plan,
        }
    }

    /// Whether the plan was applied.
    #[must_use]
    pub const fn applied(&self) -> bool {
        matches!(self, Self::Applied(_))
    }
}

/// How the Lua tests of the extension ended.
#[derive(Debug)]
pub enum LuaOutcome {
    /// The extension holds no test file, so `airsl test` did not run.
    None,
    /// `airsl test` exited with status zero.
    Passed {
        /// The last non-empty line of what it printed, `None` when it printed nothing.
        summary: Option<String>,
    },
    /// `airsl test` exited with a non-zero status.
    Failed {
        /// What it printed on standard output.
        stdout: String,
        /// What it printed on standard error.
        stderr: String,
    },
    /// `airsl test` could not be started.
    NotStarted(Diagnostic),
}

impl LuaOutcome {
    /// Whether the outcome fails the run.
    const fn fails(&self) -> bool {
        matches!(self, Self::Failed { .. } | Self::NotStarted(_))
    }
}

/// What an `ext test` run found, step by step.
///
/// A field is `None` or empty when its step did not run or did not finish. The refusals are those
/// of the step that stopped the run, if one did.
#[derive(Debug, Default)]
pub struct CheckReport {
    /// The extension, once step 2 loaded it.
    pub extension: Option<Checked>,
    /// How step 3 ended.
    pub lua: Option<LuaOutcome>,
    /// The plan of step 5, and whether step 6 applied it.
    pub plan: Option<FirstPlan>,
    /// The plan of step 7.
    pub second: Option<Plan>,
    /// Why the second plan fails the run, when it holds a step that is not `unchanged`.
    pub not_repeatable: Option<NotRepeatable>,
    /// The refusals that stopped the run.
    pub refusals: Diagnostics,
    /// How the run ended; `None` when step 1 or 2 stopped it.
    pub verdict: Option<Verdict>,
}

impl CheckReport {
    /// This report stopped by `refusals` and closed with its verdict.
    fn stopped(mut self, refusals: Diagnostics) -> Self {
        self.refusals = refusals;
        self.closed()
    }

    /// This report with the verdict of step 8: one failure for each failing step.
    fn closed(mut self) -> Self {
        let failures = usize::from(self.lua.as_ref().is_some_and(LuaOutcome::fails))
            + usize::from(!self.refusals.is_empty())
            + usize::from(self.not_repeatable.is_some());
        self.verdict = Some(Verdict::of(failures));
        self
    }
}

/// Checks the extension in `extension` against the conformance pack and the packs in `packs`.
///
/// Steps 1 and 2 stop the run without a verdict when they fail. From step 3 on, the report always
/// holds a verdict.
#[must_use]
pub fn check<B: Binary, S: Scratch, G: Git, A: Adapter>(
    binary: &B,
    scratch: &S,
    git: &G,
    adapter: &A,
    extension: Dir<'_>,
    packs: &[Dir<'_>],
) -> CheckReport {
    let mut report = CheckReport::default();
    if !BinaryName::new(AIRSL).is_ok_and(|airsl| binary.on_path(&airsl)) {
        report.refusals.push(run_refusal(CheckError::AirslMissing));
        return report;
    }
    let tree = match scratch.open(extension.resolved, Path::new(extension.label)) {
        Ok(tree) => tree,
        Err(error) => {
            report.refusals.push(io_refusal(extension.label, error));
            return report;
        }
    };
    let loaded = match Extension::load(&tree) {
        Ok(loaded) => loaded,
        Err(refusals) => {
            report.refusals = refusals;
            return report;
        }
    };
    report.extension = Some(Checked {
        name: loaded.manifest().name().clone(),
        version: loaded.manifest().version().clone(),
    });
    report.lua = Some(lua_tests(binary, extension.resolved, &tree));

    let project = match staged_project(scratch, &loaded, &tree, packs) {
        Ok(project) => project,
        Err(refusals) => return report.stopped(refusals),
    };
    let first = match plan(&project, git, binary, adapter) {
        Ok(first) => first,
        Err(refusals) => return report.stopped(refusals),
    };
    if let Err(refusals) = apply(&first, &project) {
        report.plan = Some(FirstPlan::Planned(first));
        return report.stopped(refusals);
    }
    report.plan = Some(FirstPlan::Applied(first));
    match plan(&project, git, binary, adapter) {
        Ok(second) => {
            report.not_repeatable = second_plan(&second).err();
            report.second = Some(second);
            report.closed()
        }
        Err(refusals) => report.stopped(refusals),
    }
}

/// Step 3: runs the Lua tests of the extension in `dir`, when `tree` holds a test file.
fn lua_tests<B: Binary, T: FileTree>(binary: &B, dir: &Path, tree: &T) -> LuaOutcome {
    let (walked, _) = walk(tree);
    if !has_tests(walked.files()) {
        return LuaOutcome::None;
    }
    match binary.airsl_test(dir) {
        Ok(run) if run.success => LuaOutcome::Passed {
            summary: summary(&run.stdout).map(str::to_owned),
        },
        Ok(run) => LuaOutcome::Failed {
            stdout: run.stdout,
            stderr: run.stderr,
        },
        Err(error) => LuaOutcome::NotStarted(run_refusal(CheckError::AirslTest(error))),
    }
}

/// Step 4: opens and names each pack the author gave, stages the files and creates the project.
///
/// Every pack that cannot be opened or named is reported before the run stops.
fn staged_project<S: Scratch>(
    scratch: &S,
    extension: &Extension,
    tree: &S::Tree,
    packs: &[Dir<'_>],
) -> Result<S::Project, Diagnostics> {
    let mut refusals = Diagnostics::new();
    let mut given = Vec::new();
    for dir in packs {
        match scratch.open(dir.resolved, Path::new(dir.label)) {
            Ok(tree) => match pack_name(&tree, dir.label) {
                Ok(name) => given.push(GivenPack {
                    label: dir.label.to_owned(),
                    name,
                    tree,
                }),
                Err(found) => refusals.extend(found),
            },
            Err(error) => refusals.push(io_refusal(dir.label, error)),
        }
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }
    let files = stage(extension, tree, &given)?;
    scratch
        .create(&files)
        .map_err(|error| Diagnostics::from(io_refusal(COMMAND, error)))
}

/// A refusal of the run itself.
fn run_refusal(error: CheckError) -> Diagnostic {
    Diagnostic::new(Location::verbatim(COMMAND), error)
}

/// An I/O failure at `at`: the directory the author wrote, or the command itself.
fn io_refusal(at: &str, error: io::Error) -> Diagnostic {
    Diagnostic::new(Location::verbatim(at), error)
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::num::NonZeroUsize;
    use std::path::{Path, PathBuf};

    use serde_json::Value;

    use super::*;
    use crate::port::{
        BinaryCall, FakeAdapter, FakeBinary, FakeFileTree, FakeGit, FakeScratch, LuaTestRun,
        ScratchCall, ScriptStep, ToolScript,
    };
    use crate::types::check::{CONFORMANCE, NotRepeatable};
    use crate::types::name::{ItemName, PackName};
    use crate::types::op::{
        ItemId, LuaType, OpPath, Operation, Origin, Proposal, SkipReason, ValuesInput, ValuesResult,
    };
    use crate::types::pack::Pack;
    use crate::types::tree::TreePath;

    type TestResult = Result<(), Box<dyn Error>>;

    const EXTENSION_TOML: &str = "[extension]\nname = \"mytool\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\nentry = \"adapter.lua\"\n";
    const EXT_DIR: &str = "/work/mytool";

    fn shown(found: Diagnostics) -> Vec<String> {
        found
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn plain(found: Diagnostics) -> Box<dyn Error> {
        shown(found).join("; ").into()
    }

    fn tool() -> Result<ToolName, Box<dyn Error>> {
        Ok(ToolName::new("mytool")?)
    }

    fn extension_tree(extra: &[(&str, &str)]) -> FakeFileTree {
        extra.iter().fold(
            FakeFileTree::new("elsewhere")
                .file("extension.toml", EXTENSION_TOML.as_bytes())
                .file("adapter.lua", b"return {}\n"),
            |tree, (path, text)| tree.file(path, text.as_bytes()),
        )
    }

    fn conformance_tree() -> FakeFileTree {
        CONFORMANCE.iter().fold(
            FakeFileTree::new("packs/conformance"),
            |tree, (path, bytes)| tree.file(path, bytes),
        )
    }

    /// The `values` answer of `mytool` over the conformance pack.
    fn values() -> Result<ValuesResult, Box<dyn Error>> {
        let pack = Pack::load(&conformance_tree()).map_err(plain)?;
        let input = ValuesInput::new(tool()?, &[&pack]);
        let value: Value = serde_json::from_str(
            r#"{"root":".mytool",
                "skills":{"conformance/check":{"invoke":"/check","dir":".mytool/skills/check"}},
                "agents":{"conformance/checker":"@checker"},
                "vars":{"arguments":"a","session_id":"s","tmpdir":"t","home":"h",
                        "airsstack_home":"x","project_dir":"p"}}"#,
        )?;
        ValuesResult::read(
            value.as_object().ok_or("an object")?,
            &input,
            Path::new("mytool/adapter.lua"),
            |_| LuaType::Table,
        )
        .map_err(plain)
    }

    /// Every item of the conformance pack that `mytool` gets.
    fn items() -> Result<Vec<ItemId>, Box<dyn Error>> {
        let pack = PackName::new("conformance")?;
        let skill = ItemName::new("check")?;
        let file = |parts: &[&str]| ItemId::skill_file(&pack, &skill, &TreePath::from_parts(parts));
        let mut items = vec![
            file(&["SKILL.md"]),
            file(&["references", "notes.md"]),
            file(&["logo.png"]),
            ItemId::agent(&pack, &ItemName::new("checker")?),
            ItemId::native_file(&pack, &tool()?, &TreePath::from_parts(&["note.md"])),
        ];
        items.extend((0..7).map(|index| ItemId::hook(&pack, index)));
        Ok(items)
    }

    fn proposal(line: usize, op: Operation) -> Result<ScriptStep, Box<dyn Error>> {
        Ok(ScriptStep::Propose(Proposal {
            op,
            origin: Origin::Extension {
                tool: tool()?,
                file: TreePath::from_parts(&["adapter.lua"]),
                line: NonZeroUsize::new(line).ok_or("a line starts at 1")?,
            },
        }))
    }

    /// One run of `mytool`: it skips every item, and writes `extra` when it is given.
    fn script(extra: Option<&[u8]>) -> Result<ToolScript, Box<dyn Error>> {
        let mut script = ToolScript::new().values(Ok(values()?));
        for (line, item) in items()?.into_iter().enumerate() {
            script = script.step(proposal(
                line + 1,
                Operation::Skip {
                    item,
                    reason: SkipReason::new("not supported")?,
                },
            )?);
        }
        if let Some(content) = extra {
            script = script.step(proposal(
                99,
                Operation::Write {
                    path: OpPath::new(".mytool/extra.txt")?,
                    content: content.to_vec(),
                    covers: Vec::new(),
                },
            )?);
        }
        Ok(script)
    }

    struct World {
        binary: FakeBinary,
        scratch: FakeScratch,
        adapter: FakeAdapter,
    }

    /// `airsl` on `PATH`, the extension at `EXT_DIR` and `mytool` skipping everything on both runs.
    fn world(extension: FakeFileTree) -> Result<World, Box<dyn Error>> {
        Ok(World {
            binary: FakeBinary::new().airsl("airsl 0.1.5"),
            scratch: FakeScratch::new().dir(Path::new(EXT_DIR), extension),
            adapter: FakeAdapter::new()
                .tool(&tool()?, script(None)?)
                .tool(&tool()?, script(None)?),
        })
    }

    fn run(world: &World, packs: &[Dir<'_>]) -> CheckReport {
        check(
            &world.binary,
            &world.scratch,
            &FakeGit::new(),
            &world.adapter,
            Dir {
                resolved: Path::new(EXT_DIR),
                label: "mytool",
            },
            packs,
        )
    }

    fn passing() -> LuaTestRun {
        LuaTestRun {
            success: true,
            stdout: "PASS a_test.lua\n3 passed, 0 failed (1 files)\n".to_owned(),
            stderr: String::new(),
        }
    }

    #[test]
    fn airsl_missing_stops_without_verdict() -> TestResult {
        let mut world = world(extension_tree(&[]))?;
        world.binary = FakeBinary::new();
        let report = run(&world, &[]);
        assert_eq!(
            shown(report.refusals),
            ["ext test: airsl not found on PATH (needed for the Lua tests and the hooks)"]
        );
        assert!(report.extension.is_none());
        assert!(report.lua.is_none());
        assert!(report.plan.is_none());
        assert!(report.verdict.is_none());
        assert!(world.scratch.calls().is_empty());
        Ok(())
    }

    #[test]
    fn load_refusal_stops_without_verdict() -> TestResult {
        let world = world(FakeFileTree::new("elsewhere").file("adapter.lua", b""))?;
        let report = run(&world, &[]);
        let shown = shown(report.refusals);
        assert!(!shown.is_empty());
        assert!(
            shown.iter().all(|line| line.starts_with("mytool: ")),
            "{shown:?}"
        );
        assert!(
            shown
                .iter()
                .any(|line| line.ends_with("not an extension (no extension.toml)")),
            "{shown:?}"
        );
        assert!(report.extension.is_none());
        assert!(report.lua.is_none());
        assert!(report.verdict.is_none());
        assert_eq!(world.binary.asked().len(), 1, "only the PATH lookup");
        Ok(())
    }

    #[test]
    fn extension_dir_missing_stops_without_verdict() -> TestResult {
        let mut world = world(extension_tree(&[]))?;
        world.scratch = FakeScratch::new();
        let report = run(&world, &[]);
        assert_eq!(
            shown(report.refusals),
            [format!("mytool: {}", FakeFileTree::not_found())]
        );
        assert!(report.verdict.is_none());
        Ok(())
    }

    #[test]
    fn no_tests_is_none() -> TestResult {
        let world = world(extension_tree(&[]))?;
        let report = run(&world, &[]);
        assert!(matches!(report.lua, Some(LuaOutcome::None)));
        assert!(
            !world
                .binary
                .asked()
                .iter()
                .any(|call| matches!(call, BinaryCall::AirslTest(_)))
        );
        assert_eq!(
            report.extension,
            Some(Checked {
                name: tool()?,
                version: PackVersion::new("0.1.0")?
            })
        );
        assert!(report.refusals.is_empty());
        assert!(report.plan.is_some());
        assert!(report.plan.as_ref().is_some_and(FirstPlan::applied));
        let second = report.second.as_ref().ok_or("a second plan")?;
        assert!(second.steps().iter().all(|step| !step.action.writes()));
        assert_eq!(report.not_repeatable, None);
        assert_eq!(report.verdict, Some(Verdict::Passed));
        Ok(())
    }

    #[test]
    fn passing_tests_show_the_last_line() -> TestResult {
        let mut world = world(extension_tree(&[("lib/a_test.lua", "")]))?;
        world.binary = world.binary.airsl_test_run(passing());
        let report = run(&world, &[]);
        let Some(LuaOutcome::Passed { summary }) = &report.lua else {
            return Err(format!("{:?}", report.lua).into());
        };
        assert_eq!(summary.as_deref(), Some("3 passed, 0 failed (1 files)"));
        assert!(
            world
                .binary
                .asked()
                .contains(&BinaryCall::AirslTest(PathBuf::from(EXT_DIR)))
        );
        assert_eq!(report.verdict, Some(Verdict::Passed));
        Ok(())
    }

    #[test]
    fn tests_fail_run_continues_and_fails() -> TestResult {
        let mut world = world(extension_tree(&[("test_x.lua", "")]))?;
        world.binary = world.binary.airsl_test_run(LuaTestRun {
            success: false,
            stdout: "1 passed, 1 failed (1 files)\ndetail\n".to_owned(),
            stderr: "boom\n".to_owned(),
        });
        let report = run(&world, &[]);
        let Some(LuaOutcome::Failed { stdout, stderr }) = &report.lua else {
            return Err(format!("{:?}", report.lua).into());
        };
        assert_eq!(stdout, "1 passed, 1 failed (1 files)\ndetail\n");
        assert_eq!(stderr, "boom\n");
        assert!(report.plan.is_some());
        assert!(report.plan.as_ref().is_some_and(FirstPlan::applied));
        assert!(report.second.is_some());
        assert!(report.refusals.is_empty());
        assert_eq!(report.verdict, Some(Verdict::Failed));
        Ok(())
    }

    #[test]
    fn airsl_spawn_failure_fails() -> TestResult {
        let mut world = world(extension_tree(&[("a_test.lua", "")]))?;
        world.binary = world.binary.airsl_test_spawn_failure();
        let report = run(&world, &[]);
        let Some(LuaOutcome::NotStarted(refusal)) = &report.lua else {
            return Err(format!("{:?}", report.lua).into());
        };
        assert_eq!(
            refusal.to_string(),
            "ext test: airsl test: permission denied"
        );
        assert!(report.plan.is_some());
        assert!(report.plan.as_ref().is_some_and(FirstPlan::applied));
        assert_eq!(report.verdict, Some(Verdict::Failed));
        Ok(())
    }

    #[test]
    fn pack_dir_missing() -> TestResult {
        let world = world(extension_tree(&[]))?;
        let packs = [Dir {
            resolved: Path::new("/work/nope"),
            label: "../nope",
        }];
        let report = run(&world, &packs);
        assert_eq!(
            shown(report.refusals),
            [format!("../nope: {}", FakeFileTree::not_found())]
        );
        assert!(matches!(report.lua, Some(LuaOutcome::None)));
        assert!(report.plan.is_none());
        assert_eq!(report.verdict, Some(Verdict::Failed));
        assert!(
            !world
                .scratch
                .calls()
                .iter()
                .any(|call| matches!(call, ScratchCall::Create(_)))
        );
        Ok(())
    }

    #[test]
    fn pack_dir_not_a_pack() -> TestResult {
        let mut world = world(extension_tree(&[]))?;
        world.scratch = world
            .scratch
            .dir(Path::new("/work/plain"), FakeFileTree::new("elsewhere"));
        let packs = [Dir {
            resolved: Path::new("/work/plain"),
            label: "plain",
        }];
        let report = run(&world, &packs);
        assert_eq!(
            shown(report.refusals),
            ["--pack \"plain\": not a pack (no pack.toml)"]
        );
        assert!(report.plan.is_none());
        assert_eq!(report.verdict, Some(Verdict::Failed));
        Ok(())
    }

    #[test]
    fn staging_refusal_stops_with_failed() -> TestResult {
        let mut world = world(extension_tree(&[]))?;
        let named_conformance = FakeFileTree::new("elsewhere")
            .file(
                "pack.toml",
                b"[pack]\nname = \"conformance\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\n",
            )
            .file("skills/a/SKILL.md", b"---\nname: a\ndescription: d\n---\nBody\n");
        world.scratch = world
            .scratch
            .dir(Path::new("/work/mine"), named_conformance);
        let packs = [Dir {
            resolved: Path::new("/work/mine"),
            label: "mine",
        }];
        let report = run(&world, &packs);
        assert_eq!(
            shown(report.refusals),
            [
                "--pack \"mine\": pack \"conformance\" is already given by the built-in conformance pack"
            ]
        );
        assert!(report.plan.is_none());
        assert_eq!(report.verdict, Some(Verdict::Failed));
        Ok(())
    }

    #[test]
    fn plan_refusal_stops_with_failed() -> TestResult {
        let mut world = world(extension_tree(&[]))?;
        world.adapter = FakeAdapter::new().tool(&tool()?, ToolScript::new().values(Ok(values()?)));
        let report = run(&world, &[]);
        let refusals = shown(report.refusals);
        assert_eq!(refusals.len(), items()?.len());
        assert!(
            refusals
                .iter()
                .all(|line| line.starts_with("mytool/adapter.lua: conformance/")
                    && line.ends_with(
                        "got no operation (copy it, write it, or skip it with a reason)"
                    )),
            "{refusals:?}"
        );
        assert!(report.plan.is_none());
        assert!(report.second.is_none());
        assert_eq!(report.verdict, Some(Verdict::Failed));
        Ok(())
    }

    #[test]
    fn not_repeatable_fails() -> TestResult {
        let mut world = world(extension_tree(&[]))?;
        world.adapter = FakeAdapter::new()
            .tool(&tool()?, script(Some(b"first\n"))?)
            .tool(&tool()?, script(Some(b"second\n"))?);
        let report = run(&world, &[]);
        assert!(report.refusals.is_empty());
        assert!(report.plan.as_ref().is_some_and(FirstPlan::applied));
        assert!(report.second.is_some());
        assert_eq!(
            report.not_repeatable,
            Some(NotRepeatable {
                write: 1,
                delete: 0,
                steps: 1
            })
        );
        assert_eq!(report.verdict, Some(Verdict::Failed));
        Ok(())
    }

    #[test]
    fn skips_pass() -> TestResult {
        let world = world(extension_tree(&[]))?;
        let report = run(&world, &[]);
        let plan = report.plan.as_ref().ok_or("a plan")?.plan();
        assert_eq!(plan.skips().len(), items()?.len());
        assert_eq!(report.verdict, Some(Verdict::Passed));
        Ok(())
    }
}
