use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "ny", version, about = "Ny Lang compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a Ny source file to a native executable
    Build {
        /// Path to .ny source file
        file: PathBuf,

        /// Output executable path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output type
        #[arg(long, default_value = "exe")]
        emit: EmitType,

        /// Optimization level (0-3)
        #[arg(short = 'O', long = "opt-level", default_value = "0")]
        opt_level: u8,
    },
}

#[derive(Clone, ValueEnum)]
enum EmitType {
    Exe,
    LlvmIr,
    Obj,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            file,
            output,
            emit,
            opt_level,
        } => {
            if !file.exists() {
                eprintln!(
                    "error: could not read file `{}`: No such file or directory",
                    file.display()
                );
                process::exit(2);
            }

            let source = match std::fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read file `{}`: {}", file.display(), e);
                    process::exit(2);
                }
            };

            let output_path = output.unwrap_or_else(|| file.with_extension(""));

            match ny::compile(&source, &file, &output_path, opt_level, emit_to_str(&emit)) {
                Ok(()) => process::exit(0),
                Err(errors) => {
                    ny::diagnostics::print_errors(&file, &source, &errors);
                    process::exit(1);
                }
            }
        }
    }
}

fn emit_to_str(emit: &EmitType) -> &'static str {
    match emit {
        EmitType::Exe => "exe",
        EmitType::LlvmIr => "llvm-ir",
        EmitType::Obj => "obj",
    }
}
