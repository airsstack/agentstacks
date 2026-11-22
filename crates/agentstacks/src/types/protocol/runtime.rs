//! The hook runtime: the Lua file every hook command runs through.
//!
//! # Responsibilities
//!
//! - [`RUNTIME_FILE`]: the runtime's file name under the extension root.
//! - [`RUNTIME_LUA`]: the runtime's text, compiled into the library.

/// The runtime's file name under the extension root.
pub const RUNTIME_FILE: &str = "runtime.lua";

/// The text of the hook runtime, as written to `<root>/runtime.lua`.
///
/// The runtime loads the extension's `translate` module and reads the hook module's name, the
/// `may_block` flag and the hook's kind from the command line's first three arguments; the rest are
/// the hook's own. For a neutral hook it builds a neutral event from the payload the tool sends and
/// stays silent when the translator has no event for it; for a native hook the event holds the raw
/// payload. It then runs the hook module. A `block` from a hook not declared `may_block` is dropped
/// with one line on standard error, and the translator's `result` is printed; it receives the hook's
/// result and the raw payload, for neutral and native hooks alike.
///
/// # Examples
///
/// ```
/// use agentstacks::types::protocol::RUNTIME_LUA;
///
/// assert!(RUNTIME_LUA.starts_with("-- agentstacks hook runtime"));
/// ```
pub const RUNTIME_LUA: &str = include_str!("../../../assets/runtime.lua");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ContentHash;

    #[test]
    fn runtime_lua_hash_is_pinned() {
        assert_eq!(
            ContentHash::of_bytes(RUNTIME_LUA.as_bytes()).to_string(),
            "sha256:05ba59c1af4002b5a212f9eacd1cb74253f69cc79c9bf3a95d4dda312f89a282"
        );
        assert_eq!(RUNTIME_LUA.len(), 914);
    }

    #[test]
    fn runtime_lua_starts_and_ends_as_shipped() {
        assert!(RUNTIME_LUA.starts_with("-- agentstacks hook runtime, api 0."));
        assert!(RUNTIME_LUA.ends_with("end\n"));
        assert!(!RUNTIME_LUA.ends_with("\n\n"));
        assert_eq!(RUNTIME_FILE, "runtime.lua");
    }
}
