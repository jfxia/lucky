
use super::span::Span;
use crate::ast::stmt::MatchArm;
use crate::ast::types::TypedIdent;

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Not,
}

impl UnaryOp {
    pub fn as_str(&self) -> &str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "not",
        }
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Concat,   // '+' for strings/lists
    Repeat,   // '*' for strings
}

impl BinOp {
    pub fn as_str(&self) -> &str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Neq => "!=",
            BinOp::Lt => "<",
            BinOp::Gt => ">",
            BinOp::Le => "<=",
            BinOp::Ge => ">=",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::Concat => "+",
            BinOp::Repeat => "*",
        }
    }

    pub fn precedence(&self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => 3,
            BinOp::Add | BinOp::Sub | BinOp::Concat => 4,
            BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::Repeat => 5,
        }
    }
}

/// Comparison operators (used in confidence expressions and guards).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
}

impl CmpOp {
    pub fn as_str(&self) -> &str {
        match self {
            CmpOp::Eq => "==",
            CmpOp::Neq => "!=",
            CmpOp::Lt => "<",
            CmpOp::Gt => ">",
            CmpOp::Le => "<=",
            CmpOp::Ge => ">=",
        }
    }
}

/// A qualified name like `company.security.Reviewer`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedName {
    pub parts: Vec<String>,
    pub span: Span,
}

impl QualifiedName {
    pub fn new(parts: Vec<String>, span: Span) -> Self {
        Self { parts, span }
    }

    pub fn simple(name: &str, span: Span) -> Self {
        Self { parts: vec![name.to_string()], span }
    }

    pub fn last(&self) -> &str {
        self.parts.last().map(|s| s.as_str()).unwrap_or("")
    }

    pub fn to_string(&self) -> String {
        self.parts.join(".")
    }
}

/// A function argument: `name = value` or just `value`.
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Box<Expr>,
    pub span: Span,
}

/// Reasoning mode (for `reason` expressions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReasonMode {
    Deep,
    Fast,
    None,
}

/// Query operations (for query expressions).
#[derive(Debug, Clone, PartialEq)]
pub enum QueryOp {
    Where(Box<Expr>),
    Select(Box<Expr>),
    OrderBy { expr: Box<Expr>, ascending: bool },
    GroupBy(Box<Expr>),
    Limit(Box<Expr>),
    Skip(Box<Expr>),
}

/// Expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal value.
    Lit { value: super::literal::Literal, span: Span },

    /// A variable or qualified name reference.
    Var { name: QualifiedName, span: Span },

    /// Function/task/agent call: `foo(args)`.
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
        span: Span,
    },

    /// Index access: `expr[index]`.
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },

    /// Field access: `expr.field`.
    FieldAccess {
        base: Box<Expr>,
        field: String,
        span: Span,
    },

    /// Nullable field access: `expr.?field`.
    NullableFieldAccess {
        base: Box<Expr>,
        field: String,
        span: Span,
    },

    /// Nullable index: `expr?[index]`.
    NullableIndex {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },

    /// Binary operation.
    BinaryOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },

    /// Unary operation.
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },

    /// Lambda: `fn (params) => body` or `fn x => x * 2`.
    Lambda {
        params: Vec<TypedIdent>,
        body: Box<Expr>,
        span: Span,
    },

    /// Pipeline: `a |> b |> c`.
    Pipeline {
        stages: Vec<Expr>,
        span: Span,
    },

    /// Query expression: `users where age > 18 select name`.
    Query {
        source: Box<Expr>,
        ops: Vec<QueryOp>,
        span: Span,
    },

    /// List literal: `[1, 2, 3]`.
    List {
        elements: Vec<Expr>,
        span: Span,
    },

    /// Set literal: `{1, 2, 3}`. (Distinguished from Map by element type.)
    Set {
        elements: Vec<Expr>,
        span: Span,
    },

    /// Map literal: `{"key": value}`.
    Map {
        entries: Vec<(Expr, Expr)>,
        span: Span,
    },

    /// String interpolation part.
    InterpolatedString {
        parts: Vec<InterpolatedPart>,
        span: Span,
    },

    /// `ask ModelName: ...`.
    Ask {
        model: String,
        body: Vec<String>,
        span: Span,
    },

    /// `ask human: ...`.
    AskHuman {
        body: Vec<String>,
        span: Span,
    },

    /// `reason deep` / `reason fast` / `reason none`.
    Reason {
        mode: ReasonMode,
        span: Span,
    },

    /// `use ModelName`.
    Use {
        model: String,
        span: Span,
    },

    /// Confidence check: `expr confidence > 0.9`.
    Confidence {
        expr: Box<Expr>,
        op: CmpOp,
        threshold: Box<Expr>,
        span: Span,
    },

    /// Null-coalescing: `expr ?| default`.
    NullCoalesce {
        expr: Box<Expr>,
        default: Box<Expr>,
        span: Span,
    },

    /// Range: `start..end` or `start..=end` or `..end` or `start..`.
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
        span: Span,
    },

    /// Parenthesized expression.
    Paren {
        expr: Box<Expr>,
        span: Span,
    },

    /// If-then-else expression: `if cond then a else b`.
    IfExpr {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Box<Expr>,
        span: Span,
    },

    /// Match expression: `match x { pat => val, ... }`.
    MatchExpr {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },

    /// An error expression (recovery placeholder).
    Error { span: Span },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Lit { span, .. } => *span,
            Expr::Var { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::FieldAccess { span, .. } => *span,
            Expr::NullableFieldAccess { span, .. } => *span,
            Expr::NullableIndex { span, .. } => *span,
            Expr::BinaryOp { span, .. } => *span,
            Expr::UnaryOp { span, .. } => *span,
            Expr::Lambda { span, .. } => *span,
            Expr::Pipeline { span, .. } => *span,
            Expr::Query { span, .. } => *span,
            Expr::List { span, .. } => *span,
            Expr::Set { span, .. } => *span,
            Expr::Map { span, .. } => *span,
            Expr::InterpolatedString { span, .. } => *span,
            Expr::Ask { span, .. } => *span,
            Expr::AskHuman { span, .. } => *span,
            Expr::Reason { span, .. } => *span,
            Expr::Use { span, .. } => *span,
            Expr::Confidence { span, .. } => *span,
            Expr::NullCoalesce { span, .. } => *span,
            Expr::Range { span, .. } => *span,
            Expr::Paren { span, .. } => *span,
            Expr::IfExpr { span, .. } => *span,
            Expr::MatchExpr { span, .. } => *span,
            Expr::Error { span } => *span,
        }
    }
}

/// Part of an interpolated string: either literal text or an embedded expression.
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolatedPart {
    Text(String),
    Expr(Box<Expr>),
}
