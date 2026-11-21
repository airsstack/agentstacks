//! Hooks: neutral `[[hooks]]` in `pack.toml` and tool-specific `native/<tool>/hooks.toml`.
//!
//! Both files describe the same thing: an event, an optional selector, an action. One set of
//! event, action, run and grant rules reads them, so two copies cannot drift apart.
//!
//! # Responsibilities
//!
//! - [`Hook`]: a neutral hook: event, `match` values, `agents` and action.
//! - [`HookEvent`]: the closed vocabulary of neutral events.
//! - [`MatchValue`]: the closed set of `match` selector values.
//! - [`HookAction`]: a message or a Lua script, chosen by `kind`.
//! - [`MessageHook`]: a message action: its text.
//! - [`LuaHook`]: a Lua action: script, arguments, `may_block` and grants.
//! - [`HookRun`]: a Lua hook's `run`: as written, the pack it names and its pack-relative path.
//! - [`Grants`]: what a Lua hook may touch: paths, environment variables and binaries.
//! - [`Policy`]: the sandbox policy, `confined` or `pure`.
//! - [`InvalidHookEvent`], [`InvalidMatchValue`], [`InvalidPolicy`]: the refusals [`HookEvent`],
//!   [`MatchValue`] and [`Policy`] return.
//! - [`NativeHook`]: a hook of a tool's own event vocabulary, kept opaque.
//! - [`NativeHooks`]: a whole `native/<tool>/hooks.toml`.
//!
//! # Non-responsibilities
//!
//! Whether a `run` file or an `agents` entry exists belongs to the pack loader, and whether a tool
//! knows a native event belongs to that tool's extension. Turning grants and policy into airsl
//! flags belongs to the extension host.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod action;
pub mod event;
pub mod grants;
pub mod invalid;
pub mod lua;
pub mod matcher;
pub mod message;
pub mod native;
pub mod neutral;
pub mod policy;
pub mod run;

#[doc(inline)]
pub use action::HookAction;
#[doc(inline)]
pub use event::HookEvent;
#[doc(inline)]
pub use grants::Grants;
#[doc(inline)]
pub use invalid::InvalidHookEvent;
#[doc(inline)]
pub use invalid::InvalidMatchValue;
#[doc(inline)]
pub use invalid::InvalidPolicy;
#[doc(inline)]
pub use lua::LuaHook;
#[doc(inline)]
pub use matcher::MatchValue;
#[doc(inline)]
pub use message::MessageHook;
#[doc(inline)]
pub use native::NativeHook;
#[doc(inline)]
pub use native::NativeHooks;
#[doc(inline)]
pub use neutral::Hook;
#[doc(inline)]
pub use policy::Policy;
#[doc(inline)]
pub use run::HookRun;
