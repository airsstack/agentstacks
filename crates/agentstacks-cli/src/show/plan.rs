//! The plan as text.
//!
//! # Responsibilities
//!
//! - [`plan_text`] renders a [`Plan`]: one row per listed step with the diff of an updated shared
//!   file under it, the skips, the airsl pin, the notes and a summary line.
//! - [`nothing_to_change`] says whether applying the plan would change nothing.
//! - [`written_and_deleted`] counts the files applying the plan writes and deletes.
//!
//! # Non-responsibilities
//!
//! Printing, colour and the confirmation belong to the commands.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use agentstacks::types::lock::write_lock;
use agentstacks::types::op::SkipSubject;
use agentstacks::types::plan::{Action, AirslPin, Plan, Skipped, Step};

use super::diff_lines;

/// The width the action column is padded to.
const ACTION_WIDTH: usize = 8;

/// The text of `plan`, lines joined by `\n` with no trailing newline.
///
/// The layout is the header, then steps and skips, then the airsl row and notes, then the summary.
/// A section with no rows is left out together with the blank line before it.
pub(crate) fn plan_text(plan: &Plan) -> String {
    let mut out = vec!["agentstacks plan".to_owned()];

    let mut changes = step_rows(plan.steps());
    changes.extend(plan.skips().iter().map(skip_row));
    push_section(&mut out, changes);

    let mut facts: Vec<String> = plan.airsl().map(airsl_row).into_iter().collect();
    facts.extend(
        plan.notes()
            .iter()
            .map(|note| row("note", &note.to_string())),
    );
    push_section(&mut out, facts);

    push_section(&mut out, vec![summary(plan)]);
    out.join("\n")
}

/// Whether applying `plan` would change nothing: no listed step, and a lock that already holds
/// the text the plan would write.
///
/// An absent lock (`None`) is never equal.
pub(crate) fn nothing_to_change(plan: &Plan, current_lock: Option<&str>) -> bool {
    plan.steps().iter().all(|step| !listed(step))
        && current_lock == Some(write_lock(plan.lock()).as_str())
}

/// Adds `rows` after a blank line, or nothing when `rows` is empty.
fn push_section(out: &mut Vec<String>, rows: Vec<String>) {
    if rows.is_empty() {
        return;
    }
    out.push(String::new());
    out.extend(rows);
}

/// A step is listed unless it leaves its path as it is.
const fn listed(step: &Step) -> bool {
    !matches!(step.action, Action::Unchanged)
}

/// `  <label padded to 8> <text>`.
fn row(label: &str, text: &str) -> String {
    format!("  {label:<ACTION_WIDTH$} {text}")
}

/// The rows of the listed steps, each `Update` of a shared file followed by its diff.
fn step_rows(steps: &[Step]) -> Vec<String> {
    let width = steps
        .iter()
        .filter(|step| listed(step))
        .map(|step| step.path.as_str().len())
        .max()
        .unwrap_or(0);
    let mut rows = Vec::new();
    for step in steps.iter().filter(|step| listed(step)) {
        let owner = step
            .owner
            .as_ref()
            .map_or_else(|| "shared".to_owned(), ToString::to_string);
        let path = step.path.as_str();
        rows.push(row(
            action_word(step.action),
            &format!("{path:<width$}  {owner}"),
        ));
        if let (Action::Update, Some(old), Some(new)) = (step.action, &step.old, &step.new) {
            rows.extend(diff_lines(path, old, new));
        }
    }
    rows
}

/// The lower-case word of a listed action.
const fn action_word(action: Action) -> &'static str {
    match action {
        Action::Create => "create",
        Action::Update => "update",
        Action::Restore => "restore",
        Action::Unchanged => "unchanged",
        Action::Delete => "delete",
        Action::Forget => "forget",
    }
}

/// `  skip     <subject>: <reason>`.
fn skip_row(skip: &Skipped) -> String {
    let subject = match skip.subject() {
        SkipSubject::Item(item) => item.to_string(),
        SkipSubject::RootFile { pack, rel } => format!("{pack}/{rel}"),
    };
    row("skip", &format!("{subject}: {}", skip.reason()))
}

/// `  airsl    <version text>`.
fn airsl_row(pin: &AirslPin) -> String {
    let text = match &pin.locked {
        Some(locked) if *locked == pin.installed => format!("{locked} (pinned)"),
        Some(locked) => format!("{locked} \u{2192} {}", pin.installed),
        None => format!("{} (new pin)", pin.installed),
    };
    row("airsl", &text)
}

/// How many steps of `plan` have one of the `wanted` actions.
fn count(plan: &Plan, wanted: &[Action]) -> usize {
    plan.steps()
        .iter()
        .filter(|step| wanted.contains(&step.action))
        .count()
}

/// How many files applying `plan` writes and how many it deletes, as the summary line counts them:
/// a create, an update or a restore is a write, and a forget is neither.
pub(crate) fn written_and_deleted(plan: &Plan) -> (usize, usize) {
    (
        count(plan, &[Action::Create, Action::Update, Action::Restore]),
        count(plan, &[Action::Delete]),
    )
}

/// The count line.
fn summary(plan: &Plan) -> String {
    let (write, delete) = written_and_deleted(plan);
    let unchanged = count(plan, &[Action::Unchanged]);
    let forgotten = count(plan, &[Action::Forget]);
    let skipped = plan.skips().len();
    let mut text =
        format!("{write} to write, {delete} to delete, {unchanged} unchanged, {skipped} skipped");
    if forgotten > 0 {
        text = format!("{text}, {forgotten} forgotten");
    }
    text
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use agentstacks::fs::StdFileTree;
    use agentstacks::types::lock::{LockContents, Lockfile, Owner};
    use agentstacks::types::manifest::PackVersion;
    use agentstacks::types::name::{ItemName, PackName};
    use agentstacks::types::op::{CoreSkip, ItemId, OpPath, Origin, SkipReason};
    use agentstacks::types::plan::Note;
    use agentstacks::types::tree::walk;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn step(
        path: &str,
        action: Action,
        owner: Option<&str>,
    ) -> Result<Step, Box<dyn std::error::Error>> {
        Ok(Step {
            path: OpPath::new(path)?,
            action,
            owner: owner.map(Owner::new).transpose()?,
            origin: None,
            new: Some(b"new\n".to_vec()),
            old: None,
            decided_on: None,
        })
    }

    fn lock() -> Result<Lockfile, Box<dyn std::error::Error>> {
        Lockfile::new(Path::new("agentstacks.lock"), LockContents::default())
            .map_err(|refusals| format!("{refusals:?}").into())
    }

    fn plan_of(
        steps: Vec<Step>,
        skips: Vec<Skipped>,
        notes: Vec<Note>,
        airsl: Option<AirslPin>,
    ) -> Result<Plan, Box<dyn std::error::Error>> {
        Ok(Plan::new(
            steps,
            skips,
            Vec::new(),
            notes,
            airsl,
            lock()?,
            Vec::new(),
        ))
    }

    fn pin(locked: Option<&str>, installed: &str) -> Result<AirslPin, Box<dyn std::error::Error>> {
        Ok(AirslPin {
            locked: locked.map(PackVersion::new).transpose()?,
            installed: PackVersion::new(installed)?,
        })
    }

    fn item_skip(reason: &str) -> Result<Skipped, Box<dyn std::error::Error>> {
        Ok(Skipped::Extension {
            item: ItemId::hook(&PackName::new("core")?, 2),
            reason: SkipReason::new(reason)?,
            origin: Origin::Core {
                pack: PackName::new("core")?,
            },
        })
    }

    #[test]
    fn spec_example_rows_render_exactly() -> TestResult {
        let mut steps = vec![
            step(
                ".claude/skills/status/SKILL.md",
                Action::Create,
                Some("core"),
            )?,
            step(
                ".claude/agentstacks/runtime.lua",
                Action::Update,
                Some("claude"),
            )?,
            step(".claude/agents/old.md", Action::Delete, Some("team-lint"))?,
        ];
        let mut settings = step(".claude/settings.json", Action::Update, None)?;
        settings.old = Some(b"{\n  \"a\": 1\n}\n".to_vec());
        settings.new = Some(b"{\n  \"a\": 2\n}\n".to_vec());
        steps.push(settings);
        for n in 0..12 {
            steps.push(step(&format!("x/{n}"), Action::Unchanged, Some("core"))?);
        }
        let skip = item_skip(
            "claude gives hooks the session id in the event (event.session_id), not as {{var session_id}}",
        )?;
        let note = Note::UnusedNative {
            dir: "cluade".parse()?,
            tool: "claude".parse()?,
        };
        let plan = plan_of(
            steps,
            vec![skip],
            vec![note],
            Some(pin(Some("0.1.4"), "0.1.4")?),
        )?;
        let expected = "\
agentstacks plan

  delete   .claude/agents/old.md            team-lint
  update   .claude/agentstacks/runtime.lua  claude
  update   .claude/settings.json            shared
    --- .claude/settings.json
    +++ .claude/settings.json
    @@ -1,3 +1,3 @@
     {
    -  \"a\": 1
    +  \"a\": 2
     }
  create   .claude/skills/status/SKILL.md   core
  skip     core/hook/2: claude gives hooks the session id in the event (event.session_id), not as {{var session_id}}

  airsl    0.1.4 (pinned)
  note     native/cluade: not for claude

3 to write, 1 to delete, 12 unchanged, 1 skipped";
        assert_eq!(plan_text(&plan), expected);
        Ok(())
    }

    #[test]
    fn root_file_skip_subject_is_pack_slash_rel() -> TestResult {
        let pack = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../agentstacks/tests/fixtures/packs/valid-hooks"
        );
        let (walked, _) = walk(&StdFileTree::new(pack));
        let rel = walked.files().first().ok_or("a fixture file")?.clone();
        assert_eq!(rel.as_str(), "hooks/enforce.lua");
        let core = Skipped::Core(CoreSkip::new(
            SkipSubject::RootFile {
                pack: PackName::new("core")?,
                rel,
            },
            SkipReason::new("why")?,
        ));
        let plan = plan_of(Vec::new(), vec![core], Vec::new(), None)?;
        assert!(
            plan_text(&plan).contains("\n  skip     core/hooks/enforce.lua: why\n"),
            "{}",
            plan_text(&plan)
        );
        Ok(())
    }

    #[test]
    fn item_skip_subject_is_the_item_id() -> TestResult {
        let skill = ItemId::skill(&PackName::new("core")?, &ItemName::new("status")?);
        let skip = Skipped::Extension {
            item: skill,
            reason: SkipReason::new("no")?,
            origin: Origin::Core {
                pack: PackName::new("core")?,
            },
        };
        let plan = plan_of(Vec::new(), vec![skip], Vec::new(), None)?;
        assert!(plan_text(&plan).contains("  skip     core/skill/status: no\n"));
        Ok(())
    }

    #[test]
    fn unchanged_steps_are_counted_not_listed() -> TestResult {
        let plan = plan_of(
            vec![step("a", Action::Unchanged, Some("core"))?],
            Vec::new(),
            Vec::new(),
            None,
        )?;
        let text = plan_text(&plan);
        assert!(!text.contains("  a "), "{text}");
        assert!(text.ends_with("0 to write, 0 to delete, 1 unchanged, 0 skipped"));
        Ok(())
    }

    #[test]
    fn path_column_pads_to_the_longest_listed_path() -> TestResult {
        let plan = plan_of(
            vec![
                step("a", Action::Create, Some("core"))?,
                step("long/path", Action::Restore, Some("core"))?,
                step("unchanged/very/long/path", Action::Unchanged, Some("core"))?,
            ],
            Vec::new(),
            Vec::new(),
            None,
        )?;
        let text = plan_text(&plan);
        assert!(text.contains("\n  create   a          core\n"), "{text}");
        assert!(text.contains("\n  restore  long/path  core\n"), "{text}");
        Ok(())
    }

    #[test]
    fn owner_none_prints_shared() -> TestResult {
        let plan = plan_of(
            vec![step("s.json", Action::Create, None)?],
            Vec::new(),
            Vec::new(),
            None,
        )?;
        assert!(plan_text(&plan).contains("  create   s.json  shared\n"));
        Ok(())
    }

    #[test]
    fn airsl_changed_prints_an_arrow() -> TestResult {
        let plan = plan_of(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some(pin(Some("0.1.4"), "0.1.5")?),
        )?;
        assert!(plan_text(&plan).contains("\n  airsl    0.1.4 \u{2192} 0.1.5\n"));
        Ok(())
    }

    #[test]
    fn airsl_new_pin_prints_new_pin() -> TestResult {
        let plan = plan_of(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some(pin(None, "0.1.5")?),
        )?;
        assert!(plan_text(&plan).contains("\n  airsl    0.1.5 (new pin)\n"));
        Ok(())
    }

    #[test]
    fn forget_count_appears_only_when_positive() -> TestResult {
        let without = plan_of(Vec::new(), Vec::new(), Vec::new(), None)?;
        assert!(!plan_text(&without).contains("forgotten"));
        let with = plan_of(
            vec![step("gone", Action::Forget, Some("core"))?],
            Vec::new(),
            Vec::new(),
            None,
        )?;
        let text = plan_text(&with);
        assert!(text.contains("\n  forget   gone  core\n"), "{text}");
        assert!(text.ends_with("0 to write, 0 to delete, 0 unchanged, 0 skipped, 1 forgotten"));
        Ok(())
    }

    #[test]
    fn empty_sections_are_left_out() -> TestResult {
        let plan = plan_of(
            vec![step("a", Action::Unchanged, Some("core"))?],
            Vec::new(),
            Vec::new(),
            None,
        )?;
        assert_eq!(
            plan_text(&plan),
            "agentstacks plan\n\n0 to write, 0 to delete, 1 unchanged, 0 skipped"
        );
        Ok(())
    }

    #[test]
    fn nothing_to_change_is_false_without_a_lock() -> TestResult {
        let plan = plan_of(Vec::new(), Vec::new(), Vec::new(), None)?;
        assert!(!nothing_to_change(&plan, None));
        Ok(())
    }

    #[test]
    fn nothing_to_change_is_false_when_lock_text_differs() -> TestResult {
        let plan = plan_of(Vec::new(), Vec::new(), Vec::new(), None)?;
        assert!(!nothing_to_change(&plan, Some("# other\n")));
        Ok(())
    }

    #[test]
    fn nothing_to_change_is_true_when_equal_and_no_listed_step() -> TestResult {
        let plan = plan_of(
            vec![step("a", Action::Unchanged, Some("core"))?],
            Vec::new(),
            Vec::new(),
            None,
        )?;
        let text = write_lock(plan.lock());
        assert!(nothing_to_change(&plan, Some(&text)));
        let listed = plan_of(
            vec![step("a", Action::Forget, Some("core"))?],
            Vec::new(),
            Vec::new(),
            None,
        )?;
        assert!(!nothing_to_change(&listed, Some(&text)));
        Ok(())
    }

    #[test]
    fn written_and_deleted_count_as_the_summary_does() -> TestResult {
        let plan = plan_of(
            vec![
                step("a", Action::Create, Some("core"))?,
                step("b", Action::Update, Some("core"))?,
                step("c", Action::Restore, Some("core"))?,
                step("d", Action::Delete, Some("core"))?,
                step("e", Action::Forget, Some("core"))?,
                step("f", Action::Unchanged, Some("core"))?,
            ],
            Vec::new(),
            Vec::new(),
            None,
        )?;
        assert_eq!(written_and_deleted(&plan), (3, 1));
        Ok(())
    }
}
