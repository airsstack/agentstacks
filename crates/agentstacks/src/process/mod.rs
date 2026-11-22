//! The adapter that drives the `git` CLI and looks up binaries on `PATH`, and the only place that
//! calls `std::process`.
//!
//! It translates between the ports and the processes it starts and decides nothing. It is the one
//! adapter allowed to use another adapter's item, `fs::cache`.
//!
//! This module currently holds no items. It is an export-only module with no executable logic,
//! so it carries no inline tests.
