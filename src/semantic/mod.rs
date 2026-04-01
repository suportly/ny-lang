pub mod resolver;
pub mod typechecker;

use crate::common::CompileError;
use crate::parser::ast::Program;
use resolver::Resolver;
use typechecker::TypeChecker;

pub fn analyze(program: &Program) -> Result<(), Vec<CompileError>> {
    let resolved = Resolver::resolve(program)?;
    TypeChecker::check(program, &resolved)?;
    Ok(())
}
