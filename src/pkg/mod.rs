pub mod commands;
pub mod fetch;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
}

#[derive(Debug)]
pub enum PkgError {
    NotFound(String),
    AlreadyExists(String),
    GitFailed(String),
    IoError(std::io::Error),
    ParseError(String),
}

impl fmt::Display for PkgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PkgError::NotFound(msg) => write!(f, "{}", msg),
            PkgError::AlreadyExists(msg) => write!(f, "{}", msg),
            PkgError::GitFailed(msg) => write!(f, "git: {}", msg),
            PkgError::IoError(e) => write!(f, "io: {}", e),
            PkgError::ParseError(msg) => write!(f, "parse: {}", msg),
        }
    }
}

impl From<std::io::Error> for PkgError {
    fn from(e: std::io::Error) -> Self {
        PkgError::IoError(e)
    }
}

const MANIFEST_FILE: &str = "ny.pkg";
const DEPS_DIR: &str = ".ny_deps";

impl Manifest {
    pub fn find_project_root(start: &Path) -> Option<PathBuf> {
        let mut dir = if start.is_file() {
            start.parent()?.to_path_buf()
        } else {
            start.to_path_buf()
        };
        loop {
            if dir.join(MANIFEST_FILE).exists() {
                return Some(dir);
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    pub fn load(project_root: &Path) -> Result<Self, PkgError> {
        let path = project_root.join(MANIFEST_FILE);
        let content = std::fs::read_to_string(&path).map_err(|_| {
            PkgError::NotFound(format!(
                "no {} found — run `ny pkg init` first",
                MANIFEST_FILE
            ))
        })?;
        serde_json::from_str(&content)
            .map_err(|e| PkgError::ParseError(format!("invalid {}: {}", MANIFEST_FILE, e)))
    }

    pub fn save(&self, project_root: &Path) -> Result<(), PkgError> {
        let path = project_root.join(MANIFEST_FILE);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| PkgError::ParseError(e.to_string()))?;
        std::fs::write(&path, format!("{}\n", json))?;
        Ok(())
    }

    pub fn deps_dir(project_root: &Path) -> PathBuf {
        project_root.join(DEPS_DIR)
    }
}
