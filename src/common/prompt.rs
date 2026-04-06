// src/common/prompt.rs

/// Represents a prompt for the AI code generator.
#[derive(Debug, Clone)]
pub struct Prompt {
    content: String,
}

impl Prompt {
    /// Creates a new prompt from a string.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Creates a new prompt from a natural language description.
    pub fn from_description(description: &str) -> Self {
        let content = format!(
            "Generate a Ny Lang code snippet for the following description:\n\n{}",
            description
        );
        Self { content }
    }

    /// Returns the prompt as a string.
    pub fn to_string(&self) -> String {
        self.content.clone()
    }
}
