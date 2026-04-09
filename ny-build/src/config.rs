use serde::Deserialize;
use std::fs;
use std::path::Path;
use anyhow::Result;
use crate::error::BuildError;

#[derive(Deserialize, Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub authors: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub package: Package,
}

pub fn load_config(manifest_path: &Path) -> Result<Config> {
    if !manifest_path.exists() {
        return Err(BuildError::ManifestNotFound(manifest_path.to_path_buf()).into());
    }

    let contents = fs::read_to_string(manifest_path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}
