/// A compilation error with location info.
#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(col)) => write!(f, "{}:{}: {}", line, col, self.message),
            (Some(line), None) => write!(f, "{}: {}", line, self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

impl CompileError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
        }
    }

    pub fn at(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    /// Compute line/column from a byte offset in source text.
    pub fn from_offset(message: impl Into<String>, source: &str, offset: usize) -> Self {
        let (line, col) = offset_to_line_col(source, offset);
        Self {
            message: message.into(),
            line: Some(line),
            column: Some(col),
        }
    }
}

/// Convert a byte offset to 1-based line and column.
pub fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
