//! What a pack installs: which of its files are installed and rendered, and the checks on that.
//!
//! Installing is judged from the manifest's `exclude` and `render` lists, which the layout never
//! reads, so the rules about it sit together apart from the layout.
//!
//! # Responsibilities
//!
//! - [`InstallSet`]: which files of a pack are installed and which of those are rendered.
//! - [`check_airsl_declared()`]: refuses a pack with `.lua` files and no `requires.airsl`.
//! - [`check_something_to_install()`]: refuses a pack that installs no skill, agent or hook.
//!
//! # Non-responsibilities
//!
//! Where a file belongs in a pack is the layout's, and reading the files is the pack loader's.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod checks;
pub mod set;

#[doc(inline)]
pub use checks::check_airsl_declared;
#[doc(inline)]
pub use checks::check_something_to_install;
#[doc(inline)]
pub use set::InstallSet;
