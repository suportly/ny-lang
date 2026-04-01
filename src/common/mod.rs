mod span;
mod types;

pub use span::Span;
pub use types::NyType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax,
    Type,
    Name,
    Immutability,
    IO,
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
