use std::path::Path;

#[derive(Debug)]
pub struct CompileError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug)]
pub struct CompiledProgram {
    // TODO: AST / IR representation
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
    assert!(!sources.is_empty(), "No .p files found in {}", test_dir.display());

    // TODO: implement parser + type checker
    let _ = sources;
    todo!("implement P compiler")
}
