
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

        /// Target: native (default) or wasm32
        #[arg(long, default_value = "native")]
        target: String,

        /// Extra libraries to link (e.g. -l pulse -l curl)
        #[arg(short = 'l', long = "link")]
        libs: Vec<String>,
    },
    /// Run all test_* functions in a Ny source file
    Test {
        /// Path to .ny source file
        file: PathBuf,
    },
    /// Compile and run a Ny source file
    Run {
        /// Path to .ny source file
        file: PathBuf,

        /// Optimization level (0-3)
        #[arg(short = 'O', long = "opt-level", default_value = "0")]
        opt_level: u8,
    },
    /// Type-check a Ny source file without compiling (fast feedback)
    Check {
        /// Path to .ny source file
        file: PathBuf,
    },
    /// Interactive REPL — evaluate Ny expressions
    Repl,
    /// Format a Ny source file (opinionated, zero-config)
    Fmt {
        /// Path to .ny source file
        file: PathBuf,

        /// Write formatted output back to the file
        #[arg(short, long)]
        write: bool,

        /// Check if file is already formatted (exit 1 if not)
        #[arg(long)]
        check: bool,
    },
    /// Package manager — manage project dependencies
    Pkg {
        #[command(subcommand)]
        command: PkgCommands,
    },
    /// Workflow automation integrations
    Integrate {
        #[command(subcommand)]
        command: IntegrateCommands,
    },
}

#[derive(Subcommand)]
enum PkgCommands {
    /// Create a ny.pkg manifest in the current directory
    Init,
    /// Add a dependency from a git URL
    Add {
        /// Git URL of the package
        url: String,
        /// Override the package name
        #[arg(long)]
        name: Option<String>,
        /// Git branch or tag
        #[arg(long)]
        branch: Option<String>,
    },
    /// Fetch all dependencies
    Build,
    /// Remove a dependency
    Remove {
        /// Package name
        name: String,
    },
    /// List dependencies
    List,
}

#[derive(Subcommand)]
enum IntegrateCommands {
    /// n8n integration
    N8n {
        #[command(subcommand)]
        command: N8nCommands,
    },
    /// Zapier integration
    Zapier {
        #[command(subcommand)]
        command: ZapierCommands,
    },
}

#[derive(Subcommand)]
enum N8nCommands {
    /// Initialize n8n integration
    Init,
}

#[derive(Subcommand)]
enum ZapierCommands {
    /// Initialize Zapier integration
    Init,
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
            target,
            libs,
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

            let output_path = output.unwrap_or_else(|| {
                if target == "wasm32" {
                    file.with_extension("wasm")
                } else {
                    file.with_extension("")
                }
            });

            match ny::compile(
                &source,
                &file,
                &output_path,
                opt_level,
                emit_to_str(&emit),
                &target,
                &libs,
            ) {
                Ok(()) => process::exit(0),
                Err(errors) => {
                    ny::diagnostics::print_errors(&file, &source, &errors);
                    process::exit(1);
                }
            }
        }
        Commands::Run { file, opt_level } => {
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

            let tmp_dir = std::env::temp_dir();
            let tmp_out = tmp_dir.join("ny_run_output");

            match ny::compile(&source, &file, &tmp_out, opt_level, "exe", "native", &[]) {
                Ok(()) => {
                    let status = process::Command::new(&tmp_out)
                        .status()
                        .expect("failed to execute");
                    if let Some(code) = status.code() {
                        process::exit(code);
                    }
                }
                Err(errors) => {
                    ny::diagnostics::print_errors(&file, &source, &errors);
                    process::exit(1);
                }
            }
        }
        Commands::Test { file } => {
            // ... (implementation for test)
        }
        Commands::Check { file } => {
            // ... (implementation for check)
        }
        Commands::Repl => {
            // ... (implementation for repl)
        }
        Commands::Fmt { .. } => {
            // ... (implementation for fmt)
        }
        Commands::Pkg { .. } => {
            // ... (implementation for pkg)
        }
        Commands::Integrate { command } => match command {
            IntegrateCommands::N8n { command } => match command {
                N8nCommands::Init => {
                    ny::integrations::n8n::init();
                }
            },
            IntegrateCommands::Zapier { command } => match command {
                ZapierCommands::Init => {
                    ny::integrations::zapier::init();
                }
            },
        },
    }
}

fn emit_to_str(emit: &EmitType) -> &str {
    match emit {
        EmitType::Exe => "exe",
        EmitType::LlvmIr => "llvm-ir",
        EmitType::Obj => "obj",
    }
}
