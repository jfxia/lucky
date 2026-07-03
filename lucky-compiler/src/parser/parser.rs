use crate::ast::span::{FileId, Span};
use crate::lexer::token::{Token, TokenKind};
use crate::diagnostics::diagnostic::DiagnosticBag;

/// Core parser infrastructure. Wraps a token stream with peek/bump/expect helpers
/// and error recovery.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    pub diagnostics: DiagnosticBag,
    file_id: FileId,
    /// For error recovery: skip to the next token after one of these kinds
    sync_tokens: Vec<TokenKind>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file_id: FileId) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: DiagnosticBag::new(),
            file_id,
            sync_tokens: vec![TokenKind::Newline, TokenKind::Eof],
        }
    }

    // ---- Token navigation ----

    pub fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&EOF_TOKEN)
    }

    pub fn peek(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&EOF_TOKEN)
    }

    pub fn kind(&self) -> TokenKind {
        self.current().kind
    }

    pub fn text(&self) -> &str {
        &self.current().text
    }

    pub fn span(&self) -> Span {
        self.current().span
    }

    pub fn is_eof(&self) -> bool {
        self.kind() == TokenKind::Eof
    }

    pub fn is(&self, kind: TokenKind) -> bool {
        self.kind() == kind
    }

    pub fn is_keyword(&self, kw: &str) -> bool {
        self.kind() == TokenKind::Keyword && self.text() == kw
    }

    pub fn is_ident(&self) -> bool {
        self.kind() == TokenKind::Ident
    }

    /// Advance one token.
    pub fn bump(&mut self) -> &Token {
        if !self.is_eof() {
            self.pos += 1;
        }
        self.current()
    }

    /// Advance and return the span of the consumed token.
    pub fn bump_span(&mut self) -> Span {
        let span = self.span();
        self.bump();
        span
    }

    /// If current token matches `kind`, advance and return true.
    pub fn eat(&mut self, kind: TokenKind) -> bool {
        if self.kind() == kind {
            self.bump();
            true
        } else {
            false
        }
    }

    /// If current token is the keyword `kw`, advance and return true.
    pub fn eat_keyword(&mut self, kw: &str) -> bool {
        if self.is_keyword(kw) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Expect a token kind. If not found, emit an error and recover.
    pub fn expect(&mut self, kind: TokenKind, context: &str) -> bool {
        if self.kind() == kind {
            self.bump();
            true
        } else {
            self.error(format!(
                "Expected {} but found {} '{}'",
                token_name(kind),
                token_name(self.kind()),
                self.text()
            ));
            false
        }
    }

    /// Expect a keyword. If not found, emit an error.
    pub fn expect_keyword(&mut self, kw: &str, context: &str) -> bool {
        if self.is_keyword(kw) {
            self.bump();
            true
        } else {
            self.error(format!(
                "Expected '{}' but found '{}'",
                kw, self.text()
            ));
            false
        }
    }

    /// Expect and consume an identifier, returning its text and span.
    pub fn expect_ident(&mut self, context: &str) -> Option<(String, Span)> {
        if self.is_ident() {
            let text = self.text().to_string();
            let span = self.span();
            self.bump();
            Some((text, span))
        } else if self.kind() == TokenKind::Keyword {
            // Keywords can sometimes be used where identifiers are expected (recovery)
            let text = self.text().to_string();
            let span = self.span();
            self.bump();
            Some((text, span))
        } else {
            self.error(format!("Expected identifier in {} but found '{}'", context, self.text()));
            None
        }
    }

    // ---- Error reporting ----

    pub fn error(&mut self, message: String) {
        use crate::diagnostics::diagnostic::Diagnostic;
        self.diagnostics.emit(
            Diagnostic::error(message).with_label(self.span(), "here")
        );
    }

    pub fn warning(&mut self, message: String) {
        use crate::diagnostics::diagnostic::Diagnostic;
        self.diagnostics.emit(
            Diagnostic::warning(message).with_label(self.span(), "here")
        );
    }

    // ---- Error recovery ----

    /// Skip tokens until we find a synchronization point (NEWLINE, EOF, etc.).
    pub fn sync(&mut self) {
        while !self.is_eof() {
            if self.sync_tokens.contains(&self.kind()) {
                return;
            }
            self.bump();
        }
    }

    /// Skip to the next NEWLINE or EOF.
    pub fn sync_to_newline(&mut self) {
        while !self.is_eof() && self.kind() != TokenKind::Newline {
            self.bump();
        }
        if self.kind() == TokenKind::Newline {
            self.bump();
        }
    }

    // ---- Indentation handling ----

    /// Expect an INDENT, enter a new block. Returns true if INDENT found.
    /// If INDENT is missing on a keyword that requires it, emits error and recovers.
    pub fn expect_indent(&mut self) -> bool {
        if self.kind() == TokenKind::Indent {
            self.bump();
            true
        } else {
            self.error(format!(
                "Expected indented block but found '{}'. Lucky uses indentation for blocks.",
                self.text()
            ));
            // Recovery: try to continue parsing without INDENT
            false
        }
    }

    /// Check if we've reached DEDENT (end of current block).
    pub fn at_dedent(&self) -> bool {
        self.kind() == TokenKind::Dedent
    }

    /// Consume DEDENT token if present.
    pub fn eat_dedent(&mut self) -> bool {
        self.eat(TokenKind::Dedent)
    }

    /// Check if we've reached the end of a statement (NEWLINE, DEDENT, EOF).
    pub fn at_stmt_end(&self) -> bool {
        matches!(self.kind(), TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof)
    }

    // ---- Helper: peek multiple ----

    pub fn peek_kind(&self, n: usize) -> TokenKind {
        self.tokens.get(self.pos + n).map(|t| t.kind).unwrap_or(TokenKind::Eof)
    }

    pub fn peek_keyword(&self, n: usize) -> Option<&str> {
        self.tokens.get(self.pos + n)
            .filter(|t| t.kind == TokenKind::Keyword)
            .map(|t| t.text.as_str())
    }
}

/// A dummy EOF token used for safe access when position is past the token stream.
static EOF_TOKEN: Token = Token {
    kind: TokenKind::Eof,
    text: String::new(),
    span: Span::DUMMY,
};

/// Human-readable name for a token kind (for error messages).
fn token_name(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::IntLit => "integer literal",
        TokenKind::FloatLit => "float literal",
        TokenKind::StringLit => "string literal",
        TokenKind::BoolLit => "boolean literal",
        TokenKind::NullLit => "'null'",
        TokenKind::UnknownLit => "'unknown'",
        TokenKind::Ident => "identifier",
        TokenKind::Keyword => "keyword",
        TokenKind::Plus => "'+'",
        TokenKind::Minus => "'-'",
        TokenKind::Star => "'*'",
        TokenKind::Slash => "'/'",
        TokenKind::Percent => "'%'",
        TokenKind::Eq => "'='",
        TokenKind::EqEq => "'=='",
        TokenKind::Neq => "'!='",
        TokenKind::Lt => "'<'",
        TokenKind::Gt => "'>'",
        TokenKind::Le => "'<='",
        TokenKind::Ge => "'>='",
        TokenKind::Arrow => "'->'",
        TokenKind::Pipe => "'|>'",
        TokenKind::Dot => "'.'",
        TokenKind::DotDot => "'..'",
        TokenKind::DotDotEq => "'..='",
        TokenKind::Comma => "','",
        TokenKind::Colon => "':'",
        TokenKind::LParen => "'('",
        TokenKind::RParen => "')'",
        TokenKind::LBrack => "'['",
        TokenKind::RBrack => "']'",
        TokenKind::LBrace => "'{'",
        TokenKind::RBrace => "'}'",
        TokenKind::Question => "'?'",
        TokenKind::Exclamation => "'!'",
        TokenKind::NullableAccess => "'.?'",
        TokenKind::NullCoalesce => "'?|'",
        TokenKind::NullableIndex => "'?['",
        TokenKind::Newline => "newline",
        TokenKind::Indent => "indent",
        TokenKind::Dedent => "dedent",
        TokenKind::Comment => "comment",
        TokenKind::DocComment => "doc comment",
        TokenKind::Eof => "end of file",
        TokenKind::Error => "error token",
    }
}
