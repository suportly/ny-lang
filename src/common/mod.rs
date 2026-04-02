mod span;
mod types;

pub use span::Span;
pub use types::NyType;

/// Levenshtein edit distance between two strings.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate() {
        *val = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax,
    Type,
    Name,
    Immutability,
    IO,
    Internal,
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub span: Span,
    pub kind: ErrorKind,
    pub secondary: Vec<(Span, String)>,
    pub notes: Vec<String>,
}

impl CompileError {
    pub fn syntax(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            kind: ErrorKind::Syntax,
            secondary: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn type_error(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            kind: ErrorKind::Type,
            secondary: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn name_error(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            kind: ErrorKind::Name,
            secondary: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn immutability(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            kind: ErrorKind::Immutability,
            secondary: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.secondary.push((span, message.into()));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}
