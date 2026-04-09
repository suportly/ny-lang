use clap::Parser;
use anyhow::Result;

mod commands;
mod config;
mod error;

use commands::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build(args) => {
            commands::build::run(args)?;
        }
        // Other commands will be handled here
    }

    Ok(())
}
