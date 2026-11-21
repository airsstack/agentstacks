//! The checked frontmatter of an agent and the tool-neutral capabilities it may be given.
//!
//! Agents have their own keys and vocabulary, apart from skills.
//!
//! # Responsibilities
//!
//! - [`AgentMeta`]: an agent's checked frontmatter.
//! - [`AgentTool`]: a tool-neutral capability an agent may be given.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod meta;
pub mod tool;

#[doc(inline)]
pub use meta::AgentMeta;
#[doc(inline)]
pub use tool::AgentTool;
