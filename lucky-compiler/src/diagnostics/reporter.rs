use crate::diagnostics::diagnostic::{Diagnostic, DiagnosticBag, Severity};

const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";

pub fn print_diagnostics(diagnostics: &[Diagnostic], source: &str, filename: &str) {
    for diag in diagnostics {
        let (color, prefix) = match diag.severity {
            Severity::Error => (RED, "error"),
            Severity::Warning => (YELLOW, "warning"),
            Severity::Note => (CYAN, "note"),
        };

        let code_str = diag.code.as_ref()
            .map(|c| format!("[{}]", c))
            .unwrap_or_default();

        eprintln!("{}{}{}{}: {}{}{}{}: {}{}{}",
            BOLD, color, filename, RESET,
            BOLD, prefix, code_str, RESET,
            BOLD, diag.message, RESET);

        for label in &diag.labels {
            let line_num = get_line_number(source, label.span.start);
            let col = get_column(source, label.span.start);
            let span_len = label.span.end.saturating_sub(label.span.start).max(1);

            eprintln!("{}  --> {}:{}:{}", DIM, filename, line_num, col);
            eprintln!("{}   |{}", DIM, RESET);

            let context_start = line_num.saturating_sub(3);
            let context_end = line_num + 1;
            for ctx_line in context_start..=context_end {
                let line_text = get_line_by_number(source, ctx_line);
                if line_text.is_empty() && ctx_line != line_num { continue; }
                let marker = if ctx_line == line_num { ">" } else { " " };
                eprintln!("{}{:>3} {}{} {}", DIM, ctx_line, marker, RESET, line_text);
                if ctx_line == line_num {
                    let padding = " ".repeat(col.saturating_sub(1) + 2);
                    let squiggles = "^".repeat(span_len.min(80));
                    eprintln!("{}     |{}{}{}{}{}", DIM, RESET, padding, color, squiggles, RESET);
                    if !label.message.is_empty() {
                        eprintln!("{}     |{}{}{}{}{} {}{}{}",
                            DIM, RESET, padding, color, squiggles, RESET, DIM, label.message, RESET);
                    }
                }
            }
            eprintln!("{}   |{}", DIM, RESET);
        }

        for note in &diag.notes {
            eprintln!("{}  = note:{} {}", CYAN, RESET, note);
        }

        if let Some(ref suggestion) = diag.suggestion {
            eprintln!("{}{}  = help:{} {}", BOLD, YELLOW, RESET, suggestion.message);
            if !suggestion.replacement.is_empty() {
                eprintln!("{}    replace with:{} {}{}{}", DIM, RESET, BOLD, suggestion.replacement, RESET);
            }
            let sug_line = get_line_number(source, suggestion.span.start);
            let sug_text = get_line_by_number(source, sug_line);
            if !sug_text.is_empty() {
                eprintln!("{}{:>3} {}{}  {}", DIM, sug_line, DIM, RESET, sug_text);
            }
        }

        let msg_lower = diag.message.to_lowercase();
        if msg_lower.contains("missing") && msg_lower.contains("indent") {
            eprintln!("{}{}  = fix:{} Expected an indented block here. Lucky uses indentation for blocks.", BOLD, YELLOW, RESET);
        } else if msg_lower.contains("expected") && msg_lower.contains("colon") {
            eprintln!("{}{}  = fix:{} Did you forget a colon? Try `name: Type`", BOLD, YELLOW, RESET);
        } else if msg_lower.contains("unexpected keyword") {
            let word = extract_unexpected_word(&diag.message);
            if !word.is_empty() {
                if let Some(suggestion) = find_closest_match(&word) {
                    eprintln!("{}{}  = fix:{} Did you mean `{}`?", BOLD, YELLOW, RESET, suggestion);
                }
            }
        } else if msg_lower.contains("expected") && msg_lower.contains("arrow") || msg_lower.contains("->") {
            eprintln!("{}{}  = fix:{} Workflow steps need `->` between sequential nodes", BOLD, YELLOW, RESET);
        }

        eprintln!();
    }
}

fn get_line_number(source: &str, offset: usize) -> usize {
    let offset = offset.min(source.len());
    let safe_offset = find_char_boundary(source, offset);
    source[..safe_offset].chars().filter(|c| *c == '\n').count() + 1
}

fn get_column(source: &str, offset: usize) -> usize {
    let offset = offset.min(source.len());
    let safe_offset = find_char_boundary(source, offset);
    let line_start = source[..safe_offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    offset.saturating_sub(line_start) + 1
}

fn find_char_boundary(source: &str, offset: usize) -> usize {
    let mut o = offset.min(source.len());
    while o > 0 && !source.is_char_boundary(o) {
        o -= 1;
    }
    o
}

fn get_line_by_number(source: &str, line: usize) -> String {
    if line == 0 { return String::new(); }
    source.lines().nth(line.saturating_sub(1)).unwrap_or("").to_string()
}

fn extract_unexpected_word(msg: &str) -> String {
    let prefix = "Unexpected keyword '";
    if let Some(pos) = msg.find(prefix) {
        let after = &msg[pos + prefix.len()..];
        if let Some(end) = after.find('\'') {
            return after[..end].to_string();
        }
    }
    String::new()
}

fn find_closest_match(word: &str) -> Option<&'static str> {
    let candidates = &[
        "agent", "task", "workflow", "goal", "memory", "tool", "model",
        "prompt", "policy", "type", "context", "permissions", "approval",
        "project", "import", "pub", "use", "let", "const", "if", "else",
        "match", "loop", "for", "parallel", "await", "when", "return",
        "break", "continue", "attempt", "swarm", "retry", "input",
        "output", "steps", "rollback", "success", "tools", "deny", "allow",
        "deep", "fast", "ask", "reason", "fn", "true", "false", "null",
    ];
    let word_lower = word.to_lowercase();
    let mut best: Option<(&str, usize)> = None;
    for &cand in candidates {
        let dist = levenshtein(&word_lower, cand);
        let threshold = (word.len().max(cand.len()) / 3).max(1);
        if dist <= threshold {
            if best.map_or(true, |(_, d)| dist < d) {
                best = Some((cand, dist));
            }
        }
    }
    best.map(|(c, _)| c)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let n = a_chars.len();
    let m = b_chars.len();
    let mut prev = (0..=m).collect::<Vec<_>>();
    let mut curr = vec![0usize; m + 1];
    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}
