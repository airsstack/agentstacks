//! The [`Git`] port on the `git` command line.
//!
//! # Responsibilities
//!
//! - [`GitCli`] lists a remote's tags (`ls-remote`), fetches a tag or a commit into the cache
//!   (`init --bare`, `fetch --depth 1`, `rev-parse`) and reads a commit's files (`ls-tree`,
//!   `cat-file --batch`). It builds the argument vectors, runs them, and hands the output to the
//!   readers of `types::resolve`.
//! - A fetch fills a scratch repository first. The cache renames it into place once the commit is
//!   known, and removes it when any step failed.
//! - The cache is reached only when a call needs it, so listing tags needs no `HOME`.
//!
//! # Non-responsibilities
//!
//! No disk step of its own: the scratch directory, its rename and its removal are `fs::Cache`.
//! Locating a refusal at `agentstacks.toml` and merging the git-not-found answers of every source
//! belong to the flows.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::fs::Cache;
use crate::port::Git;
use crate::process::git::blobs::read_blobs;
use crate::process::git::command::GitRun;
use crate::process::git::snapshot::Snapshot;
use crate::types::project::{CommitId, GitTag, GitUrl, Pin, SubPath};
use crate::types::resolve::{
    FetchError, GitStep, RemoteTag, parse_ls_remote, temp_name, unreadable,
};

/// How many scratch directories this process has asked for, the `<n>` of their names.
static SCRATCHES: AtomicU64 = AtomicU64::new(0);

/// The `git` command line as a [`Git`].
///
/// # Examples
///
/// ```
/// use agentstacks::port::Git;
/// use agentstacks::process::git::GitCli;
/// use agentstacks::types::GitUrl;
/// use agentstacks::types::resolve::FetchError;
///
/// // A program that does not exist is the per-source git-not-found answer.
/// let git = GitCli::new("/nonexistent/dir/git", None);
/// let url = GitUrl::new("https://github.com/airsstack/agentstacks")?;
/// let refusal = git.list_tags(&url, "packs-v").err().ok_or("expected a refusal")?;
/// assert!(matches!(refusal, FetchError::GitNotFound { needed_by } if needed_by.is_empty()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct GitCli {
    program: OsString,
    home: Option<OsString>,
}

impl GitCli {
    /// Runs `program` as git, and keeps its cache under `home`, the value of `HOME`.
    #[must_use]
    pub fn new(program: impl Into<OsString>, home: Option<OsString>) -> Self {
        Self {
            program: program.into(),
            home,
        }
    }

    /// Runs `git` from `PATH`, with the cache under the `HOME` of the environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new("git", std::env::var_os("HOME"))
    }

    /// A run of `subcommand` by this git.
    fn run(&self, subcommand: &'static str) -> GitRun<'_> {
        GitRun::new(&self.program, subcommand)
    }

    /// The cache, which needs a home.
    fn cache(&self) -> Result<Cache, FetchError> {
        Cache::from_home(self.home.as_deref())
    }

    /// Fetches `refspec` of `url` into a scratch repository, asks `name` for the commit it holds,
    /// and installs it in the cache under that commit. Returns the commit and the cached
    /// directory. A scratch repository left by a failure is removed.
    fn install_fetched<F>(
        &self,
        cache: &Cache,
        url: &GitUrl,
        refspec: &str,
        name: F,
    ) -> Result<(CommitId, PathBuf), FetchError>
    where
        F: FnOnce(&Path) -> Result<CommitId, FetchError>,
    {
        let scratch = cache.temp(
            url,
            &temp_name(
                std::process::id(),
                SCRATCHES.fetch_add(1, Ordering::Relaxed),
            ),
        )?;
        let outcome = self
            .fetch_into(&scratch, url, refspec)
            .and_then(|()| name(&scratch))
            .and_then(|rev| {
                cache
                    .install(&scratch, url, &rev)
                    .map(|installed| (rev, installed))
            });
        if outcome.is_err() {
            // The failure that matters is the one already in hand.
            let _ = Cache::discard(&scratch);
        }
        outcome
    }

    /// Makes `scratch` a bare repository and fetches `refspec` of `url` into it, one commit deep.
    fn fetch_into(&self, scratch: &Path, url: &GitUrl, refspec: &str) -> Result<(), FetchError> {
        self.run("init")
            .args(["--bare", "-q"])
            .args([scratch])
            .run()?;
        self.run("fetch")
            .repo(scratch)
            .args(["--no-tags", "--depth", "1"])
            .url(url)
            .operand(refspec)
            .run()?;
        Ok(())
    }
}

impl Git for GitCli {
    type Tree = Snapshot;

    fn list_tags(&self, url: &GitUrl, prefix: &str) -> Result<Vec<RemoteTag>, FetchError> {
        let stdout = self
            .run("ls-remote")
            .args(["--tags", "--refs"])
            .url(url)
            .operand(format!("refs/tags/{prefix}*"))
            .run()?;
        parse_ls_remote(&stdout)
    }

    fn fetch_tag(&self, url: &GitUrl, tag: &GitTag) -> Result<CommitId, FetchError> {
        let cache = self.cache()?;
        let reference = format!("refs/tags/{tag}");
        let (rev, _) = self.install_fetched(
            &cache,
            url,
            &format!("{reference}:{reference}"),
            |scratch| {
                let stdout = self
                    .run("rev-parse")
                    .repo(scratch)
                    .args(["--verify", "--end-of-options"])
                    .args([format!("{reference}^{{commit}}")])
                    .run()?;
                CommitId::new(String::from_utf8_lossy(&stdout).trim())
                    .map_err(|_| unreadable(GitStep::RevParse, &stdout))
            },
        )?;
        Ok(rev)
    }

    fn tree(
        &self,
        url: &GitUrl,
        rev: &CommitId,
        subdir: Option<&SubPath>,
        at: &Pin,
    ) -> Result<Self::Tree, FetchError> {
        let cache = self.cache()?;
        let repo = match cache.repo(url, rev)? {
            Some(repo) => repo,
            None => {
                self.install_fetched(&cache, url, rev.as_str(), |_| Ok(rev.clone()))?
                    .1
            }
        };
        let mut listing =
            self.run("ls-tree")
                .repo(&repo)
                .args(["-r", "-z", "--full-tree", rev.as_str()]);
        let normalized = subdir.map(SubPath::normalized).unwrap_or_default();
        if !normalized.is_empty() {
            listing = listing.operand(normalized);
        }
        let stdout = listing.run()?;
        Snapshot::from_listing(&stdout, subdir, at, |ids| {
            read_blobs(&self.program, &repo, ids)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;
    use crate::port::{EntryKind, FileTree};
    use crate::process::git::testing::{CRLF, SourceRepo, TestResult};
    use crate::types::resolve::{rev_dir, url_dir};

    fn cli(home: &tempfile::TempDir) -> GitCli {
        GitCli::new("git", Some(home.path().as_os_str().to_owned()))
    }

    fn tag(name: &str) -> Result<GitTag, Box<dyn std::error::Error>> {
        Ok(GitTag::new(name)?)
    }

    fn remote(name: &str, rev: &CommitId) -> Result<RemoteTag, Box<dyn std::error::Error>> {
        Ok(RemoteTag {
            rev: rev.clone(),
            tag: tag(name)?,
        })
    }

    fn pin(name: &str) -> Result<Pin, Box<dyn std::error::Error>> {
        Ok(Pin::Tag(tag(name)?))
    }

    /// The names in the cache directory of `url`.
    fn cached(home: &Path, url: &GitUrl) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(url_dir(home, url))? {
            names.push(entry?.file_name().to_string_lossy().into_owned());
        }
        Ok(names)
    }

    fn no_scratch_left(home: &Path, url: &GitUrl) -> Result<(), Box<dyn std::error::Error>> {
        let left: Vec<_> = cached(home, url)?
            .into_iter()
            .filter(|name| name.starts_with("tmp-"))
            .collect();
        assert!(left.is_empty(), "left behind: {left:?}");
        Ok(())
    }

    #[test]
    fn list_tags_lists_matching_tags_in_git_order() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let tags = cli(&home).list_tags(&source.url, "packs-v")?;
        assert_eq!(
            tags,
            [
                remote("packs-v0.1.0", &source.annotated)?,
                remote("packs-v0.10.0", &source.two)?,
                remote("packs-v0.2.0", &source.two)?,
                remote("packs-v0.9.0", &source.two)?,
            ]
        );
        Ok(())
    }

    #[test]
    fn list_tags_with_no_match_is_empty() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        assert!(cli(&home).list_tags(&source.url, "packs-v9")?.is_empty());
        Ok(())
    }

    #[test]
    fn list_tags_needs_no_home() -> TestResult {
        let source = SourceRepo::new()?;
        let tags = GitCli::new("git", None).list_tags(&source.url, "other-v")?;
        assert_eq!(tags, [remote("other-v9.9.9", &source.two)?]);
        Ok(())
    }

    #[test]
    fn fetch_tag_peels_an_annotated_tag() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let rev = cli(&home).fetch_tag(&source.url, &tag("packs-v0.1.0")?)?;
        assert_eq!(rev, source.one);
        assert_ne!(rev, source.annotated);
        let installed = rev_dir(&url_dir(home.path(), &source.url), &rev);
        assert!(installed.is_dir());
        no_scratch_left(home.path(), &source.url)
    }

    #[test]
    fn fetch_tag_of_a_lightweight_tag() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let rev = cli(&home).fetch_tag(&source.url, &tag("packs-v0.2.0")?)?;
        assert_eq!(rev, source.two);
        Ok(())
    }

    #[test]
    fn fetch_tag_missing_passes_git_stderr() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let error = cli(&home).fetch_tag(&source.url, &tag("packs-v9.9.9")?);
        let Err(FetchError::GitFailed { subcommand, stderr }) = &error else {
            return Err(format!("expected GitFailed, got {error:?}").into());
        };
        assert_eq!(subcommand, "fetch");
        assert!(stderr.contains("refs/tags/packs-v9.9.9"), "{stderr}");
        no_scratch_left(home.path(), &source.url)
    }

    #[test]
    fn fetch_by_rev_works_for_a_non_tip_commit() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let subdir = SubPath::new("packs/core")?;
        let tree = cli(&home).tree(
            &source.url,
            &source.one,
            Some(&subdir),
            &pin("packs-v0.1.0")?,
        )?;
        assert_eq!(tree.kind(Path::new("pack.toml"))?, EntryKind::File);
        assert!(tree.kind(Path::new("two.md")).is_err());
        no_scratch_left(home.path(), &source.url)
    }

    #[test]
    fn tree_reads_from_the_cache_without_the_remote() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let git = cli(&home);
        let subdir = SubPath::new("packs/core")?;
        let at = pin("packs-v0.1.0")?;
        git.tree(&source.url, &source.one, Some(&subdir), &at)?;
        source.delete()?;
        let again = git.tree(&source.url, &source.one, Some(&subdir), &at)?;
        assert_eq!(again.read_file(Path::new("crlf.md"))?, CRLF);
        Ok(())
    }

    #[test]
    fn tree_keeps_crlf_symlink_and_gitlink_as_committed() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let subdir = SubPath::new("packs/core")?;
        let tree = cli(&home).tree(
            &source.url,
            &source.one,
            Some(&subdir),
            &pin("packs-v0.1.0")?,
        )?;
        assert_eq!(tree.read_file(Path::new("crlf.md"))?, CRLF);
        assert_eq!(tree.read_file(Path::new("lf.md"))?, b"a\nb\n");
        assert_eq!(tree.kind(Path::new("link"))?, EntryKind::Symlink);
        assert_eq!(tree.kind(Path::new("sub"))?, EntryKind::Other);
        assert_eq!(tree.kind(Path::new("run.sh"))?, EntryKind::File);
        assert_eq!(
            tree.read_file(Path::new("nested/deeper/file.md"))?,
            b"deep\n"
        );
        Ok(())
    }

    #[test]
    fn tree_refuses_a_missing_subdir() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let subdir = SubPath::new("packs/nope")?;
        let error = cli(&home)
            .tree(
                &source.url,
                &source.one,
                Some(&subdir),
                &pin("packs-v0.1.0")?,
            )
            .err()
            .ok_or("expected a refusal")?;
        assert!(matches!(error, FetchError::SubdirNotFound { .. }));
        assert_eq!(
            error.to_string(),
            "\"packs/nope\" not found at packs-v0.1.0"
        );
        Ok(())
    }

    #[test]
    fn tree_refuses_a_subdir_that_is_a_file() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let subdir = SubPath::new("packs/core/pack.toml")?;
        let error = cli(&home)
            .tree(
                &source.url,
                &source.one,
                Some(&subdir),
                &pin("packs-v0.1.0")?,
            )
            .err()
            .ok_or("expected a refusal")?;
        assert!(matches!(error, FetchError::SubdirIsFile { .. }));
        Ok(())
    }

    #[test]
    fn tree_without_a_subdir_lists_the_whole_tree() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let tree = cli(&home).tree(&source.url, &source.one, None, &pin("packs-v0.1.0")?)?;
        assert_eq!(tree.kind(Path::new("zero.txt"))?, EntryKind::File);
        assert_eq!(
            tree.kind(Path::new("packs/core/pack.toml"))?,
            EntryKind::File
        );
        assert_eq!(tree.root(), Path::new("."));
        Ok(())
    }

    #[test]
    fn a_cached_rev_wins_a_rename_race() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let git = cli(&home);
        let first = git.fetch_tag(&source.url, &tag("packs-v0.1.0")?)?;
        let installed = rev_dir(&url_dir(home.path(), &source.url), &first);
        let marker = installed.join("marker");
        std::fs::write(&marker, b"first")?;
        let second = git.fetch_tag(&source.url, &tag("packs-v0.1.0")?)?;
        assert_eq!(second, first);
        assert!(marker.is_file(), "the directory in place was replaced");
        no_scratch_left(home.path(), &source.url)
    }

    #[test]
    fn a_dash_host_never_reaches_ssh() -> TestResult {
        let home = tempfile::tempdir()?;
        let url = GitUrl::new("ssh://-oProxyCommand=false/x")?;
        let error = cli(&home).list_tags(&url, "packs-v");
        let Err(FetchError::GitFailed { stderr, .. }) = &error else {
            return Err(format!("expected GitFailed, got {error:?}").into());
        };
        assert!(stderr.contains("strange hostname"), "{stderr}");
        Ok(())
    }

    #[test]
    fn git_not_found_is_reported_for_every_call() -> TestResult {
        let source = SourceRepo::new()?;
        let home = tempfile::tempdir()?;
        let git = GitCli::new(
            "/nonexistent/dir/git",
            Some(home.path().as_os_str().to_owned()),
        );
        let at = pin("packs-v0.1.0")?;
        let answers = [
            git.list_tags(&source.url, "packs-v").err(),
            git.fetch_tag(&source.url, &tag("packs-v0.1.0")?).err(),
            git.tree(&source.url, &source.one, None, &at).err(),
        ];
        for answer in answers {
            let Some(FetchError::GitNotFound { needed_by }) = &answer else {
                return Err(format!("expected GitNotFound, got {answer:?}").into());
            };
            assert!(needed_by.is_empty());
        }
        no_scratch_left(home.path(), &source.url)
    }

    #[test]
    fn no_home_is_refused_when_the_cache_is_needed() -> TestResult {
        let source = SourceRepo::new()?;
        for home in [None, Some(OsString::new())] {
            let git = GitCli::new(OsStr::new("git"), home);
            let error = git.fetch_tag(&source.url, &tag("packs-v0.1.0")?);
            assert!(matches!(error, Err(FetchError::HomeUnset)), "{error:?}");
        }
        Ok(())
    }
}
