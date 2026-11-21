//! The relative paths a source can carry: a directory inside a repository or inside the project.
//!
//! Both are checked so they can never point outside the place they are resolved against.
//!
//! # Responsibilities
//!
//! - [`SubPath`]: a relative path that only descends, inside a repository or the project.
//! - [`LocalPath`]: a relative path inside the project whose `..` segments never climb out of it.
//!
//! # Non-responsibilities
//!
//! Whether the directory exists is decided when it is read.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod local;
pub mod sub;

#[doc(inline)]
pub use local::LocalPath;
#[doc(inline)]
pub use sub::SubPath;
