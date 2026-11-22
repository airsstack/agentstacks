//! Reconciling the lock with `agentstacks.toml`, and choosing tags.
//!
//! # Responsibilities
//!
//! - [`FetchError`]: every refusal of fetching a source.
//! - [`SourceKey`]: the key of one tool or pack entry, which orders and locates every per-entry answer.
//! - [`Reconciled`], [`reconcile()`], [`same_source`]: classifying every entry against the lock.
//! - [`check_tree`]: refusing a git tree that differs from the lock.
//! - [`GitStep`], [`unreadable`], [`unreadable_at`]: the refusal for git output that cannot be read.
//! - [`RemoteTag`], [`parse_ls_remote`], [`parse_ls_remote_at`]: reading `git ls-remote` tag lines.
//! - [`TreeRecord`], [`parse_ls_tree`], [`check_subdir`], [`root_label`]: reading `git ls-tree -z`
//!   records and deciding the `subdir` from them.
//! - [`cache_home`], [`url_dir`], [`rev_dir`], [`temp_name`]: the layout of the git cache.
//! - [`TagVersion`], [`latest_tag`]: splitting a tag and choosing the latest.
//! - [`UpdateQuery`], [`UpdateEntry`], [`UpdateNote`]: what an update says about one git source.
//!
//! # Non-responsibilities
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod cache;
pub mod check;
pub mod error;
pub mod key;
pub mod listing;
pub mod reconcile;
pub mod remote;
pub mod tag;
pub mod unread;
pub mod update;

#[doc(inline)]
pub use cache::cache_home;
#[doc(inline)]
pub use cache::rev_dir;
#[doc(inline)]
pub use cache::temp_name;
#[doc(inline)]
pub use cache::url_dir;
#[doc(inline)]
pub use check::check_tree;
#[doc(inline)]
pub use error::FetchError;
#[doc(inline)]
pub use key::SourceKey;
#[doc(inline)]
pub use listing::TreeRecord;
#[doc(inline)]
pub use listing::check_subdir;
#[doc(inline)]
pub use listing::check_subdir_at;
#[doc(inline)]
pub use listing::parse_ls_tree;
#[doc(inline)]
pub use listing::parse_ls_tree_at;
#[doc(inline)]
pub use listing::root_label;
#[doc(inline)]
pub use reconcile::Reconciled;
#[doc(inline)]
pub use reconcile::reconcile;
#[doc(inline)]
pub use reconcile::same_source;
#[doc(inline)]
pub use remote::RemoteTag;
#[doc(inline)]
pub use remote::parse_ls_remote;
#[doc(inline)]
pub use remote::parse_ls_remote_at;
#[doc(inline)]
pub use tag::TagVersion;
#[doc(inline)]
pub use tag::latest_tag;
#[doc(inline)]
pub use unread::GitStep;
#[doc(inline)]
pub use unread::first_line;
#[doc(inline)]
pub use unread::unreadable;
#[doc(inline)]
pub use unread::unreadable_at;
#[doc(inline)]
pub use update::UpdateEntry;
#[doc(inline)]
pub use update::UpdateNote;
#[doc(inline)]
pub use update::UpdateOutcome;
#[doc(inline)]
pub use update::UpdateQuery;
