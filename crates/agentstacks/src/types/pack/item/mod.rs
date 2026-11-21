//! The items a loaded pack ships, and the kind of item a token or a hook can point at.
//!
//! Later stages read the checked shape of a skill or an agent without the code that built it, and
//! the token rules, the hook rules and the refusals all name the kind.
//!
//! # Responsibilities
//!
//! - [`PackSkill`]: a skill's checked frontmatter with the files installed from its directory.
//! - [`PackAgent`]: an agent's checked frontmatter with the pack-relative path of its file.
//! - [`ItemKind`]: the kind of item a token or hook points at, a skill or an agent.
//!
//! # Non-responsibilities
//!
//! Reading and checking the frontmatter belongs to the frontmatter parsers, and deciding which
//! files are installed belongs to the install set.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod kind;
pub mod loaded;

#[doc(inline)]
pub use kind::ItemKind;
#[doc(inline)]
pub use loaded::PackAgent;
#[doc(inline)]
pub use loaded::PackSkill;
