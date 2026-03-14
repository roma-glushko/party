use logos::Logos;

use super::token::{Span, SpannedToken, Token};

/// Lex source code into a vector of spanned tokens.
/// Returns an error with the byte offset of the first unrecognized token.
pub fn lex(source: &str) -> Result<Vec<SpannedToken>, LexError> {
    let mut tokens = Vec::new();
    let mut lexer = Token::lexer(source);

    while let Some(result) = lexer.next() {
        let span = lexer.span();
        match result {
            Ok(kind) => {
                tokens.push(SpannedToken {
                    kind,
                    span: Span::new(span.start, span.end),
                });
            }
            Err(()) => {
                return Err(LexError {
                    offset: span.start,
                    text: source[span.start..span.end].to_string(),
                });
            }
        }
    }

    Ok(tokens)
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub offset: usize,
    pub text: String,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unexpected token '{}' at byte offset {}",
            self.text, self.offset
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn lex_simple_machine() {
        let src = r#"
            event Ping : machine;
            event Pong;

            machine Main {
                var x: int;
                start state Init {
                    entry {
                        x = 0;
                    }
                }
            }
        "#;
        let tokens = lex(src).expect("should lex");
        assert!(!tokens.is_empty());
        assert_eq!(tokens[0].kind, Token::Event);
    }

    #[test]
    fn lex_string_literal() {
        let src = r#"assert(true, "hello \"world\"");"#;
        let tokens = lex(src).expect("should lex");
        let string_tok = tokens
            .iter()
            .find(|t| t.kind == Token::StringLiteral)
            .expect("should have string literal");
        assert_eq!(string_tok.kind, Token::StringLiteral);
    }

    #[test]
    fn lex_nondet() {
        let src = "if ($) { } if ($$) { }";
        let tokens = lex(src).expect("should lex");
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(kinds.contains(&&Token::Nondet));
        assert!(kinds.contains(&&Token::FairNondet));
    }

    #[test]
    fn lex_all_testdata() {
        let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
        let mut count = 0;
        let mut unexpected_failures = Vec::new();

        // These test cases intentionally contain lexer errors
        let expected_lex_errors = ["LexerError1", "LexerError2"];

        for entry in walkdir(&testdata) {
            if entry.extension().is_some_and(|e| e == "p") {
                count += 1;
                let source = std::fs::read_to_string(&entry).unwrap();
                let is_expected_failure = expected_lex_errors
                    .iter()
                    .any(|name| entry.to_string_lossy().contains(name));

                match (lex(&source), is_expected_failure) {
                    (Ok(_), false) => {}             // expected success
                    (Err(_), true) => {}             // expected failure
                    (Ok(_), true) => {
                        unexpected_failures
                            .push(format!("{}: expected lex error but succeeded", entry.display()));
                    }
                    (Err(e), false) => {
                        unexpected_failures.push(format!("{}: {e}", entry.display()));
                    }
                }
            }
        }

        assert!(count > 400, "should find 400+ .p files, found {count}");
        if !unexpected_failures.is_empty() {
            panic!(
                "{} of {count} files had unexpected results:\n{}",
                unexpected_failures.len(),
                unexpected_failures.join("\n")
            );
        }
    }

    fn walkdir(dir: &Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    files.extend(walkdir(&path));
                } else {
                    files.push(path);
                }
            }
        }
        files
    }
}
