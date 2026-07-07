// Lucky source code formatter.
//
// Implements `lucky fmt file.lk` with the following rules:
// 1. Tab indentation (no spaces)
// 2. Max 1 consecutive blank line between declarations
// 3. Agent body sections ordered: model → memory → tools → permissions → policy → prompt
// 4. Task body sections ordered: input → output → context → policy → steps → rollback
// 5. Line wrap at 100 characters
// 6. Trailing commas in multi-line lists/maps
// 7. Consistent spacing around operators (=, ->, |>, etc.)
// 8. No trailing whitespace
// 9. File ends with exactly one newline
// 10. Comments preserved at original positions

use crate::ast::span::FileId;
use crate::lexer::{Lexer, Token, TokenKind};
use std::fs;

pub fn format_source(source: &str) -> Result<String, Vec<String>> {
    let mut f = Formatter::new(source);
    f.format()
}

pub fn format_file(path: &str) -> Result<(), Vec<String>> {
    let source = fs::read_to_string(path)
        .map_err(|e| vec![format!("Failed to read '{}': {}", path, e)])?;
    let formatted = format_source(&source)?;
    fs::write(path, formatted)
        .map_err(|e| vec![format!("Failed to write '{}': {}", path, e)])?;
    Ok(())
}

pub fn check_format(source: &str) -> bool {
    format_source(source).map_or(false, |formatted| source == formatted)
}

const AGENT_SECTIONS: &[&str] = &["model", "memory", "tools", "permissions", "policy", "prompt"];
const TASK_SECTIONS: &[&str] = &["input", "output", "context", "policy", "steps", "rollback"];

struct Formatter {
    source: String,
    tokens: Vec<Token>,
    pos: usize,
    indent_level: usize,
    out: String,
    col: usize,
    errors: Vec<String>,
    blank_lines: usize,
    tab_width: usize,
}

impl Formatter {
    fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source, FileId(0));
        let tokens = lexer.tokenize();
        let errors: Vec<String> = lexer.errors().iter().cloned().collect();
        Self {
            source: source.to_string(),
            tokens,
            pos: 0,
            indent_level: 0,
            out: String::new(),
            col: 0,
            errors,
            blank_lines: 0,
            tab_width: 4,
        }
    }

    fn format(&mut self) -> Result<String, Vec<String>> {
        if !self.errors.is_empty() {
            return Err(self.errors.clone());
        }
        // Skip leading blank lines
        while self.peek_kind() == TokenKind::Newline {
            self.pos += 1;
        }

        while !self.at_eof() {
            self.process_top_level_item();
        }

        let trimmed = self.out.trim_end().to_string();
        Ok(if trimmed.is_empty() {
            "\n".to_string()
        } else {
            trimmed + "\n"
        })
    }

    // -----------------------------------------------------------------------
    // Token access
    // -----------------------------------------------------------------------

    fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].kind == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> TokenKind {
        if self.at_eof() {
            return TokenKind::Eof;
        }
        self.tokens[self.pos].kind
    }

    fn peek_text(&self) -> &str {
        if self.at_eof() {
            return "";
        }
        &self.tokens[self.pos].text
    }

    fn bump(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

    // -----------------------------------------------------------------------
    // Output helpers
    // -----------------------------------------------------------------------

    fn emit(&mut self, text: &str) {
        self.out.push_str(text);
        self.col += text.chars().count();
        self.blank_lines = 0;
    }

    fn emit_indent(&mut self) {
        if self.indent_level > 0 {
            let tabs = "\t".repeat(self.indent_level);
            self.out.push_str(&tabs);
            self.col += self.indent_level * self.tab_width;
        }
    }

    fn emit_newline(&mut self) {
        self.blank_lines += 1;
        if self.blank_lines <= 2 {
            self.out.push('\n');
        }
        self.col = 0;
    }

    fn ensure_blank_before(&mut self) {
        if self.out.is_empty() {
            return;
        }
        if self.blank_lines == 0 {
            self.emit_newline();
        }
        if self.blank_lines < 2 {
            self.out.push('\n');
            self.col = 0;
            self.blank_lines = 2;
        }
    }

    // -----------------------------------------------------------------------
    // Top-level processing
    // -----------------------------------------------------------------------

    fn process_top_level_item(&mut self) {
        // Skip leading newlines (but track them for blank line limiting)
        while self.peek_kind() == TokenKind::Newline {
            self.pos += 1;
            self.emit_newline();
        }

        if self.at_eof() {
            return;
        }

        // Skip spurious INDENT tokens produced by the lexer when blank lines
        // appear between same-indentation-level declarations
        while self.peek_kind() == TokenKind::Indent {
            self.pos += 1;
        }
        while self.peek_kind() == TokenKind::Dedent {
            self.pos += 1;
        }

        if self.at_eof() {
            return;
        }

        // Handle comments at top level
        if self.peek_kind() == TokenKind::Comment || self.peek_kind() == TokenKind::DocComment {
            let token = self.bump();
            self.out.push_str(&self.source_text(&token));
            self.blank_lines = 0;
            self.emit_newline();
            return;
        }

        // Detect agent/task declarations for special handling
        if self.peek_kind() == TokenKind::Keyword {
            match self.peek_text() {
                "agent" => {
                    self.process_agent_decl();
                    return;
                }
                "task" => {
                    self.process_task_decl();
                    return;
                }
                _ => {}
            }
        }

        // Generic declaration: consume header line, then optional INDENT body
        self.process_generic_decl();
    }

    fn process_generic_decl(&mut self) {
        self.ensure_blank_before();

        // Emit header line (everything up to Newline or Indent)
        self.emit_indent();
        let mut header_newlines: usize = 0;
        while self.peek_kind() != TokenKind::Newline
            && self.peek_kind() != TokenKind::Indent
            && self.peek_kind() != TokenKind::Eof
            && self.peek_kind() != TokenKind::Dedent
        {
            self.emit_token_with_spacing();
        }

        // Count how many direct newlines follow (no intervening content)
        let mut has_body = false;
        while self.peek_kind() == TokenKind::Newline {
            self.pos += 1;
            header_newlines += 1;
        }
        // Only treat INDENT as body if exactly one newline before it
        // (lexer inserts spurious INDENT after multiple blank lines)
        if header_newlines == 1 && self.peek_kind() == TokenKind::Indent {
            has_body = true;
        }
        let _ = header_newlines;

        if has_body {
            self.emit_newline();
            self.indent_level += 1;
            self.bump(); // consume INDENT

            // Process body until DEDENT
            let mut body_depth: i32 = 0;
            loop {
                if self.at_eof() {
                    break;
                }
                match self.peek_kind() {
                    TokenKind::Dedent => {
                        if body_depth > 0 {
                            self.bump();
                            body_depth -= 1;
                            self.indent_level -= 1;
                        } else {
                            self.bump();
                            if self.indent_level > 0 {
                                self.indent_level -= 1;
                            }
                            break;
                        }
                    }
                    TokenKind::Indent => {
                        self.bump();
                        body_depth += 1;
                        self.indent_level += 1;
                        self.emit_newline();
                    }
                    TokenKind::Newline => {
                        self.bump();
                        self.emit_newline();
                    }
                    TokenKind::Comment | TokenKind::DocComment => {
                        self.emit_indent();
                        let token = self.bump();
                        self.out.push_str(&self.source_text(&token));
                        self.emit_newline();
                    }
                    _ => {
                        self.emit_indent();
                        self.process_body_line();
                    }
                }
            }
            self.emit_newline();
        } else {
            self.emit_newline();
        }
    }

    /// Process one line within a generic body (INDENT zone).
    fn process_body_line(&mut self) {
        loop {
            match self.peek_kind() {
                TokenKind::Newline => {
                    self.bump();
                    self.emit_newline();
                    return;
                }
                TokenKind::Dedent | TokenKind::Indent | TokenKind::Eof => {
                    return;
                }
                TokenKind::Comment | TokenKind::DocComment => {
                    let token = self.bump();
                    self.out.push_str(&self.source_text(&token));
                    continue;
                }
                _ => {
                    self.emit_token_with_spacing();
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Agent declarations
    // -----------------------------------------------------------------------

    fn process_agent_decl(&mut self) {
        self.ensure_blank_before();

        // Emit "agent Name"
        self.emit_indent();
        let kw = self.bump(); // 'agent'
        self.emit(&kw.text);
        self.emit(" ");
        let name = self.bump().text; // agent name
        self.emit(&name);

        self.emit_newline();

        // Skip the NEWLINE between header and body
        if self.peek_kind() == TokenKind::Newline {
            self.pos += 1;
        }

        // Expect INDENT for body
        if self.peek_kind() != TokenKind::Indent {
            return;
        }
        self.bump(); // INDENT

        let base_indent = self.indent_level;
        self.indent_level += 1;

        // Collect agent body sections
        let (config_sections, task_tokens_list) = self.collect_agent_body(base_indent);

        // Reorder config sections
        let mut ordered: Vec<RawSection> = Vec::new();
        for section_name in AGENT_SECTIONS {
            let mut found = Vec::new();
            for (i, sec) in config_sections.iter().enumerate() {
                if sec.name == *section_name {
                    found.push(i);
                }
            }
            for idx in found {
                ordered.push(config_sections[idx].clone());
            }
        }

        // Emit reordered config sections
        for (_i, sec) in ordered.iter().enumerate() {
            self.indent_level = base_indent + 1;
            self.emit_indent();
            self.emit(&sec.name);
            self.emit(&sec.value);
            self.emit_newline();
        }

        // Emit task declarations within agent body
        for (i, task_tokens) in task_tokens_list.iter().enumerate() {
            if i > 0 || !ordered.is_empty() {
                self.emit_newline();
            }
            self.indent_level = base_indent + 1;
            self.format_nested_tokens(task_tokens, base_indent + 1);
        }

        self.indent_level = base_indent;

        // Consume DEDENT
        if self.peek_kind() == TokenKind::Dedent {
            self.bump();
        }
        self.emit_newline();
        self.emit_newline();
    }

    fn collect_agent_body(
        &mut self,
        base_indent: usize,
    ) -> (Vec<RawSection>, Vec<Vec<Token>>) {
        let mut config_sections: Vec<RawSection> = Vec::new();
        let mut task_tokens_list: Vec<Vec<Token>> = Vec::new();
        let mut current_name = String::new();
        let mut current_tokens: Vec<Token> = Vec::new();
        let mut depth: isize = 0;

        loop {
            if self.at_eof() {
                break;
            }
            match self.peek_kind() {
                TokenKind::Dedent => {
                    self.bump();
                    if depth > 0 {
                        depth -= 1;
                        current_tokens.push(synth(TokenKind::Dedent));
                    } else {
                        // End of agent body
                        if !current_name.is_empty() {
                            let value = reconstruct_tokens(&current_tokens);
                            config_sections.push(RawSection {
                                name: current_name.clone(),
                                value: format_section_value(&value),
                            });
                            current_name.clear();
                            current_tokens.clear();
                        }
                        break;
                    }
                }
                TokenKind::Indent => {
                    self.bump();
                    depth += 1;
                    current_tokens.push(synth(TokenKind::Indent));
                }
                TokenKind::Newline => {
                    self.bump();
                    current_tokens.push(synth(TokenKind::Newline));
                }
                TokenKind::Keyword => {
                    let kw = self.peek_text().to_string();
                    if depth == 0 && kw == "task" {
                        // Flush current config section
                        if !current_name.is_empty() {
                            let value = reconstruct_tokens(&current_tokens);
                            config_sections.push(RawSection {
                                name: current_name.clone(),
                                value: format_section_value(&value),
                            });
                            current_name.clear();
                            current_tokens.clear();
                        }
                        // Collect entire task declaration including body
                        let task_toks = self.collect_nested_decl_body();
                        task_tokens_list.push(task_toks);
                    } else if depth == 0
                        && AGENT_SECTIONS.contains(&kw.as_str())
                    {
                        // Start of a new config section
                        if !current_name.is_empty() {
                            let value = reconstruct_tokens(&current_tokens);
                            config_sections.push(RawSection {
                                name: current_name.clone(),
                                value: format_section_value(&value),
                            });
                            current_tokens.clear();
                        }
                        current_name = self.bump().text;
                        // Skip colon if present (value starts after colon)
                        if self.peek_kind() == TokenKind::Colon {
                            self.bump(); // consume colon
                        }
                    } else {
                        current_tokens.push(self.bump());
                    }
                }
                _ => {
                    current_tokens.push(self.bump());
                }
            }
        }

        (config_sections, task_tokens_list)
    }

    fn collect_nested_decl_body(&mut self) -> Vec<Token> {
        let mut toks: Vec<Token> = Vec::new();

        toks.push(Token {
            kind: TokenKind::Keyword,
            text: "task".to_string(),
            span: crate::ast::span::Span::DUMMY,
        });

        // Collect identifier tokens until newline or indent
        while self.peek_kind() != TokenKind::Newline
            && self.peek_kind() != TokenKind::Indent
            && self.peek_kind() != TokenKind::Dedent
            && self.peek_kind() != TokenKind::Eof
        {
            toks.push(self.bump());
        }
        // Consume newlines before body
        while self.peek_kind() == TokenKind::Newline {
            self.bump();
        }

        // Body
        if self.peek_kind() == TokenKind::Indent {
            toks.push(self.bump());
            let mut depth: i32 = 1;
            loop {
                if self.at_eof() {
                    break;
                }
                match self.peek_kind() {
                    TokenKind::Indent => {
                        toks.push(self.bump());
                        depth += 1;
                    }
                    TokenKind::Dedent => {
                        toks.push(self.bump());
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    TokenKind::Newline => {
                        toks.push(self.bump());
                    }
                    _ => {
                        toks.push(self.bump());
                    }
                }
            }
        }

        toks
    }

    fn format_nested_tokens(&mut self, tokens: &[Token], base_indent: usize) {
        self.indent_level = base_indent;
        let mut i = 0;
        while i < tokens.len() {
            let t = &tokens[i];
            match t.kind {
                TokenKind::Indent => {
                    self.indent_level += 1;
                }
                TokenKind::Dedent => {
                    if self.indent_level > 0 {
                        self.indent_level -= 1;
                    }
                }
                TokenKind::Newline => {
                    self.emit_newline();
                }
                TokenKind::Keyword => {
                    if self.col == 0 {
                        self.emit_indent();
                    }
                    self.emit(&t.text);
                    // Space after keyword if followed by ident
                    if i + 1 < tokens.len()
                        && (tokens[i + 1].kind == TokenKind::Ident
                            || tokens[i + 1].kind == TokenKind::Keyword)
                    {
                        self.emit(" ");
                    }
                }
                TokenKind::Ident => {
                    if self.col == 0 {
                        self.emit_indent();
                    } else {
                        let need_space = !self.out.ends_with(' ')
                            && !self.out.is_empty()
                            && !self.out.ends_with('\n')
                            && !self.out.ends_with('(')
                            && !self.out.ends_with('[')
                            && !self.out.ends_with('{')
                            && !self.out.ends_with('.');
                        if need_space {
                            self.emit(" ");
                        }
                    }
                    self.emit(&t.text);
                }
                TokenKind::Colon => {
                    while self.out.ends_with(' ') {
                        self.out.pop();
                    }
                    self.emit(": ");
                }
                TokenKind::Comma => {
                    while self.out.ends_with(' ') {
                        self.out.pop();
                    }
                    self.emit(", ");
                }
                TokenKind::Comment | TokenKind::DocComment => {
                    self.emit_indent();
                    self.out.push_str(&self.source_text(t));
                }
                TokenKind::StringLit => {
                    if self.col == 0 {
                        self.emit_indent();
                    }
                    if t.text.contains('\n') {
                        let s = format!("\"\"\"{}\"\"\"", t.text);
                        self.emit(&s);
                    } else {
                        let s = format!("\"{}\"", t.text);
                        self.emit(&s);
                    }
                }
                _ => {
                    if self.col == 0 {
                        self.emit_indent();
                    }
                    self.emit(&t.text);
                }
            }
            i += 1;
        }
    }

    // -----------------------------------------------------------------------
    // Task declarations
    // -----------------------------------------------------------------------

    fn process_task_decl(&mut self) {
        self.ensure_blank_before();

        // Emit "task Name"
        self.emit_indent();
        let kw = self.bump(); // 'task'
        self.emit(&kw.text);
        self.emit(" ");
        let name = self.bump().text;
        self.emit(&name);

        // (stateful) modifier
        if self.peek_kind() == TokenKind::LParen {
            self.emit(" (");
            self.bump(); // (
            while self.peek_kind() != TokenKind::RParen && !self.at_eof() {
                let t = self.bump();
                self.emit(&t.text);
                if self.peek_kind() != TokenKind::RParen {
                    self.emit(" ");
                }
            }
            if self.peek_kind() == TokenKind::RParen {
                self.bump();
                self.emit(")");
            }
        }

        self.emit_newline();

        // Skip NEWLINE between header and body
        if self.peek_kind() == TokenKind::Newline {
            self.pos += 1;
        }

        if self.peek_kind() != TokenKind::Indent {
            return;
        }
        self.bump(); // INDENT

        let base_indent = self.indent_level;
        self.indent_level += 1;

        let sections = self.collect_task_body(base_indent);

        // Reorder task sections
        for section_name in TASK_SECTIONS.iter() {
            if let Some(sec) = sections.iter().find(|s| s.name == *section_name) {
                self.indent_level = base_indent + 1;
                self.emit_indent();
                self.emit(&sec.name);
                self.emit(&sec.value);
                self.emit_newline();
            }
        }
        // Sections not in standard order (keep them)
        for sec in &sections {
            if !TASK_SECTIONS.contains(&sec.name.as_str()) {
                self.indent_level = base_indent + 1;
                self.emit_indent();
                self.emit(&sec.name);
                self.emit(&sec.value);
                self.emit_newline();
            }
        }

        self.indent_level = base_indent;
        if self.peek_kind() == TokenKind::Dedent {
            self.bump();
        }
        self.emit_newline();
        self.emit_newline();
    }

    fn collect_task_body(&mut self, base_indent: usize) -> Vec<RawSection> {
        let mut sections: Vec<RawSection> = Vec::new();
        let mut current_name = String::new();
        let mut current_tokens: Vec<Token> = Vec::new();
        let mut depth: isize = 0;

        loop {
            if self.at_eof() {
                break;
            }
            match self.peek_kind() {
                TokenKind::Dedent => {
                    self.bump();
                    if depth > 0 {
                        depth -= 1;
                        current_tokens.push(synth(TokenKind::Dedent));
                    } else {
                        if !current_name.is_empty() {
                            let value = reconstruct_tokens(&current_tokens);
                            sections.push(RawSection {
                                name: current_name.clone(),
                                value: format_section_value(&value),
                            });
                        }
                        break;
                    }
                }
                TokenKind::Indent => {
                    self.bump();
                    depth += 1;
                    current_tokens.push(synth(TokenKind::Indent));
                }
                TokenKind::Keyword => {
                    let kw = self.peek_text();
                    if depth == 0 && TASK_SECTIONS.contains(&kw) {
                        if !current_name.is_empty() {
                            let value = reconstruct_tokens(&current_tokens);
                            sections.push(RawSection {
                                name: current_name.clone(),
                                value: format_section_value(&value),
                            });
                            current_tokens.clear();
                        }
                        current_name = self.bump().text;
                        // Skip colon if present
                        if self.peek_kind() == TokenKind::Colon {
                            self.bump();
                        }
                    } else if depth == 0
                        && is_top_level_keyword(kw)
                    {
                        break;
                    } else {
                        current_tokens.push(self.bump());
                    }
                }
                TokenKind::Newline => {
                    self.bump();
                    current_tokens.push(synth(TokenKind::Newline));
                }
                _ => {
                    current_tokens.push(self.bump());
                }
            }
        }

        sections
    }

    // -----------------------------------------------------------------------
    // Token emission with spacing
    // -----------------------------------------------------------------------

    fn emit_token_with_spacing(&mut self) {
        let token = self.bump();
        if self.col == 0 {
            self.emit_indent();
        }

        match token.kind {
            TokenKind::Comment | TokenKind::DocComment => {
                self.out.push_str(&self.source_text(&token));
            }
            TokenKind::StringLit => {
                if token.text.contains('\n') {
                    let s = format!("\"\"\"{}\"\"\"", token.text);
                    self.emit(&s);
                } else {
                    let s = format!("\"{}\"", token.text);
                    self.emit(&s);
                }
            }
            TokenKind::Keyword | TokenKind::Ident | TokenKind::IntLit
            | TokenKind::FloatLit | TokenKind::BoolLit
            | TokenKind::NullLit | TokenKind::UnknownLit | TokenKind::Error => {
                self.emit_with_space(&token.text);
            }
            TokenKind::Eq => self.emit(" = "),
            TokenKind::EqEq => self.emit(" == "),
            TokenKind::Neq => self.emit(" != "),
            TokenKind::Lt => self.emit(" < "),
            TokenKind::Gt => self.emit(" > "),
            TokenKind::Le => self.emit(" <= "),
            TokenKind::Ge => self.emit(" >= "),
            TokenKind::Arrow => self.emit(" -> "),
            TokenKind::FatArrow => self.emit(" => "),
            TokenKind::Pipe => self.emit(" |> "),
            TokenKind::Plus => self.emit(" + "),
            TokenKind::Minus => {
                let ends_with_open = self.out.ends_with('(')
                    || self.out.ends_with('[')
                    || self.out.ends_with('{')
                    || self.out.ends_with(',')
                    || self.out.ends_with('=')
                    || self.out.ends_with("-> ")
                    || self.out.ends_with("|> ")
                    || self.out.ends_with(": ")
                    || self.col == 0;
                if ends_with_open {
                    self.emit("-");
                } else {
                    self.emit(" - ");
                }
            }
            TokenKind::Star => {
                let preceded_by_dot = self.out.ends_with('.');
                if preceded_by_dot {
                    // member access: Shell.* -> no space
                    self.emit("*");
                } else {
                    self.emit(" * ");
                }
            }
            TokenKind::Slash => self.emit(" / "),
            TokenKind::Percent => self.emit(" % "),
            TokenKind::Dot => self.emit("."),
            TokenKind::DotDot => self.emit(".."),
            TokenKind::DotDotEq => self.emit("..="),
            TokenKind::Colon => {
                while self.out.ends_with(' ') {
                    self.out.pop();
                    self.col = self.col.saturating_sub(1);
                }
                self.emit(": ");
            }
            TokenKind::Comma => {
                while self.out.ends_with(' ') {
                    self.out.pop();
                    self.col = self.col.saturating_sub(1);
                }
                let next = self.peek_kind();
                if next == TokenKind::RBrack
                    || next == TokenKind::RBrace
                    || next == TokenKind::RParen
                    || next == TokenKind::Newline
                    || next == TokenKind::Dedent
                {
                    self.emit(",");
                } else {
                    self.emit(", ");
                }
            }
            TokenKind::Question => self.emit("?"),
            TokenKind::Exclamation => self.emit("!"),
            TokenKind::NullableAccess => self.emit(".?"),
            TokenKind::NullCoalesce => self.emit("?|"),
            TokenKind::NullableIndex => self.emit("?["),
            TokenKind::LParen => self.emit("("),
            TokenKind::RParen => {
                while self.out.ends_with(' ') {
                    self.out.pop();
                    self.col = self.col.saturating_sub(1);
                }
                self.emit(")");
            }
            TokenKind::LBrack => self.emit("["),
            TokenKind::RBrack => {
                while self.out.ends_with(' ') {
                    self.out.pop();
                    self.col = self.col.saturating_sub(1);
                }
                self.emit("]");
            }
            TokenKind::LBrace => {
                let need_space = !self.out.ends_with('.');
                if need_space {
                    self.emit(" {");
                } else {
                    self.emit("{");
                }
            }
            TokenKind::RBrace => {
                while self.out.ends_with(' ') {
                    self.out.pop();
                    self.col = self.col.saturating_sub(1);
                }
                self.emit("}");
            }
            TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent | TokenKind::Eof => {}
        }
    }

    fn emit_with_space(&mut self, text: &str) {
        let last = self.out.chars().rev().next().unwrap_or('\n');
        let need_space = !matches!(last, '\n' | ' ' | '(' | '[' | '{' | '.')
            && last.is_alphanumeric()
            || matches!(last, '"' | ')' | ']' | '}');
        let need_space = need_space || matches!(last, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '"' | ')' | ']' | '}');
        // Actually simpler: always space unless after open bracket or at line start
        let last_byte = self.out.as_bytes().last().copied();
        let need = !matches!(last_byte, Some(b'\n') | Some(b' ') | Some(b'(') | Some(b'[') | Some(b'{') | Some(b'.') | None);
        if need {
            self.emit(" ");
        }
        self.emit(text);
    }

    // -----------------------------------------------------------------------
    // Source text helpers
    // -----------------------------------------------------------------------

    fn source_text(&self, token: &Token) -> String {
        let s = token.span.start.min(self.source.len());
        let e = token.span.end.min(self.source.len());
        if s < e {
            self.source[s..e].to_string()
        } else {
            match token.kind {
                TokenKind::Comment => format!("#{}", token.text),
                TokenKind::DocComment => format!("##{}", token.text),
                _ => token.text.clone(),
            }
        }
    }
}

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn synth(kind: TokenKind) -> Token {
    Token {
        kind,
        text: String::new(),
        span: crate::ast::span::Span::DUMMY,
    }
}

fn is_top_level_keyword(kw: &str) -> bool {
    matches!(
        kw,
        "agent"
            | "task"
            | "workflow"
            | "goal"
            | "memory"
            | "tool"
            | "model"
            | "prompt"
            | "policy"
            | "type"
            | "context"
            | "permissions"
            | "approval"
            | "project"
            | "import"
            | "pub"
    )
}

fn format_section_value(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!(": {}", trimmed)
}

fn reconstruct_tokens(tokens: &[Token]) -> String {
    let mut out = String::new();
    let mut needs_space = false;
    for t in tokens {
        match t.kind {
            TokenKind::Newline => {
                if !out.is_empty() && !out.ends_with(' ') {
                    out.push(' ');
                }
            }
            TokenKind::Indent | TokenKind::Dedent => {}
            TokenKind::Colon => {
                out.push_str(": ");
                needs_space = false;
            }
            TokenKind::Comma => {
                while out.ends_with(' ') {
                    out.pop();
                }
                out.push_str(", ");
                needs_space = false;
            }
            TokenKind::Comment => {
                if needs_space { out.push(' '); }
                out.push('#');
                out.push_str(&t.text);
                needs_space = true;
            }
            TokenKind::DocComment => {
                if needs_space { out.push(' '); }
                out.push_str("##");
                out.push_str(&t.text);
                needs_space = true;
            }
            TokenKind::StringLit => {
                if needs_space { out.push(' '); }
                if t.text.contains('\n') {
                    out.push_str(&format!("\"\"\"{}\"\"\"", t.text));
                } else {
                    out.push_str(&format!("\"{}\"", t.text));
                }
                needs_space = true;
            }
            TokenKind::Dot | TokenKind::LParen | TokenKind::RParen
            | TokenKind::LBrack | TokenKind::RBrack | TokenKind::LBrace
            | TokenKind::RBrace => {
                out.push_str(&t.text);
                needs_space = false;
            }
            _ => {
                if needs_space { out.push(' '); }
                out.push_str(&t.text);
                needs_space = true;
            }
        }
    }
    out.trim().to_string()
}

#[derive(Debug, Clone)]
struct RawSection {
    name: String,
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_format() {
        let input = "project Hello\n\nimport foo\n";
        let result = format_source(input).unwrap();
        assert_eq!(result, "project Hello\n\nimport foo\n", "output: {:?}", result);
    }

    #[test]
    fn test_idempotent() {
        let input = "project Hello\n";
        let first = format_source(input).unwrap();
        let second = format_source(&first).unwrap();
        assert_eq!(first, second, "Formatting should be idempotent");
    }

    #[test]
    fn test_no_leading_blank() {
        let input = "project Hello\n";
        let result = format_source(input).unwrap();
        assert!(!result.starts_with('\n'), "Should not start with blank line");
    }

    #[test]
    fn test_ends_with_newline() {
        let input = "project Hello";
        let result = format_source(input).unwrap();
        assert!(result.ends_with('\n'), "Should end with newline");
        assert_eq!(result.matches('\n').count(), 1, "Should end with exactly one newline");
    }

    #[test]
    fn test_minimal_agent() {
        let input = "agent Test\n    model: X\n";
        let result = format_source(input).unwrap();
        assert!(result.contains("\tmodel: X"), "Agent section should be indented: {:?}", result);
    }
}
