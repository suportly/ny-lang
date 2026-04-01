pub mod codegen;
pub mod common;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
pub mod semantic;

use std::path::Path;

use common::CompileError;

pub fn compile(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    opt_level: u8,
    emit: &str,
) -> Result<(), Vec<CompileError>> {
    let tokens = lexer::tokenize(source);
    let tokens = tokens?;

    let program = parser::parse(tokens);
    let program = program?;

    semantic::analyze(&program)?;

    codegen::generate(&program, source_path, output_path, opt_level, emit)
}
