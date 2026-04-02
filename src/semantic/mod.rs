pub mod resolver;
pub mod typechecker;

use crate::common::CompileError;
use crate::parser::ast::Program;
pub use resolver::ResolvedInfo;
use resolver::Resolver;
use typechecker::TypeChecker;

pub fn analyze(program: &Program) -> Result<ResolvedInfo, Vec<CompileError>> {
    let resolved = Resolver::resolve(program)?;
    TypeChecker::check(program, &resolved)?;
    Ok(resolved)
}
