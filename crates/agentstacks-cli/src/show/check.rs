//! The `ext test` report as text.
//!
//! # Responsibilities
//!
//! - [`check_text`] renders a [`CheckReport`] into the stdout and stderr texts of `ext test`: the
//!   extension line, the Lua-test line, the plan, the `applied:` and `second plan:` lines and the
//!   verdict word last on stdout; refusals and what `airsl` printed on its standard error on
//!   stderr.
//!
//! # Non-responsibilities
//!
//! Running the flow, writing to a stream and choosing the exit code belong to the command.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use agentstacks::app::ext::{CheckReport, LuaOutcome};

use super::{PlanView, plan_text, written_and_deleted};

/// The two texts of a report, lines joined by `\n` with no trailing newline; a stream with
/// nothing to say is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckText {
    /// What goes to stdout.
    pub(crate) out: String,
    /// What goes to stderr.
    pub(crate) err: String,
}

/// Renders `report` in the order of the run.
///
/// A run that ended with `airsl` printing nothing on success shows `lua tests: passed`, because
/// there is no last line to quote.
pub(crate) fn check_text(report: CheckReport) -> CheckText {
    let CheckReport {
        extension,
        lua,
        plan,
        second,
        not_repeatable,
        refusals,
        verdict,
    } = report;
    let mut out = Vec::new();
    let mut err = Vec::new();

    if let Some(checked) = extension {
        out.push(format!("ext test {} {}", checked.name, checked.version));
    }
    match lua {
        None => {}
        Some(LuaOutcome::None) => out.push("lua tests: none".to_owned()),
        Some(LuaOutcome::Passed { summary }) => out.push(format!(
            "lua tests: {}",
            summary.as_deref().unwrap_or("passed")
        )),
        Some(LuaOutcome::Failed { stdout, stderr }) => {
            out.push("lua tests: failed".to_owned());
            push_text(&mut out, &stdout);
            push_text(&mut err, &stderr);
        }
        Some(LuaOutcome::NotStarted(diagnostic)) => {
            out.push("lua tests: failed".to_owned());
            err.push(diagnostic.to_string());
        }
    }
    if let Some(first) = &plan {
        out.push(plan_text(first.plan(), PlanView::Full));
        if first.applied() {
            let (written, deleted) = written_and_deleted(first.plan());
            out.push(format!("applied: {written} written, {deleted} deleted"));
        }
    }
    match (second, not_repeatable) {
        (Some(_), None) => out.push("second plan: nothing to change".to_owned()),
        (second, Some(refusal)) => {
            out.push(refusal.to_string());
            if let Some(second) = &second {
                out.push(plan_text(second, PlanView::Full));
            }
        }
        (None, None) => {}
    }
    err.extend(refusals.into_sorted().iter().map(ToString::to_string));
    if let Some(verdict) = verdict {
        out.push(verdict.to_string());
    }
    CheckText {
        out: out.join("\n"),
        err: err.join("\n"),
    }
}

/// Adds the lines of `text` without its trailing newline; nothing for an empty text.
fn push_text(lines: &mut Vec<String>, text: &str) {
    let text = text.trim_end_matches('\n');
    if !text.is_empty() {
        lines.push(text.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use agentstacks::app::ext::{Checked, FirstPlan};
    use agentstacks::types::check::{NotRepeatable, Verdict};
    use agentstacks::types::lock::{LockContents, Lockfile, Owner};
    use agentstacks::types::manifest::PackVersion;
    use agentstacks::types::name::ToolName;
    use agentstacks::types::op::OpPath;
    use agentstacks::types::plan::{Action, Plan, Step};
    use agentstacks::types::report::{Diagnostic, Diagnostics, FieldError, Location};

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const EMPTY_PLAN: &str = "agentstacks plan\n\n0 to write, 0 to delete, 0 unchanged, 0 skipped";

    fn plan_of(steps: Vec<Step>) -> Result<Plan, Box<dyn std::error::Error>> {
        let lock = Lockfile::new(Path::new("agentstacks.lock"), LockContents::default())
            .map_err(|refusals| format!("{refusals:?}"))?;
        Ok(Plan::new(
            steps,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            lock,
            Vec::new(),
        ))
    }

    fn create(path: &str) -> Result<Step, Box<dyn std::error::Error>> {
        Ok(Step {
            path: OpPath::new(path)?,
            action: Action::Create,
            owner: Some(Owner::new("demo")?),
            origin: None,
            new: Some(b"x".to_vec()),
            old: None,
            decided_on: None,
        })
    }

    fn checked() -> Result<Checked, Box<dyn std::error::Error>> {
        Ok(Checked {
            name: ToolName::new("demo")?,
            version: PackVersion::new("1.2.0")?,
        })
    }

    #[test]
    fn check_text_passed() -> TestResult {
        let report = CheckReport {
            extension: Some(checked()?),
            lua: Some(LuaOutcome::Passed {
                summary: Some("71 passed, 0 failed (5 files)".to_owned()),
            }),
            plan: Some(FirstPlan::Applied(plan_of(vec![
                create("a")?,
                create("b")?,
            ])?)),
            second: Some(plan_of(Vec::new())?),
            verdict: Some(Verdict::Passed),
            ..CheckReport::default()
        };
        let text = check_text(report);
        let expected = "ext test demo 1.2.0\n\
             lua tests: 71 passed, 0 failed (5 files)\n\
             agentstacks plan\n\
             \n  create   a  demo\n  create   b  demo\n\
             \n2 to write, 0 to delete, 0 unchanged, 0 skipped\n\
             applied: 2 written, 0 deleted\n\
             second plan: nothing to change\n\
             passed";
        assert_eq!(text.out, expected);
        assert_eq!(text.err, "");
        Ok(())
    }

    #[test]
    fn check_text_failed() -> TestResult {
        let report = CheckReport {
            extension: Some(checked()?),
            lua: Some(LuaOutcome::Failed {
                stdout: "1 failed\n".to_owned(),
                stderr: "boom\n".to_owned(),
            }),
            plan: Some(FirstPlan::Applied(plan_of(Vec::new())?)),
            second: Some(plan_of(vec![create("a")?])?),
            not_repeatable: Some(NotRepeatable {
                write: 1,
                delete: 0,
                steps: 1,
            }),
            verdict: Some(Verdict::Failed),
            ..CheckReport::default()
        };
        let text = check_text(report);
        let expected = format!(
            "ext test demo 1.2.0\n\
             lua tests: failed\n\
             1 failed\n\
             {EMPTY_PLAN}\n\
             applied: 0 written, 0 deleted\n\
             second plan: 1 to write, 0 to delete (apply is not repeatable)\n\
             agentstacks plan\n\
             \n  create   a  demo\n\
             \n1 to write, 0 to delete, 0 unchanged, 0 skipped\n\
             failed"
        );
        assert_eq!(text.out, expected);
        assert_eq!(text.err, "boom");
        Ok(())
    }

    #[test]
    fn check_text_a_plan_that_was_not_applied_has_no_applied_line() -> TestResult {
        let text = check_text(CheckReport {
            plan: Some(FirstPlan::Planned(plan_of(Vec::new())?)),
            verdict: Some(Verdict::Failed),
            ..CheckReport::default()
        });
        assert_eq!(text.out, format!("{EMPTY_PLAN}\nfailed"));
        Ok(())
    }

    #[test]
    fn check_text_stopped_has_no_verdict() {
        let mut refusals = Diagnostics::new();
        refusals.push(Diagnostic::new(
            Location::file("ext/x"),
            FieldError::Required,
        ));
        let text = check_text(CheckReport {
            refusals,
            ..CheckReport::default()
        });
        assert_eq!(text.out, "");
        assert_eq!(text.err, "ext/x: required");
    }

    #[test]
    fn check_text_shows_none_and_silent_passes() -> TestResult {
        let none = check_text(CheckReport {
            extension: Some(checked()?),
            lua: Some(LuaOutcome::None),
            ..CheckReport::default()
        });
        assert_eq!(none.out, "ext test demo 1.2.0\nlua tests: none");
        let silent = check_text(CheckReport {
            lua: Some(LuaOutcome::Passed { summary: None }),
            ..CheckReport::default()
        });
        assert_eq!(silent.out, "lua tests: passed");
        Ok(())
    }

    #[test]
    fn check_text_unstartable_airsl_goes_to_stderr() {
        let diagnostic = Diagnostic::new(Location::file("ext test"), FieldError::Required);
        let expected = diagnostic.to_string();
        let text = check_text(CheckReport {
            lua: Some(LuaOutcome::NotStarted(diagnostic)),
            verdict: Some(Verdict::Failed),
            ..CheckReport::default()
        });
        assert_eq!(text.out, "lua tests: failed\nfailed");
        assert_eq!(text.err, expected);
    }
}
