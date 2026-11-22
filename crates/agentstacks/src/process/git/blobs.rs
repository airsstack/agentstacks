//! Reading blob bytes through `git cat-file --batch`.
//!
//! # Responsibilities
//!
//! - [`read_blobs`] gives the bytes of a list of objects of one repository, in the order asked,
//!   with one `git` run. The bytes are exactly the committed ones, whatever `core.autocrlf` or
//!   `.gitattributes` say, because `cat-file --batch` converts nothing.
//! - [`parse_batch`] reads the answer: for each object a header `<oid> SP <type> SP <size> LF`,
//!   exactly `<size>` content bytes, then `LF`.
//!
//! # Non-responsibilities
//!
//! Which objects to read belongs to the tree listing that names them.

use std::ffi::OsStr;
use std::path::Path;

use crate::process::git::command::GitRun;
use crate::types::resolve::{FetchError, GitStep, unreadable};

/// The bytes of each object in `ids`, in the order of `ids`.
///
/// # Errors
///
/// The [`FetchError`] of the run, or [`FetchError::GitUnreadable`] for `cat-file` when the answer
/// is not one well-formed frame per id, for example an object git reports as missing.
pub(super) fn read_blobs(
    program: &OsStr,
    repo: &Path,
    ids: &[&str],
) -> Result<Vec<Vec<u8>>, FetchError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let input = ids.join("\n") + "\n";
    let output = GitRun::new(program, "cat-file")
        .repo(repo)
        .args(["--batch"])
        .stdin(input.as_bytes())
        .run()?;
    let blobs = parse_batch(&output)?;
    if blobs.len() == ids.len() {
        Ok(blobs)
    } else {
        Err(unreadable(GitStep::CatFile, &output))
    }
}

/// The content of every frame of `output`, in order.
///
/// # Errors
///
/// [`FetchError::GitUnreadable`] for `cat-file`, quoting the first line of `output`, when a frame
/// is a `missing` answer, has a header of another shape, is shorter than its size, or lacks its
/// closing line feed.
pub(super) fn parse_batch(output: &[u8]) -> Result<Vec<Vec<u8>>, FetchError> {
    let mut rest = output;
    let mut blobs = Vec::new();
    while !rest.is_empty() {
        let (content, after) =
            next_frame(rest).ok_or_else(|| unreadable(GitStep::CatFile, output))?;
        blobs.push(content.to_vec());
        rest = after;
    }
    Ok(blobs)
}

/// The content of the frame at the start of `bytes` and what follows it, or `None` when the frame
/// is not well formed.
fn next_frame(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let line_end = bytes.iter().position(|byte| *byte == b'\n')?;
    let header = std::str::from_utf8(&bytes[..line_end]).ok()?;
    let mut fields = header.split(' ');
    let (_id, _kind, size) = (fields.next()?, fields.next()?, fields.next()?);
    if fields.next().is_some() {
        return None;
    }
    let size: usize = size.parse().ok()?;
    let body = &bytes[line_end + 1..];
    let content = body.get(..size)?;
    let after = body.get(size..)?.strip_prefix(b"\n")?;
    Some((content, after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::git::testing::{CRLF, SourceRepo, TestResult};

    const OID: &str = "c30dea8c30dea8c30dea8c30dea8c30dea8c30de";

    fn frame(content: &[u8]) -> Vec<u8> {
        let mut bytes = format!("{OID} blob {}\n", content.len()).into_bytes();
        bytes.extend_from_slice(content);
        bytes.push(b'\n');
        bytes
    }

    #[test]
    fn parse_batch_reads_crlf_bytes_verbatim() -> TestResult {
        assert_eq!(parse_batch(&frame(CRLF))?, [CRLF.to_vec()]);
        Ok(())
    }

    #[test]
    fn parse_batch_reads_several_frames_in_order() -> TestResult {
        let mut output = frame(CRLF);
        output.extend(frame(b"a\nb\n"));
        assert_eq!(parse_batch(&output)?, [CRLF.to_vec(), b"a\nb\n".to_vec()]);
        Ok(())
    }

    #[test]
    fn parse_batch_keeps_a_newline_inside_the_content() -> TestResult {
        assert_eq!(parse_batch(&frame(b"x\n"))?, [b"x\n".to_vec()]);
        assert_eq!(parse_batch(&frame(b""))?, [Vec::<u8>::new()]);
        Ok(())
    }

    #[test]
    fn parse_batch_missing_object_is_unreadable() {
        let output = b"0123456789abcdef0123456789abcdef01234567 missing\n";
        let shown = parse_batch(output).err().map(|error| error.to_string());
        assert_eq!(
            shown.as_deref(),
            Some(
                "git cat-file printed output agentstacks cannot read: \
                 \"0123456789abcdef0123456789abcdef01234567 missing\""
            )
        );
    }

    #[test]
    fn parse_batch_short_frame_is_unreadable() {
        let output = format!("{OID} blob 6\nabc");
        let shown = parse_batch(output.as_bytes())
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            shown,
            Some(format!(
                "git cat-file printed output agentstacks cannot read: \"{OID} blob 6\""
            ))
        );
    }

    #[test]
    fn parse_batch_frame_without_its_closing_newline_is_unreadable() {
        let output = format!("{OID} blob 1\nx");
        assert!(parse_batch(output.as_bytes()).is_err());
    }

    #[test]
    fn read_blobs_returns_crlf_bytes_from_git() -> TestResult {
        let source = SourceRepo::new()?;
        source.config("core.autocrlf", "true")?;
        let blobs = read_blobs(
            OsStr::new("git"),
            &source.path(),
            &[source.crlf_blob.as_str()],
        )?;
        assert_eq!(blobs, [CRLF.to_vec()]);
        Ok(())
    }

    #[test]
    fn read_blobs_of_nothing_runs_nothing() -> TestResult {
        assert!(read_blobs(OsStr::new("/nonexistent/dir/git"), Path::new("/r"), &[])?.is_empty());
        Ok(())
    }

    #[test]
    fn read_blobs_reports_a_missing_object() -> TestResult {
        let source = SourceRepo::new()?;
        let missing = "0123456789abcdef0123456789abcdef01234567";
        let shown = read_blobs(OsStr::new("git"), &source.path(), &[missing])
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            shown,
            Some(format!(
                "git cat-file printed output agentstacks cannot read: \"{missing} missing\""
            ))
        );
        Ok(())
    }
}
