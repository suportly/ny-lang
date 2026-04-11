//! Ny Lang Compiler
//!
//! This crate is the main compiler for the Ny language. It includes the lexer,
//! parser, semantic analysis, and code generation. It is designed to be used
//! both as a command-line tool and as a library for other tools like the LSP
//! server.

pub mod cdp;
pub mod codegen;
pub mod common;
pub mod diagnostics;
pub mod formatter;
pub mod lexer;
pub mod monomorphize;
pub mod parser;
pub mod pkg;
pub mod semantic;
pub mod ai;
