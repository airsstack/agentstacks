//! An in-memory [`Git`] for tests.
//!
//! It lets a flow resolve and read git sources without `git`, a network or a cache, and makes the
//! failures a real run cannot be made to produce on demand, such as git missing or `HOME` unset,
//! one builder call.
//!
//! # Responsibilities
//!
//! - [`FakeGit`] answers each [`Git`] method from what was configured for the URL: the tags of a
//!   remote, the commit a tag names, and the files of a commit. A tag listing is filtered by
//!   prefix and sorted by bytes, as git lists them.
//! - [`FakeGit::fail`] queues one failure for the next call on a URL, and the outage builders make
//!   every call, or every call on one URL, fail with git missing or `HOME` unset.
//! - A call with nothing configured answers git missing, so a source a test forgot to configure
//!   is a visible refusal and not a silent success.
//! - [`GitCall`] is one entry of the call log that [`FakeGit::calls`] returns.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests only. How the real `git` behaves is proven by the tests of the
//! `process` module.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::port::FakeFileTree;
use crate::port::git::seam::Git;
use crate::types::project::{CommitId, GitTag, GitUrl, Pin, SubPath};
use crate::types::resolve::{FetchError, RemoteTag};

/// One call to a [`FakeGit`], as the call log keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitCall {
    /// [`Git::list_tags`].
    ListTags {
        /// The remote.
        url: GitUrl,
        /// The prefix asked for.
        prefix: String,
    },
    /// [`Git::fetch_tag`].
    FetchTag {
        /// The remote.
        url: GitUrl,
        /// The tag asked for.
        tag: GitTag,
    },
    /// [`Git::tree`].
    Tree {
        /// The remote.
        url: GitUrl,
        /// The commit asked for.
        rev: CommitId,
        /// The directory asked for.
        subdir: Option<SubPath>,
    },
}

/// A failure that answers every call.
#[derive(Debug, Clone, Copy)]
enum Outage {
    GitMissing,
    HomeUnset,
}

impl Outage {
    /// The refusal, built fresh because [`FetchError`] is not `Clone`.
    const fn error(self) -> FetchError {
        match self {
            Self::GitMissing => git_missing(),
            Self::HomeUnset => FetchError::HomeUnset,
        }
    }
}

/// The per-source answer for git not on `PATH`, before the flow merges the sources that got it.
const fn git_missing() -> FetchError {
    FetchError::GitNotFound {
        needed_by: Vec::new(),
    }
}

/// One configured tree.
#[derive(Debug)]
struct Served {
    url: GitUrl,
    rev: CommitId,
    subdir: Option<SubPath>,
    files: FakeFileTree,
}

/// An in-memory git remote for each URL it was configured for.
#[derive(Debug, Default)]
pub struct FakeGit {
    tags: BTreeMap<GitUrl, Vec<RemoteTag>>,
    fetched: BTreeMap<(GitUrl, GitTag), CommitId>,
    trees: Vec<Served>,
    everywhere: Option<Outage>,
    per_url: BTreeMap<GitUrl, Outage>,
    queued: RefCell<Vec<(GitUrl, FetchError)>>,
    calls: RefCell<Vec<GitCall>>,
}

impl FakeGit {
    /// A fake with nothing configured.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the tags `url` lists.
    #[must_use]
    pub fn tags(mut self, url: &GitUrl, tags: Vec<RemoteTag>) -> Self {
        self.tags.insert(url.clone(), tags);
        self
    }

    /// Sets the commit that fetching `tag` of `url` returns.
    #[must_use]
    pub fn tag(mut self, url: &GitUrl, tag: &GitTag, rev: &CommitId) -> Self {
        self.fetched.insert((url.clone(), tag.clone()), rev.clone());
        self
    }

    /// Sets the files of `rev` under `subdir` of `url`. The tree is cloned for each call, so one
    /// commit can be served more than once.
    #[must_use]
    pub fn files(
        mut self,
        url: &GitUrl,
        rev: &CommitId,
        subdir: Option<&SubPath>,
        files: FakeFileTree,
    ) -> Self {
        self.trees.push(Served {
            url: url.clone(),
            rev: rev.clone(),
            subdir: subdir.cloned(),
            files,
        });
        self
    }

    /// Queues `error` for the next call on `url`; each queued error answers one call.
    #[must_use]
    pub fn fail(self, url: &GitUrl, error: FetchError) -> Self {
        self.queued.borrow_mut().push((url.clone(), error));
        self
    }

    /// Makes every call answer git not on `PATH`.
    #[must_use]
    pub const fn git_missing(mut self) -> Self {
        self.everywhere = Some(Outage::GitMissing);
        self
    }

    /// Makes every call on `url` answer git not on `PATH`.
    #[must_use]
    pub fn git_missing_for(mut self, url: &GitUrl) -> Self {
        self.per_url.insert(url.clone(), Outage::GitMissing);
        self
    }

    /// Makes every call answer `HOME` unset.
    #[must_use]
    pub const fn home_unset(mut self) -> Self {
        self.everywhere = Some(Outage::HomeUnset);
        self
    }

    /// Makes every call on `url` answer `HOME` unset.
    #[must_use]
    pub fn home_unset_for(mut self, url: &GitUrl) -> Self {
        self.per_url.insert(url.clone(), Outage::HomeUnset);
        self
    }

    /// Every call so far, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<GitCall> {
        self.calls.borrow().clone()
    }

    /// Logs `call`, then returns the failure that answers it, if any: a queued error first, then
    /// an outage on the URL, then an outage everywhere.
    fn enter(&self, call: GitCall, url: &GitUrl) -> Result<(), FetchError> {
        self.calls.borrow_mut().push(call);
        let mut queued = self.queued.borrow_mut();
        if let Some(at) = queued.iter().position(|(queued_url, _)| queued_url == url) {
            return Err(queued.remove(at).1);
        }
        self.per_url
            .get(url)
            .copied()
            .or(self.everywhere)
            .map_or(Ok(()), |outage| Err(outage.error()))
    }
}

impl Git for FakeGit {
    type Tree = FakeFileTree;

    fn list_tags(&self, url: &GitUrl, prefix: &str) -> Result<Vec<RemoteTag>, FetchError> {
        let call = GitCall::ListTags {
            url: url.clone(),
            prefix: prefix.to_owned(),
        };
        self.enter(call, url)?;
        let mut found: Vec<RemoteTag> = self
            .tags
            .get(url)
            .ok_or_else(git_missing)?
            .iter()
            .filter(|remote| remote.tag.as_str().starts_with(prefix))
            .cloned()
            .collect();
        found.sort_by(|a, b| a.tag.as_str().as_bytes().cmp(b.tag.as_str().as_bytes()));
        Ok(found)
    }

    fn fetch_tag(&self, url: &GitUrl, tag: &GitTag) -> Result<CommitId, FetchError> {
        let call = GitCall::FetchTag {
            url: url.clone(),
            tag: tag.clone(),
        };
        self.enter(call, url)?;
        self.fetched
            .get(&(url.clone(), tag.clone()))
            .cloned()
            .ok_or_else(git_missing)
    }

    fn tree(
        &self,
        url: &GitUrl,
        rev: &CommitId,
        subdir: Option<&SubPath>,
        _at: &Pin,
    ) -> Result<Self::Tree, FetchError> {
        let call = GitCall::Tree {
            url: url.clone(),
            rev: rev.clone(),
            subdir: subdir.cloned(),
        };
        self.enter(call, url)?;
        self.trees
            .iter()
            .find(|served| {
                &served.url == url && &served.rev == rev && served.subdir.as_ref() == subdir
            })
            .map(|served| served.files.clone())
            .ok_or_else(git_missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::tree::FileTree;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const REV_A: &str = "0123456789abcdef0123456789abcdef01234567";
    const REV_B: &str = "89abcdef0123456789abcdef0123456789abcdef";

    fn url(text: &str) -> Result<GitUrl, Box<dyn std::error::Error>> {
        Ok(GitUrl::new(text)?)
    }

    fn rev(text: &str) -> Result<CommitId, Box<dyn std::error::Error>> {
        Ok(CommitId::new(text)?)
    }

    fn tag(text: &str) -> Result<GitTag, Box<dyn std::error::Error>> {
        Ok(GitTag::new(text)?)
    }

    fn remote(text: &str, rev_text: &str) -> Result<RemoteTag, Box<dyn std::error::Error>> {
        Ok(RemoteTag {
            rev: rev(rev_text)?,
            tag: tag(text)?,
        })
    }

    fn at() -> Result<Pin, Box<dyn std::error::Error>> {
        Ok(Pin::Tag(tag("v1")?))
    }

    fn shown<T>(result: Result<T, FetchError>) -> Option<String> {
        result.err().map(|error| error.to_string())
    }

    fn first_file<G: Git>(
        git: &G,
        url: &GitUrl,
        rev: &CommitId,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let tree = git.tree(url, rev, None, &Pin::Rev(rev.clone()))?;
        Ok(tree.read_file(std::path::Path::new("a.md"))?)
    }

    #[test]
    fn list_tags_filters_by_prefix_in_byte_order() -> TestResult {
        let u = url("https://example.com/a.git")?;
        let git = FakeGit::new().tags(
            &u,
            vec![
                remote("packs-v0.2.0", REV_A)?,
                remote("other-v9.9.9", REV_A)?,
                remote("packs-v0.10.0", REV_B)?,
                remote("packs-v0.1.0", REV_A)?,
            ],
        );
        let found = git.list_tags(&u, "packs-v")?;
        let listed: Vec<(&str, &str)> = found
            .iter()
            .map(|t| (t.tag.as_str(), t.rev.as_str()))
            .collect();
        assert_eq!(
            listed,
            [
                ("packs-v0.1.0", REV_A),
                ("packs-v0.10.0", REV_B),
                ("packs-v0.2.0", REV_A)
            ]
        );
        assert!(git.list_tags(&u, "packs-v9")?.is_empty());
        Ok(())
    }

    #[test]
    fn fetch_tag_returns_the_configured_rev() -> TestResult {
        let u = url("https://example.com/a.git")?;
        let git = FakeGit::new().tag(&u, &tag("v1")?, &rev(REV_A)?);
        assert_eq!(git.fetch_tag(&u, &tag("v1")?)?, rev(REV_A)?);
        assert_eq!(
            git.calls(),
            [GitCall::FetchTag {
                url: u,
                tag: tag("v1")?
            }]
        );
        Ok(())
    }

    #[test]
    fn tree_returns_the_configured_files_twice() -> TestResult {
        let u = url("https://example.com/a.git")?;
        let files = FakeFileTree::new("src").file("a.md", b"x");
        let git = FakeGit::new().files(&u, &rev(REV_A)?, None, files);
        assert_eq!(first_file(&git, &u, &rev(REV_A)?)?, b"x");
        assert_eq!(first_file(&git, &u, &rev(REV_A)?)?, b"x");
        Ok(())
    }

    #[test]
    fn tree_distinguishes_subdirs() -> TestResult {
        let u = url("https://example.com/a.git")?;
        let sub = SubPath::new("packs/core")?;
        let git = FakeGit::new().files(
            &u,
            &rev(REV_A)?,
            Some(&sub),
            FakeFileTree::new("packs/core").file("a.md", b"sub"),
        );
        let tree = git.tree(&u, &rev(REV_A)?, Some(&sub), &at()?)?;
        assert_eq!(tree.read_file(std::path::Path::new("a.md"))?, b"sub");
        assert!(git.tree(&u, &rev(REV_A)?, None, &at()?).is_err());
        Ok(())
    }

    #[test]
    fn queued_failure_is_returned_once() -> TestResult {
        let u = url("https://example.com/a.git")?;
        let git = FakeGit::new().tag(&u, &tag("v1")?, &rev(REV_A)?).fail(
            &u,
            FetchError::GitFailed {
                subcommand: "fetch".to_owned(),
                stderr: "boom".to_owned(),
            },
        );
        assert_eq!(
            shown(git.fetch_tag(&u, &tag("v1")?)).as_deref(),
            Some("git fetch failed: boom")
        );
        assert_eq!(git.fetch_tag(&u, &tag("v1")?)?, rev(REV_A)?);
        Ok(())
    }

    #[test]
    fn git_missing_answers_every_call() -> TestResult {
        let a = url("https://example.com/a.git")?;
        let b = url("https://example.com/b.git")?;
        let git = FakeGit::new().git_missing();
        let outcomes = [
            shown(git.list_tags(&a, "v")),
            shown(git.fetch_tag(&a, &tag("v1")?)),
            shown(git.tree(&a, &rev(REV_A)?, None, &at()?)),
        ];
        let missing = Some("git not found on PATH (needed by )".to_owned());
        assert_eq!(
            outcomes,
            [missing.clone(), missing.clone(), missing.clone()]
        );

        let git = FakeGit::new()
            .git_missing_for(&a)
            .tags(&b, vec![remote("v1", REV_A)?]);
        assert_eq!(shown(git.list_tags(&a, "v")), missing);
        assert_eq!(git.list_tags(&b, "v")?.len(), 1);
        Ok(())
    }

    #[test]
    fn home_unset_answers_every_call() -> TestResult {
        let a = url("https://example.com/a.git")?;
        let b = url("https://example.com/b.git")?;
        let git = FakeGit::new().home_unset();
        let outcomes = [
            shown(git.list_tags(&a, "v")),
            shown(git.fetch_tag(&a, &tag("v1")?)),
            shown(git.tree(&a, &rev(REV_A)?, None, &at()?)),
        ];
        let unset = FetchError::HomeUnset.to_string();
        assert!(
            outcomes
                .iter()
                .all(|o| o.as_deref() == Some(unset.as_str()))
        );

        let git = FakeGit::new()
            .home_unset_for(&a)
            .tags(&b, vec![remote("v1", REV_A)?]);
        assert_eq!(shown(git.list_tags(&a, "v")), Some(unset));
        assert_eq!(git.list_tags(&b, "v")?.len(), 1);
        Ok(())
    }

    #[test]
    fn unconfigured_call_reports_git_missing() -> TestResult {
        let u = url("https://example.com/a.git")?;
        let git = FakeGit::new();
        assert_eq!(
            shown(git.tree(&u, &rev(REV_B)?, None, &at()?)).as_deref(),
            Some("git not found on PATH (needed by )")
        );
        Ok(())
    }

    #[test]
    fn calls_are_logged_in_order() -> TestResult {
        let u = url("https://example.com/a.git")?;
        let sub = SubPath::new("packs")?;
        let git = FakeGit::new();
        let _ = git.list_tags(&u, "packs-v");
        let _ = git.fetch_tag(&u, &tag("v1")?);
        let _ = git.tree(&u, &rev(REV_A)?, Some(&sub), &at()?);
        assert_eq!(
            git.calls(),
            [
                GitCall::ListTags {
                    url: u.clone(),
                    prefix: "packs-v".to_owned()
                },
                GitCall::FetchTag {
                    url: u.clone(),
                    tag: tag("v1")?
                },
                GitCall::Tree {
                    url: u,
                    rev: rev(REV_A)?,
                    subdir: Some(sub)
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn fake_is_usable_through_a_generic_bound() -> TestResult {
        let u = url("https://example.com/a.git")?;
        let git = FakeGit::new().files(
            &u,
            &rev(REV_A)?,
            None,
            FakeFileTree::new("src").file("a.md", b"hi"),
        );
        assert_eq!(first_file(&git, &u, &rev(REV_A)?)?, b"hi");
        Ok(())
    }
}
