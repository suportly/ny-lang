//! Ny Lang LSP Server
//!
//! Provides: diagnostics (inline errors), hover (type info), go-to-definition.
//! Runs as a separate process, communicates via stdio with LSP protocol.

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{DidChangeTextDocument, DidOpenTextDocument, Notification as _};
use lsp_types::request::{GotoDefinition, HoverRequest, Request as _};
use lsp_types::*;
type Url = lsp_types::Uri;
use std::collections::HashMap;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("ny-lsp: starting Ny Lang Language Server");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
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
    let mut documents: HashMap<Url, String> = HashMap::new();

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

                if let Some((id, params)) = cast_request::<GotoDefinition>(req_clone) {
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
            }
            Message::Notification(notif) => {
                if notif.method == DidOpenTextDocument::METHOD {
                    let params: DidOpenTextDocumentParams = serde_json::from_value(notif.params)?;
                    let uri = params.text_document.uri.clone();
                    let text = params.text_document.text.clone();
                    documents.insert(uri.clone(), text.clone());
                    publish_diagnostics(&connection, &uri, &text)?;
                } else if notif.method == DidChangeTextDocument::METHOD {
                    let params: DidChangeTextDocumentParams = serde_json::from_value(notif.params)?;
                    let uri = params.text_document.uri.clone();
                    if let Some(change) = params.content_changes.into_iter().last() {
                        let text = change.text;
                        documents.insert(uri.clone(), text.clone());
                        publish_diagnostics(&connection, &uri, &text)?;
                    }
                }
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn publish_diagnostics(
    connection: &Connection,
    uri: &Url,
    source: &str,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut diagnostics = Vec::new();

    // Run lexer + parser + semantic analysis to find errors
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

fn handle_hover(documents: &HashMap<Url, String>, uri: &Url, pos: Position) -> Option<Hover> {
    let source = documents.get(uri)?;
    let offset = line_col_to_byte_offset(source, pos.line as usize, pos.character as usize)?;

    // Find the token at the cursor position
    let tokens = ny::lexer::tokenize(source).ok()?;
    for token in &tokens {
        if token.span.start <= offset && offset <= token.span.end {
            if let ny::lexer::token::TokenKind::Ident(name) = &token.kind {
                // Try to find type info
                let type_info = infer_type_at_position(source, name);
                return Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(type_info)),
                    range: None,
                });
            }
            if let ny::lexer::token::TokenKind::Fn = &token.kind {
                return Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(
                        "keyword: fn — function definition".to_string(),
                    )),
                    range: None,
                });
            }
        }
    }
    None
}

fn handle_goto_def(
    documents: &HashMap<Url, String>,
    uri: &Url,
    pos: Position,
) -> Option<GotoDefinitionResponse> {
    let source = documents.get(uri)?;
    let offset = line_col_to_byte_offset(source, pos.line as usize, pos.character as usize)?;

    let tokens = ny::lexer::tokenize(source).ok()?;
    for token in &tokens {
        if token.span.start <= offset && offset <= token.span.end {
            if let ny::lexer::token::TokenKind::Ident(name) = &token.kind {
                // Search for function or struct definition with this name
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

fn infer_type_at_position(source: &str, name: &str) -> String {
    // Simple heuristic: find "name : Type" or "name := expr" patterns
    for line in source.lines() {
        let trimmed = line.trim();
        // Check "name : Type = ..."
        if let Some(rest) = trimmed.strip_prefix(name) {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix(':') {
                let rest = rest.trim();
                if let Some(rest) = rest.strip_prefix('~') {
                    // mutable
                    let ty = rest.trim().split('=').next().unwrap_or("").trim();
                    if !ty.is_empty() {
                        return format!("{}: {} (mutable)", name, ty);
                    }
                } else if let Some(rest) = rest.strip_prefix('=') {
                    // type inference
                    return format!("{} (type inferred)", name);
                } else {
                    let ty = rest.split('=').next().unwrap_or("").trim();
                    if !ty.is_empty() {
                        return format!("{}: {}", name, ty);
                    }
                }
            }
        }
        // Check "fn name("
        if trimmed.starts_with("fn ") && trimmed.contains(name) {
            return format!("function {}", name);
        }
        // Check "struct name"
        if trimmed.starts_with("struct ") && trimmed.contains(name) {
            return format!("struct {}", name);
        }
    }
    format!("{} (unknown type)", name)
}

fn find_definition(source: &str, name: &str) -> Option<usize> {
    // Search for "fn name(" or "struct name" or "enum name"
    let patterns = [
        format!("fn {}(", name),
        format!("fn {}<", name),
        format!("struct {} ", name),
        format!("struct {}{{", name.trim()),
        format!("enum {} ", name),
        format!("enum {}{{", name.trim()),
    ];
    for pattern in &patterns {
        if let Some(pos) = source.find(pattern.as_str()) {
            // Return position of the name within the match
            return Some(pos + pattern.find(name).unwrap_or(0));
        }
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
