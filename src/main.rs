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
    /// Run all test_* functions in a Ny source file
    Test {
        /// Path to .ny source file
        file: PathBuf,
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
        Commands::Test { file } => {
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

            // Parse the source to find test functions
            let tokens = match ny::lexer::tokenize(&source) {
                Ok(t) => t,
                Err(errors) => {
                    ny::diagnostics::print_errors(&file, &source, &errors);
                    process::exit(1);
                }
            };
            let program = match ny::parser::parse(tokens) {
                Ok(p) => p,
                Err(errors) => {
                    ny::diagnostics::print_errors(&file, &source, &errors);
                    process::exit(1);
                }
            };

            // Find all test_* functions and their return types
            let test_fns: Vec<(String, bool)> = program
                .items
                .iter()
                .filter_map(|item| match item {
                    ny::parser::ast::Item::FunctionDef {
                        name, return_type, ..
                    } if name.starts_with("test_") => {
                        let returns_i32 = return_type.name_str() == "i32";
                        Some((name.clone(), returns_i32))
                    }
                    _ => None,
                })
                .collect();

            if test_fns.is_empty() {
                println!("no test functions found (functions must be named test_*)");
                process::exit(0);
            }

            println!("running {} tests", test_fns.len());

            // For each test function, generate a wrapper program that calls it
            let mut passed = 0;
            let mut failed = 0;

            // Remove existing main function from source using simple text matching
            let source_no_main = remove_main_function(&source);

            for (test_name, returns_i32) in &test_fns {
                // Build a wrapper: main() calls test_fn()
                let wrapper_source = if *returns_i32 {
                    format!(
                        "{}\nfn main() -> i32 {{ return {}(); }}",
                        source_no_main, test_name,
                    )
                } else {
                    format!(
                        "{}\nfn main() -> i32 {{ {}(); return 0; }}",
                        source_no_main, test_name,
                    )
                };

                let tmp_dir = std::env::temp_dir();
                let tmp_src = tmp_dir.join(format!("ny_test_{}.ny", test_name));
                let tmp_out = tmp_dir.join(format!("ny_test_{}", test_name));

                // Only compile tests that have void/unit return (return 0 on success)
                std::fs::write(&tmp_src, &wrapper_source).unwrap();

                match ny::compile(
                    &wrapper_source,
                    &tmp_src,
                    &tmp_out,
                    0,
                    "exe",
                ) {
                    Ok(()) => {
                        let status = process::Command::new(&tmp_out)
                            .status()
                            .unwrap_or_else(|_| process::exit(1));
                        if status.success() {
                            println!("  test {} ... ok", test_name);
                            passed += 1;
                        } else {
                            println!("  test {} ... FAILED (exit code {})", test_name, status.code().unwrap_or(-1));
                            failed += 1;
                        }
                    }
                    Err(_errors) => {
                        println!("  test {} ... FAILED (compile error)", test_name);
                        failed += 1;
                    }
                }

                let _ = std::fs::remove_file(&tmp_src);
                let _ = std::fs::remove_file(&tmp_out);
            }

            println!("\ntest result: {} passed, {} failed", passed, failed);
            if failed > 0 {
                process::exit(1);
            }
        }
    }
}

/// Remove the `fn main() -> i32 { ... }` function from source text.
fn remove_main_function(source: &str) -> String {
    let mut result = String::new();
    let mut in_main = false;
    let mut brace_depth = 0;
    for line in source.lines() {
        let trimmed = line.trim();
        if !in_main && trimmed.starts_with("fn main(") {
            in_main = true;
            brace_depth = 0;
            for ch in line.chars() {
                if ch == '{' {
                    brace_depth += 1;
                } else if ch == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        in_main = false;
                    }
                }
            }
            continue;
        }
        if in_main {
            for ch in line.chars() {
                if ch == '{' {
                    brace_depth += 1;
                } else if ch == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        in_main = false;
                    }
                }
            }
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}

fn emit_to_str(emit: &EmitType) -> &'static str {
    match emit {
        EmitType::Exe => "exe",
        EmitType::LlvmIr => "llvm-ir",
        EmitType::Obj => "obj",
    }
}
