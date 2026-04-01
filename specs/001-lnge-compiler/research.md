# Research: LNGE Compiler MVP

**Date**: 2026-04-01
**Feature**: 001-lnge-compiler

## R1: LLVM Bindings — inkwell

**Decision**: Use `inkwell 0.8.0` with feature `llvm18-0`

**Rationale**: inkwell provides safe Rust wrappers over the LLVM C API. It is the most mature LLVM binding crate for Rust, supporting LLVM 11–21. The safe API prevents common LLVM misuse (dangling values, builder positioning errors). Pre-1.0 but stable enough for production use.

**Alternatives considered**:
- `llvm-sys` (raw C FFI): Maximum control but unsafe, verbose, error-prone. Not justified for MVP.
- `cranelift`: Faster compilation but no optimization pipeline, less mature for ahead-of-time compilation. Would require building our own optimization passes.

**Key patterns**:
- `Context` → `Module` → `Builder` lifetime chain; everything borrows from Context
- `TargetMachine::write_to_file(FileType::Object)` emits `.o` files
- Linking requires invoking `cc` externally via `std::process::Command`
- PassManager removed in LLVM 17+; use `module.run_passes("instcombine,mem2reg", ...)` instead
- All `build_*` methods return `Result` — must handle errors

**Gotchas**:
- Lifetime coupling: Builder/Module/values borrow from Context. Must be careful with struct design.
- No linker integration: Must shell out to `cc`/`clang` for linking `.o` → executable
- JIT requires `unsafe`; we won't use JIT in MVP

## R2: Error Diagnostics

**Decision**: Use `codespan-reporting 0.13.1`

**Rationale**: Stable, well-maintained, imperative API that maps directly to compiler span types. `SimpleFiles` + `Diagnostic` + `Label` is minimal and easy to test. Byte-range labels align with our lexer spans. Supports colored terminal output and plain text for CI.

**Alternatives considered**:
- `ariadne`: Visually fancier but GitHub repo archived (March 2026), moved to Codeberg. Maintenance risk.
- `miette`: Derive-macro approach better suited for application errors than compiler diagnostics. More ceremony for our use case.

**Key pattern**:
```rust
let mut files = SimpleFiles::new();
let file_id = files.add("main.lnge", source);
let diag = Diagnostic::error()
    .with_message("type mismatch")
    .with_labels(vec![Label::primary(file_id, span.start..span.end)]);
term::emit(&mut writer, &config, &files, &diag)?;
```

## R3: Parser Strategy

**Decision**: Hand-written recursive descent + Pratt parsing for expressions

**Rationale**: Full control over error messages, source spans, and recovery. Pratt parsing is elegant for expression precedence. Parser combinator crates (chumsky) add complexity and compile time without proportional benefit for a compiler that needs precise diagnostics.

**Alternatives considered**:
- `chumsky`: Good error recovery but fights the abstraction for precise span control. Slower compilation.
- `pratt` crate: Too minimal — adds a dependency without meaningfully reducing code.
- `lalrpop` / `pest`: Grammar-file approaches that reduce control over error messages.

**Binding power table** (left, right for left-associative):
| Operator | Left BP | Right BP |
|----------|---------|----------|
| `\|\|`   | 1       | 2        |
| `&&`     | 3       | 4        |
| `== != < > <= >=` | 5 | 6  |
| `+ -`    | 7       | 8        |
| `* / %`  | 9       | 10       |
| unary `- !` | —    | 11       |

## R4: AST Representation

**Decision**: Enum-based AST nodes with `Box<T>` for recursive types

**Rationale**: Standard Rust pattern for compilers of this scale. Simple to pattern-match, easy to extend. Arena allocation (bumpalo/typed-arena) is premature for MVP — Box is sufficient.

**Pattern**:
```rust
enum Expr {
    Literal(LitValue, Span),
    Ident(String, Span),
    BinOp(BinOp, Box<Expr>, Box<Expr>, Span),
    UnaryOp(UnaryOp, Box<Expr>, Span),
    Call(String, Vec<Expr>, Span),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>, Span),
    Block(Vec<Stmt>, Option<Box<Expr>>, Span),
}
```

## R5: CLI Framework

**Decision**: Use `clap 4.x` with derive macros

**Rationale**: Industry standard for Rust CLIs. Derive API reduces boilerplate. Handles argument parsing, help generation, and error messages.

**Alternative considered**:
- Manual `std::env::args()`: Works but requires re-implementing help text, validation, error formatting.

## R6: Linking Strategy

**Decision**: Shell out to `cc` for linking

**Rationale**: `cc` automatically handles C runtime linkage (`crt1.o`, etc.), platform-specific startup files, and linker flags. Using `ld` directly requires manually specifying all of these. The MVP compiles single files, so the linking step is simply: `cc output.o -o output_binary`.

**Workflow**: Source → Lexer → Parser → Semantic → LLVM IR → `.o` (via inkwell) → executable (via `cc`)
