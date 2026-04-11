#![allow(
    clippy::collapsible_match,
    clippy::collapsible_if,
    clippy::type_complexity,
    clippy::manual_filter_map,
    clippy::unnecessary_filter_map
)]

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
    target: &str,
    extra_libs: &[String],
) -> Result<(), Vec<CompileError>> {
    let tokens = lexer::tokenize(source)?;
    let mut program = parser::parse(tokens)?;

    // Resolve `use` declarations: load and merge referenced modules
    let base_dir = source_path.parent().unwrap_or(Path::new("."));
    let extra_paths = collect_deps_search_paths(source_path);
    let mut visited = HashSet::new();
    visited.insert(source_path.to_path_buf());
    resolve_uses(&mut program, base_dir, &mut visited, &extra_paths)?;

    // Monomorphize generic functions before semantic analysis
    monomorphize::monomorphize(&mut program);

    let _resolved = semantic::analyze(&program)?;
    codegen::generate(
        &program,
        source_path,
        output_path,
        opt_level,
        emit,
        target,
        extra_libs,
    )
}

pub fn resolve_uses_pub(
    program: &mut Program,
    base_dir: &Path,
    visited: &mut HashSet<std::path::PathBuf>,
) -> Result<(), Vec<CompileError>> {
    resolve_uses(program, base_dir, visited, &[])
}

/// Collect package dependency directories from .ny_deps/ (if ny.pkg exists).
fn collect_deps_search_paths(source_path: &Path) -> Vec<std::path::PathBuf> {
    let start = source_path.parent().unwrap_or(Path::new("."));
    let mut dir = start.to_path_buf();
    loop {
        if dir.join("ny.pkg").exists() {
            let deps_dir = dir.join(".ny_deps");
            if deps_dir.is_dir() {
                return std::fs::read_dir(&deps_dir)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.path())
                    .collect();
            }
            return vec![];
        }
        if !dir.pop() {
            return vec![];
        }
    }
}

fn resolve_uses(
    program: &mut Program,
    base_dir: &Path,
    visited: &mut HashSet<std::path::PathBuf>,
    extra_search_paths: &[std::path::PathBuf],
) -> Result<(), Vec<CompileError>> {
    let mut new_items: Vec<Item> = Vec::new();
    let mut remaining_items: Vec<Item> = Vec::new();

    for item in program.items.drain(..) {
        if let Item::Use { path, span } = &item {
            // Search order: relative to source file → CWD → stdlib/ → .ny_deps/*/
            let module_path = base_dir.join(path);
            let module_path = if module_path.exists() {
                module_path
            } else {
                let cwd_path = std::path::Path::new(path);
                if cwd_path.exists() {
                    cwd_path.to_path_buf()
                } else {
                    // Try stdlib/ prefix
                    let stdlib_path = std::path::Path::new("stdlib")
                        .join(path.strip_prefix("stdlib/").unwrap_or(path));
                    if stdlib_path.exists() {
                        stdlib_path
                    } else {
                        // Try package dependencies (.ny_deps/*/path)
                        let mut found = None;
                        for search_dir in extra_search_paths {
                            let pkg_path = search_dir.join(path);
                            if pkg_path.exists() {
                                found = Some(pkg_path);
                                break;
                            }
                        }
                        if let Some(p) = found {
                            p
                        } else {
                            return Err(vec![CompileError::syntax(
                                format!("module file not found: '{}'", path),
                                *span,
                            )]);
                        }
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

            // Recursively resolve `use`s in the imported module
            let module_base_dir = module_path.parent().unwrap_or_else(|| Path::new("."));
            resolve_uses(&mut module_program, module_base_dir, visited, extra_search_paths)?;

            new_items.extend(module_program.items);
        } else {
            remaining_items.push(item);
        }
    }

    program.items = remaining_items;
    program.items.extend(new_items);
    Ok(())
}
