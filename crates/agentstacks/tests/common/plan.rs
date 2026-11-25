//! Running the real plan flow and reading its steps and skips as plain text.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use agentstacks::app::plan::plan;
use agentstacks::fs::StdWorkspace;
use agentstacks::host::AirslAdapter;
use agentstacks::process::{GitCli, SystemBinary};
use agentstacks::types::op::SkipSubject;
use agentstacks::types::plan::Plan;
use tempfile::TempDir;

use super::tree::{fixture, project};

/// Runs the real plan flow over `dir`; a refusal comes back as its lines.
pub(crate) fn run_plan(dir: &Path) -> Result<Result<Plan, Vec<String>>, Box<dyn Error>> {
    let workspace = StdWorkspace::open(dir)?;
    Ok(plan(
        &workspace,
        &GitCli::from_env(),
        &SystemBinary::from_env(),
        &AirslAdapter::new(),
    )
    .map_err(|found| {
        found
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }))
}

/// The plan of the fixture `case` of `tool`.
pub(crate) fn planned(tool: &str, case: &str) -> Result<(TempDir, Plan), Box<dyn Error>> {
    let dir = project(tool, case)?;
    let plan = run_plan(dir.path())?.map_err(|lines| lines.join("\n"))?;
    Ok((dir, plan))
}

/// What a byte string looks like in a failed comparison.
fn show(bytes: &[u8]) -> String {
    std::str::from_utf8(bytes).map_or_else(|_| format!("{bytes:?}"), str::to_owned)
}

/// The bytes the plan installs at each path.
pub(crate) fn proposed(plan: &Plan) -> BTreeMap<String, String> {
    plan.steps()
        .iter()
        .filter_map(|step| {
            step.new
                .as_deref()
                .map(|bytes| (step.path.as_str().to_owned(), show(bytes)))
        })
        .collect()
}

/// The checked-in expected files of `case` of `tool`, keyed by project-relative path. `skips.txt`
/// is not one.
pub(crate) fn expected(tool: &str, case: &str) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    fn walk(
        root: &Path,
        dir: &Path,
        out: &mut BTreeMap<String, String>,
    ) -> Result<(), Box<dyn Error>> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                walk(root, &entry.path(), out)?;
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(root)?
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if rel != "skips.txt" {
                out.insert(rel, show(&fs::read(entry.path())?));
            }
        }
        Ok(())
    }
    let root = fixture(tool, case).join("expected");
    let mut out = BTreeMap::new();
    walk(&root, &root, &mut out)?;
    Ok(out)
}

/// The skips of the plan as `<item>: <reason>`, sorted.
pub(crate) fn skips(plan: &Plan) -> Vec<String> {
    let mut lines: Vec<String> = plan
        .skips()
        .iter()
        .map(|skip| {
            let subject = match skip.subject() {
                SkipSubject::Item(item) => item.to_string(),
                SkipSubject::RootFile { pack, rel } => format!("{pack}/{rel}"),
            };
            format!("{subject}: {}", skip.reason())
        })
        .collect();
    lines.sort();
    lines
}
