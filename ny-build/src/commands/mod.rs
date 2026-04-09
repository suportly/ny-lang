use clap::{Parser, Subcommand};

pub mod build;

#[derive(Parser)]
#[command(name = "ny-build")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compiles the current package
    Build(build::BuildArgs),
}
