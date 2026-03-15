pub mod ast;
pub mod errors;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod token;
pub mod typecheck;
pub mod types;

use std::path::Path;

use errors::CompileError;

/// Placeholder for a successfully compiled P program.
#[derive(Debug)]
pub struct CompiledProgram {
    pub programs: Vec<ast::Program>,
}

/// Compile all .p files in a directory into a program.
pub fn compile(test_dir: &Path) -> Result<CompiledProgram, Vec<CompileError>> {
    let mut sources = Vec::new();
    if test_dir.is_dir() {
        for entry in std::fs::read_dir(test_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "p") {
                let content = std::fs::read_to_string(&path).unwrap();
                sources.push((path, content));
            }
        }
    }
    if sources.is_empty() {
        return Err(vec![CompileError::new(format!(
            "No .p files found in {}",
            test_dir.display()
        ))]);
    }

    // Phase 1: Lex all sources
    let mut all_tokens = Vec::new();
    for (path, source) in &sources {
        match lexer::lex(source) {
            Ok(tokens) => all_tokens.push((path.clone(), tokens, source.clone())),
            Err(e) => {
                return Err(vec![CompileError::from_offset(
                    format!("Lexer error in {}: {e}", path.display()),
                    source,
                    e.offset,
                )]);
            }
        }
    }

    // Phase 2: Parse all sources
    let mut programs = Vec::new();
    let mut combined_source = String::new();
    for (_path, tokens, source) in all_tokens {
        let mut p = parser::Parser::new(tokens, source.clone());
        match p.parse_program() {
            Ok(prog) => {
                programs.push(prog);
                if !combined_source.is_empty() {
                    combined_source.push('\n');
                }
                combined_source.push_str(&source);
            }
            Err(e) => {
                return Err(vec![CompileError::from_offset(
                    format!("Parse error: {e}"),
                    &source,
                    e.span.start,
                )]);
            }
        }
    }

    // Phase 3: Type check
    typecheck::check_program(&programs, &combined_source)?;

    Ok(CompiledProgram { programs })
}
