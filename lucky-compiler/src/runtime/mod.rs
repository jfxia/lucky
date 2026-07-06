//! Lucky Runtime Engine — executes Lucky IR programs.
//!
//! The runtime consumes HIR graphs and executes them through a
//! priority-based DAG scheduler with context propagation, permission
//! enforcement, agent memory, and tool execution.

pub mod scheduler;
pub mod context;
pub mod memory;
pub mod permissions;
pub mod tools;
pub mod executor;
pub mod checkpoint;
pub mod audit;

use std::collections::HashMap;
use crate::diagnostics::DiagnosticBag;

/// Opaque runtime value — the data type flowing through the execution graph.
#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<RuntimeValue>),
    Map(HashMap<String, RuntimeValue>),
    Bytes(Vec<u8>),
    Artifact { id: String, kind: String, uri: String },
    Error { code: i32, message: String, recoverable: bool },
    Probabilistic { value: Box<RuntimeValue>, confidence: f64 },
}

impl RuntimeValue {
    pub fn type_name(&self) -> &str {
        match self {
            RuntimeValue::Null => "null",
            RuntimeValue::Bool(_) => "Bool",
            RuntimeValue::Int(_) => "Int",
            RuntimeValue::Float(_) => "Float",
            RuntimeValue::String(_) => "String",
            RuntimeValue::List(_) => "List",
            RuntimeValue::Map(_) => "Map",
            RuntimeValue::Bytes(_) => "Bytes",
            RuntimeValue::Artifact { .. } => "Artifact",
            RuntimeValue::Error { .. } => "Error",
            RuntimeValue::Probabilistic { .. } => "Probabilistic",
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self { RuntimeValue::Bool(b) => Some(*b), _ => None }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self { RuntimeValue::Int(i) => Some(*i), _ => None }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self { RuntimeValue::String(s) => Some(s), _ => None }
    }
}

impl std::fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeValue::Null => write!(f, "null"),
            RuntimeValue::Bool(b) => write!(f, "{}", b),
            RuntimeValue::Int(i) => write!(f, "{}", i),
            RuntimeValue::Float(n) => write!(f, "{}", n),
            RuntimeValue::String(s) => write!(f, "\"{}\"", s),
            RuntimeValue::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            RuntimeValue::Map(entries) => {
                write!(f, "{{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            RuntimeValue::Bytes(b) => write!(f, "<{} bytes>", b.len()),
            RuntimeValue::Artifact { id, kind, .. } => write!(f, "Artifact({}: {})", id, kind),
            RuntimeValue::Error { message, .. } => write!(f, "Error({})", message),
            RuntimeValue::Probabilistic { value, confidence } =>
                write!(f, "~{:.0}% {}", confidence * 100.0, value),
        }
    }
}

/// Execution status for a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

/// Execution status for a workflow run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Created,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// The runtime engine ties all components together.
pub struct Runtime {
    pub scheduler: scheduler::Scheduler,
    pub context_manager: context::ContextManager,
    pub memory_manager: memory::MemoryManager,
    pub permission_enforcer: permissions::PermissionEnforcer,
    pub tool_registry: tools::ToolRegistry,
    pub diagnostics: DiagnosticBag,
    pub status: RunStatus,
    pub cost_usd: f64,
    pub tokens_used: u64,
}

impl Runtime {
    pub fn new() -> Self {
        let mut rt = Self {
            scheduler: scheduler::Scheduler::new(),
            context_manager: context::ContextManager::new(),
            memory_manager: memory::MemoryManager::new(),
            permission_enforcer: permissions::PermissionEnforcer::new(),
            tool_registry: tools::ToolRegistry::new(),
            diagnostics: DiagnosticBag::new(),
            status: RunStatus::Created,
            cost_usd: 0.0,
            tokens_used: 0,
        };
        // Register built-in tools
        tools::register_builtin_tools(&mut rt.tool_registry);
        rt
    }

    /// Load a HIR graph into the scheduler for execution.
    pub fn load_graph(&mut self, graph: &crate::hir::HirGraph) {
        self.scheduler.load_graph(graph);
        self.status = RunStatus::Running;
    }

    /// Execute one scheduling step: pick a ready node, dispatch it, and update state.
    /// Returns true if there is more work to do.
    pub fn step(&mut self) -> bool {
        self.scheduler.step(
            &mut self.context_manager,
            &mut self.memory_manager,
            &mut self.permission_enforcer,
            &mut self.tool_registry,
            &mut self.diagnostics,
            &mut self.cost_usd,
            &mut self.tokens_used,
        );

        // Check if all done
        if self.scheduler.all_completed() {
            self.status = RunStatus::Completed;
            return false;
        }
        if self.scheduler.has_terminal_failure() {
            self.status = RunStatus::Failed;
            return false;
        }
        true
    }

    /// Run the full execution to completion.
    pub fn run(&mut self, graph: &crate::hir::HirGraph) {
        self.load_graph(graph);
        while self.step() {}
    }
}
