//! The two payloads the core hands a tool extension, as domain values.
//!
//! The `values` payload lists what the extension must say where to install; the `plan` payload
//! lists every item the extension must account for. The extension host turns these values into
//! the tables the extension reads; nothing here knows Lua or JSON.
//!
//! # Responsibilities
//!
//! - [`ValuesInput`] and [`ValuesPack`]: the packs of a run with their skills and agents, and
//!   the vars their texts use.
//! - [`PlanInput`], [`PlanPack`], [`PlanAgent`] and [`PlanNative`]: the rendered packs, and the
//!   rules that resolve an id an extension copies back and list the ids that need an operation.
//! - [`PlanSkill`] and [`PlanFile`]: a skill with its install directory and files.
//! - [`PlanHook`], [`PlanNativeHook`] and [`PlanAction`]: the hooks of a pack and of its
//!   `native/<tool>/` directory.
//!
//! # Non-responsibilities
//!
//! Rendering the texts, and building a [`PlanInput`] from the rendered packs, belong to the plan
//! flow. The keys of the tables belong to the extension host.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod file;
pub mod hook;
pub mod plan;
pub mod skill;
pub mod values;

#[doc(inline)]
pub use file::PlanFile;
#[doc(inline)]
pub use hook::PlanAction;
#[doc(inline)]
pub use hook::PlanHook;
#[doc(inline)]
pub use hook::PlanNativeHook;
#[doc(inline)]
pub use plan::PlanAgent;
#[doc(inline)]
pub use plan::PlanInput;
#[doc(inline)]
pub use plan::PlanNative;
#[doc(inline)]
pub use plan::PlanPack;
#[doc(inline)]
pub use skill::PlanSkill;
#[doc(inline)]
pub use values::ValuesInput;
#[doc(inline)]
pub use values::ValuesPack;
