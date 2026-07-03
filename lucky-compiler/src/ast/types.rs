
use super::span::Span;

/// A typed identifier: `name: Type`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedIdent {
    pub name: String,
    pub typ: Option<Box<TypeExpr>>,
    pub span: Span,
}

/// Type expression.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// A primitive type: `Bool`, `Int`, `Float`, `String`, etc.
    Primitive {
        name: String,
        span: Span,
    },

    /// A named type (could be a user-defined type or an AI type): `Agent`, `MyStruct`.
    Named {
        name: String,
        args: Vec<TypeExpr>,
        span: Span,
    },

    /// Nullable type: `T?`.
    Nullable {
        inner: Box<TypeExpr>,
        span: Span,
    },

    /// Optional type: `T!`.
    Optional {
        inner: Box<TypeExpr>,
        span: Span,
    },

    /// Union type: `T | U`.
    Union {
        left: Box<TypeExpr>,
        right: Box<TypeExpr>,
        span: Span,
    },

    /// List type: `List<T>`.
    List {
        element: Box<TypeExpr>,
        span: Span,
    },

    /// Set type: `Set<T>`.
    Set {
        element: Box<TypeExpr>,
        span: Span,
    },

    /// Map type: `Map<K, V>`.
    Map {
        key: Box<TypeExpr>,
        value: Box<TypeExpr>,
        span: Span,
    },

    /// Tuple type: `(T1, T2, ...)`.
    Tuple {
        elements: Vec<TypeExpr>,
        span: Span,
    },

    /// Function type: `fn(T1, T2) -> R`.
    Function {
        params: Vec<TypeExpr>,
        returns: Vec<TypeExpr>,
        span: Span,
    },

    /// A qualified type name: `Module.Type`.
    Qualified {
        path: Vec<String>,
        args: Vec<TypeExpr>,
        span: Span,
    },

    /// Parenthesized type: `(T)`.
    Paren {
        inner: Box<TypeExpr>,
        span: Span,
    },

    /// Error type (recovery).
    Error { span: Span },
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Primitive { span, .. } => *span,
            TypeExpr::Named { span, .. } => *span,
            TypeExpr::Nullable { span, .. } => *span,
            TypeExpr::Optional { span, .. } => *span,
            TypeExpr::Union { span, .. } => *span,
            TypeExpr::List { span, .. } => *span,
            TypeExpr::Set { span, .. } => *span,
            TypeExpr::Map { span, .. } => *span,
            TypeExpr::Tuple { span, .. } => *span,
            TypeExpr::Function { span, .. } => *span,
            TypeExpr::Qualified { span, .. } => *span,
            TypeExpr::Paren { span, .. } => *span,
            TypeExpr::Error { span } => *span,
        }
    }
}

/// The set of primitive type names in Lucky.
pub const PRIMITIVE_TYPES: &[&str] = &[
    "Bool", "Int", "Float", "Decimal", "String", "Bytes",
    "Char", "Time", "Duration", "UUID", "URI", "Version", "Path",
];

/// The AI type names (not primitives but built-in named types).
pub const AI_TYPES: &[&str] = &[
    "Agent", "Task", "Workflow", "Goal", "Prompt", "Memory",
    "Knowledge", "Context", "Tool", "Model", "Artifact", "Result",
    "Capability", "Approval", "Embedding", "Observation", "Plan",
    "Reasoning",
];

/// All built-in type names.
pub fn is_builtin_type(name: &str) -> bool {
    PRIMITIVE_TYPES.contains(&name) || AI_TYPES.contains(&name)
        || name == "Any" || name == "Nothing" || name == "Error"
        || name == "List" || name == "Set" || name == "Map"
        || name == "Queue" || name == "Stack" || name == "Graph"
        || name == "Tree" || name == "Stream" || name == "Channel"
        || name == "Promise" || name == "Probabilistic" || name == "Secret"
}
