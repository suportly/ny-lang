pub mod codegen;
pub mod common;
pub mod diagnostics;
pub mod lexer;
pub mod monomorphize;
pub mod parser;
pub mod semantic;

use std::collections::HashSet;
use std::path::Path;

use common::CompileError;
use parser::ast::{Item, Program};

pub fn compile(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    opt_level: u8,
    emit: &str,
) -> Result<(), Vec<CompileError>> {
    let tokens = lexer::tokenize(source)?;
    let mut program = parser::parse(tokens)?;

    // Resolve `use` declarations: load and merge referenced modules
    let base_dir = source_path.parent().unwrap_or(Path::new("."));
    let mut visited = HashSet::new();
    visited.insert(source_path.to_path_buf());
    resolve_uses(&mut program, base_dir, &mut visited)?;

    // Monomorphize generic functions before semantic analysis
    monomorphize::monomorphize(&mut program);

    semantic::analyze(&program)?;
    codegen::generate(&program, source_path, output_path, opt_level, emit)
}

fn resolve_uses(
    program: &mut Program,
    base_dir: &Path,
    visited: &mut HashSet<std::path::PathBuf>,
) -> Result<(), Vec<CompileError>> {
    let mut new_items: Vec<Item> = Vec::new();
    let mut remaining_items: Vec<Item> = Vec::new();

    for item in program.items.drain(..) {
        if let Item::Use { path, span } = &item {
            // Search order: relative to source file → CWD → stdlib/
            let module_path = base_dir.join(path);
            let module_path = if module_path.exists() {
                module_path
            } else {
                let cwd_path = std::path::Path::new(path);
                if cwd_path.exists() {
                    cwd_path.to_path_buf()
                } else {
                    // Try stdlib/ prefix
                    let stdlib_path = std::path::Path::new("stdlib").join(
                        path.strip_prefix("stdlib/").unwrap_or(path),
                    );
                    if stdlib_path.exists() {
                        stdlib_path
                    } else {
                        return Err(vec![CompileError::syntax(
                            format!("module file not found: '{}'", path),
                            *span,
                        )]);
                    }
                }
            };
            if visited.contains(&module_path) {
                // Already included, skip (prevent circular imports)
                continue;
            }
            visited.insert(module_path.clone());

            let module_source = std::fs::read_to_string(&module_path).map_err(|e| {
                vec![CompileError::syntax(
                    format!("failed to read module '{}': {}", module_path.display(), e),
                    *span,
                )]
            })?;

            let module_tokens = lexer::tokenize(&module_source)?;
            let mut module_program = parser::parse(module_tokens)?;

            // Recursively resolve uses in the imported module
            let module_dir = module_path.parent().unwrap_or(base_dir);
            resolve_uses(&mut module_program, module_dir, visited)?;

            // Merge all items from the module (later: filter by pub)
            new_items.extend(module_program.items);
        } else {
            remaining_items.push(item);
        }
    }

    // Put imported items first, then the original items
    new_items.extend(remaining_items);
    program.items = new_items;

    Ok(())
}
