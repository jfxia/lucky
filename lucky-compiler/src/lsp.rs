//! LSP-compatible API surface for the Lucky compiler.
//!
//! This module defines the types and entry points that an LSP server
//! (or any IDE integration) would use to interact with the Lucky compiler.

use crate::ast::span::Span;

/// A position in a source file (LSP-compatible: zero-based line and character).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

/// A range in a source file (LSP-compatible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// A diagnostic as exposed to LSP clients.
#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub range: Range,
    pub severity: LspSeverity,
    pub code: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// Result of compiling a single file.
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub diagnostics: Vec<LspDiagnostic>,
    pub ast_json: Option<String>,
    pub ir_json: Option<String>,
}

/// Convert a compiler Span to an LSP Range.
pub fn span_to_range(span: Span, source: &str) -> Range {
    let start = offset_to_position(span.start, source);
    let end = offset_to_position(span.end, source);
    Range { start, end }
}

fn offset_to_position(offset: usize, source: &str) -> Position {
    let offset = offset.min(source.len());
    let line = source[..offset].chars().filter(|c| *c == '\n').count();
    let line_start = source[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = offset - line_start;
    Position { line, character     }
}

/// Compile a Lucky source file and return LSP-compatible diagnostics.
pub fn compile_for_lsp(source: &str, _file_path: &str) -> CompileResult {
    use crate::ast::span::FileId;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::diagnostics::DiagnosticBag;
    use crate::diagnostics::diagnostic::Severity;

    let file_id = FileId(0);
    let mut lexer = Lexer::new(source, file_id);
    let tokens = lexer.tokenize();
    let mut diagnostics = DiagnosticBag::new();

    let mut parser = Parser::new(tokens, file_id);
    let module = parser.parse_module();
    diagnostics.extend(parser.diagnostics);

    let mut lsp_diags = Vec::new();

    for diag in &diagnostics.diagnostics {
        let severity = match diag.severity {
            Severity::Error => LspSeverity::Error,
            Severity::Warning => LspSeverity::Warning,
            Severity::Note => LspSeverity::Information,
        };

        let range = if let Some(label) = diag.labels.first() {
            span_to_range(label.span, source)
        } else {
            Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } }
        };

        lsp_diags.push(LspDiagnostic {
            range,
            severity,
            code: diag.code.clone(),
            message: diag.message.clone(),
        });
    }

    let ast_json = Some(format!("{:#?}", module));

    CompileResult {
        diagnostics: lsp_diags,
        ast_json,
        ir_json: None,
    }
}
