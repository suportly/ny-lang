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

            match ny::compile(&source, &file, &tmp_out, opt_level, "exe", "native") {
                Ok(()) => {
                    let status = process::Command::new(&tmp_out)
                        .status()
                        .unwrap_or_else(|e| {
                            eprintln!("error: failed to execute: {}", e);
                            process::exit(1);
                        });
                    let _ = std::fs::remove_file(&tmp_out);
                    process::exit(status.code().unwrap_or(1));
                }
                Err(errors) => {
                    ny::diagnostics::print_errors(&file, &source, &errors);
                    let _ = std::fs::remove_file(&tmp_out);
                    process::exit(1);
                }
            }
        }
        Commands::Check { file } => {
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

            let start = std::time::Instant::now();

            // Lexer
            let tokens = match ny::lexer::tokenize(&source) {
                Ok(t) => t,
                Err(errors) => {
                    ny::diagnostics::print_errors(&file, &source, &errors);
                    process::exit(1);
                }
            };
            let lex_time = start.elapsed();

            // Parser
            let mut program = match ny::parser::parse(tokens) {
                Ok(p) => p,
                Err(errors) => {
                    ny::diagnostics::print_errors(&file, &source, &errors);
                    process::exit(1);
                }
            };
            let parse_time = start.elapsed() - lex_time;

            // Module resolution
            let base_dir = file.parent().unwrap_or(std::path::Path::new("."));
            let mut visited = std::collections::HashSet::new();
            visited.insert(file.to_path_buf());
            if let Err(errors) = ny::resolve_uses_pub(&mut program, base_dir, &mut visited) {
                ny::diagnostics::print_errors(&file, &source, &errors);
                process::exit(1);
            }

            // Monomorphize
            ny::monomorphize::monomorphize(&mut program);

            // Semantic analysis
            match ny::semantic::analyze(&program) {
                Ok(_) => {
                    let total = start.elapsed();
                    let lines = source.lines().count();
                    eprintln!(
                        "ok: {} ({} lines, {:.0}ms — lex {:.0}ms, parse {:.0}ms, check {:.0}ms)",
                        file.display(),
                        lines,
                        total.as_secs_f64() * 1000.0,
                        lex_time.as_secs_f64() * 1000.0,
                        parse_time.as_secs_f64() * 1000.0,
                        (total - lex_time - parse_time).as_secs_f64() * 1000.0,
                    );
                }
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

            let total_start = std::time::Instant::now();
            println!("running {} tests", test_fns.len());

            let mut passed = 0;
            let mut failed = 0;

            let source_no_main = remove_main_function(&source);

            for (test_name, returns_i32) in &test_fns {
                let test_start = std::time::Instant::now();

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

                std::fs::write(&tmp_src, &wrapper_source).unwrap();

                match ny::compile(&wrapper_source, &tmp_src, &tmp_out, 0, "exe", "native") {
                    Ok(()) => {
                        let output = process::Command::new(&tmp_out)
                            .output()
                            .unwrap_or_else(|_| process::exit(1));
                        let elapsed = test_start.elapsed();
                        if output.status.success() {
                            println!(
                                "  test {} ... ok ({:.0}ms)",
                                test_name,
                                elapsed.as_secs_f64() * 1000.0
                            );
                            passed += 1;
                        } else {
                            println!(
                                "  test {} ... FAILED (exit code {}, {:.0}ms)",
                                test_name,
                                output.status.code().unwrap_or(-1),
                                elapsed.as_secs_f64() * 1000.0
                            );
                            // Show stderr if any
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            if !stderr.is_empty() {
                                eprintln!("    {}", stderr.trim().replace('\n', "\n    "));
                            }
                            failed += 1;
                        }
                    }
                    Err(errors) => {
                        let elapsed = test_start.elapsed();
                        println!(
                            "  test {} ... FAILED (compile error, {:.0}ms)",
                            test_name,
                            elapsed.as_secs_f64() * 1000.0
                        );
                        for err in &errors {
                            eprintln!("    error: {}", err.message);
                        }
                        failed += 1;
                    }
                }

                let _ = std::fs::remove_file(&tmp_src);
                let _ = std::fs::remove_file(&tmp_out);
            }

            let total_elapsed = total_start.elapsed();
            println!(
                "\ntest result: {} passed, {} failed ({:.0}ms)",
                passed,
                failed,
                total_elapsed.as_secs_f64() * 1000.0
            );
            if failed > 0 {
                process::exit(1);
            }
        }
        Commands::Repl => {
            eprintln!("Ny Lang REPL v0.1.0 — type expressions, :q to quit");
            eprintln!("  Wrap in fn main() -> i32 {{ ... }} automatically.");
            eprintln!();

            let tmp_dir = std::env::temp_dir();
            let tmp_src = tmp_dir.join("ny_repl.ny");
            let tmp_out = tmp_dir.join("ny_repl_bin");

            // Accumulate declarations (structs, functions, etc.)
            let mut decls = String::new();
            use std::io::BufRead;
            let stdin = std::io::stdin();
            loop {
                eprint!("ny> ");
                let mut line = String::new();
                match stdin.lock().read_line(&mut line) {
                    Ok(0) => break, // EOF
                    Err(_) => break,
                    _ => {}
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed == ":q" || trimmed == ":quit" || trimmed == "exit" {
                    eprintln!("Goodbye!");
                    break;
                }
                if trimmed == ":clear" {
                    decls.clear();
                    eprintln!("  (declarations cleared)");
                    continue;
                }

                // If it's a declaration (fn, struct, enum, use, extern, trait, impl), accumulate
                if trimmed.starts_with("fn ")
                    || trimmed.starts_with("struct ")
                    || trimmed.starts_with("enum ")
                    || trimmed.starts_with("use ")
                    || trimmed.starts_with("extern ")
                    || trimmed.starts_with("trait ")
                    || trimmed.starts_with("impl ")
                {
                    // Read multi-line block until brace count balances
                    let mut block = line.clone();
                    let mut depth: i32 = 0;
                    for ch in block.chars() {
                        if ch == '{' {
                            depth += 1;
                        }
                        if ch == '}' {
                            depth -= 1;
                        }
                    }
                    while depth > 0 {
                        eprint!("... ");
                        let mut cont = String::new();
                        if stdin.lock().read_line(&mut cont).unwrap_or(0) == 0 {
                            break;
                        }
                        for ch in cont.chars() {
                            if ch == '{' {
                                depth += 1;
                            }
                            if ch == '}' {
                                depth -= 1;
                            }
                        }
                        block.push_str(&cont);
                    }
                    decls.push_str(&block);
                    decls.push('\n');
                    eprintln!("  (defined)");
                    continue;
                }

                // Otherwise, treat as expression/statement inside main
                let source = format!(
                    "{}\nfn main() -> i32 {{\n  {};\n  return 0;\n}}\n",
                    decls, trimmed
                );

                std::fs::write(&tmp_src, &source).unwrap();

                match ny::compile(&source, &tmp_src, &tmp_out, 0, "exe", "native") {
                    Ok(()) => {
                        let output = process::Command::new(&tmp_out)
                            .output()
                            .unwrap_or_else(|e| {
                                eprintln!("  error: {}", e);
                                process::exit(1);
                            });
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if !stdout.is_empty() {
                            print!("{}", stdout);
                        }
                        let code = output.status.code().unwrap_or(0);
                        if code != 0 {
                            eprintln!("  (exit code: {})", code);
                        }
                    }
                    Err(errors) => {
                        for err in &errors {
                            eprintln!("  error: {}", err.message);
                        }
                    }
                }

                let _ = std::fs::remove_file(&tmp_out);
            }
            let _ = std::fs::remove_file(&tmp_src);
        }
        Commands::Fmt { file, write, check } => {
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

            let formatted = ny::formatter::format_program_with_source(&program, &source);

            if check {
                if formatted != source {
                    eprintln!("{} needs formatting", file.display());
                    process::exit(1);
                }
            } else if write {
                if formatted != source {
                    std::fs::write(&file, &formatted).unwrap();
                    eprintln!("formatted {}", file.display());
                }
            } else {
                print!("{}", formatted);
            }
        }
        Commands::Pkg { command } => {
            let cwd = std::env::current_dir().unwrap();
            let result = match command {
                PkgCommands::Init => ny::pkg::commands::cmd_init(&cwd),
                PkgCommands::Add { url, name, branch } => {
                    ny::pkg::commands::cmd_add(&cwd, &url, name.as_deref(), branch.as_deref())
                }
                PkgCommands::Build => ny::pkg::commands::cmd_build(&cwd),
                PkgCommands::Remove { name } => ny::pkg::commands::cmd_remove(&cwd, &name),
                PkgCommands::List => ny::pkg::commands::cmd_list(&cwd),
            };
            if let Err(e) = result {
                eprintln!("error: {}", e);
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
