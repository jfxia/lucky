
use super::literal::Literal;
use super::span::Span;

/// Pattern for match arms and for-loop bindings.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// `_` wildcard.
    Wildcard { span: Span },

    /// Variable binding: `name`.
    Variable { name: String, span: Span },

    /// Literal pattern: `42`, `"hello"`, `true`.
    Literal { value: Literal, span: Span },

    /// Constructor/destructure pattern: `Success(data)`.
    Constructor {
        name: String,
        fields: Vec<Pattern>,
        span: Span,
    },

    /// List pattern: `[a, b, ..rest]`.
    List {
        elements: Vec<Pattern>,
        rest: Option<String>,
        span: Span,
    },

    /// Map pattern: `{"key": pat}`.
    Map {
        entries: Vec<(Pattern, Pattern)>,
        span: Span,
    },

    /// Error pattern (recovery).
    Error { span: Span },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard { span } => *span,
            Pattern::Variable { span, .. } => *span,
            Pattern::Literal { span, .. } => *span,
            Pattern::Constructor { span, .. } => *span,
            Pattern::List { span, .. } => *span,
            Pattern::Map { span, .. } => *span,
            Pattern::Error { span } => *span,
        }
    }
}
