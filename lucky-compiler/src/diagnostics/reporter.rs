use crate::diagnostics::diagnostic::{Diagnostic, DiagnosticBag, Severity};

/// Prints diagnostics to stderr in a human-readable format.
pub fn print_diagnostics(diagnostics: &[Diagnostic], source: &str, filename: &str) {
    for diag in diagnostics {
        let prefix = match diag.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        };

        let code_str = diag.code.as_ref()
            .map(|c| format!("[{}]", c))
            .unwrap_or_default();

        eprintln!("{}: {} {}", filename, prefix, diag.message);
        eprintln!("  {} {}", prefix, code_str);

        for label in &diag.labels {
            let line = get_line_number(source, label.span.start);
            let col = get_column(source, label.span.start);
            let line_text = get_line_text(source, label.span.start);
            eprintln!("  {}:{}:{}: {}", filename, line, col, label.message);
            if !line_text.is_empty() {
                eprintln!("    {}", line_text);
                let padding = " ".repeat(col.saturating_sub(1).min(80));
                let squiggles = "^".repeat((label.span.end - label.span.start).max(1).min(60));
                eprintln!("    {}{}", padding, squiggles);
            }
        }

        for note in &diag.notes {
            eprintln!("  = note: {}", note);
        }

        if let Some(ref suggestion) = diag.suggestion {
            eprintln!("  = help: {}", suggestion.message);
            eprintln!("    replace with: {}", suggestion.replacement);
        }

        eprintln!();
    }
}

fn get_line_number(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].lines().count()
}

fn get_column(source: &str, offset: usize) -> usize {
    let line_start = source[..offset.min(source.len())].rfind('\n').map(|i| i + 1).unwrap_or(0);
    offset - line_start + 1
}

fn get_line_text(source: &str, offset: usize) -> String {
    let line_start = source[..offset.min(source.len())].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = source[line_start..].find('\n').map(|i| line_start + i).unwrap_or(source.len());
    source[line_start..line_end].to_string()
}
