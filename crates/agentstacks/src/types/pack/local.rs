//! The rules over the tokens a pack kept that need only the pack itself.
//!
//! The tokens come from frontmatter values, hook fields, Markdown bodies and `render` files, so
//! each rule reads a token's [`Location`](crate::types::Location) and target and never the text it came from.
//!
//! It is separate from the loader because these rules need only the kept tokens and the layout, so
//! they run once after every file has been scanned.
//!
//! # Responsibilities
//!
//! - [`check_native_references`] refuses a `root` token followed by `/native/` in neutral content,
//!   that is, in any file outside `<dir>/native/`. The refusal sits at the token's own location:
//!   `<file>:<line>` in a body or a `render` file, `<file>: <key.path>` in a frontmatter value or a
//!   hook field.
//! - [`check_local_tokens`] refuses `{{var arguments}}` located anywhere but in a
//!   `skills/<name>/SKILL.md` of the pack, and a `skill`, `skill_dir` or `agent` token that names
//!   this pack (no pack, or its own name) but an item the pack does not ship. An item the walk
//!   refused is not reported as not shipped.
//!
//! # Non-responsibilities
//!
//! Tokens that name another pack, `option` tokens and the airsl and hook `run` rules over several
//! packs belong to the checks over a whole set of packs.

use std::path::Path;

use crate::types::name::ItemName;
use crate::types::name::PackName;
use crate::types::pack::error::PackError;
use crate::types::pack::item::kind::ItemKind;
use crate::types::pack::layout::{NATIVE_DIR, PackLayout, SKILL_FILE, SKILLS_DIR};
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::tokens::ItemRef;
use crate::types::tokens::LocatedToken;
use crate::types::tokens::TokenTarget;
use crate::types::tokens::Var;
use crate::types::tokens::names_own_pack;

/// Refuses each token of neutral content (outside `<dir>/native/`) that reaches into `native/`.
#[must_use]
pub(super) fn check_native_references(dir: &Path, tokens: &[LocatedToken]) -> Diagnostics {
    let native = dir.join(NATIVE_DIR);
    let mut refusals = Diagnostics::new();
    refusals.extend(
        tokens
            .iter()
            .filter(|found| found.token().reaches_native())
            .filter(|found| {
                found
                    .location()
                    .file_path()
                    .is_none_or(|file| !file.starts_with(&native))
            })
            .map(|found| Diagnostic::new(found.location().clone(), PackError::NativeReference)),
    );
    refusals
}

/// Applies the placement and own-pack target rules to every token of the pack `pack` in the
/// directory `dir`.
#[must_use]
pub(super) fn check_local_tokens(
    dir: &Path,
    pack: &PackName,
    layout: &PackLayout,
    tokens: &[LocatedToken],
) -> Diagnostics {
    let mut refusals = Diagnostics::new();
    for found in tokens {
        let error = match found.token().target() {
            TokenTarget::Var(Var::Arguments) if !in_skill_file(dir, found) => {
                Some(PackError::ArgumentsOutsideSkill)
            }
            TokenTarget::Skill(item) | TokenTarget::SkillDir(item) => {
                missing(pack, item, ItemKind::Skill, |name| layout.lacks_skill(name))
            }
            TokenTarget::Agent(item) => {
                missing(pack, item, ItemKind::Agent, |name| layout.lacks_agent(name))
            }
            _ => None,
        };
        if let Some(error) = error {
            refusals.push(Diagnostic::new(found.location().clone(), error));
        }
    }
    refusals
}

/// Whether the token sits in a `skills/<name>/SKILL.md` of the pack in `dir`.
fn in_skill_file(dir: &Path, found: &LocatedToken) -> bool {
    let Some(relative) = found
        .location()
        .file_path()
        .and_then(|file| file.strip_prefix(dir).ok())
    else {
        return false;
    };
    let parts: Vec<Option<&str>> = relative
        .components()
        .map(|part| part.as_os_str().to_str())
        .collect();
    matches!(
        parts.as_slice(),
        [Some(SKILLS_DIR), Some(_), Some(SKILL_FILE)]
    )
}

/// The refusal for `item` when it names the pack `pack` itself but `lacks` says the pack has no
/// such item.
fn missing(
    pack: &PackName,
    item: &ItemRef,
    kind: ItemKind,
    lacks: impl Fn(&ItemName) -> bool,
) -> Option<PackError> {
    (names_own_pack(item.pack(), pack) && lacks(item.item())).then(|| PackError::NoSuchItem {
        pack: pack.clone(),
        kind,
        item: item.item().clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::pack::testing::{DIR, layout, shown};
    use crate::types::tokens::TextOrigin;
    use crate::types::tokens::scan;
    use std::error::Error;
    use std::num::NonZeroUsize;
    use std::path::PathBuf;

    type TestResult = Result<(), Box<dyn Error>>;

    fn at_line(file: &str, line: usize, text: &str) -> Result<Vec<LocatedToken>, Box<dyn Error>> {
        let first_line = NonZeroUsize::new(line).ok_or("line must be positive")?;
        let origin = TextOrigin::Lines {
            file: PathBuf::from(file),
            first_line,
        };
        let (tokens, refused) = scan(text, &origin);
        assert!(refused.is_empty(), "{}", shown(refused).join("; "));
        Ok(tokens)
    }

    fn at_key(file: &str, key: &str, text: &str) -> Vec<LocatedToken> {
        let origin = TextOrigin::Field {
            file: PathBuf::from(file),
            key_path: key.to_owned(),
        };
        scan(text, &origin).0
    }

    fn core() -> Result<PackName, Box<dyn Error>> {
        Ok(PackName::new("core")?)
    }

    #[test]
    fn native_reference_is_refused_after_a_root_token() -> TestResult {
        let file = format!("{DIR}/skills/a/SKILL.md");
        for (text, refused) in [
            ("{{root}}/native/claude/x", true),
            ("{{root core}}/native/x", true),
            ("{{root}}/natives/x", false),
            ("{{root}}/hooks/native/x", false),
            ("{{skill_dir a}}/native/x", false),
        ] {
            let found = check_native_references(Path::new(DIR), &at_line(&file, 3, text)?);
            let expected: Vec<String> = if refused {
                vec![format!(
                    "{file}:3: neutral content must not reference native/"
                )]
            } else {
                Vec::new()
            };
            assert_eq!(shown(found), expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn native_reference_in_frontmatter_keeps_its_key_path() {
        let file = format!("{DIR}/skills/a/SKILL.md");
        let tokens = at_key(&file, "description", "see {{root}}/native/claude/x");
        assert_eq!(
            shown(check_native_references(Path::new(DIR), &tokens)),
            [format!(
                "{file}: description: neutral content must not reference native/"
            )]
        );
    }

    #[test]
    fn native_reference_in_a_hook_field_keeps_its_key_path() {
        let file = format!("{DIR}/pack.toml");
        let tokens = at_key(&file, "hooks[0].args[0]", "{{root}}/native/claude/x");
        assert_eq!(
            shown(check_native_references(Path::new(DIR), &tokens)),
            [format!(
                "{file}: hooks[0].args[0]: neutral content must not reference native/"
            )]
        );
    }

    #[test]
    fn native_content_may_reference_native() -> TestResult {
        let file = format!("{DIR}/native/claude/x.md");
        let tokens = at_line(&file, 1, "{{root}}/native/claude/x")?;
        assert!(check_native_references(Path::new(DIR), &tokens).is_empty());
        Ok(())
    }

    #[test]
    fn arguments_var_is_refused_outside_skill_md() -> TestResult {
        let agent = format!("{DIR}/agents/b.md");
        let manifest = format!("{DIR}/pack.toml");
        let other_skill_md = format!("{DIR}/docs/SKILL.md");
        let native_skill_md = format!("{DIR}/native/claude/skills/a/SKILL.md");
        let too_deep = format!("{DIR}/skills/a/references/SKILL.md");
        let cases = [
            (
                at_line(&other_skill_md, 2, "{{var arguments}}")?,
                format!("{other_skill_md}:2: var \"arguments\" is only valid in SKILL.md"),
            ),
            (
                at_line(&native_skill_md, 2, "{{var arguments}}")?,
                format!("{native_skill_md}:2: var \"arguments\" is only valid in SKILL.md"),
            ),
            (
                at_line(&too_deep, 2, "{{var arguments}}")?,
                format!("{too_deep}:2: var \"arguments\" is only valid in SKILL.md"),
            ),
            (
                at_line(&agent, 6, "{{var arguments}}")?,
                format!("{agent}:6: var \"arguments\" is only valid in SKILL.md"),
            ),
            (
                at_key(&manifest, "hooks[0].args[0]", "{{var arguments}}"),
                format!(
                    "{manifest}: hooks[0].args[0]: var \"arguments\" is only valid in SKILL.md"
                ),
            ),
        ];
        for (tokens, expected) in cases {
            let found =
                check_local_tokens(Path::new(DIR), &core()?, &layout(&["pack.toml"]), &tokens);
            assert_eq!(shown(found), [expected]);
        }
        Ok(())
    }

    #[test]
    fn arguments_var_is_accepted_in_skill_md() -> TestResult {
        let file = format!("{DIR}/skills/a/SKILL.md");
        let mut tokens = at_line(&file, 7, "{{var arguments}}")?;
        tokens.extend(at_key(&file, "argument_hint", "{{var arguments}}"));
        let found = check_local_tokens(Path::new(DIR), &core()?, &layout(&["pack.toml"]), &tokens);
        assert!(found.is_empty());
        Ok(())
    }

    #[test]
    fn own_pack_targets_must_exist() -> TestResult {
        let files = layout(&["agents/coder.md", "pack.toml", "skills/discuss/SKILL.md"]);
        let file = format!("{DIR}/skills/a/SKILL.md");
        for (text, message) in [
            (
                "{{skill_dir core/coder}}",
                "pack \"core\" has no skill \"coder\"",
            ),
            ("{{agent codr}}", "pack \"core\" has no agent \"codr\""),
            ("{{skill status}}", "pack \"core\" has no skill \"status\""),
        ] {
            let tokens = at_line(&file, 6, text)?;
            let found = check_local_tokens(Path::new(DIR), &core()?, &files, &tokens);
            assert_eq!(shown(found), [format!("{file}:6: {message}")], "{text}");
        }
        Ok(())
    }

    #[test]
    fn own_targets_the_walk_refused_are_not_reported_missing() -> TestResult {
        use crate::types::pack::testing::refused_walk;

        let walk = refused_walk(&["pack.toml"], &["agents/coder.md", "skills/a"]);
        let (files, _) =
            PackLayout::classify(Path::new(DIR), &walk).map_err(|refusal| refusal.to_string())?;
        let file = format!("{DIR}/skills/b/SKILL.md");
        let text = "{{agent coder}} {{skill a}} {{skill_dir a}} {{agent codr}}";
        let tokens = at_line(&file, 6, text)?;
        let found = check_local_tokens(Path::new(DIR), &core()?, &files, &tokens);
        assert_eq!(
            shown(found),
            [format!("{file}:6: pack \"core\" has no agent \"codr\"")]
        );
        Ok(())
    }

    #[test]
    fn existing_own_targets_and_other_pack_targets_pass() -> TestResult {
        let files = layout(&["agents/coder.md", "pack.toml", "skills/discuss/SKILL.md"]);
        let file = format!("{DIR}/skills/a/SKILL.md");
        let text = "{{skill discuss}} {{skill_dir core/discuss}} {{agent coder}} {{root}} {{skill sdlc/status}}";
        let tokens = at_line(&file, 6, text)?;
        let found = check_local_tokens(Path::new(DIR), &core()?, &files, &tokens);
        assert!(found.is_empty(), "{}", shown(found).join("; "));
        Ok(())
    }
}
