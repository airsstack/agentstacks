//! An operation a tool extension proposes, and the validation of it.
//!
//! Extensions only propose; the core validates each operation before it can be planned or
//! applied. Everything here is pure: values in, an answer or a refusal out. Nothing reads Lua,
//! the disk, git or `PATH`.
//!
//! # Responsibilities
//!
//! - [`LuaType`] and [`ApiFunction`]: the type words and the function names an operation refusal
//!   prints.
//! - [`OpError`]: every refusal of an operation, a call, or an extension's handlers.
//! - [`PathShape`]: the one wording of a project path that is empty, absolute, climbing out of
//!   the project, or not `/`-separated.
//! - [`OpPath`]: a project-relative path in its normalised form. [`OpPath::proposed`] is the
//!   entry point for the path of a `write` or a `copy`.
//! - [`ItemId`]: the id the plan payload gives each item.
//! - [`Operation`], [`Proposal`] and [`Origin`]: what an extension proposes and who proposed it;
//!   [`SkipReason`], [`SkipSubject`] and [`CoreSkip`]: the skips, including the ones the core
//!   makes. `owner` derives the owner of a proposal.
//! - [`Proposals`]: the record of accepted proposals in call order, with the duplicate-path and
//!   skip/cover conflict rules. [`check_coverage`] and [`check_translate`] are the checks at the
//!   end of a tool's plan.
//! - [`payload`]: the `values` and `plan` payloads the core hands an extension, as values.
//! - [`answer`]: the `values` result read and checked ([`ValuesResult`]), and a token's value
//!   looked up in it. [`first_missing`] and the `skip_*` functions are the skips the core makes
//!   when a value is missing.
//! - [`root_file_write`], [`runtime_write`] and [`runtime_needed`]: the writes the core makes
//!   under the extension's root.
//! - [`collect_binaries`] and [`BinaryRequirement`]: the binaries to look up on `PATH`;
//!   [`parse_airsl_version`] and the `airsl_*` refusals: the rule for the `airsl` binary.
//!
//! # Non-responsibilities
//!
//! Reading the arguments of a call from Lua, reading a file, and the phase a call is allowed in
//! belong to the extension host. Rendering the payloads and driving the order of a run belong to
//! the plan flow; deciding the step for each path belongs to the plan rules.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod airsl;
pub mod answer;
pub mod binaries;
pub mod coverage;
pub mod error;
pub mod id;
pub mod missing;
pub mod origin;
pub mod owner;
pub mod path;
pub mod payload;
pub mod proposal;
pub mod proposals;
pub mod root;
pub mod shape;
pub mod skip;
#[cfg(test)]
mod testing;
pub mod translate;
pub mod word;

#[doc(inline)]
pub use airsl::airsl_not_found;
#[doc(inline)]
pub use airsl::airsl_unparseable;
#[doc(inline)]
pub use airsl::parse_airsl_version;
#[doc(inline)]
pub use answer::SkillValues;
#[doc(inline)]
pub use answer::TokenValue;
#[doc(inline)]
pub use answer::ValuesResult;
#[doc(inline)]
pub use binaries::BinaryRequirement;
#[doc(inline)]
pub use binaries::collect_binaries;
#[doc(inline)]
pub use coverage::check_coverage;
#[doc(inline)]
pub use error::OpError;
#[doc(inline)]
pub use id::ItemId;
#[doc(inline)]
pub use missing::first_missing;
#[doc(inline)]
pub use missing::skip_for_missing_value;
#[doc(inline)]
pub use missing::skip_root_file_for_missing_value;
#[doc(inline)]
pub use missing::skip_skill_without_location;
#[doc(inline)]
pub use origin::Origin;
#[doc(inline)]
pub use owner::owner;
#[doc(inline)]
pub use path::OpPath;
#[doc(inline)]
pub use payload::PlanAction;
#[doc(inline)]
pub use payload::PlanAgent;
#[doc(inline)]
pub use payload::PlanFile;
#[doc(inline)]
pub use payload::PlanHook;
#[doc(inline)]
pub use payload::PlanInput;
#[doc(inline)]
pub use payload::PlanNative;
#[doc(inline)]
pub use payload::PlanNativeHook;
#[doc(inline)]
pub use payload::PlanPack;
#[doc(inline)]
pub use payload::PlanSkill;
#[doc(inline)]
pub use payload::ValuesInput;
#[doc(inline)]
pub use payload::ValuesPack;
#[doc(inline)]
pub use proposal::Operation;
#[doc(inline)]
pub use proposal::Proposal;
#[doc(inline)]
pub use proposals::Proposals;
#[doc(inline)]
pub use root::root_file_write;
#[doc(inline)]
pub use root::runtime_needed;
#[doc(inline)]
pub use root::runtime_write;
#[doc(inline)]
pub use shape::PathShape;
#[doc(inline)]
pub use skip::CoreSkip;
#[doc(inline)]
pub use skip::SkipReason;
#[doc(inline)]
pub use skip::SkipSubject;
#[doc(inline)]
pub use translate::check_translate;
#[doc(inline)]
pub use word::ApiFunction;
#[doc(inline)]
pub use word::LuaType;
