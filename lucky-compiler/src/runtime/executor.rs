//! Execution Engine — state machine and dispatch for node execution.

use std::collections::HashMap;
use super::{RuntimeValue, NodeStatus, RunStatus};
use super::context::ContextManager;
use super::memory::MemoryManager;
use super::permissions::PermissionEnforcer;
use super::tools::ToolRegistry;
use super::scheduler::Scheduler;
use super::checkpoint::{CheckpointManager, DagProgress};
use super::audit::AuditLogger;
use crate::hir::HirGraph;
use crate::backends::BackendRouter;
use crate::diagnostics::DiagnosticBag;

/// The execution engine wraps the scheduler and provides a higher-level
/// execution interface with lifecycle management.
pub struct ExecutionEngine {
    pub scheduler: Scheduler,
    pub context: ContextManager,
    pub memory: MemoryManager,
    pub permissions: PermissionEnforcer,
    pub tools: ToolRegistry,
    pub diagnostics: DiagnosticBag,
    pub status: RunStatus,
    pub cost_usd: f64,
    pub tokens_used: u64,
    pub max_steps: usize,
    pub budget: Option<f64>,
    pub auto_approve: bool,
    pub approved_gates: Vec<String>,
    pub stream_output: bool,
    pub checkpoint_manager: CheckpointManager,
    pub audit_logger: AuditLogger,
    step_count: usize,
    pub events: Vec<ExecutionEvent>,
}

#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    NodeStarted { node_id: usize, label: String, kind: String },
    NodeCompleted { node_id: usize, label: String, output: Option<RuntimeValue> },
    NodeFailed { node_id: usize, label: String, error: String },
    NodeRetrying { node_id: usize, attempt: u32 },
    CheckpointCreated { id: String },
    ApprovalRequested { node_id: usize, message: String },
    CostUpdated { total_usd: f64 },
    CostBudgetExceeded { limit: f64, current: f64 },
    ExecutionCompleted { result: String },
    ExecutionFailed { error: String },
}

impl ExecutionEngine {
    pub fn new() -> Self {
        let mut tools = ToolRegistry::new();
        super::tools::register_builtin_tools(&mut tools);

        Self {
            scheduler: Scheduler::new(),
            context: ContextManager::new(),
            memory: MemoryManager::new(),
            permissions: PermissionEnforcer::new(),
            tools,
            diagnostics: DiagnosticBag::new(),
            status: RunStatus::Created,
            cost_usd: 0.0,
            tokens_used: 0,
            max_steps: 10000,
            budget: None,
            auto_approve: false,
            approved_gates: Vec::new(),
            stream_output: false,
            checkpoint_manager: CheckpointManager::new(),
            audit_logger: AuditLogger::new(),
            step_count: 0,
            events: Vec::new(),
        }
    }

    /// Load a HIR graph for execution.
    pub fn load(&mut self, graph: &HirGraph) {
        self.scheduler.load_graph(graph);
        self.status = RunStatus::Running;
        self.step_count = 0;
        self.events.clear();
    }

    pub fn set_backend_router(&mut self, router: BackendRouter) {
        self.scheduler.backend_router = Some(router);
    }

    /// Execute one step: dispatch a node and process its result.
    pub fn step(&mut self) -> bool {
        if self.status != RunStatus::Running && self.status != RunStatus::Created {
            return false;
        }
        if self.step_count >= self.max_steps {
            self.status = RunStatus::Failed;
            self.events.push(ExecutionEvent::ExecutionFailed {
                error: "Max steps exceeded".to_string(),
            });
            return false;
        }

        // Record node_started events for nodes about to be dispatched
        let ready_ids: Vec<usize> = self.scheduler.ready_queue.iter().copied().collect();
        for &nid in &ready_ids {
            let label = self.scheduler.hir_nodes.get(&nid)
                .map(|n| node_label(n))
                .unwrap_or_else(|| "?".to_string());
            let kind = self.scheduler.hir_nodes.get(&nid)
                .map(|n| format!("{:?}", std::mem::discriminant(n)))
                .unwrap_or_else(|| "?".to_string());
            self.events.push(ExecutionEvent::NodeStarted {
                node_id: nid, label, kind,
            });
        }

        let had_work = self.scheduler.step(
            &mut self.context,
            &mut self.memory,
            &mut self.permissions,
            &mut self.tools,
            &mut self.diagnostics,
            &mut self.cost_usd,
            &mut self.tokens_used,
        );

        self.step_count += 1;

        if !had_work {
            if self.scheduler.all_completed() {
                self.status = RunStatus::Completed;
                self.events.push(ExecutionEvent::ExecutionCompleted {
                    result: "success".to_string(),
                });
            }
            return false;
        }

        if self.scheduler.has_terminal_failure() {
            self.status = RunStatus::Failed;
            self.events.push(ExecutionEvent::ExecutionFailed {
                error: "Terminal failure".to_string(),
            });
            return false;
        }

        true
    }

    /// Run to completion (up to max_steps).
    pub fn run(&mut self, graph: &HirGraph) -> &[ExecutionEvent] {
        self.load(graph);
        while self.step() {}
        &self.events
    }

    /// Run synchronously and return final outputs.
    pub fn run_sync(&mut self, graph: &HirGraph) -> (RunStatus, HashMap<usize, RuntimeValue>) {
        self.run(graph);
        let outputs = self.scheduler.outputs();
        (self.status, outputs)
    }

    /// Get a summary of the current execution state.
    pub fn summary(&self) -> ExecutionSummary {
        let total = self.scheduler.nodes.len();
        let completed = self.scheduler.completed_nodes.len();
        let failed = self.scheduler.failed_nodes.len();
        let active = self.scheduler.active_nodes.len();
        let ready = self.scheduler.ready_queue.len();
        let pending = total.saturating_sub(completed + failed + active + ready);

        ExecutionSummary {
            status: self.status,
            total_nodes: total,
            completed_nodes: completed,
            failed_nodes: failed,
            active_nodes: active,
            ready_nodes: ready,
            pending_nodes: pending,
            cost_usd: self.cost_usd,
            tokens_used: self.tokens_used,
            step_count: self.step_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionSummary {
    pub status: RunStatus,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub failed_nodes: usize,
    pub active_nodes: usize,
    pub ready_nodes: usize,
    pub pending_nodes: usize,
    pub cost_usd: f64,
    pub tokens_used: u64,
    pub step_count: usize,
}

impl std::fmt::Display for ExecutionSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "Execution {:?}: {}/{} completed, {} failed, {} active, {} ready, {} pending | ${:.4} | {} tokens | {} steps",
            self.status,
            self.completed_nodes, self.total_nodes,
            self.failed_nodes,
            self.active_nodes,
            self.ready_nodes,
            self.pending_nodes,
            self.cost_usd,
            self.tokens_used,
            self.step_count,
        )
    }
}

fn node_label(node: &crate::hir::HirNode) -> String {
    match node {
        crate::hir::HirNode::Goal { goal_ref, .. } => format!("Goal:{}", goal_ref),
        crate::hir::HirNode::Workflow { workflow_ref, .. } => format!("Workflow:{}", workflow_ref),
        crate::hir::HirNode::Task { task_ref, .. } => format!("Task:{}", task_ref),
        crate::hir::HirNode::AgentInvoke { agent_ref, task_ref, .. } => format!("{}.{}", agent_ref, task_ref),
        crate::hir::HirNode::ToolCall { tool_ref, method, .. } =>
            format!("{}.{}", tool_ref, method.as_deref().unwrap_or("?")),
        crate::hir::HirNode::LlmCall { model_ref, .. } => format!("LLM:{}", model_ref),
        crate::hir::HirNode::Decision { condition, .. } => format!("if {}", condition),
        crate::hir::HirNode::Match { .. } => "match".into(),
        crate::hir::HirNode::Parallel { .. } => "parallel".into(),
        crate::hir::HirNode::Join { .. } => "join".into(),
        crate::hir::HirNode::Loop { .. } => "loop".into(),
        crate::hir::HirNode::ForEach { binding, .. } => format!("for {}", binding),
        crate::hir::HirNode::Pipeline { .. } => "pipeline".into(),
        crate::hir::HirNode::Attempt { .. } => "attempt".into(),
        crate::hir::HirNode::Approval { operation, .. } => format!("approval:{}", operation),
        crate::hir::HirNode::Let { name, .. } => format!("let {}", name),
        crate::hir::HirNode::Return { .. } => "return".into(),
        crate::hir::HirNode::Noop { .. } => "noop".into(),
    }
}
