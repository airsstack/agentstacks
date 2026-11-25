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
/// the hook's own, which every event the hook runs on carries as `args`. For a neutral hook the
/// translator's `event` turns the payload the tool sends into one neutral event or a list of them;
/// when it returns nothing the runtime exits without running the hook or printing anything. For a
/// native hook the event is the raw payload, with `args` added. The hook module runs once per event. A `block`
/// from a hook not declared `may_block` is dropped, with one line on standard error, in either kind
/// and for each event of a list. A list of events runs in order until one event's result carries a
/// `block`, which is kept and ends the run; the `context` strings of the events run so far, that
/// one's included, are joined with a newline. The translator's `result` is then called with the
/// hook's result and the raw payload, for neutral and native hooks alike, and whatever text it
/// returns is printed.
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
            "sha256:1d9e23f6d26d49ae14e28d0425d5d31fd7be5dae7e6c8fa33a3153ba8acde459"
        );
        assert_eq!(RUNTIME_LUA.len(), 1715);
    }

    #[test]
    fn runtime_lua_starts_and_ends_as_shipped() {
        assert!(RUNTIME_LUA.starts_with("-- agentstacks hook runtime, api 0."));
        assert!(RUNTIME_LUA.ends_with("end\n"));
        assert!(!RUNTIME_LUA.ends_with("\n\n"));
        assert_eq!(RUNTIME_FILE, "runtime.lua");
    }
}
