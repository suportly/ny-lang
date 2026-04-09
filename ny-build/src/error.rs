use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Manifest file not found at: {0:?}")]
    ManifestNotFound(PathBuf),

    #[error("Failed to create build directory: {0}")]
    BuildDirCreation(String),

    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
}
