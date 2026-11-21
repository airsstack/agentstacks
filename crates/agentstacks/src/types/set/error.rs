//! The refusals the checks over a whole set of packs own.
//!
//! A refusal is a message without its location: the diagnostic that carries it names the project
//! file, the pack file, the line or the key path.
//!
//! It is separate from the pack loader's errors because these refusals are about the relation
//! between several packs and the project file, which no single pack's loader can see.
//!
//! # Responsibilities
//!
//! - [`SetError`] has a variant per refusal that depends on several packs or on the project file:
//!   a fetched name that differs from its key, a requirement that is not listed, a requirement
//!   cycle, a token aimed at a pack that is not required, a missing option, a hook `run` that does
//!   not exist in another pack, option values that do not resolve, airsl ranges that do not meet,
//!   and a skill or agent that two packs ship. Each displays exactly the message the author reads
//!   after the location.
//!
//! # Non-responsibilities
//!
//! A refusal that names a skill or agent a pack does not ship is
//! [`PackError::NoSuchItem`](crate::types::PackError::NoSuchItem), the same wording the pack's own
//! checks use. Where a refusal is located belongs to the
//! [`Diagnostic`](crate::types::Diagnostic) that carries it.

use std::fmt::{self, Display};

use crate::types::manifest::AirslReq;
use crate::types::manifest::PackVersion;
use crate::types::name::ItemName;
use crate::types::name::OptionName;
use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::pack::ItemKind;
use crate::types::report::TypeWord;
use crate::types::tokens::Token;

/// A refusal found while checking a set of packs.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{AirslReq, PackName, SetError};
///
/// let unlisted = SetError::RequiresUnlisted {
///     pack: PackName::new("sdlc")?,
///     required: PackName::new("core")?,
/// };
/// assert_eq!(
///     unlisted.to_string(),
///     "sdlc requires pack \"core\", which agentstacks.toml does not list (add it: --pack core)"
/// );
///
/// let disjoint = SetError::AirslDisjoint {
///     needs: vec![
///         (PackName::new("core")?, AirslReq::new(">=0.2")?),
///         (PackName::new("sdlc")?, AirslReq::new("<0.2")?),
///     ],
/// };
/// assert_eq!(disjoint.to_string(), "airsl: core needs >=0.2, sdlc needs <0.2");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SetError {
    /// A `[packs.*]` key that differs from the name inside its source.
    #[error("source contains pack \"{name}\"")]
    SourcePack {
        /// The name the source holds.
        name: PackName,
    },
    /// A `[tools.*]` key that differs from the name inside its source.
    #[error("source contains extension \"{name}\"")]
    SourceExtension {
        /// The name the source holds.
        name: ToolName,
    },
    /// A required pack that `agentstacks.toml` does not list.
    #[error(
        "{pack} requires pack \"{required}\", which agentstacks.toml does not list (add it: --pack {required})"
    )]
    RequiresUnlisted {
        /// The pack that requires.
        pack: PackName,
        /// The pack that is not listed.
        required: PackName,
    },
    /// Packs that require each other in a loop, printed from the smallest name back to itself.
    #[error("requires cycle: {}", Joined { items: path, separator: " → " })]
    RequiresCycle {
        /// The packs of the cycle in order, the first repeated at the end.
        path: Vec<PackName>,
    },
    /// A token that names a pack the token's own pack does not require.
    #[error(
        "{token} targets pack \"{target}\", which {pack} does not require. Add it to requires.packs or remove the ref."
    )]
    TargetNotRequired {
        /// The token, shown in its canonical form.
        token: Token,
        /// The pack the token names.
        target: PackName,
        /// The pack the token sits in.
        pack: PackName,
    },
    /// An `option` token that names an option the pack does not declare.
    #[error("pack \"{pack}\" has no option \"{option}\"")]
    NoSuchOption {
        /// The pack that declares the options.
        pack: PackName,
        /// The option the token names.
        option: OptionName,
    },
    /// A hook `run` into another pack that names no installed root file there.
    #[error("\"{text}\" does not exist in {pack}")]
    RunMissing {
        /// The `run` as written.
        text: String,
        /// The pack the `run` points into.
        pack: PackName,
    },
    /// An option with no value in the project file and no default.
    #[error("option \"{option}\" is required")]
    OptionRequired {
        /// The option that has no value.
        option: OptionName,
    },
    /// An option value of another type than the option declares.
    #[error("option {option} expects {expected}, got {got}")]
    OptionType {
        /// The option the value is for.
        option: OptionName,
        /// The type the option declares.
        expected: TypeWord,
        /// The type the value has.
        got: TypeWord,
    },
    /// An option value for an option the pack does not declare.
    #[error("no option \"{text}\"")]
    OptionUndeclared {
        /// The key as written in the project file.
        text: String,
    },
    /// `requires.airsl` ranges that no version satisfies together.
    #[error("airsl: {}", Needs(needs))]
    AirslDisjoint {
        /// Each pack that declares a range, in name order, with the range.
        needs: Vec<(PackName, AirslReq)>,
    },
    /// An installed airsl outside the ranges the packs need.
    #[error("airsl {installed} installed, packs need {}", Joined { items: ranges, separator: ", " })]
    AirslInstalled {
        /// The installed version.
        installed: PackVersion,
        /// Each declared range, in pack name order, as written.
        ranges: Vec<AirslReq>,
    },
    /// A skill or an agent that two or more packs ship.
    #[error("{kind} \"{name}\" is shipped by {}", Shippers(packs))]
    NameClash {
        /// Whether a skill or an agent clashes.
        kind: ItemKind,
        /// The name the packs share.
        name: ItemName,
        /// The packs that ship it, in name order; at least two.
        packs: Vec<PackName>,
    },
}

/// Items written one after another with a separator.
struct Joined<'a, T> {
    items: &'a [T],
    separator: &'static str,
}

impl<T: Display> Display for Joined<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, item) in self.items.iter().enumerate() {
            if index > 0 {
                f.write_str(self.separator)?;
            }
            item.fmt(f)?;
        }
        Ok(())
    }
}

/// `<pack> needs <range>` for each pack, joined by `, `.
struct Needs<'a>(&'a [(PackName, AirslReq)]);

impl Display for Needs<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, (pack, range)) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{pack} needs {range}")?;
        }
        Ok(())
    }
}

/// The packs that ship one name: `both <a> and <b>` for two, else all but the last joined by `, `
/// and then ` and ` before the last.
struct Shippers<'a>(&'a [PackName]);

impl Display for Shippers<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            [] => Ok(()),
            [only] => only.fmt(f),
            [first, second] => write!(f, "both {first} and {second}"),
            [init @ .., last] => write!(
                f,
                "{} and {last}",
                Joined {
                    items: init,
                    separator: ", "
                }
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::num::NonZeroUsize;

    use super::*;
    use crate::types::tokens::TextOrigin;
    use crate::types::tokens::scan;

    type TestResult = Result<(), Box<dyn Error>>;

    fn pack(name: &str) -> Result<PackName, Box<dyn Error>> {
        Ok(PackName::new(name)?)
    }

    fn packs(names: &[&str]) -> Result<Vec<PackName>, Box<dyn Error>> {
        names.iter().map(|name| pack(name)).collect()
    }

    fn range(text: &str) -> Result<AirslReq, Box<dyn Error>> {
        Ok(AirslReq::new(text)?)
    }

    fn token(text: &str) -> Result<Token, Box<dyn Error>> {
        let origin = TextOrigin::Lines {
            file: "a.md".into(),
            first_line: NonZeroUsize::MIN,
        };
        let (found, _) = scan(text, &origin);
        Ok(found.first().ok_or("no token")?.token().clone())
    }

    #[test]
    fn set_error_displays_each_message() -> TestResult {
        let cases: Vec<(SetError, &str)> = vec![
            (
                SetError::SourcePack {
                    name: pack("kore")?,
                },
                "source contains pack \"kore\"",
            ),
            (
                SetError::SourceExtension {
                    name: ToolName::new("claud")?,
                },
                "source contains extension \"claud\"",
            ),
            (
                SetError::RequiresUnlisted {
                    pack: pack("sdlc")?,
                    required: pack("core")?,
                },
                "sdlc requires pack \"core\", which agentstacks.toml does not list (add it: --pack core)",
            ),
            (
                SetError::RequiresCycle {
                    path: packs(&["a", "b", "a"])?,
                },
                "requires cycle: a → b → a",
            ),
            (
                SetError::TargetNotRequired {
                    token: token("{{skill guideline-rust/rust-guidelines}}")?,
                    target: pack("guideline-rust")?,
                    pack: pack("sdlc")?,
                },
                "{{skill guideline-rust/rust-guidelines}} targets pack \"guideline-rust\", which sdlc does not require. Add it to requires.packs or remove the ref.",
            ),
            (
                SetError::NoSuchOption {
                    pack: pack("core")?,
                    option: OptionName::new("archive_kep")?,
                },
                "pack \"core\" has no option \"archive_kep\"",
            ),
            (
                SetError::RunMissing {
                    text: "{{root core}}/hooks/enforc.lua".to_owned(),
                    pack: pack("core")?,
                },
                "\"{{root core}}/hooks/enforc.lua\" does not exist in core",
            ),
            (
                SetError::OptionRequired {
                    option: OptionName::new("x")?,
                },
                "option \"x\" is required",
            ),
            (
                SetError::OptionType {
                    option: OptionName::new("archive_keep")?,
                    expected: TypeWord::Integer,
                    got: TypeWord::String,
                },
                "option archive_keep expects integer, got string",
            ),
            (
                SetError::OptionUndeclared {
                    text: "archive_kep".to_owned(),
                },
                "no option \"archive_kep\"",
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
        Ok(())
    }

    #[test]
    fn set_error_displays_the_airsl_and_clash_messages() -> TestResult {
        let clash = |kind, name: &str, who: &[&str]| -> Result<SetError, Box<dyn Error>> {
            Ok(SetError::NameClash {
                kind,
                name: ItemName::new(name)?,
                packs: packs(who)?,
            })
        };
        let cases: Vec<(SetError, &str)> = vec![
            (
                SetError::AirslDisjoint {
                    needs: vec![
                        (pack("core")?, range(">=0.2")?),
                        (pack("sdlc")?, range("<0.2")?),
                    ],
                },
                "airsl: core needs >=0.2, sdlc needs <0.2",
            ),
            (
                SetError::AirslInstalled {
                    installed: PackVersion::new("0.1.3")?,
                    ranges: vec![range(">=0.1.4")?],
                },
                "airsl 0.1.3 installed, packs need >=0.1.4",
            ),
            (
                SetError::AirslInstalled {
                    installed: PackVersion::new("0.1.3")?,
                    ranges: vec![range(">=0.1.4")?, range("<0.3")?],
                },
                "airsl 0.1.3 installed, packs need >=0.1.4, <0.3",
            ),
            (
                clash(ItemKind::Skill, "status", &["sdlc", "team-lint"])?,
                "skill \"status\" is shipped by both sdlc and team-lint",
            ),
            (
                clash(ItemKind::Agent, "coder", &["core", "team-lint"])?,
                "agent \"coder\" is shipped by both core and team-lint",
            ),
            (
                clash(ItemKind::Skill, "status", &["sdlc", "team-lint", "x"])?,
                "skill \"status\" is shipped by sdlc, team-lint and x",
            ),
            (
                clash(ItemKind::Agent, "y", &["a", "b", "c", "d"])?,
                "agent \"y\" is shipped by a, b, c and d",
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
        Ok(())
    }
}
