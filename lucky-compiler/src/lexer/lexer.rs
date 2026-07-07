use crate::ast::span::{FileId, Span};
use super::token::{Token, TokenKind, is_keyword, is_ident_start, is_ident_continue};
use super::indent::IndentProcessor;

/// The Lucky lexer. Tokenizes source text into a stream of tokens,
/// including INDENT/DEDENT synthesis.
pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    file_id: FileId,
    tokens: Vec<Token>,
    errors: Vec<String>,
}

impl Lexer {
    pub fn new(source: &str, file_id: FileId) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            file_id,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Tokenize the entire source and return the tokens (including INDENT/DEDENT).
    pub fn tokenize(&mut self) -> Vec<Token> {
        // Phase 1: Produce raw tokens (no INDENT/DEDENT, NEWLINE at EOL)
        let mut raw_tokens = Vec::new();
        let mut line_start = self.pos;

        while !self.is_eof() {
            self.skip_spaces();

            if self.is_eof() { break; }

            let c = self.current();

            if c == '\n' || c == '\r' {
                // Emit NEWLINE if we saw content on this line
                let start = self.pos;
                self.advance_newline();
                let span = Span::new(start, self.pos, self.file_id);
                raw_tokens.push(Token::new(TokenKind::Newline, "\\n", span));
                line_start = self.pos;
                continue;
            }

            if c == '#' {
                self.lex_comment(&mut raw_tokens);
                continue;
            }

            let start = self.pos;
            let token = self.lex_token();
            if let Some(t) = token {
                raw_tokens.push(t);
            }
        }

        // Final NEWLINE if file doesn't end with one
        let end = self.pos;
        raw_tokens.push(Token::new(TokenKind::Newline, "\\n", Span::new(end, end, self.file_id)));
        raw_tokens.push(Token::new(TokenKind::Eof, "", Span::new(end, end, self.file_id)));

        // Phase 2: Process INDENT/DEDENT
        let mut processor = IndentProcessor::new(self.file_id);
        self.tokens = processor.process(raw_tokens);

        self.tokens.clone()
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    // --- Helpers ---

    fn current(&self) -> char {
        self.source.get(self.pos).copied().unwrap_or('\0')
    }

    fn peek(&self, offset: usize) -> char {
        self.source.get(self.pos + offset).copied().unwrap_or('\0')
    }

    fn advance(&mut self) -> char {
        let c = self.current();
        self.pos += 1;
        c
    }

    fn advance_newline(&mut self) {
        let c = self.current();
        if c == '\r' && self.peek(1) == '\n' {
            self.pos += 2;
        } else {
            self.pos += 1;
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn skip_spaces(&mut self) {
        while !self.is_eof() {
            let c = self.current();
            if c == ' ' || c == '\t' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn span(&self, start: usize) -> Span {
        Span::new(start, self.pos, self.file_id)
    }

    fn error(&mut self, msg: String) {
        self.errors.push(msg);
    }

    // --- Token dispatch ---

    fn lex_token(&mut self) -> Option<Token> {
        let start = self.pos;
        let c = self.advance();

        match c {
            // Multi-character operators
            '-' if self.current() == '>' => {
                self.advance();
                Some(Token::new(TokenKind::Arrow, "->", self.span(start)))
            }
            '|' if self.current() == '>' => {
                self.advance();
                Some(Token::new(TokenKind::Pipe, "|>", self.span(start)))
            }
            '=' if self.current() == '>' => {
                self.advance();
                Some(Token::new(TokenKind::FatArrow, "=>", self.span(start)))
            }
            '=' if self.current() == '=' => {
                self.advance();
                Some(Token::new(TokenKind::EqEq, "==", self.span(start)))
            }
            '!' if self.current() == '=' => {
                self.advance();
                Some(Token::new(TokenKind::Neq, "!=", self.span(start)))
            }
            '<' if self.current() == '=' => {
                self.advance();
                Some(Token::new(TokenKind::Le, "<=", self.span(start)))
            }
            '>' if self.current() == '=' => {
                self.advance();
                Some(Token::new(TokenKind::Ge, ">=", self.span(start)))
            }
            '.' if self.current() == '.' => {
                self.advance();
                if self.current() == '=' {
                    self.advance();
                    Some(Token::new(TokenKind::DotDotEq, "..=", self.span(start)))
                } else {
                    Some(Token::new(TokenKind::DotDot, "..", self.span(start)))
                }
            }
            '.' if self.current() == '?' => {
                self.advance();
                Some(Token::new(TokenKind::NullableAccess, ".?", self.span(start)))
            }
            '?' if self.current() == '|' => {
                self.advance();
                Some(Token::new(TokenKind::NullCoalesce, "?|", self.span(start)))
            }
            '?' if self.current() == '[' => {
                self.advance();
                Some(Token::new(TokenKind::NullableIndex, "?[", self.span(start)))
            }

            // Single-character tokens
            '+' => Some(Token::new(TokenKind::Plus, "+", self.span(start))),
            '-' => Some(Token::new(TokenKind::Minus, "-", self.span(start))),
            '*' => Some(Token::new(TokenKind::Star, "*", self.span(start))),
            '/' => Some(Token::new(TokenKind::Slash, "/", self.span(start))),
            '%' => Some(Token::new(TokenKind::Percent, "%", self.span(start))),
            '=' => Some(Token::new(TokenKind::Eq, "=", self.span(start))),
            '<' => Some(Token::new(TokenKind::Lt, "<", self.span(start))),
            '>' => Some(Token::new(TokenKind::Gt, ">", self.span(start))),
            '.' => Some(Token::new(TokenKind::Dot, ".", self.span(start))),
            ',' => Some(Token::new(TokenKind::Comma, ",", self.span(start))),
            ':' => Some(Token::new(TokenKind::Colon, ":", self.span(start))),
            '(' => Some(Token::new(TokenKind::LParen, "(", self.span(start))),
            ')' => Some(Token::new(TokenKind::RParen, ")", self.span(start))),
            '[' => Some(Token::new(TokenKind::LBrack, "[", self.span(start))),
            ']' => Some(Token::new(TokenKind::RBrack, "]", self.span(start))),
            '{' => Some(Token::new(TokenKind::LBrace, "{", self.span(start))),
            '}' => Some(Token::new(TokenKind::RBrace, "}", self.span(start))),
            '?' => Some(Token::new(TokenKind::Question, "?", self.span(start))),
            '!' => Some(Token::new(TokenKind::Exclamation, "!", self.span(start))),

            // String literals
            '"' => self.lex_string(start),

            // Numbers
            '0'..='9' => self.lex_number(start),

            // Identifiers or keywords
            c if is_ident_start(c) => self.lex_ident_or_keyword(start),

            // Unexpected character
            _ => {
                self.error(format!("Unexpected character: '{}'", c));
                Some(Token::new(TokenKind::Error, c.to_string(), self.span(start)))
            }
        }
    }

    // --- String lexing ---

    fn lex_string(&mut self, start: usize) -> Option<Token> {
        // Check for multi-line string: """
        let is_multiline = self.current() == '"' && self.peek(1) == '"';
        if is_multiline {
            self.pos += 2; // skip the two extra quotes
            let content_start = self.pos;
            // Consume until """
            while !self.is_eof() {
                if self.current() == '"' && self.peek(1) == '"' && self.peek(2) == '"' {
                    let content = self.source[content_start..self.pos].iter().collect::<String>();
                    self.pos += 3;
                    return Some(Token::new(TokenKind::StringLit, content, self.span(start)));
                }
                self.pos += 1;
            }
            self.error("Unterminated multi-line string literal".to_string());
            return Some(Token::new(TokenKind::Error, "unterminated string", self.span(start)));
        }

        // Single-line string
        let content_start = self.pos;
        while !self.is_eof() && self.current() != '"' && self.current() != '\n' && self.current() != '\r' {
            if self.current() == '\\' {
                self.pos += 1; // skip escape char
            }
            self.pos += 1;
        }
        if self.current() == '"' {
            let content = self.source[content_start..self.pos].iter().collect::<String>();
            self.advance(); // skip closing quote
            Some(Token::new(TokenKind::StringLit, content, self.span(start)))
        } else {
            self.error("Unterminated string literal".to_string());
            Some(Token::new(TokenKind::Error, "unterminated string", self.span(start)))
        }
    }

    // --- Number lexing ---

    fn lex_number(&mut self, start: usize) -> Option<Token> {
        // Check for hex (0x) or binary (0b) prefix
        if self.source[start] == '0' {
            match self.current() {
                'x' | 'X' => return self.lex_hex_number(start),
                'b' | 'B' => return self.lex_bin_number(start),
                _ => {}
            }
        }

        let mut is_float = false;

        // Integer part
        while !self.is_eof() && self.current().is_ascii_digit() {
            self.pos += 1;
        }

        // Allow underscore separators within digits (re-lex needed for simplicity)
        // Fractional part
        if self.current() == '.' && self.peek(1).is_ascii_digit() {
            is_float = true;
            self.pos += 1; // skip dot
            while !self.is_eof() && self.current().is_ascii_digit() {
                self.pos += 1;
            }
        }

        // Exponent
        if self.current() == 'e' || self.current() == 'E' {
            is_float = true;
            self.pos += 1;
            if self.current() == '+' || self.current() == '-' {
                self.pos += 1;
            }
            while !self.is_eof() && self.current().is_ascii_digit() {
                self.pos += 1;
            }
        }

        let text = self.source[start..self.pos].iter().collect::<String>();
        let text_clean: String = text.chars().filter(|c| *c != '_').collect();

        if is_float {
            match text_clean.parse::<f64>() {
                Ok(_) => Some(Token::new(TokenKind::FloatLit, text_clean, self.span(start))),
                Err(_) => {
                    self.error(format!("Invalid float literal: {}", text_clean));
                    Some(Token::new(TokenKind::Error, text_clean, self.span(start)))
                }
            }
        } else {
            match text_clean.parse::<i64>() {
                Ok(_) => Some(Token::new(TokenKind::IntLit, text_clean, self.span(start))),
                Err(_) => {
                    self.error(format!("Invalid integer literal: {}", text_clean));
                    Some(Token::new(TokenKind::Error, text_clean, self.span(start)))
                }
            }
        }
    }

    fn lex_hex_number(&mut self, start: usize) -> Option<Token> {
        self.pos += 1; // skip 'x'
        while !self.is_eof() && self.current().is_ascii_hexdigit() {
            self.pos += 1;
        }
        let text = self.source[start..self.pos].iter().collect::<String>();
        match i64::from_str_radix(&text[2..].replace('_', ""), 16) {
            Ok(v) => Some(Token::new(TokenKind::IntLit, v.to_string(), self.span(start))),
            Err(_) => {
                self.error(format!("Invalid hex literal: {}", text));
                Some(Token::new(TokenKind::Error, text, self.span(start)))
            }
        }
    }

    fn lex_bin_number(&mut self, start: usize) -> Option<Token> {
        self.pos += 1; // skip 'b'
        while !self.is_eof() && (self.current() == '0' || self.current() == '1') {
            self.pos += 1;
        }
        let text = self.source[start..self.pos].iter().collect::<String>();
        match i64::from_str_radix(&text[2..].replace('_', ""), 2) {
            Ok(v) => Some(Token::new(TokenKind::IntLit, v.to_string(), self.span(start))),
            Err(_) => {
                self.error(format!("Invalid binary literal: {}", text));
                Some(Token::new(TokenKind::Error, text, self.span(start)))
            }
        }
    }

    // --- Identifier / Keyword lexing ---

    fn lex_ident_or_keyword(&mut self, start: usize) -> Option<Token> {
        while !self.is_eof() && is_ident_continue(self.current()) {
            self.pos += 1;
        }
        let text: String = self.source[start..self.pos].iter().collect();

        // Check for special literals
        match text.as_str() {
            "true" => return Some(Token::new(TokenKind::BoolLit, text, self.span(start))),
            "false" => return Some(Token::new(TokenKind::BoolLit, text, self.span(start))),
            "null" => return Some(Token::new(TokenKind::NullLit, text, self.span(start))),
            "unknown" => return Some(Token::new(TokenKind::UnknownLit, text, self.span(start))),
            _ => {}
        }

        if is_keyword(&text) {
            Some(Token::new(TokenKind::Keyword, text, self.span(start)))
        } else {
            Some(Token::new(TokenKind::Ident, text, self.span(start)))
        }
    }

    // --- Comment lexing ---

    fn lex_comment(&mut self, tokens: &mut Vec<Token>) {
        let start = self.pos - 1; // account for the '#' we already consumed
        let is_doc = self.current() == '#';
        if is_doc {
            self.pos += 1;
        }

        let content_start = self.pos;
        while !self.is_eof() && self.current() != '\n' && self.current() != '\r' {
            self.pos += 1;
        }
        let content: String = self.source[content_start..self.pos].iter().collect();
        let kind = if is_doc { TokenKind::DocComment } else { TokenKind::Comment };
        tokens.push(Token::new(kind, content.trim().to_string(), self.span(start)));
    }
}
