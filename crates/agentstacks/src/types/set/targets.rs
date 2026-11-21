//! The rules over the tokens that aim outside their own pack, and over `option` tokens.
//!
//! It is a set-level rule because a token that names another pack can only be judged against that
//! pack's requirements and items, which a single pack's loader does not hold.
//!
//! # Responsibilities
//!
//! - `check_token_targets` reads every token each loaded pack kept, wherever it was found, and
//!   refuses it at the token's own location.
//! - A `skill`, `agent`, `skill_dir` or `root` token that names another pack is refused when that
//!   pack is not in the token's `requires.packs`.
//! - When the other pack is required and loaded, a `skill`, `skill_dir` or `agent` token is also
//!   refused when that pack ships no such item. A `root` token names no item.
//! - An `option` token is refused when its own pack declares no such option.
//! - A target in a pack that is not loaded is not looked into: the pack's failure, or the
//!   requirement that no one listed, is reported where it happened.
//!
//! # Non-responsibilities
//!
//! A token that names the pack it sits in is checked when that pack is loaded, and is skipped
//! here. A hook `run` into another pack is checked by
//! `check_hook_runs`.

use std::collections::BTreeMap;

use crate::types::name::PackName;
use crate::types::pack::ItemKind;
use crate::types::pack::Pack;
use crate::types::pack::PackError;
use crate::types::report::Diagnostic;
use crate::types::report::FormatError;
use crate::types::set::error::SetError;
use crate::types::tokens::ItemRef;
use crate::types::tokens::Token;
use crate::types::tokens::TokenTarget;
use crate::types::tokens::names_own_pack;

/// Where a token's target pack stands in relation to the pack the token sits in.
enum Reach<'a> {
    /// The token's own pack, or no pack named.
    Own,
    /// A pack that the token's pack does not list in `requires.packs`.
    Unrequired(&'a PackName),
    /// A pack that the token's pack requires.
    Required(&'a PackName),
}

/// Refuses every token of the loaded `packs` that aims at an unrequired pack, at an item another
/// required pack does not ship, or at an option its own pack does not declare. `packs` are keyed
/// by their `[packs.*]` name.
pub(crate) fn check_token_targets(packs: &BTreeMap<PackName, Pack>) -> Vec<Diagnostic> {
    let mut refusals = Vec::new();
    for (key, pack) in packs {
        for found in pack.tokens() {
            if let Some(error) = refusal(key, pack, packs, found.token()) {
                refusals.push(Diagnostic::new(found.location().clone(), error));
            }
        }
    }
    refusals
}

/// The refusal for `token`, which sits in the pack `pack` listed as `key`, if the set breaks a rule.
fn refusal(
    key: &PackName,
    pack: &Pack,
    packs: &BTreeMap<PackName, Pack>,
    token: &Token,
) -> Option<FormatError> {
    match token.target() {
        TokenTarget::Skill(item) | TokenTarget::SkillDir(item) => {
            item_refusal(key, pack, packs, token, item, ItemKind::Skill)
        }
        TokenTarget::Agent(item) => item_refusal(key, pack, packs, token, item, ItemKind::Agent),
        TokenTarget::Root(named) => match reach(pack, named.as_ref()) {
            Reach::Unrequired(target) => Some(not_required(key, token, target)),
            Reach::Own | Reach::Required(_) => None,
        },
        TokenTarget::Option(option) => {
            let declared = pack
                .manifest()
                .options()
                .iter()
                .any(|(name, _)| name == option);
            (!declared).then(|| {
                SetError::NoSuchOption {
                    pack: key.clone(),
                    option: option.clone(),
                }
                .into()
            })
        }
        TokenTarget::Var(_) => None,
    }
}

/// The refusal for a `skill`, `skill_dir` or `agent` token naming `item`.
fn item_refusal(
    key: &PackName,
    pack: &Pack,
    packs: &BTreeMap<PackName, Pack>,
    token: &Token,
    item: &ItemRef,
    kind: ItemKind,
) -> Option<FormatError> {
    let target = match reach(pack, item.pack()) {
        Reach::Own => return None,
        Reach::Unrequired(target) => return Some(not_required(key, token, target)),
        Reach::Required(target) => target,
    };
    let other = packs.get(target)?;
    let ships = match kind {
        ItemKind::Skill => other
            .skills()
            .iter()
            .any(|skill| skill.name() == item.item()),
        ItemKind::Agent => other
            .agents()
            .iter()
            .any(|agent| agent.name() == item.item()),
    };
    (!ships).then(|| {
        PackError::NoSuchItem {
            pack: target.clone(),
            kind,
            item: item.item().to_string(),
        }
        .into()
    })
}

/// Where `named` stands for a token in the pack `pack`.
fn reach<'a>(pack: &Pack, named: Option<&'a PackName>) -> Reach<'a> {
    match named {
        Some(target) if !names_own_pack(named, pack.manifest().name()) => {
            if pack.manifest().requires().packs().contains(target) {
                Reach::Required(target)
            } else {
                Reach::Unrequired(target)
            }
        }
        _ => Reach::Own,
    }
}

/// The refusal for `token` naming the pack `target`, which the pack `key` does not require.
fn not_required(key: &PackName, token: &Token, target: &PackName) -> FormatError {
    SetError::TargetNotRequired {
        token: token.clone(),
        target: target.clone(),
        pack: key.clone(),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::set::testing::{PackSpec, doc, loaded, shown};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    const OTHER: &str = "{{skill guideline-rust/rust-guidelines}}";
    const NOT_REQUIRED: &str = "{{skill guideline-rust/rust-guidelines}} targets pack \"guideline-rust\", which sdlc does not require. Add it to requires.packs or remove the ref.";

    fn core() -> PackSpec {
        PackSpec::new("core")
            .skill("discuss", "body")
            .agent("coder", "body")
    }

    fn found(specs: &[PackSpec]) -> Result<Vec<String>, Box<dyn Error>> {
        let mut shown = shown(&check_token_targets(&loaded(specs)?));
        shown.sort();
        Ok(shown)
    }

    #[test]
    fn targets_outside_requires_are_refused_with_the_message() -> TestResult {
        for written in [OTHER, "{{ skill guideline-rust/rust-guidelines }}"] {
            let sdlc = PackSpec::new("sdlc")
                .requires(&["core"])
                .skill("plan", written);
            assert_eq!(
                found(&[core(), sdlc])?,
                [format!("packs/sdlc/skills/plan/SKILL.md:6: {NOT_REQUIRED}")]
            );
        }
        Ok(())
    }

    #[test]
    fn targets_in_required_packs_resolve() -> TestResult {
        let tail =
            "{{skill core/discuss}} {{agent core/coder}} {{skill_dir core/discuss}} {{root core}}";
        let sdlc = PackSpec::new("sdlc").requires(&["core"]).skill("a", tail);
        assert!(found(&[core(), sdlc])?.is_empty());
        Ok(())
    }

    #[test]
    fn root_tokens_into_unrequired_packs_are_refused() -> TestResult {
        let sdlc = PackSpec::new("sdlc").skill("a", "{{root core}}/hooks/x.lua");
        assert_eq!(
            found(&[core(), sdlc])?,
            [
                "packs/sdlc/skills/a/SKILL.md:6: {{root core}} targets pack \"core\", which sdlc does not require. Add it to requires.packs or remove the ref."
            ]
        );
        Ok(())
    }

    #[test]
    fn missing_items_are_refused_per_kind() -> TestResult {
        for (token, message) in [
            ("{{agent core/codr}}", "pack \"core\" has no agent \"codr\""),
            (
                "{{skill_dir core/coder}}",
                "pack \"core\" has no skill \"coder\"",
            ),
            (
                "{{skill core/statuss}}",
                "pack \"core\" has no skill \"statuss\"",
            ),
        ] {
            let sdlc = PackSpec::new("sdlc").requires(&["core"]).skill("a", token);
            assert_eq!(
                found(&[core(), sdlc])?,
                [format!("packs/sdlc/skills/a/SKILL.md:6: {message}")],
                "{token}"
            );
        }
        Ok(())
    }

    #[test]
    fn targets_are_checked_wherever_the_token_was_found() -> TestResult {
        let hooks = format!(
            "[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"hooks/x.lua\"\nargs = [\"a\", \"{OTHER}\"]\n"
        );
        let native_hooks =
            format!("[[hooks]]\nkind = \"message\"\nevent = \"Stop\"\ntext = \"{OTHER}\"\n");
        let sdlc = PackSpec::new("sdlc")
            .airsl(">=0.1.4")
            .top("render = [\"enforcement.json\"]\n")
            .tables(&hooks)
            .file("hooks/x.lua", "print(1)\n")
            .file(
                "skills/a/SKILL.md",
                &format!("---\nname: a\ndescription: \"{OTHER}\"\n---\n\nbody\n"),
            )
            .file("agents/b.md", &doc("b", OTHER))
            .file("enforcement.json", &format!("{OTHER}\n"))
            .file("native/claude/x.md", &format!("{OTHER}\n"))
            .file("native/claude/hooks.toml", &native_hooks);
        let mut expected: Vec<String> = [
            "packs/sdlc/skills/a/SKILL.md: description",
            "packs/sdlc/agents/b.md:6",
            "packs/sdlc/enforcement.json:1",
            "packs/sdlc/native/claude/x.md:1",
            "packs/sdlc/pack.toml: hooks[0].args[1]",
            "packs/sdlc/native/claude/hooks.toml: hooks[0].text",
        ]
        .iter()
        .map(|place| format!("{place}: {NOT_REQUIRED}"))
        .collect();
        expected.sort();
        assert_eq!(found(&[sdlc])?, expected);
        Ok(())
    }

    #[test]
    fn own_pack_targets_are_left_to_the_pack() -> TestResult {
        let sdlc = PackSpec::new("sdlc")
            .skill("status", "{{skill sdlc/status}} {{skill status}}")
            .skill("a", "{{skill_dir status}} {{root sdlc}} {{root}}");
        let own = core().skill("b", "{{agent coder}} {{skill core/discuss}}");
        assert!(found(&[sdlc, own])?.is_empty());
        Ok(())
    }

    #[test]
    fn option_tokens_must_name_a_declared_option() -> TestResult {
        let hook = "[[hooks]]\nkind = \"message\"\nevent = \"session.end\"\ntext = \"{{option archive_kep}}\"\n";
        let spec = |tail: &str| {
            PackSpec::new("core")
                .integer_option("archive_keep", 20)
                .skill("a", tail)
        };
        let message = "pack \"core\" has no option \"archive_kep\"";
        assert_eq!(
            found(&[spec("{{option archive_kep}}")])?,
            [format!("packs/core/skills/a/SKILL.md:6: {message}")]
        );
        assert_eq!(
            found(&[spec("body").tables(hook)])?,
            [format!("packs/core/pack.toml: hooks[0].text: {message}")]
        );
        let described = spec("body").file(
            "skills/a/SKILL.md",
            "---\nname: a\ndescription: \"{{option archive_kep}}\"\n---\n\nbody\n",
        );
        assert_eq!(
            found(&[described])?,
            [format!(
                "packs/core/skills/a/SKILL.md: description: {message}"
            )]
        );
        assert!(found(&[spec("{{option archive_keep}}")])?.is_empty());
        Ok(())
    }

    #[test]
    fn targets_into_packs_that_are_not_loaded_skip_the_item_check() -> TestResult {
        // `core` failed to load, or `agentstacks.toml` never listed it: either way it is absent.
        let sdlc = PackSpec::new("sdlc")
            .requires(&["core"])
            .skill("a", "{{agent core/coder}} {{skill core/x}}");
        assert!(found(&[sdlc])?.is_empty());
        Ok(())
    }
}
