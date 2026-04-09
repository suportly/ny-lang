// Copyright © 2022-2024, Ny Lang Team.
//
// See ../LICENSE for license information.

//! # The Ny Programming Language
//!
//! ## Disambiguation
//!
//! The official name of the language is "Ny". When referring to the compiler,
//! please use "the Ny compiler".
//!
//! The official file extension for Ny source files is `.ny`.
//!
//! ## About
//!
//! Ny is a low-level, high-performance, compiled systems programming language
//! that is designed to be simple, fast, and safe. It is inspired by Rust, but
//! with a simpler syntax and a smaller feature set.
//!
//! ## Usage
//!
//! The compiler is not yet ready for public use. Please check back later.

pub mod codegen;
pub mod common;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod slm;

use std::fs;
use std::path::{Path, PathBuf};

use codegen::CodeGenerator;
use common::{SourceFile, Target};
use diagnostics::{Diagnostic, Reporter};
use lexer::Lexer;
use parser::Parser;
use semantic::{analyze, GlobalScope};

/// The main entry point for the compiler.
#[derive(Debug, Default)]
pub struct Compiler {
    /// The input source files to compile.
    sources: Vec<SourceFile>,

    /// The output file path.
    output: Option<PathBuf>,

    /// The target to compile for.
    target: Target,

    /// Whether to dump the AST to stdout.
    dump_ast: bool,

    /// The reporter for diagnostics.
    reporter: Reporter,
}

impl Compiler {
    /// Creates a new compiler instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a source file to the compiler.
    pub fn with_source(mut self, path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        match fs::read_to_string(path) {
            Ok(contents) => {
                self.sources
                    .push(SourceFile::new(path.to_path_buf(), contents));
            }
            Err(err) => {
                self.reporter.add(Diagnostic::error(
                    format!("failed to read `{}`: {}", path.display(), err),
                    None,
                ));
            }
        }
        self
    }

    /// Sets the output file path.
    pub fn with_output(mut self, path: impl AsRef<Path>) -> Self {
        self.output = Some(path.as_ref().to_path_buf());
        self
    }

    /// Sets the target to compile for.
    pub fn with_target(mut self, target: Target) -> Self {
        self.target = target;
        self
    }

    /// Sets whether to dump the AST to stdout.
    pub fn with_dump_ast(mut self, dump_ast: bool) -> Self {
        self.dump_ast = dump_ast;
        self
    }

    /// Compiles the source files.
    pub fn compile(mut self) {
        if self.reporter.has_errors() {
            self.reporter.emit();
            return;
        }

        let mut asts = Vec::new();
        for source in &self.sources {
            let lexer = Lexer::new(source);
            let mut parser = Parser::new(lexer);
            let ast = parser.parse();
            asts.push(ast);
            self.reporter.add_all(parser.diagnostics());
        }

        if self.reporter.has_errors() {
            self.reporter.emit();
            return;
        }

        if self.dump_ast {
            for ast in &asts {
                println!("{:#?}", ast);
            }
            return;
        }

        let global_scope = GlobalScope::new();
        let mut typed_asts = Vec::new();
        for ast in asts {
            let (typed_ast, diagnostics) = analyze(ast, &global_scope);
            typed_asts.push(typed_ast);
            self.reporter.add_all(diagnostics);
        }

        if self.reporter.has_errors() {
            self.reporter.emit();
            return;
        }

        let output = self.output.unwrap_or_else(|| {
            let stem = self.sources[0].path().file_stem().unwrap();
            PathBuf::from(stem).with_extension("o")
        });

        let mut codegen = CodeGenerator::new(self.target);
        codegen.run(typed_asts, &output);
    }
}
