//! The validating name newtypes: [`CaseName`], [`MarketplaceName`] and
//! [`PluginName`].
//!
//! They sit together because each wraps one validated string that names a
//! thing claudevs reports or locates, and each rejects at construction what
//! cannot be that name, so every later use can trust the value. Each error
//! struct carries only the rejected value.
//!
//! `marketplace` and `plugin` depend on the segment predicate in the sibling
//! `types::ident` module; nothing here depends on anything outside this crate.
//! Each file tests its own newtype with unit tests.

mod case;
mod marketplace;
mod plugin;

pub use case::{CaseName, InvalidCaseName};
pub use marketplace::{InvalidMarketplaceName, MarketplaceName};
pub use plugin::{InvalidPluginName, PluginName};
