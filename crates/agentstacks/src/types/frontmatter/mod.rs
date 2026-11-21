//! Skill and agent frontmatter: splitting the YAML block from the body, mapping YAML syntax errors
//! and checking every field.
//!
//! Frontmatter has its own vocabulary (invocation, model tier, effort, tools) that no other format
//! uses.
//!
//! # Responsibilities
//!
//! - [`Frontmatter`], [`BodyStart`]: the YAML block of a Markdown file and where its body begins.
//! - [`SkillMeta`], [`AgentMeta`]: a skill's or an agent's checked frontmatter.
//! - [`Invocation`]: who may start a skill.
//! - [`ModelTier`], [`Effort`]: the tool-neutral model tier and reasoning effort of an agent.
//! - [`AgentTool`]: a tool-neutral capability an agent may be given.
//! - [`FrontmatterError`]: the refusals worded by frontmatter rather than by a generic field shape.
//!
//! # Non-responsibilities
//!
//! Where a token in a value may appear belongs to the checks over a whole pack, and values under
//! `native` are opaque.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod agent;
pub mod block;
pub mod body;
pub mod effort;
pub mod error;
mod fields;
pub mod invocation;
pub mod model;
pub mod skill;
mod syntax;
mod wording;

#[doc(inline)]
pub use agent::AgentMeta;
#[doc(inline)]
pub use agent::AgentTool;
#[doc(inline)]
pub use block::Frontmatter;
#[doc(inline)]
pub use body::BodyStart;
#[doc(inline)]
pub use effort::Effort;
#[doc(inline)]
pub use error::FrontmatterError;
#[doc(inline)]
pub use invocation::Invocation;
#[doc(inline)]
pub use model::ModelTier;
#[doc(inline)]
pub use skill::SkillMeta;
