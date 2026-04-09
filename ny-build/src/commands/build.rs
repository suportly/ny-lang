use clap::Args;
use std::path::{Path, PathBuf};
use anyhow::Result;
use std::fs;
use fs_extra::dir::{self, CopyOptions};

use crate::config::load_config;
use crate::error::BuildError;

#[derive(Args, Debug)]
pub struct BuildArgs {
    /// Path to the project manifest
    #[arg(long, default_value = "NyProject.toml")]
    pub manifest_path: PathBuf,

    /// Build for release
    #[arg(long)]
    pub release: bool,
}

pub fn run(args: BuildArgs) -> Result<()> {
    let manifest_path = &args.manifest_path;
    let config = load_config(manifest_path)?;

    println!("Building package: {}", config.package.name);

    let project_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let build_dir = project_dir.join("build");
    
    if !build_dir.exists() {
        fs::create_dir_all(&build_dir)
            .map_err(|e| BuildError::BuildDirCreation(e.to_string()))?;
    }

    // This is a placeholder for the actual compilation logic.
    // In a real build tool, this would invoke the `ny` compiler.
    // For now, we'll just copy the source files to the build directory.
    let src_dir = project_dir.join("src");
    if src_dir.exists() {
        let mut options = CopyOptions::new();
        options.overwrite = true;
        dir::copy(&src_dir, &build_dir, &options)?;
    }

    println!("Finished build for package: {}", config.package.name);
    Ok(())
}
