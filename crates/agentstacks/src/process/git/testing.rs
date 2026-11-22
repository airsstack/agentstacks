//! A local source repository for the contract tests of the git adapter.
//!
//! # Responsibilities
//!
//! - [`SourceRepo`] is a repository built with `git` in a temp directory and reached through a
//!   `file://` URL, so no test touches the network. Its setup commands ignore the developer's git
//!   configuration; the adapter under test never sets that.
//!
//! The history has three commits:
//!
//! - `zero` holds `zero.txt` and has no tag.
//! - `one` adds `packs/core` with `pack.toml`, a file with CRLF line ends, `lf.md`, a symlink
//!   `link`, an executable `run.sh`, `nested/deeper/file.md` and a gitlink `sub`. The annotated tag
//!   `packs-v0.1.0` names it.
//! - `two` adds `packs/core/two.md`. The tags `packs-v0.2.0`, `packs-v0.9.0`, `packs-v0.10.0` and
//!   `other-v9.9.9` name it.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests only and makes no assertion of its own.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use crate::types::project::{CommitId, GitUrl};

/// The result type of a test.
pub(super) type TestResult = Result<(), Box<dyn Error>>;

/// The bytes of the CRLF file, which a checkout with `core.autocrlf=true` would rewrite.
pub(super) const CRLF: &[u8] = b"a\r\nb\r\n";

/// A source repository in a temp directory.
pub(super) struct SourceRepo {
    dir: TempDir,
    /// The `file://` URL of the repository.
    pub(super) url: GitUrl,
    /// The untagged first commit.
    pub(super) zero: CommitId,
    /// The commit the annotated tag `packs-v0.1.0` names.
    pub(super) one: CommitId,
    /// The commit the lightweight tags name.
    pub(super) two: CommitId,
    /// The tag object of `packs-v0.1.0`, which `ls-remote --refs` lists for that tag.
    pub(super) annotated: CommitId,
    /// The object id of the CRLF file.
    pub(super) crlf_blob: String,
}

impl SourceRepo {
    /// Builds the repository.
    pub(super) fn new() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        run(dir.path(), &["init", "-q", "-b", "main", "work"], DAY_ZERO)?;
        let work = dir.path().join("work");
        let builder = Builder {
            work: work.clone(),
            scratch: dir.path().join("blob"),
        };

        builder.stage("100644", "zero.txt", b"zero\n")?;
        let zero = builder.commit("zero", DAY_ZERO)?;

        builder.stage("100644", "packs/core/pack.toml", b"name = \"core\"\n")?;
        let crlf_blob = builder.stage("100644", "packs/core/crlf.md", CRLF)?;
        builder.stage("100644", "packs/core/lf.md", b"a\nb\n")?;
        builder.stage("120000", "packs/core/link", b"pack.toml")?;
        builder.stage("100755", "packs/core/run.sh", b"#!/bin/sh\necho hi\n")?;
        builder.stage("100644", "packs/core/nested/deeper/file.md", b"deep\n")?;
        builder.gitlink("packs/core/sub", "0123456789abcdef0123456789abcdef01234567")?;
        let one = builder.commit("one", DAY_ONE)?;
        run(
            &work,
            &["tag", "-a", "packs-v0.1.0", "-m", "annotated 0.1.0"],
            DAY_ONE,
        )?;
        let annotated = CommitId::new(&run(&work, &["rev-parse", "packs-v0.1.0"], DAY_ONE)?)?;

        builder.stage("100644", "packs/core/two.md", b"two\n")?;
        let two = builder.commit("two", DAY_TWO)?;
        for tag in [
            "packs-v0.2.0",
            "packs-v0.9.0",
            "packs-v0.10.0",
            "other-v9.9.9",
        ] {
            run(&work, &["tag", tag], DAY_TWO)?;
        }

        let url = GitUrl::new(&format!("file://{}", work.display()))?;
        Ok(Self {
            dir,
            url,
            zero,
            one,
            two,
            annotated,
            crlf_blob,
        })
    }

    /// The working directory of the repository, which holds its `.git`.
    pub(super) fn path(&self) -> PathBuf {
        self.dir.path().join("work")
    }

    /// Sets `key` to `value` in the repository's own configuration.
    pub(super) fn config(&self, key: &str, value: &str) -> Result<(), Box<dyn Error>> {
        run(&self.path(), &["config", key, value], DAY_ZERO)?;
        Ok(())
    }

    /// Deletes the repository, so a later read cannot reach it.
    pub(super) fn delete(&self) -> Result<(), Box<dyn Error>> {
        std::fs::remove_dir_all(self.path())?;
        Ok(())
    }
}

const DAY_ZERO: &str = "2026-01-01T00:00:00Z";
const DAY_ONE: &str = "2026-01-01T00:00:01Z";
const DAY_TWO: &str = "2026-01-02T00:00:00Z";

/// The repository being built, with a scratch file the blobs pass through.
struct Builder {
    work: PathBuf,
    scratch: PathBuf,
}

impl Builder {
    /// Stores `bytes` as a blob exactly as given and puts it in the index at `path` with `mode`.
    /// Returns the object id.
    fn stage(&self, mode: &str, path: &str, bytes: &[u8]) -> Result<String, Box<dyn Error>> {
        std::fs::write(&self.scratch, bytes)?;
        let scratch = self.scratch.display().to_string();
        let id = run(
            &self.work,
            &["hash-object", "-w", "--no-filters", &scratch],
            DAY_ZERO,
        )?;
        let entry = format!("{mode},{id},{path}");
        run(
            &self.work,
            &["update-index", "--add", "--cacheinfo", &entry],
            DAY_ZERO,
        )?;
        Ok(id)
    }

    /// Puts a gitlink to `commit` in the index at `path`.
    fn gitlink(&self, path: &str, commit: &str) -> Result<(), Box<dyn Error>> {
        let entry = format!("160000,{commit},{path}");
        run(
            &self.work,
            &["update-index", "--add", "--cacheinfo", &entry],
            DAY_ZERO,
        )?;
        Ok(())
    }

    /// Commits the index on `date` and returns the commit.
    fn commit(&self, message: &str, date: &str) -> Result<CommitId, Box<dyn Error>> {
        run(&self.work, &["commit", "-q", "-m", message], date)?;
        Ok(CommitId::new(&run(
            &self.work,
            &["rev-parse", "HEAD"],
            date,
        )?)?)
    }
}

/// Runs `git` in `cwd` with the developer's configuration out of the way and a fixed identity and
/// date. Returns its trimmed standard output.
fn run(cwd: &Path, args: &[&str], date: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "tester")
        .env("GIT_AUTHOR_EMAIL", "tester@example.invalid")
        .env("GIT_COMMITTER_NAME", "tester")
        .env("GIT_COMMITTER_EMAIL", "tester@example.invalid")
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_repository_has_the_three_commits() -> TestResult {
        let repo = SourceRepo::new()?;
        assert_ne!(repo.zero, repo.one);
        assert_ne!(repo.one, repo.two);
        assert_ne!(repo.annotated, repo.one);
        assert!(repo.url.as_str().starts_with("file://"));
        Ok(())
    }
}
