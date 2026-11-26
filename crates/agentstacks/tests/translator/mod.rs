//! The translator test harness the `claude` and `musecode` test binaries share.
//!
//! A test runs a hook module under the shipped runtime and the `translate.lua` the `full` fixture
//! of a tool expects, on the real `airsl` binary, and compares what it printed with the recorded
//! files under the tool's `payloads/expected`. `opencode.rs` keeps its own harness: its payloads sit one
//! level higher and its checks have no stderr file.

pub mod run;
