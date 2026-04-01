use std::path::Path;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::{
    self,
    termcolor::{ColorChoice, StandardStream},
};

use crate::common::{CompileError, ErrorKind};

pub fn print_errors(file_path: &Path, source: &str, errors: &[CompileError]) {
    let mut files = SimpleFiles::new();
    let file_id = files.add(file_path.to_string_lossy().to_string(), source.to_string());

    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = term::Config::default();

    for error in errors {
        let diag = build_diagnostic(file_id, error);
        #[allow(deprecated)]
        let _ = term::emit(&mut writer.lock(), &config, &files, &diag);
    }
}

fn error_code(kind: &ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Syntax => "E001",
        ErrorKind::Type => "E002",
        ErrorKind::Name => "E003",
        ErrorKind::Immutability => "E004",
        ErrorKind::IO => "E005",
    }
}

fn build_diagnostic(file_id: usize, error: &CompileError) -> Diagnostic<usize> {
    let mut labels = vec![
        Label::primary(file_id, error.span.start..error.span.end).with_message(&error.message)
    ];

    for (span, msg) in &error.secondary {
        labels.push(Label::secondary(file_id, span.start..span.end).with_message(msg));
    }

    let mut diag = Diagnostic::error()
        .with_code(error_code(&error.kind))
        .with_message(&error.message)
        .with_labels(labels);

    if !error.notes.is_empty() {
        diag = diag.with_notes(error.notes.clone());
    }

    diag
}
