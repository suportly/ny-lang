//! Ny Lang LSP Server
//!
//! Provides: diagnostics, hover, go-to-definition, and completion.
//! Runs as a separate process, communicates via stdio with LSP protocol.
//! Uses the compiler's semantic analysis for accurate type information.

#![allow(clippy::mutable_key_type)]

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{DidChangeTextDocument, DidOpenTextDocument, Notification as _};
use lsp_types::request::{Completion, DocumentSymbolRequest, GotoDefinition, HoverRequest};
use lsp_types::*;
type Url = lsp_types::Uri;
use std::collections::HashMap;
use std::error::Error;

use ny::common::NyType;
use ny::semantic::ResolvedInfo;

/// Cached analysis result for a document
struct DocumentState {
    source: String,
    resolved: Option<ResolvedInfo>,
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("ny-lsp: starting Ny Lang Language Server");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_string()]),
            ..Default::default()
        }),
        document_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })?;

    let init_params = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };

    eprintln!("ny-lsp: initialized with params: {}", init_params);

    main_loop(connection)?;
    io_threads.join()?;

    eprintln!("ny-lsp: shutting down");
    Ok(())
}

fn main_loop(connection: Connection) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut documents: HashMap<Url, DocumentState> = HashMap::new();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                let req_clone = req.clone();
                if let Some((id, params)) = cast_request::<HoverRequest>(req_clone.clone()) {
                    let uri = params.text_document_position_params.text_document.uri;
                    let pos = params.text_document_position_params.position;
                    let hover = handle_hover(&documents, &uri, pos);
                    let result = serde_json::to_value(hover)?;
                    connection.sender.send(Message::Response(Response {
                        id,
                        result: Some(result),
                        error: None,
                    }))?;
                    continue;
                }

                if let Some((id, params)) = cast_request::<GotoDefinition>(req_clone.clone()) {
                    let uri = params.text_document_position_params.text_document.uri;
                    let pos = params.text_document_position_params.position;
                    let def = handle_goto_def(&documents, &uri, pos);
                    let result = serde_json::to_value(def)?;
                    connection.sender.send(Message::Response(Response {
                        id,
                        result: Some(result),
                        error: None,
                    }))?;
                    continue;
                }

                if let Some((id, params)) = cast_request::<Completion>(req_clone.clone()) {
                    let uri = params.text_document_position.text_document.uri;
                    let pos = params.text_document_position.position;
                    let items = handle_completion(&documents, &uri, pos);
                    let result = serde_json::to_value(CompletionResponse::Array(items))?;
                    connection.sender.send(Message::Response(Response {
                        id,
                        result: Some(result),
                        error: None,
                    }))?;
                    continue;
                }

                if let Some((id, params)) = cast_request::<DocumentSymbolRequest>(req_clone) {
                    let uri = params.text_document.uri;
                    let symbols = handle_document_symbols(&documents, &uri);
                    let result = serde_json::to_value(DocumentSymbolResponse::Flat(symbols))?;
                    connection.sender.send(Message::Response(Response {
                        id,
                        result: Some(result),
                        error: None,
                    }))?;
                    continue;
                }
            }
            Message::Notification(notif) => {
                if notif.method == DidOpenTextDocument::METHOD {
                    let params: DidOpenTextDocumentParams = serde_json::from_value(notif.params)?;
                    let uri = params.text_document.uri.clone();
                    let text = params.text_document.text.clone();
                    let resolved = analyze_document(&text);
                    publish_diagnostics(&connection, &uri, &text)?;
                    documents.insert(
                        uri,
                        DocumentState {
                            source: text,
                            resolved,
                        },
                    );
                } else if notif.method == DidChangeTextDocument::METHOD {
                    let params: DidChangeTextDocumentParams = serde_json::from_value(notif.params)?;
                    let uri = params.text_document.uri.clone();
                    if let Some(change) = params.content_changes.into_iter().last() {
                        let text = change.text;
                        let resolved = analyze_document(&text);
                        publish_diagnostics(&connection, &uri, &text)?;
                        documents.insert(
                            uri,
                            DocumentState {
                                source: text,
                                resolved,
                            },
                        );
                    }
                }
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

/// Run lexer + parser + semantic analysis, returning ResolvedInfo if successful.
fn analyze_document(source: &str) -> Option<ResolvedInfo> {
    let tokens = ny::lexer::tokenize(source).ok()?;
    let program = ny::parser::parse(tokens).ok()?;
    ny::semantic::analyze(&program).ok()
}

fn publish_diagnostics(
    connection: &Connection,
    uri: &Url,
    source: &str,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut diagnostics = Vec::new();

    match ny::lexer::tokenize(source) {
        Err(errors) => {
            for err in errors {
                diagnostics.push(error_to_diagnostic(source, &err));
            }
        }
        Ok(tokens) => match ny::parser::parse(tokens) {
            Err(errors) => {
                for err in errors {
                    diagnostics.push(error_to_diagnostic(source, &err));
                }
            }
            Ok(program) => {
                if let Err(errors) = ny::semantic::analyze(&program) {
                    for err in errors {
                        diagnostics.push(error_to_diagnostic(source, &err));
                    }
                }
            }
        },
    }

    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };

    connection.sender.send(Message::Notification(Notification {
        method: "textDocument/publishDiagnostics".to_string(),
        params: serde_json::to_value(params)?,
    }))?;

    Ok(())
}

fn error_to_diagnostic(source: &str, error: &ny::common::CompileError) -> Diagnostic {
    let (line, col) = byte_offset_to_line_col(source, error.span.start);
    let (end_line, end_col) = byte_offset_to_line_col(source, error.span.end);

    Diagnostic {
        range: Range {
            start: Position {
                line: line as u32,
                character: col as u32,
            },
            end: Position {
                line: end_line as u32,
                character: end_col as u32,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        message: error.message.clone(),
        source: Some("ny".to_string()),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Hover: semantic type info from ResolvedInfo
// ---------------------------------------------------------------------------

fn handle_hover(documents: &HashMap<Url, DocumentState>, uri: &Url, pos: Position) -> Option<Hover> {
    let doc = documents.get(uri)?;
    let source = &doc.source;
    let offset = line_col_to_byte_offset(source, pos.line as usize, pos.character as usize)?;

    let tokens = ny::lexer::tokenize(source).ok()?;
    for token in &tokens {
        if token.span.start <= offset && offset <= token.span.end {
            if let ny::lexer::token::TokenKind::Ident(name) = &token.kind {
                let info = if let Some(resolved) = &doc.resolved {
                    semantic_type_info(source, resolved, name)
                } else {
                    format!("{} (analysis unavailable)", name)
                };
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: info,
                    }),
                    range: Some(span_to_range(source, token.span)),
                });
            }
            if let ny::lexer::token::TokenKind::Fn = &token.kind {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: "`fn` — function definition keyword".to_string(),
                    }),
                    range: None,
                });
            }
        }
    }
    None
}

/// Build hover info from semantic analysis results.
fn semantic_type_info(source: &str, resolved: &ResolvedInfo, name: &str) -> String {
    // Check functions
    if let Some((param_types, ret_type, _)) = resolved.functions.get(name) {
        let params: Vec<String> = param_types.iter().map(|t| format!("{}", t)).collect();
        let ret = if *ret_type == NyType::Unit {
            String::new()
        } else {
            format!(" -> {}", ret_type)
        };
        return format!("```ny\nfn {}({}){}\n```", name, params.join(", "), ret);
    }

    // Check structs
    if let Some(fields) = resolved.structs.get(name) {
        let field_lines: Vec<String> = fields
            .iter()
            .map(|(fname, ftype)| format!("    {}: {}", fname, ftype))
            .collect();
        return format!("```ny\nstruct {} {{\n{}\n}}\n```", name, field_lines.join(",\n"));
    }

    // Check enums
    if let Some(variants) = resolved.enums.get(name) {
        let var_lines: Vec<String> = variants
            .iter()
            .map(|(vname, payload)| {
                if payload.is_empty() {
                    format!("    {}", vname)
                } else {
                    let types: Vec<String> = payload.iter().map(|t| format!("{}", t)).collect();
                    format!("    {}({})", vname, types.join(", "))
                }
            })
            .collect();
        return format!("```ny\nenum {} {{\n{}\n}}\n```", name, var_lines.join(",\n"));
    }

    // Fall back to source-level heuristic for local variables
    infer_type_from_source(source, name)
}

/// Fallback: infer variable types from source text patterns.
fn infer_type_from_source(source: &str, name: &str) -> String {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(name) {
            let rest = rest.trim();
            // name :~ Type = ...  (mutable with type)
            if let Some(rest) = rest.strip_prefix(":~") {
                let ty = rest.trim().split('=').next().unwrap_or("").trim();
                if !ty.is_empty() {
                    return format!("`{}: {}` (mutable)", name, ty);
                }
            }
            // name : Type = ...  (immutable with type)
            if let Some(rest) = rest.strip_prefix(':') {
                let rest = rest.trim();
                if !rest.starts_with('=') {
                    let ty = rest.split('=').next().unwrap_or("").trim();
                    if !ty.is_empty() {
                        return format!("`{}: {}`", name, ty);
                    }
                }
            }
            // name := expr  (inferred)
            if rest.starts_with(":=") {
                return format!("`{}` (type inferred)", name);
            }
        }
    }
    format!("`{}`", name)
}

// ---------------------------------------------------------------------------
// Go-to-definition: use ResolvedInfo spans when available
// ---------------------------------------------------------------------------

fn handle_goto_def(
    documents: &HashMap<Url, DocumentState>,
    uri: &Url,
    pos: Position,
) -> Option<GotoDefinitionResponse> {
    let doc = documents.get(uri)?;
    let source = &doc.source;
    let offset = line_col_to_byte_offset(source, pos.line as usize, pos.character as usize)?;

    let tokens = ny::lexer::tokenize(source).ok()?;
    for token in &tokens {
        if token.span.start <= offset && offset <= token.span.end {
            if let ny::lexer::token::TokenKind::Ident(name) = &token.kind {
                // Try semantic resolution first (functions have accurate spans)
                if let Some(resolved) = &doc.resolved {
                    if let Some((_, _, def_span)) = resolved.functions.get(name.as_str()) {
                        return Some(GotoDefinitionResponse::Scalar(Location {
                            uri: uri.clone(),
                            range: span_to_range(source, *def_span),
                        }));
                    }
                }

                // Fallback: text search for struct/enum definitions
                if let Some(def_offset) = find_definition(source, name) {
                    let (line, col) = byte_offset_to_line_col(source, def_offset);
                    return Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: line as u32,
                                character: col as u32,
                            },
                            end: Position {
                                line: line as u32,
                                character: (col + name.len()) as u32,
                            },
                        },
                    }));
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Completion: functions, builtins, structs, enums, keywords, dot-completions
// ---------------------------------------------------------------------------

fn handle_completion(
    documents: &HashMap<Url, DocumentState>,
    uri: &Url,
    pos: Position,
) -> Vec<CompletionItem> {
    let doc = match documents.get(uri) {
        Some(d) => d,
        None => return vec![],
    };
    let source = &doc.source;

    // Check if the character before cursor is '.' (dot completion)
    if let Some(offset) = line_col_to_byte_offset(source, pos.line as usize, pos.character as usize)
    {
        if offset > 0 && source.as_bytes().get(offset - 1) == Some(&b'.') {
            return dot_completions(doc, source, offset);
        }
    }

    // General completions: functions, structs, enums, builtins, keywords
    let mut items = Vec::new();

    // From semantic analysis
    if let Some(resolved) = &doc.resolved {
        for (name, (param_types, ret_type, _)) in &resolved.functions {
            let params: Vec<String> = param_types.iter().map(|t| format!("{}", t)).collect();
            let ret = if *ret_type == NyType::Unit {
                String::new()
            } else {
                format!(" -> {}", ret_type)
            };
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("fn({}){}", params.join(", "), ret)),
                ..Default::default()
            });
        }
        for name in resolved.structs.keys() {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some("struct".to_string()),
                ..Default::default()
            });
        }
        for name in resolved.enums.keys() {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::ENUM),
                detail: Some("enum".to_string()),
                ..Default::default()
            });
        }
    }

    // Builtin functions
    for &name in ny::codegen::builtins::BUILTIN_NAMES {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("builtin".to_string()),
            ..Default::default()
        });
    }

    // Keywords
    for kw in &[
        "fn", "if", "else", "while", "for", "in", "return", "struct", "enum", "match",
        "break", "continue", "defer", "pub", "use", "trait", "impl", "loop", "unsafe",
        "extern", "let", "true", "false", "as",
    ] {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }

    items
}

/// Provide completions after a dot: methods for Vec, str, structs.
fn dot_completions(doc: &DocumentState, source: &str, dot_offset: usize) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Try to figure out the type of the expression before the dot.
    // Simple heuristic: get the identifier before the dot.
    let before = &source[..dot_offset - 1];
    let ident: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if ident.is_empty() {
        return items;
    }

    // Check if this identifier is a known variable by looking at source patterns
    let var_type = infer_variable_type(source, &ident);

    if var_type.contains("Vec") {
        for (name, detail) in &[
            ("push", "push(value) — append element"),
            ("pop", "pop() -> T — remove and return last"),
            ("get", "get(index) -> T — read element"),
            ("set", "set(index, value) — write element"),
            ("len", "len() -> i64 — element count"),
            ("sort", "sort() — ascending in-place sort"),
        ] {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(detail.to_string()),
                ..Default::default()
            });
        }
    } else if var_type == "str" || var_type.contains("str") {
        for (name, detail) in &[
            ("len", "len() -> i64 — byte length"),
            ("substr", "substr(start, end) -> str"),
            ("char_at", "char_at(index) -> i32"),
            ("contains", "contains(needle) -> bool"),
            ("starts_with", "starts_with(prefix) -> bool"),
            ("ends_with", "ends_with(suffix) -> bool"),
            ("index_of", "index_of(needle) -> i32 (-1 if not found)"),
        ] {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(detail.to_string()),
                ..Default::default()
            });
        }
    }

    // If we have resolved info, check struct fields
    if let Some(resolved) = &doc.resolved {
        // Find which struct type this variable is
        for (struct_name, fields) in &resolved.structs {
            if var_type.contains(struct_name) {
                for (fname, ftype) in fields {
                    items.push(CompletionItem {
                        label: fname.clone(),
                        kind: Some(CompletionItemKind::FIELD),
                        detail: Some(format!("{}", ftype)),
                        ..Default::default()
                    });
                }
            }
        }
    }

    items
}

/// Simple heuristic to infer a variable's type string from source patterns.
fn infer_variable_type(source: &str, name: &str) -> String {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(name) {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix(":~") {
                let ty = rest.trim().split('=').next().unwrap_or("").trim();
                if !ty.is_empty() {
                    return ty.to_string();
                }
            }
            if let Some(rest) = rest.strip_prefix(':') {
                let rest = rest.trim();
                if !rest.starts_with('=') {
                    let ty = rest.split('=').next().unwrap_or("").trim();
                    if !ty.is_empty() {
                        return ty.to_string();
                    }
                }
            }
            // := with string literal
            if rest.starts_with(":= \"") {
                return "str".to_string();
            }
        }
    }
    String::new()
}

// ---------------------------------------------------------------------------
// Document Symbols: outline of functions, structs, enums, traits
// ---------------------------------------------------------------------------

fn handle_document_symbols(
    documents: &HashMap<Url, DocumentState>,
    uri: &Url,
) -> Vec<SymbolInformation> {
    let doc = match documents.get(uri) {
        Some(d) => d,
        None => return vec![],
    };
    let source = &doc.source;
    let resolved = match &doc.resolved {
        Some(r) => r,
        None => return vec![],
    };

    let mut symbols = Vec::new();

    for (name, (param_types, ret_type, span)) in &resolved.functions {
        let params: Vec<String> = param_types.iter().map(|t| format!("{}", t)).collect();
        let ret = if *ret_type == NyType::Unit {
            String::new()
        } else {
            format!(" -> {}", ret_type)
        };
        #[allow(deprecated)]
        symbols.push(SymbolInformation {
            name: name.clone(),
            kind: SymbolKind::FUNCTION,
            location: Location {
                uri: uri.clone(),
                range: span_to_range(source, *span),
            },
            tags: None,
            container_name: None,
            deprecated: None,
        });
        let _ = (params, ret); // used for detail if needed
    }

    for name in resolved.structs.keys() {
        if let Some(offset) = source.find(&format!("struct {} ", name))
            .or_else(|| source.find(&format!("struct {}{{", name)))
        {
            let (line, col) = byte_offset_to_line_col(source, offset);
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name: name.clone(),
                kind: SymbolKind::STRUCT,
                location: Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position { line: line as u32, character: col as u32 },
                        end: Position { line: line as u32, character: (col + name.len()) as u32 },
                    },
                },
                tags: None,
                container_name: None,
                deprecated: None,
            });
        }
    }

    for name in resolved.enums.keys() {
        if let Some(offset) = source.find(&format!("enum {} ", name))
            .or_else(|| source.find(&format!("enum {}{{", name)))
        {
            let (line, col) = byte_offset_to_line_col(source, offset);
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name: name.clone(),
                kind: SymbolKind::ENUM,
                location: Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position { line: line as u32, character: col as u32 },
                        end: Position { line: line as u32, character: (col + name.len()) as u32 },
                    },
                },
                tags: None,
                container_name: None,
                deprecated: None,
            });
        }
    }

    symbols
}

fn find_definition(source: &str, name: &str) -> Option<usize> {
    let patterns = [
        format!("struct {} ", name),
        format!("struct {}{{", name.trim()),
        format!("enum {} ", name),
        format!("enum {}{{", name.trim()),
        format!("trait {} ", name),
        format!("trait {}{{", name.trim()),
    ];
    for pattern in &patterns {
        if let Some(pos) = source.find(pattern.as_str()) {
            return Some(pos + pattern.find(name).unwrap_or(0));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn span_to_range(source: &str, span: ny::common::Span) -> Range {
    let (start_line, start_col) = byte_offset_to_line_col(source, span.start);
    let (end_line, end_col) = byte_offset_to_line_col(source, span.end);
    Range {
        start: Position {
            line: start_line as u32,
            character: start_col as u32,
        },
        end: Position {
            line: end_line as u32,
            character: end_col as u32,
        },
    }
}

fn byte_offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn line_col_to_byte_offset(source: &str, target_line: usize, target_col: usize) -> Option<usize> {
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in source.char_indices() {
        if line == target_line && col == target_col {
            return Some(i);
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    if line == target_line && col == target_col {
        return Some(source.len());
    }
    None
}

fn cast_request<R: lsp_types::request::Request>(req: Request) -> Option<(RequestId, R::Params)> {
    if req.method == R::METHOD {
        let params = serde_json::from_value(req.params).ok()?;
        Some((req.id, params))
    } else {
        None
    }
}
