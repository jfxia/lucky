
use super::expr::Expr;
use super::pattern::Pattern;
use super::span::Span;

/// A recovery action within an `attempt/recover` block.
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryAction {
    Retry { count: Option<Expr>, backoff: Option<String>, max_delay: Option<Expr>, span: Span },
    Fallback { task: Expr, span: Span },
    Human { message: Option<String>, span: Span },
    Abort { span: Span },
    Skip { span: Span },
}

/// An `if`/`elif` branch.
#[derive(Debug, Clone, PartialEq)]
pub struct IfBranch {
    pub condition: Expr,
    pub body: Block,
}

/// A `match` arm.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Block,
    pub span: Span,
}

/// A pipeline stage: `|> operation [args]`.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineStage {
    pub operation: String,
    pub args: Vec<Expr>,
    pub span: Span,
}

/// A block is a sequence of statements at a common indentation level.
/// INDENT/DEDENT are handled by the parser; the AST stores just the statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

impl Block {
    pub fn new(stmts: Vec<Stmt>, span: Span) -> Self {
        Self { stmts, span }
    }

    pub fn empty(span: Span) -> Self {
        Self { stmts: vec![], span }
    }

    pub fn is_empty(&self) -> bool {
        self.stmts.is_empty()
    }
}

/// Statement node.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let name [: Type] = value`.
    Let {
        name: String,
        typ: Option<Box<super::types::TypeExpr>>,
        value: Expr,
        span: Span,
    },

    /// `const NAME [: Type] = value`.
    Const {
        name: String,
        typ: Option<Box<super::types::TypeExpr>>,
        value: Expr,
        span: Span,
    },

    /// `target = value` (for mutable agent memory fields).
    Assign {
        target: Expr,
        value: Expr,
        span: Span,
    },

    /// An expression used as a statement (call, pipeline, etc.).
    ExprStmt {
        expr: Expr,
        span: Span,
    },

    /// `if cond: ... elif cond: ... else: ...`.
    If {
        branches: Vec<IfBranch>,
        else_body: Option<Block>,
        span: Span,
    },

    /// `match scrutinee { pat => body, ... }`.
    Match {
        scrutinee: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },

    /// `loop { body }`.
    Loop {
        body: Block,
        span: Span,
    },

    /// `for pat in iterable { body }`.
    For {
        pattern: Pattern,
        iterable: Expr,
        body: Block,
        span: Span,
    },

    /// `parallel { body } [wait]`.
    Parallel {
        body: Block,
        has_wait: bool,
        span: Span,
    },

    /// `await expr`.
    Await {
        expr: Expr,
        span: Span,
    },

    /// `when { conditions } run { body }`.
    When {
        conditions: Vec<Expr>,
        body: Block,
        span: Span,
    },

    /// `return [value]`.
    Return {
        value: Option<Expr>,
        span: Span,
    },

    /// `break [label] [value]`.
    Break {
        label: Option<String>,
        value: Option<Expr>,
        span: Span,
    },

    /// `continue [label]`.
    Continue {
        label: Option<String>,
        span: Span,
    },

    /// `attempt { body } recover { ... } [recover { ... }]`.
    Attempt {
        body: Block,
        recovery_blocks: Vec<Vec<RecoveryAction>>,
        span: Span,
    },

    /// `swarm N Agent.task(args)`.
    Swarm {
        count: Expr,
        target: Expr,
        span: Span,
    },

    /// Pipeline as a statement: `expr |> op |> op`.
    Pipeline {
        stages: Vec<PipelineStage>,
        span: Span,
    },

    /// An error statement (recovery placeholder).
    Error { span: Span },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Const { span, .. } => *span,
            Stmt::Assign { span, .. } => *span,
            Stmt::ExprStmt { span, .. } => *span,
            Stmt::If { span, .. } => *span,
            Stmt::Match { span, .. } => *span,
            Stmt::Loop { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::Parallel { span, .. } => *span,
            Stmt::Await { span, .. } => *span,
            Stmt::When { span, .. } => *span,
            Stmt::Return { span, .. } => *span,
            Stmt::Break { span, .. } => *span,
            Stmt::Continue { span, .. } => *span,
            Stmt::Attempt { span, .. } => *span,
            Stmt::Swarm { span, .. } => *span,
            Stmt::Pipeline { span, .. } => *span,
            Stmt::Error { span } => *span,
        }
    }
}
