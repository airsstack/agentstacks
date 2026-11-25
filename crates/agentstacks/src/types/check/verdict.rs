//! The pass or fail of an `ext test` run, and the check that a second plan has nothing to do.
//!
//! # Responsibilities
//!
//! - [`second_plan`] decides whether applying twice is safe: the plan made right after an apply
//!   must hold no step that writes, deletes or forgets. A skip is an extension saying it cannot
//!   express an item, so it never fails the check.
//! - [`NotRepeatable`] is the refusal, with the counts its message shows.
//! - [`Verdict`] is the last word of a run, [`Verdict::of`] chosen by how many steps failed.
//!
//! # Non-responsibilities
//!
//! Making the plans and applying the first one belong to the flow; showing the plan's steps
//! belongs to the command line.

use std::fmt;

use crate::types::plan::{Action, Plan};

/// A second plan that still has work to do.
///
/// # Examples
///
/// ```
/// use agentstacks::types::check::NotRepeatable;
///
/// let refusal = NotRepeatable { write: 2, delete: 1, steps: 3 };
/// assert_eq!(
///     refusal.to_string(),
///     "second plan: 2 to write, 1 to delete (apply is not repeatable)"
/// );
/// let forgotten = NotRepeatable { write: 0, delete: 0, steps: 1 };
/// assert_eq!(
///     forgotten.to_string(),
///     "second plan: 0 to write, 0 to delete, 1 forgotten (apply is not repeatable)"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("second plan: {write} to write, {delete} to delete{} (apply is not repeatable)", forgotten(*.write, *.delete, *.steps))]
pub struct NotRepeatable {
    /// How many steps create, update or restore a file.
    pub write: usize,
    /// How many steps delete a file.
    pub delete: usize,
    /// How many steps are not `Unchanged`: the writes, the deletes and the forgets.
    pub steps: usize,
}

/// `, <n> forgotten` for the steps that are neither writes nor deletes, or nothing when there are
/// none, as the plan summary words it.
fn forgotten(write: usize, delete: usize, steps: usize) -> String {
    match steps.saturating_sub(write + delete) {
        0 => String::new(),
        forgotten => format!(", {forgotten} forgotten"),
    }
}

/// Whether a plan made right after an apply has nothing to change.
///
/// # Errors
///
/// A [`NotRepeatable`] when any step creates, updates, restores, deletes or forgets a path.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use agentstacks::types::check::second_plan;
/// use agentstacks::types::lock::{LockContents, Lockfile};
/// use agentstacks::types::plan::Plan;
///
/// let lock = Lockfile::new(Path::new("agentstacks.lock"), LockContents::default())
///     .map_err(|refusals| format!("{refusals:?}"))?;
/// let plan = Plan::new(vec![], vec![], vec![], vec![], None, lock, vec![]);
/// assert!(second_plan(&plan).is_ok());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn second_plan(plan: &Plan) -> Result<(), NotRepeatable> {
    let count = |wanted: fn(Action) -> bool| {
        plan.steps()
            .iter()
            .filter(|step| wanted(step.action))
            .count()
    };
    let write = count(Action::writes);
    let delete = count(|action| action == Action::Delete);
    let steps = count(|action| action != Action::Unchanged);
    if steps == 0 {
        Ok(())
    } else {
        Err(NotRepeatable {
            write,
            delete,
            steps,
        })
    }
}

/// How an `ext test` run ended.
///
/// # Examples
///
/// ```
/// use agentstacks::types::check::Verdict;
///
/// assert_eq!(Verdict::Passed.to_string(), "passed");
/// assert_eq!(Verdict::Failed.to_string(), "failed");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Every step that ran passed.
    Passed,
    /// A step failed.
    Failed,
}

impl Verdict {
    /// The verdict of a run in which `failures` steps failed.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::check::Verdict;
    ///
    /// assert_eq!(Verdict::of(0), Verdict::Passed);
    /// assert_eq!(Verdict::of(2), Verdict::Failed);
    /// ```
    #[must_use]
    pub const fn of(failures: usize) -> Self {
        if failures == 0 {
            Self::Passed
        } else {
            Self::Failed
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::lock::{LockContents, Lockfile};
    use crate::types::name::{ItemName, PackName, ToolName};
    use crate::types::op::{ItemId, OpPath, Origin, SkipReason};
    use crate::types::plan::{Skipped, Step};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn step(path: &str, action: Action) -> Result<Step, Box<dyn std::error::Error>> {
        Ok(Step {
            path: OpPath::new(path)?,
            action,
            owner: None,
            origin: None,
            new: None,
            old: None,
            decided_on: None,
        })
    }

    fn plan(steps: Vec<Step>, skips: Vec<Skipped>) -> Result<Plan, Box<dyn std::error::Error>> {
        let lock = Lockfile::new(Path::new("agentstacks.lock"), LockContents::default())
            .map_err(|refusals| format!("{refusals:?}"))?;
        Ok(Plan::new(
            steps,
            skips,
            Vec::new(),
            Vec::new(),
            None,
            lock,
            Vec::new(),
        ))
    }

    #[test]
    fn second_plan_write_fails() -> TestResult {
        let plan = plan(
            vec![
                step("a", Action::Create)?,
                step("b", Action::Update)?,
                step("c", Action::Restore)?,
                step("d", Action::Unchanged)?,
            ],
            Vec::new(),
        )?;
        let refusal = second_plan(&plan).err().ok_or("a write must fail")?;
        assert_eq!(
            refusal,
            NotRepeatable {
                write: 3,
                delete: 0,
                steps: 3
            }
        );
        assert_eq!(
            refusal.to_string(),
            "second plan: 3 to write, 0 to delete (apply is not repeatable)"
        );
        Ok(())
    }

    #[test]
    fn second_plan_delete_fails() -> TestResult {
        let plan = plan(
            vec![step("a", Action::Delete)?, step("b", Action::Unchanged)?],
            Vec::new(),
        )?;
        let refusal = second_plan(&plan).err().ok_or("a delete must fail")?;
        assert_eq!(
            refusal.to_string(),
            "second plan: 0 to write, 1 to delete (apply is not repeatable)"
        );
        Ok(())
    }

    #[test]
    fn second_plan_forget_fails() -> TestResult {
        let plan = plan(vec![step("a", Action::Forget)?], Vec::new())?;
        let refusal = second_plan(&plan).err().ok_or("a forget must fail")?;
        assert_eq!(
            refusal.to_string(),
            "second plan: 0 to write, 0 to delete, 1 forgotten (apply is not repeatable)"
        );
        assert_eq!(
            refusal,
            NotRepeatable {
                write: 0,
                delete: 0,
                steps: 1
            }
        );
        Ok(())
    }

    #[test]
    fn skips_do_not_fail() -> TestResult {
        let skip = Skipped::Extension {
            item: ItemId::agent(&PackName::new("conformance")?, &ItemName::new("checker")?),
            reason: SkipReason::new("not for this tool")?,
            origin: Origin::Tool {
                tool: ToolName::new("mytool")?,
            },
        };
        let plan = plan(
            vec![step("a", Action::Unchanged)?, step("b", Action::Unchanged)?],
            vec![skip],
        )?;
        assert_eq!(second_plan(&plan), Ok(()));
        Ok(())
    }

    #[test]
    fn verdict_of_counts_failed_steps() {
        assert_eq!(Verdict::of(0), Verdict::Passed);
        assert_eq!(Verdict::of(1), Verdict::Failed);
        assert_eq!(Verdict::of(3), Verdict::Failed);
    }

    #[test]
    fn verdict_shows_its_word() {
        assert_eq!(Verdict::Passed.to_string(), "passed");
        assert_eq!(Verdict::Failed.to_string(), "failed");
    }
}
