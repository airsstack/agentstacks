//! Reads pack text into located tokens.
//!
//! Every text a pack holds is scanned exactly once, by this one function, whatever it came from.
//!
//! # Responsibilities
//!
//! - [`scan`] walks the text line by line (`\n` ends a line) and finds each `{{...}}`:
//!   - `\{{` is the escape and is skipped, and a `}}` without an opener is plain text;
//!   - a token closes with the first `}}` on its line, else it is refused as unterminated;
//!   - space and tab are the only whitespace between the braces; the first word is the kind and
//!     the rest are the target words.
//! - It never stops at the first refusal: it reports every refusal and keeps every good token, both
//!   in text order, and resumes after each closing `}}`.
//! - It records on a `root` token whether the text right after its closing `}}` reaches `native/`:
//!   one or more `/`, then the segment `native` that ends the text, a whitespace character or a
//!   `/`. It looks across the rest of the whole text.
//! - It applies every grammar rule that needs no knowledge of a pack.
//!
//! # Non-responsibilities
//!
//! Rules that need the pack or the file a token sits in (where `{{var arguments}}` may appear,
//! whether a target or an option exists) belong to the checks over a whole pack and a whole set of
//! packs, which read the [`LocatedToken`]s this function returns.

use std::ops::Range;

use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::tokens::error::TokenError;
use crate::types::tokens::kind::TokenKind;
use crate::types::tokens::located::LocatedToken;
use crate::types::tokens::origin::TextOrigin;
use crate::types::tokens::parsed::Token;
use crate::types::tokens::target::TokenTarget;

/// The path segment a `root` token must be followed by to reach `native/`.
const NATIVE_SEGMENT: &str = "native";

/// Whether `rest`, the text right after a `root` token, names the `native` segment: one or more
/// `/` (a repeated `/` is the same separator), then `native` ending the text, a whitespace
/// character or a `/`.
fn reaches_native(rest: &str) -> bool {
    let after_slashes = rest.trim_start_matches('/');
    if after_slashes.len() == rest.len() {
        return false;
    }
    after_slashes
        .strip_prefix(NATIVE_SEGMENT)
        .is_some_and(|after| {
            after
                .chars()
                .next()
                .is_none_or(|next| next == '/' || next.is_whitespace())
        })
}

/// Scans `text` into located tokens and the refusals met on the way.
///
/// Each token's span indexes `text`. `origin` says how to locate what is found.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use agentstacks::types::{TextOrigin, scan};
///
/// let origin = TextOrigin::Lines { file: "SKILL.md".into(), first_line: NonZeroUsize::MIN };
/// let (tokens, refusals) = scan("Use {{skill review}} first.\n{{bogus x}}", &origin);
///
/// let first = tokens.first().ok_or("no token found")?;
/// assert_eq!(first.token().to_string(), "{{skill review}}");
/// assert_eq!(first.token().span(), 4..20);
/// let refusals = refusals.into_sorted();
/// let refusal = refusals.first().ok_or("no refusal found")?;
/// assert_eq!(refusal.to_string(), "SKILL.md:2: unknown token kind \"bogus\"");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn scan(text: &str, origin: &TextOrigin) -> (Vec<LocatedToken>, Diagnostics) {
    let mut tokens = Vec::new();
    let mut diagnostics = Diagnostics::new();
    let mut line_start = 0;
    for (line_index, line) in text.split('\n').enumerate() {
        for found in scan_line(text, line, line_start) {
            match found {
                Ok(token) => tokens.push(LocatedToken::new(token, origin.locate(line_index))),
                Err(error) => diagnostics.push(Diagnostic::new(origin.locate(line_index), error)),
            }
        }
        line_start += line.len() + 1;
    }
    (tokens, diagnostics)
}

/// Reads every token on one line. `line_start` is the byte offset of `line` in `text`.
fn scan_line(text: &str, line: &str, line_start: usize) -> Vec<Result<Token, TokenError>> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = line[from..].find("{{") {
        let open = from + offset;
        if line[..open].ends_with('\\') {
            from = open + 2;
            continue;
        }
        let inside_start = open + 2;
        let Some(length) = line[inside_start..].find("}}") else {
            found.push(Err(TokenError::Unterminated));
            break;
        };
        let close = inside_start + length;
        let end = close + 2;
        let span = line_start + open..line_start + end;
        found.push(read_token(text, &line[inside_start..close], span));
        from = end;
    }
    found
}

/// Reads the text between a token's braces; `span` covers the whole token in `text`.
fn read_token(text: &str, inside: &str, span: Range<usize>) -> Result<Token, TokenError> {
    let mut words = inside.split([' ', '\t']).filter(|word| !word.is_empty());
    let Some(first) = words.next() else {
        return Err(TokenError::Empty);
    };
    let kind = TokenKind::parse(first)?;
    let target_words: Vec<&str> = words.collect();
    let target = TokenTarget::parse(kind, &target_words)?;
    let reaches_native =
        kind == TokenKind::Root && text.get(span.end..).is_some_and(reaches_native);
    Ok(Token::new(target, span, reaches_native))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::path::PathBuf;

    use super::*;
    use crate::types::tokens::render::render;
    use crate::types::tokens::values::TokenValues;

    fn lines(file: &str, first_line: usize) -> TextOrigin {
        TextOrigin::Lines {
            file: PathBuf::from(file),
            first_line: NonZeroUsize::new(first_line).unwrap_or(NonZeroUsize::MIN),
        }
    }

    fn field(file: &str, key_path: &str) -> TextOrigin {
        TextOrigin::Field {
            file: PathBuf::from(file),
            key_path: key_path.to_owned(),
        }
    }

    fn shown_tokens(tokens: &[LocatedToken]) -> Vec<String> {
        tokens.iter().map(|t| t.token().to_string()).collect()
    }

    fn shown_diagnostics(diagnostics: Diagnostics) -> Vec<String> {
        diagnostics
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    const KNOWN: &str = "session_id, arguments, tmpdir, home, airsstack_home, project_dir";

    #[test]
    fn scan_accepts_skill_root_and_skill_dir_tokens() {
        let origin = lines("notes.md", 1);
        let (tokens, diagnostics) = scan("run {{skill sdlc/status}} now", &origin);
        assert_eq!(shown_tokens(&tokens), ["{{skill sdlc/status}}"]);
        assert_eq!(
            tokens.first().map(|t| t.token().target().clone()),
            TokenTarget::parse(TokenKind::Skill, &["sdlc/status"]).ok()
        );
        assert!(diagnostics.is_empty());

        let (tokens, diagnostics) = scan("{{ root }}/scripts/x.lua", &origin);
        assert_eq!(shown_tokens(&tokens), ["{{root}}"]);
        assert!(diagnostics.is_empty());

        let (tokens, diagnostics) = scan("{{skill_dir discuss}}/references/protocol.md", &origin);
        assert_eq!(shown_tokens(&tokens), ["{{skill_dir discuss}}"]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn scan_treats_escape_and_lone_closer_as_text() {
        let origin = lines("notes.md", 1);
        for text in ["$\\{{ secrets.X }}", "a }} b"] {
            let (tokens, diagnostics) = scan(text, &origin);
            assert!(tokens.is_empty(), "{text}");
            assert!(diagnostics.is_empty(), "{text}");
        }
    }

    #[test]
    fn scan_refuses_each_malformed_token() {
        let origin = lines("notes.md", 1);
        let mut cases: Vec<(String, String)> = vec![
            (
                "{{skil core/x}}".to_owned(),
                "unknown token kind \"skil\"".to_owned(),
            ),
            ("{{skill core/x".to_owned(), "unterminated token".to_owned()),
            (
                "{{skill core/x\n}}".to_owned(),
                "unterminated token".to_owned(),
            ),
            ("{{}}".to_owned(), "empty token".to_owned()),
            ("{{ }}".to_owned(), "empty token".to_owned()),
            (
                "{{root core extra}}".to_owned(),
                "token \"root\" takes one target".to_owned(),
            ),
            (
                "{{skill a b}}".to_owned(),
                "token \"skill\" takes one target".to_owned(),
            ),
            (
                "{{root Core}}".to_owned(),
                "\"Core\" is not a valid name".to_owned(),
            ),
            (
                "{{skill Core/x}}".to_owned(),
                "\"Core\" is not a valid name".to_owned(),
            ),
            (
                "{{root core/x}}".to_owned(),
                "token \"root\" takes a pack name, not \"core/x\"".to_owned(),
            ),
            (
                "{{option a-b}}".to_owned(),
                "\"a-b\" is not a valid option name".to_owned(),
            ),
            (
                "{{option core/archive_keep}}".to_owned(),
                "option tokens name this pack's options only".to_owned(),
            ),
        ];
        for kind in ["skill", "agent", "skill_dir", "var", "option"] {
            cases.push((
                format!("{{{{{kind}}}}}"),
                format!("token \"{kind}\" needs a target"),
            ));
        }
        for target in ["a/b/c", "/x", "core/"] {
            cases.push((
                format!("{{{{skill {target}}}}}"),
                format!("\"{target}\" is not a valid target (expected [pack/]name)"),
            ));
        }
        for name in ["core/tmpdir", "session"] {
            cases.push((
                format!("{{{{var {name}}}}}"),
                format!("unknown var \"{name}\" (known: {KNOWN})"),
            ));
        }
        for (input, message) in cases {
            let (tokens, diagnostics) = scan(&input, &origin);
            assert!(tokens.is_empty(), "{input}");
            assert_eq!(
                shown_diagnostics(diagnostics),
                [format!("notes.md:1: {message}")],
                "{input}"
            );
        }
    }

    #[test]
    fn scan_accepts_tokens_whose_validity_depends_on_the_pack() {
        let origin = lines("notes.md", 1);
        for text in [
            "{{var arguments}}",
            "{{option archive_kep}}",
            "{{skill_dir core/coder}}",
        ] {
            let (tokens, diagnostics) = scan(text, &origin);
            assert_eq!(tokens.len(), 1, "{text}");
            assert!(diagnostics.is_empty(), "{text}");
        }
    }

    #[test]
    fn scan_reports_every_error_in_one_pass() {
        let origin = lines("notes.md", 1);
        let (tokens, diagnostics) = scan("{{skil x}}\n{{root}}\n{{}}", &origin);
        assert_eq!(shown_tokens(&tokens), ["{{root}}"]);
        assert_eq!(
            tokens.first().map(|t| t.location().to_string()),
            Some("notes.md:2".to_owned())
        );
        assert_eq!(
            shown_diagnostics(diagnostics),
            [
                "notes.md:1: unknown token kind \"skil\"",
                "notes.md:3: empty token"
            ]
        );
    }

    #[test]
    fn scan_gives_a_token_on_a_later_line_its_text_span_and_line() {
        let text = "intro\n{{root}}/native/x";
        let (tokens, diagnostics) = scan(text, &lines("n.md", 1));
        assert!(diagnostics.is_empty());
        let [found] = tokens.as_slice() else {
            assert_eq!(tokens.len(), 1);
            return;
        };
        assert_eq!(found.token().span(), 6..14);
        assert_eq!(text.get(found.token().span()), Some("{{root}}"));
        assert!(found.token().reaches_native());
        assert_eq!(found.location().to_string(), "n.md:2");

        let mut values = TokenValues::new();
        values.insert(found.token(), ".agentstacks/core".to_owned());
        assert_eq!(render(text, &values), "intro\n.agentstacks/core/native/x");
    }

    #[test]
    fn scan_continues_after_an_error_on_the_same_line() {
        let origin = lines("notes.md", 1);
        let (tokens, diagnostics) = scan("{{skil x}} then {{root}}", &origin);
        assert_eq!(shown_tokens(&tokens), ["{{root}}"]);
        assert_eq!(
            shown_diagnostics(diagnostics),
            ["notes.md:1: unknown token kind \"skil\""]
        );
    }

    #[test]
    fn scan_finds_several_tokens_and_an_escape_on_one_line() {
        let origin = lines("notes.md", 1);
        let text = "see {{skill status}}, \\{{literal}} and {{root}}/x";
        let (tokens, diagnostics) = scan(text, &origin);
        assert!(diagnostics.is_empty());
        assert_eq!(shown_tokens(&tokens), ["{{skill status}}", "{{root}}"]);
        let spans: Vec<&str> = tokens
            .iter()
            .filter_map(|t| text.get(t.token().span()))
            .collect();
        assert_eq!(spans, ["{{skill status}}", "{{root}}"]);
    }

    #[test]
    fn scan_offsets_lines_from_first_line() {
        let origin = lines("SKILL.md", 6);
        let (tokens, diagnostics) = scan("intro\n{{skil x}}", &origin);
        assert!(tokens.is_empty());
        assert_eq!(
            shown_diagnostics(diagnostics),
            ["SKILL.md:7: unknown token kind \"skil\""]
        );
    }

    #[test]
    fn scan_locates_field_tokens_by_key_path() {
        let origin = field("pack.toml", "hooks[1].args[1]");
        let (tokens, diagnostics) = scan("{{root}}\n{{skil x}}", &origin);
        assert_eq!(shown_tokens(&tokens), ["{{root}}"]);
        assert_eq!(
            tokens.first().map(|t| t.location().to_string()),
            Some("pack.toml: hooks[1].args[1]".to_owned())
        );
        assert_eq!(
            shown_diagnostics(diagnostics),
            ["pack.toml: hooks[1].args[1]: unknown token kind \"skil\""]
        );
    }

    #[test]
    fn scan_applies_option_rows_to_field_text() {
        let origin = field("pack.toml", "hooks[0].text");
        let cases = [
            (
                "{{option core/archive_keep}}",
                "pack.toml: hooks[0].text: option tokens name this pack's options only",
            ),
            (
                "{{option a-b}}",
                "pack.toml: hooks[0].text: \"a-b\" is not a valid option name",
            ),
        ];
        for (text, message) in cases {
            let (tokens, diagnostics) = scan(text, &origin);
            assert!(tokens.is_empty(), "{text}");
            assert_eq!(shown_diagnostics(diagnostics), [message], "{text}");
        }
    }

    #[test]
    fn scan_marks_root_followed_by_native() {
        let origin = lines("notes.md", 1);
        let cases = [
            ("{{root}}/native/claude/x", true),
            ("{{root core}}/native/x", true),
            ("{{root}}/scripts/x", false),
            ("{{root}}/natives/x", false),
            ("{{root}}/hooks/native/x", false),
            ("{{root}}", false),
            ("{{skill_dir a}}/native/x", false),
            ("{{root}}/native", true),
            ("{{root}}//native/x", true),
            ("{{root}}///native/x", true),
            ("{{root}}//native", true),
            ("{{root}}/native\nnext line", true),
            ("{{root}}/nativex", false),
            ("{{root}}//natives/x", false),
            ("{{root}}//", false),
            ("{{root}}/./native/x", false),
        ];
        for (text, reaches) in cases {
            let (tokens, diagnostics) = scan(text, &origin);
            assert!(diagnostics.is_empty(), "{text}");
            assert_eq!(
                tokens
                    .iter()
                    .map(|t| t.token().reaches_native())
                    .collect::<Vec<_>>(),
                [reaches],
                "{text}"
            );
        }
    }

    #[test]
    fn scan_marks_native_reach_in_field_text() {
        let origin = field("pack.toml", "hooks[0].args[0]");
        let (tokens, diagnostics) = scan("{{root}}/native/claude/x", &origin);
        assert!(diagnostics.is_empty());
        assert_eq!(
            tokens.first().map(|t| t.token().reaches_native()),
            Some(true)
        );
        assert_eq!(
            tokens.first().map(|t| t.location().to_string()),
            Some("pack.toml: hooks[0].args[0]".to_owned())
        );
    }
}
