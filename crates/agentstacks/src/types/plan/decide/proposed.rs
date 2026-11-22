//! The step for a path an extension proposed.
//!
//! # Responsibilities
//!
//! - [`ProposedPath`] gathers everything the decision needs: the new bytes, who proposed them,
//!   the derived owner, what a read returned for a shared path, the lock's hash for the path,
//!   and where the path leads on disk.
//! - [`decide_proposed`] answers with a [`Step`], or with the refusal located at the path.
//!   A proposed path that is a symlink inside the project is decided on its target's state,
//!   because the write goes through it.
//!
//! # Non-responsibilities
//!
//! Reading the disk and the lock, and the order in which paths are decided, belong to the plan
//! flow.

use crate::types::lock::Owner;
use crate::types::op::{OpPath, Origin};
use crate::types::plan::{Action, DiskFile, DiskState, ReadOutcome, Resolution, Step, StepError};
use crate::types::report::{Diagnostic, Location};
use crate::types::tree::ContentHash;

/// A proposed path and the facts its step is decided on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedPath {
    /// The project-relative path.
    pub path: OpPath,
    /// The bytes the extension proposes.
    pub new: Vec<u8>,
    /// Who proposed them.
    pub origin: Origin,
    /// The owner derived for the proposal. Ignored for a shared path, which has none.
    pub owner: Option<Owner>,
    /// What the read of this path returned, when the path is shared with the user.
    pub shared: Option<ReadOutcome>,
    /// The hash the lock records for this path, when it has a file entry.
    pub locked: Option<ContentHash>,
    /// Where the path leads on disk.
    pub resolution: Resolution,
}

/// The refusal of `path` for `error`.
pub(super) fn refuse(path: &OpPath, error: StepError) -> Diagnostic {
    Diagnostic::new(Location::file(path.as_str()), error)
}

/// Decides the step for a proposed path.
///
/// # Errors
///
/// The refusal, located at the path: it resolves outside the project, it is not a regular file,
/// a shared file changed after it was read, or an unshared file differs from what agentstacks
/// wrote or does not belong to it.
pub fn decide_proposed(input: ProposedPath) -> Result<Step, Diagnostic> {
    let ProposedPath {
        path,
        new,
        origin,
        owner,
        shared,
        locked,
        resolution,
    } = input;
    let state = match resolution {
        Resolution::Outside { .. } => return Err(refuse(&path, StepError::Outside)),
        Resolution::Inside {
            state: DiskState::NotRegular,
            ..
        } => return Err(refuse(&path, StepError::NotRegular)),
        Resolution::Inside { state, .. } => state,
    };
    let (action, owner, old) = match (shared, locked) {
        (Some(read), _) => {
            let (action, old) = decide_shared(&read, &state, &new)
                .ok_or_else(|| refuse(&path, StepError::ChangedAfterRead))?;
            (action, None, old)
        }
        (None, Some(lock_hash)) => {
            let action = decide_locked(&lock_hash, &state, &new)
                .ok_or_else(|| refuse(&path, StepError::Changed))?;
            (action, owner, None)
        }
        (None, None) => {
            let action =
                decide_unlocked(&state, &new).ok_or_else(|| refuse(&path, StepError::NotOurs))?;
            (action, owner, None)
        }
    };
    Ok(Step {
        path,
        action,
        owner,
        origin: Some(origin),
        new: Some(new),
        old,
        decided_on: Some(state),
    })
}

/// The action for `new` over a file `read` returned, and the bytes it replaces; `None` when the
/// disk no longer matches what the read returned.
fn decide_shared(
    read: &ReadOutcome,
    state: &DiskState,
    new: &[u8],
) -> Option<(Action, Option<Vec<u8>>)> {
    match (read, state) {
        (ReadOutcome::Absent, DiskState::Absent) => Some((Action::Create, None)),
        (ReadOutcome::Bytes(read_hash), DiskState::File(file)) if file.hash() == read_hash => {
            Some((write_or_unchanged(file, new), Some(file.bytes().to_vec())))
        }
        _ => None,
    }
}

/// The action for `new` over a path the lock records with `lock_hash`; `None` when the file was
/// edited and differs from `new`.
fn decide_locked(lock_hash: &ContentHash, state: &DiskState, new: &[u8]) -> Option<Action> {
    match state {
        DiskState::Absent => Some(Action::Restore),
        DiskState::File(file) if file.hash() == lock_hash => Some(write_or_unchanged(file, new)),
        DiskState::File(file) if file.bytes() == new => Some(Action::Unchanged),
        DiskState::File(_) | DiskState::NotRegular => None,
    }
}

/// The action for `new` over a path the lock does not record; `None` when another file is
/// there.
fn decide_unlocked(state: &DiskState, new: &[u8]) -> Option<Action> {
    match state {
        DiskState::Absent => Some(Action::Create),
        DiskState::File(file) if file.bytes() == new => Some(Action::Unchanged),
        DiskState::File(_) | DiskState::NotRegular => None,
    }
}

/// An update, or nothing to do when `file` already holds `new`.
fn write_or_unchanged(file: &DiskFile, new: &[u8]) -> Action {
    if file.bytes() == new {
        Action::Unchanged
    } else {
        Action::Update
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::ToolName;
    use crate::types::plan::Link;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const X: &str = ".claude/x";
    const SETTINGS: &str = ".claude/settings.json";

    fn base(
        path: &str,
        resolution: Resolution,
    ) -> Result<ProposedPath, Box<dyn std::error::Error>> {
        Ok(ProposedPath {
            path: OpPath::new(path)?,
            new: b"new".to_vec(),
            origin: Origin::Tool {
                tool: ToolName::new("claude")?,
            },
            owner: Some(Owner::new("core")?),
            shared: None,
            locked: None,
            resolution,
        })
    }

    fn file(bytes: &[u8]) -> DiskState {
        DiskState::File(DiskFile::new(bytes.to_vec(), 0o644))
    }

    fn inside(state: DiskState) -> Resolution {
        Resolution::Inside {
            link: Link::Direct,
            state,
        }
    }

    fn hash(bytes: &[u8]) -> ContentHash {
        ContentHash::of_bytes(bytes)
    }

    fn refused(input: ProposedPath) -> Result<String, Box<dyn std::error::Error>> {
        Ok(decide_proposed(input)
            .err()
            .ok_or("the path was accepted")?
            .to_string())
    }

    fn accepted(input: ProposedPath) -> Result<Step, Box<dyn std::error::Error>> {
        decide_proposed(input).map_err(|refusal| refusal.to_string().into())
    }

    #[test]
    fn proposed_outside_is_refused() -> TestResult {
        let input = base(X, Resolution::Outside { link: Link::Direct })?;
        assert_eq!(refused(input)?, ".claude/x: resolves outside the project");
        Ok(())
    }

    #[test]
    fn proposed_not_regular_is_refused() -> TestResult {
        let plain = base(X, inside(DiskState::NotRegular))?;
        let shared = ProposedPath {
            shared: Some(ReadOutcome::Absent),
            owner: None,
            ..plain.clone()
        };
        let locked = ProposedPath {
            locked: Some(hash(b"old")),
            ..plain.clone()
        };
        for input in [plain, shared, locked] {
            assert_eq!(refused(input)?, ".claude/x: is not a regular file");
        }
        Ok(())
    }

    #[test]
    fn shared_absent_read_absent_creates() -> TestResult {
        let input = ProposedPath {
            shared: Some(ReadOutcome::Absent),
            ..base(SETTINGS, inside(DiskState::Absent))?
        };
        let step = accepted(input)?;
        assert_eq!(step.action, Action::Create);
        assert_eq!(step.owner, None);
        assert_eq!(step.old, None);
        assert_eq!(step.new.as_deref(), Some(&b"new"[..]));
        Ok(())
    }

    #[test]
    fn shared_equal_to_read_updates_with_old_bytes() -> TestResult {
        let input = ProposedPath {
            shared: Some(ReadOutcome::Bytes(hash(b"user"))),
            ..base(SETTINGS, inside(file(b"user")))?
        };
        let step = accepted(input)?;
        assert_eq!(step.action, Action::Update);
        assert_eq!(step.old.as_deref(), Some(&b"user"[..]));
        assert_eq!(step.owner, None);
        Ok(())
    }

    #[test]
    fn shared_equal_to_read_and_new_is_unchanged() -> TestResult {
        let input = ProposedPath {
            shared: Some(ReadOutcome::Bytes(hash(b"new"))),
            ..base(SETTINGS, inside(file(b"new")))?
        };
        let step = accepted(input)?;
        assert_eq!(step.action, Action::Unchanged);
        assert_eq!(step.owner, None);
        Ok(())
    }

    #[test]
    fn shared_changed_after_read_is_refused() -> TestResult {
        let message = ".claude/settings.json: changed after it was read; run plan again";
        let cases = [
            (inside(DiskState::Absent), ReadOutcome::Bytes(hash(b"user"))),
            (inside(file(b"user")), ReadOutcome::Absent),
            (inside(file(b"user")), ReadOutcome::Bytes(hash(b"other"))),
        ];
        for (resolution, read) in cases {
            let input = ProposedPath {
                shared: Some(read),
                ..base(SETTINGS, resolution)?
            };
            assert_eq!(refused(input)?, message);
        }
        Ok(())
    }

    #[test]
    fn locked_absent_restores() -> TestResult {
        let input = ProposedPath {
            locked: Some(hash(b"old")),
            ..base(X, inside(DiskState::Absent))?
        };
        let step = accepted(input)?;
        assert_eq!(step.action, Action::Restore);
        assert_eq!(step.owner, Some(Owner::new("core")?));
        Ok(())
    }

    #[test]
    fn locked_hash_equal_updates() -> TestResult {
        let update = ProposedPath {
            locked: Some(hash(b"old")),
            ..base(X, inside(file(b"old")))?
        };
        assert_eq!(accepted(update)?.action, Action::Update);
        let same = ProposedPath {
            locked: Some(hash(b"new")),
            ..base(X, inside(file(b"new")))?
        };
        assert_eq!(accepted(same)?.action, Action::Unchanged);
        Ok(())
    }

    #[test]
    fn locked_hash_differs_bytes_equal_new_is_unchanged() -> TestResult {
        let input = ProposedPath {
            locked: Some(hash(b"old")),
            ..base(X, inside(file(b"new")))?
        };
        assert_eq!(accepted(input)?.action, Action::Unchanged);
        Ok(())
    }

    #[test]
    fn locked_hash_differs_bytes_differ_is_refused() -> TestResult {
        let input = ProposedPath {
            locked: Some(hash(b"old")),
            ..base(X, inside(file(b"edited")))?
        };
        assert_eq!(refused(input)?, ".claude/x: changed since the last apply");
        Ok(())
    }

    #[test]
    fn unlocked_absent_creates() -> TestResult {
        let step = accepted(base(X, inside(DiskState::Absent))?)?;
        assert_eq!(step.action, Action::Create);
        assert_eq!(step.owner, Some(Owner::new("core")?));
        Ok(())
    }

    #[test]
    fn unlocked_equal_bytes_is_adopted_unchanged() -> TestResult {
        let step = accepted(base(X, inside(file(b"new")))?)?;
        assert_eq!(step.action, Action::Unchanged);
        assert_eq!(step.owner, Some(Owner::new("core")?));
        Ok(())
    }

    #[test]
    fn unlocked_different_bytes_is_refused() -> TestResult {
        let input = base(X, inside(file(b"mine")))?;
        assert_eq!(
            refused(input)?,
            ".claude/x: exists and was not written by agentstacks"
        );
        Ok(())
    }

    #[test]
    fn proposed_inside_symlink_uses_the_target_state() -> TestResult {
        let input = base(
            X,
            Resolution::Inside {
                link: Link::Symlink,
                state: DiskState::Absent,
            },
        )?;
        assert_eq!(accepted(input)?.action, Action::Create);
        Ok(())
    }

    #[test]
    fn step_keeps_disk_state_it_was_decided_on() -> TestResult {
        let accepted_rows = [
            ProposedPath {
                shared: Some(ReadOutcome::Bytes(hash(b"user"))),
                ..base(SETTINGS, inside(file(b"user")))?
            },
            ProposedPath {
                locked: Some(hash(b"old")),
                ..base(X, inside(DiskState::Absent))?
            },
            base(X, inside(file(b"new")))?,
            base(X, inside(DiskState::Absent))?,
        ];
        for input in accepted_rows {
            let expected = match &input.resolution {
                Resolution::Inside { state, .. } => state.clone(),
                Resolution::Outside { .. } => return Err("an inside path".into()),
            };
            assert_eq!(accepted(input)?.decided_on, Some(expected));
        }
        Ok(())
    }

    // Examples with the paths of real packs.

    const SKILL: &str = ".claude/skills/status/SKILL.md";

    #[test]
    fn example_first_apply_creates() -> TestResult {
        let step = accepted(base(SKILL, inside(DiskState::Absent))?)?;
        assert_eq!(step.action, Action::Create);
        Ok(())
    }

    #[test]
    fn example_user_edit_is_refused() -> TestResult {
        let input = ProposedPath {
            locked: Some(hash(b"new")),
            ..base(SKILL, inside(file(b"edited by the user")))?
        };
        assert_eq!(
            refused(input)?,
            ".claude/skills/status/SKILL.md: changed since the last apply"
        );
        Ok(())
    }

    #[test]
    fn example_user_delete_restores() -> TestResult {
        let input = ProposedPath {
            locked: Some(hash(b"new")),
            ..base(SKILL, inside(DiskState::Absent))?
        };
        assert_eq!(accepted(input)?.action, Action::Restore);
        Ok(())
    }

    #[test]
    fn example_settings_shared_update() -> TestResult {
        let user = br#"{"permissions":{}}"#;
        let input = ProposedPath {
            new: br#"{"permissions":{},"hooks":{}}"#.to_vec(),
            shared: Some(ReadOutcome::Bytes(hash(user))),
            ..base(SETTINGS, inside(file(user)))?
        };
        let step = accepted(input)?;
        assert_eq!(step.action, Action::Update);
        assert_eq!(step.old.as_deref(), Some(&user[..]));
        assert_eq!(step.owner, None);
        Ok(())
    }

    #[test]
    fn example_settings_absent_shared_create() -> TestResult {
        let input = ProposedPath {
            shared: Some(ReadOutcome::Absent),
            ..base(SETTINGS, inside(DiskState::Absent))?
        };
        let step = accepted(input)?;
        assert_eq!(step.action, Action::Create);
        assert_eq!(step.owner, None);
        assert_eq!(step.decided_on, Some(DiskState::Absent));
        Ok(())
    }

    #[test]
    fn example_foreign_agent_is_refused() -> TestResult {
        let input = base(".claude/agents/coder.md", inside(file(b"someone else's")))?;
        assert_eq!(
            refused(input)?,
            ".claude/agents/coder.md: exists and was not written by agentstacks"
        );
        Ok(())
    }

    #[test]
    fn example_symlink_to_etc_hosts_is_refused() -> TestResult {
        let input = base(
            ".claude/hosts",
            Resolution::Outside {
                link: Link::Symlink,
            },
        )?;
        assert_eq!(
            refused(input)?,
            ".claude/hosts: resolves outside the project"
        );
        Ok(())
    }
}
