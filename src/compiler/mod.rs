pub mod ast;
pub mod errors;
pub mod lexer;
pub mod parser;
pub mod token;

use std::path::Path;

use errors::CompileError;

/// Placeholder for a successfully compiled P program.
#[derive(Debug)]
pub struct CompiledProgram {
    // TODO: Will be populated with typed IR in Phase 3
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
    for (path, source) in &sources {
        if let Err(e) = lexer::lex(source) {
            return Err(vec![CompileError::from_offset(
                format!("Lexer error in {}: {e}", path.display()),
                source,
                e.offset,
            )]);
        }
    }

    // TODO: Phase 2 - Parse
    // TODO: Phase 3 - Type check
    // For now, just return success if lexing passed
    Ok(CompiledProgram {})
}
