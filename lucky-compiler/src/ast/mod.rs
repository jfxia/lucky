pub mod span;
pub mod literal;
pub mod expr;
pub mod stmt;
pub mod pattern;
pub mod types;

use span::Span;
use stmt::Block;
use types::TypedIdent;

pub use expr::*;
pub use literal::Literal;
pub use pattern::Pattern;
pub use stmt::*;
pub use types::*;

/// The root AST node for a Lucky source file (module).
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub project: Option<ProjectDecl>,
    pub items: Vec<ModuleItem>,
    pub span: Span,
}

/// `project Name`.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectDecl {
    pub name: String,
    pub span: Span,
}

/// Top-level items in a module.
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleItem {
    Import(ImportDecl),
    Agent(AgentDecl),
    Task(TaskDecl),
    Workflow(WorkflowDecl),
    Goal(GoalDecl),
    Memory(MemoryDecl),
    Tool(ToolDecl),
    Model(ModelDecl),
    Prompt(PromptDecl),
    Policy(PolicyDecl),
    Type(TypeDecl),
    Context(ContextDecl),
    Permission(PermissionDecl),
    Approval(ApprovalDecl),
    Error { span: Span },
}

impl ModuleItem {
    pub fn span(&self) -> Span {
        match self {
            ModuleItem::Import(d) => d.span,
            ModuleItem::Agent(d) => d.span,
            ModuleItem::Task(d) => d.span,
            ModuleItem::Workflow(d) => d.span,
            ModuleItem::Goal(d) => d.span,
            ModuleItem::Memory(d) => d.span,
            ModuleItem::Tool(d) => d.span,
            ModuleItem::Model(d) => d.span,
            ModuleItem::Prompt(d) => d.span,
            ModuleItem::Policy(d) => d.span,
            ModuleItem::Type(d) => d.span,
            ModuleItem::Context(d) => d.span,
            ModuleItem::Permission(d) => d.span,
            ModuleItem::Approval(d) => d.span,
            ModuleItem::Error { span } => *span,
        }
    }
}

// --- Import ---

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub path: QualifiedName,
    pub select: ImportSelect,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportSelect {
    All,
    Named(Vec<String>),
    Nothing,  // import the module itself
}

// --- Agent ---

#[derive(Debug, Clone, PartialEq)]
pub struct AgentDecl {
    pub name: String,
    pub model: Option<QualifiedName>,
    pub memory: Option<QualifiedName>,
    pub tools: Vec<QualifiedName>,
    pub permissions: Option<PermissionDecl>,
    pub policy: Option<QualifiedName>,
    pub prompt: Option<QualifiedName>,
    pub tasks: Vec<TaskDecl>,
    pub span: Span,
}

// --- Task ---

#[derive(Debug, Clone, PartialEq)]
pub struct TaskDecl {
    pub name: String,
    pub is_stateful: bool,
    pub type_params: Vec<String>,
    pub inputs: Vec<TypedIdent>,
    pub outputs: Vec<TypedIdent>,
    pub context: Vec<TypedIdent>,
    pub policy: Option<QualifiedName>,
    pub steps: Option<Block>,
    pub rollback: Option<Block>,
    pub span: Span,
}

// --- Workflow ---

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowDecl {
    pub name: String,
    pub context: Vec<TypedIdent>,
    pub body: Block,
    pub span: Span,
}

// --- Goal ---

#[derive(Debug, Clone, PartialEq)]
pub struct GoalDecl {
    pub name: String,
    pub success_criteria: Vec<String>,
    pub workflows: Vec<String>,
    pub span: Span,
}

// --- Memory ---

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryDecl {
    pub name: String,
    pub scope: Option<String>,
    pub backend: Option<String>,
    pub config: Vec<(String, Expr)>,
    pub span: Span,
}

// --- Tool ---

#[derive(Debug, Clone, PartialEq)]
pub struct ToolDecl {
    pub name: String,
    pub config: Vec<(String, Expr)>,
    pub methods: Vec<ToolMethod>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolMethod {
    pub name: String,
    pub params: Vec<TypedIdent>,
    pub return_type: Option<Box<TypeExpr>>,
    pub span: Span,
}

// --- Model ---

#[derive(Debug, Clone, PartialEq)]
pub struct ModelDecl {
    pub name: String,
    pub config: Vec<(String, Expr)>,
    pub span: Span,
}

// --- Prompt ---

#[derive(Debug, Clone, PartialEq)]
pub struct PromptDecl {
    pub name: String,
    pub sections: Vec<PromptSection>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptSection {
    Role { text: String, span: Span },
    Rules { items: Vec<String>, span: Span },
    Context { text: String, span: Span },
    Examples { pairs: Vec<(String, String)>, span: Span },
    Format { text: String, span: Span },
}

// --- Policy ---

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyDecl {
    pub name: String,
    pub entries: Vec<PolicyEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyEntry {
    Retry { count: u32, backoff: Option<String>, max_delay: Option<Expr>, span: Span },
    Timeout { duration: Expr, span: Span },
    Checkpoint { trigger: CheckpointTrigger, span: Span },
    Cache { ttl: Option<Expr>, span: Span },
    Sandbox { enabled: bool, span: Span },
    Model { name: String, span: Span },
    CostLimit { amount: Expr, span: Span },
    Priority { level: String, span: Span },
    Other { key: String, value: String, span: Span },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckpointTrigger {
    AfterEachTask,
    AfterEachWorkflow,
    Interval(Box<Expr>),
    BeforeRetry,
}

// --- Type ---

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub typ: Box<TypeExpr>,
    pub span: Span,
}

// --- Context ---

#[derive(Debug, Clone, PartialEq)]
pub struct ContextDecl {
    pub entries: Vec<TypedIdent>,
    pub span: Span,
}

// --- Permission ---

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionDecl {
    pub allow: Vec<PermissionEntry>,
    pub deny: Vec<PermissionEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionEntry {
    pub path: Vec<String>,
    pub span: Span,
}

// --- Approval ---

#[derive(Debug, Clone, PartialEq)]
pub struct ApprovalDecl {
    pub gates: Vec<ApprovalGate>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApprovalGate {
    pub operation: String,
    pub timeout: Option<Expr>,
    pub escalation: Option<String>,
    pub span: Span,
}
