
use crate::ast::span::{FileId, Span};

/// All token kinds produced by the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Literals
    IntLit,
    FloatLit,
    StringLit,
    BoolLit,
    NullLit,
    UnknownLit,

    // Identifiers & Keywords
    Ident,
    Keyword,

    // Operators & Punctuation
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Eq,          // =
    EqEq,        // ==
    Neq,         // !=
    Lt,          // <
    Gt,          // >
    Le,          // <=
    Ge,          // >=
    Arrow,       // ->
    Pipe,        // |>
    Dot,         // .
    DotDot,      // ..
    DotDotEq,    // ..=
    Comma,       // ,
    Colon,       // :
    LParen,      // (
    RParen,      // )
    LBrack,      // [
    RBrack,      // ]
    LBrace,      // {
    RBrace,      // }
    Question,    // ?
    Exclamation, // !
    NullableAccess,  // .?
    NullCoalesce,    // ?|
    NullableIndex,   // ?[

    // Indentation
    Newline,
    Indent,
    Dedent,

    // Special
    Comment,
    DocComment,
    Eof,
    Error,
}

/// A token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, text: impl Into<String>, span: Span) -> Self {
        Self { kind, text: text.into(), span }
    }

    pub fn dummy(kind: TokenKind) -> Self {
        Self {
            kind,
            text: String::new(),
            span: Span::DUMMY,
        }
    }
}

/// All reserved keywords in Lucky.
pub const KEYWORDS: &[&str] = &[
    "agent", "allow", "and", "approval", "ask",
    "attempt", "await", "break", "capability", "const",
    "context", "continue", "deep", "deny", "else",
    "error", "fallback", "false", "fast", "fn",
    "for", "goal", "human", "if", "import",
    "in", "input", "knowledge", "let", "loop",
    "match", "memory", "model", "none", "not",
    "null", "or", "output", "parallel", "permission",
    "permissions", "policy", "project", "prompt", "pub",
    "recover", "retry", "return", "run", "select",
    "skip", "steps", "success", "swarm", "task",
    "then", "tool", "true", "unknown", "use",
    "wait", "when", "where", "workflow",
];

/// Check if a string is a reserved keyword.
pub fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

/// Check if a character can start an identifier.
pub fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

/// Check if a character can continue an identifier.
pub fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
