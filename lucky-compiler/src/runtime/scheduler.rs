//! Task Scheduler — priority-based topological traversal of the execution DAG.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use crate::hir::{HirGraph, HirNode, HirEdgeKind};
use crate::backends::{BackendRouter, CompleteOptions};
use super::{NodeStatus, RuntimeValue};
use super::context::ContextManager;
use super::memory::MemoryManager;
use super::permissions::PermissionEnforcer;
use super::tools::ToolRegistry;
use crate::diagnostics::DiagnosticBag;

/// Priority levels for scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

impl Priority {
    pub fn from_depth(depth: usize, max_depth: usize) -> Self {
        if max_depth == 0 { return Priority::Normal; }
        let ratio = depth as f64 / max_depth as f64;
        if ratio > 0.8 { Priority::High }
        else if ratio > 0.5 { Priority::Normal }
        else if ratio > 0.2 { Priority::Low }
        else { Priority::Background }
    }
}

/// State tracked per node during execution.
#[derive(Debug, Clone)]
pub struct NodeState {
    pub node_id: usize,
    pub status: NodeStatus,
    pub output: Option<RuntimeValue>,
    pub agent_name: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
    pub pending_inputs: usize,
    pub depth: usize,
    pub failure_timestamps: Vec<u64>,
    pub backoff_ms: u64,
}

impl NodeState {
    pub fn new(node_id: usize, depth: usize) -> Self {
        Self {
            node_id,
            status: NodeStatus::Pending,
            output: None,
            agent_name: None,
            retry_count: 0,
            max_retries: 3,
            started_at_ms: 0,
            completed_at_ms: 0,
            pending_inputs: 0,
            depth,
            failure_timestamps: Vec::new(),
            backoff_ms: 0,
        }
    }
}

/// The DAG-based task scheduler.
pub struct Scheduler {
    /// All nodes in the execution graph, indexed by NodeId.
    pub nodes: HashMap<usize, NodeState>,
    /// The original HIR nodes.
    pub hir_nodes: HashMap<usize, HirNode>,
    /// Incoming edges: node → list of predecessors.
    pub in_edges: HashMap<usize, Vec<(usize, HirEdgeKind)>>,
    /// Outgoing edges: node → list of successors.
    pub out_edges: HashMap<usize, Vec<(usize, HirEdgeKind)>>,
    /// Nodes that are ready for execution (all dependencies satisfied).
    pub ready_queue: VecDeque<usize>,
    /// Nodes currently executing.
    pub active_nodes: HashSet<usize>,
    /// Nodes that have completed.
    pub completed_nodes: HashSet<usize>,
    /// Nodes that have failed.
    pub failed_nodes: HashSet<usize>,
    /// Entry points into the graph.
    entry_points: Vec<usize>,
    /// Max depth in the DAG.
    max_depth: usize,
    /// Counter for generating unique IDs.
    next_id: usize,
    /// Whether there's been a terminal failure.
    terminal_failure: bool,
    /// LLM backend router for executing LlmCall nodes.
    pub backend_router: Option<BackendRouter>,
    /// Budget limit in USD (None = unlimited).
    pub budget: Option<f64>,
    /// Whether to auto-approve all approval gates.
    pub auto_approve: bool,
    /// Gates that should be auto-approved.
    pub approved_gates: Vec<String>,
    /// Callback for approval requests (returns true to approve).
    pub approval_callback: Option<Box<dyn Fn(&str) -> bool>>,
    /// Audit event callback.
    pub audit_callback: Option<Box<dyn Fn(&str, Option<usize>, Option<f64>, Option<u64>, Option<&str>)>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            hir_nodes: HashMap::new(),
            in_edges: HashMap::new(),
            out_edges: HashMap::new(),
            ready_queue: VecDeque::new(),
            active_nodes: HashSet::new(),
            completed_nodes: HashSet::new(),
            failed_nodes: HashSet::new(),
            entry_points: Vec::new(),
            max_depth: 0,
            next_id: 0,
            terminal_failure: false,
            backend_router: None,
            budget: None,
            auto_approve: false,
            approved_gates: Vec::new(),
            approval_callback: None,
            audit_callback: None,
        }
    }

    /// Load a HIR graph into the scheduler.
    pub fn load_graph(&mut self, graph: &HirGraph) {
        self.nodes.clear();
        self.hir_nodes.clear();
        self.in_edges.clear();
        self.out_edges.clear();
        self.ready_queue.clear();
        self.active_nodes.clear();
        self.completed_nodes.clear();
        self.failed_nodes.clear();
        self.entry_points.clear();
        self.terminal_failure = false;
        self.max_depth = 0;

        // First pass: register all nodes
        for (i, node) in graph.nodes.iter().enumerate() {
            self.hir_nodes.insert(i, node.clone());
            let state = NodeState::new(i, 0);
            self.nodes.insert(i, state);
            self.in_edges.entry(i).or_default();
            self.out_edges.entry(i).or_default();
        }

        // Second pass: build edge maps
        for edge in &graph.edges {
            self.in_edges.entry(edge.to).or_default()
                .push((edge.from, edge.kind.clone()));
            self.out_edges.entry(edge.from).or_default()
                .push((edge.to, edge.kind.clone()));
        }

        // Find entry points (nodes with no incoming control edges)
        for (id, _state) in &self.nodes {
            let has_control_in = self.in_edges.get(id)
                .map(|edges| edges.iter().any(|(_, k)| matches!(k, HirEdgeKind::Control)))
                .unwrap_or(false);

            if !has_control_in {
                self.entry_points.push(*id);
            }
        }

        // Compute depths via BFS from entry points
        self.compute_depths();

        // Initialize pending inputs count
        for (id, state) in self.nodes.iter_mut() {
            let control_deps: usize = self.in_edges.get(id)
                .map(|edges| edges.iter().filter(|(_, k)| matches!(k, HirEdgeKind::Control)).count())
                .unwrap_or(0);
            // Also count data dependencies
            let data_deps: usize = self.in_edges.get(id)
                .map(|edges| edges.iter().filter(|(_, k)| matches!(k, HirEdgeKind::Data)).count())
                .unwrap_or(0);
            state.pending_inputs = control_deps + data_deps;
        }

        // Enqueue entry points (nodes with pending_inputs == 0)
        self.refresh_ready_queue();
    }

    /// Compute the depth of each node in the DAG using BFS.
    fn compute_depths(&mut self) {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        for &entry in &self.entry_points {
            if let Some(state) = self.nodes.get_mut(&entry) {
                state.depth = 0;
                queue.push_back(entry);
                visited.insert(entry);
            }
        }

        while let Some(current) = queue.pop_front() {
            let current_depth = self.nodes.get(&current).map(|s| s.depth).unwrap_or(0);

            if let Some(outs) = self.out_edges.get(&current) {
                for &(succ, _) in outs {
                    if !visited.contains(&succ) {
                        if let Some(state) = self.nodes.get_mut(&succ) {
                            state.depth = state.depth.max(current_depth + 1);
                            self.max_depth = self.max_depth.max(state.depth);
                        }
                        visited.insert(succ);
                        queue.push_back(succ);
                    }
                }
            }
        }
    }

    /// Refresh the ready queue with nodes whose dependencies are all satisfied.
    fn refresh_ready_queue(&mut self) {
        let ready: Vec<usize> = self.nodes.iter()
            .filter(|(id, state)| {
                state.status == NodeStatus::Pending
                    && state.pending_inputs == 0
                    && !self.active_nodes.contains(id)
            })
            .map(|(id, _)| *id)
            .collect();

        for id in ready {
            if let Some(state) = self.nodes.get_mut(&id) {
                state.status = NodeStatus::Ready;
            }
            // Insert in priority order (by depth, deeper = higher priority)
            let depth = self.nodes.get(&id).map(|s| s.depth).unwrap_or(0);
            let priority = Priority::from_depth(depth, self.max_depth);

            // Insert sorted by priority then depth
            let pos = self.ready_queue.iter().position(|&existing| {
                let existing_depth = self.nodes.get(&existing).map(|s| s.depth).unwrap_or(0);
                let existing_prio = Priority::from_depth(existing_depth, self.max_depth);
                priority < existing_prio || (priority == existing_prio && depth > existing_depth)
            });

            match pos {
                Some(p) => self.ready_queue.insert(p, id),
                None => self.ready_queue.push_back(id),
            }
        }
    }

    /// Execute one scheduling step: dispatch one ready node.
    /// Returns true if work was done.
    pub fn step(
        &mut self,
        ctx: &mut ContextManager,
        memory: &mut MemoryManager,
        permissions: &mut PermissionEnforcer,
        tools: &mut ToolRegistry,
        diagnostics: &mut DiagnosticBag,
        cost_usd: &mut f64,
        tokens_used: &mut u64,
    ) -> bool {
        let node_id = match self.ready_queue.pop_front() {
            Some(id) => id,
            None => return false,
        };

        // Check budget for LLM nodes
        if let Some(budget) = self.budget {
            if let Some(hir) = self.hir_nodes.get(&node_id) {
                if matches!(hir, HirNode::LlmCall { .. }) {
                    let estimated = 0.005; // est. cost per LLM call
                    if *cost_usd + estimated > budget {
                        if let Some(ref cb) = self.audit_callback {
                            cb("cost_budget_exceeded", Some(node_id), Some(*cost_usd), Some(*tokens_used), None);
                        }
                        let err = format!(
                            "CostBudgetExceeded: estimate ${:.4} + current ${:.4} exceeds budget ${:.4}",
                            estimated, *cost_usd, budget
                        );
                        if let Some(state) = self.nodes.get_mut(&node_id) {
                            state.status = NodeStatus::Failed;
                            state.output = Some(RuntimeValue::Error {
                                code: 402, message: err.clone(), recoverable: false,
                            });
                            self.failed_nodes.insert(node_id);
                        }
                        self.terminal_failure = true;
                        return true;
                    }
                }
            }
        }

        self.active_nodes.insert(node_id);
        if let Some(state) = self.nodes.get_mut(&node_id) {
            state.status = NodeStatus::Running;
            state.started_at_ms = current_time_ms();
        }

        if let Some(ref cb) = self.audit_callback {
            let label = self.hir_nodes.get(&node_id)
                .map(|n| format!("{:?}", std::mem::discriminant(n)))
                .unwrap_or_else(|| "?".to_string());
            cb("node_started", Some(node_id), Some(*cost_usd), Some(*tokens_used), Some(&label));
        }

        let result = self.execute_node(node_id, ctx, memory, permissions, tools);

        self.active_nodes.remove(&node_id);

        match result {
            Ok(output) => {
                self.completed_nodes.insert(node_id);
                if let Some(state) = self.nodes.get_mut(&node_id) {
                    state.status = NodeStatus::Completed;
                    state.output = Some(output);
                    state.completed_at_ms = current_time_ms();
                }
                if let Some(ref cb) = self.audit_callback {
                    cb("node_completed", Some(node_id), Some(*cost_usd), Some(*tokens_used), None);
                }
                self.signal_successors(node_id);
            }
            Err(err) => {
                let (should_retry, delay_ms) = self.compute_retry(node_id, &err);
                if let Some(state) = self.nodes.get_mut(&node_id) {
                    if should_retry {
                        state.retry_count += 1;
                        state.backoff_ms = delay_ms;
                        let now = current_time_ms();
                        state.failure_timestamps.push(now);
                        state.status = NodeStatus::Pending;
                        state.pending_inputs = 0;
                        if let Some(ref cb) = self.audit_callback {
                            cb("node_retrying", Some(node_id), Some(*cost_usd), Some(*tokens_used),
                                Some(&format!("attempt {}/{} after {}ms", state.retry_count, state.max_retries, delay_ms)));
                        }
                        // Sleep for backoff delay (busy-wait for simplicity)
                        let start = Instant::now();
                        while start.elapsed() < Duration::from_millis(delay_ms) {
                            std::hint::spin_loop();
                        }
                        // Re-queue
                        if let Some(state) = self.nodes.get_mut(&node_id) {
                            state.status = NodeStatus::Ready;
                        }
                        self.ready_queue.push_back(node_id);
                    } else {
                        state.status = NodeStatus::Failed;
                        state.output = Some(RuntimeValue::Error {
                            code: 1,
                            message: err.clone(),
                            recoverable: false,
                        });
                        self.failed_nodes.insert(node_id);
                        if let Some(ref cb) = self.audit_callback {
                            cb("node_failed", Some(node_id), Some(*cost_usd), Some(*tokens_used),
                                Some(&err));
                        }
                        self.terminal_failure = true;
                    }
                }
            }
        }

        self.refresh_ready_queue();
        true
    }

    /// Compute whether to retry and the backoff delay.
    fn compute_retry(&self, node_id: usize, _err: &str) -> (bool, u64) {
        let state = match self.nodes.get(&node_id) {
            Some(s) => s,
            None => return (false, 0),
        };

        if state.retry_count >= state.max_retries { return (false, 0); }

        // Circuit breaker: fail 5+ times within 60s
        let now = current_time_ms();
        let window_ms: u64 = 60000;
        let recent_failures = state.failure_timestamps.iter()
            .filter(|&&t| now.saturating_sub(t) < window_ms)
            .count();
        if recent_failures >= 5 { return (false, 0); }

        let base: u64 = 1000;
        let max_delay: u64 = 600000; // 10 minutes
        let attempt = state.retry_count as u64;

        let delay = (base * (1u64 << attempt)).min(max_delay);

        // Jitter: multiply by (0.5 + random(0, 1))
        let jitter = (now % 1000) as f64 / 2000.0 + 0.5; // 0.5 to 1.0
        let delay_ms = (delay as f64 * jitter) as u64;

        (true, delay_ms)
    }

    /// Signal that a node's dependencies are now satisfied.
    fn signal_successors(&mut self, completed_id: usize) {
        let successors: Vec<usize> = self.out_edges.get(&completed_id)
            .map(|edges| edges.iter().map(|(id, _)| *id).collect())
            .unwrap_or_default();

        for succ_id in successors {
            if let Some(state) = self.nodes.get_mut(&succ_id) {
                if state.pending_inputs > 0 {
                    state.pending_inputs -= 1;
                }
            }
        }
    }

    /// Execute a single node, dispatching based on its kind.
    fn execute_node(
        &mut self,
        node_id: usize,
        ctx: &mut ContextManager,
        memory: &mut MemoryManager,
        permissions: &mut PermissionEnforcer,
        tools: &mut ToolRegistry,
    ) -> Result<RuntimeValue, String> {
        let node = match self.hir_nodes.get(&node_id) {
            Some(n) => n.clone(),
            None => return Err(format!("Node {} not found", node_id)),
        };

        let agent_name = self.nodes.get(&node_id)
            .and_then(|s| s.agent_name.clone());

        match &node {
            HirNode::Task { task_ref, inputs, outputs, .. } => {
                // Check permissions
                if !permissions.check(agent_name.as_deref(), &format!("task.{}", task_ref)) {
                    // Default allow for tasks
                }

                // Execute the task — for now, produce a simple result
                ctx.push_scope(format!("task:{}", task_ref));

                // Set inputs in context
                for (name, _type_str) in inputs {
                    ctx.set(name.clone(), RuntimeValue::String(format!("[input:{}]", name)));
                }

                let result = RuntimeValue::Map(ctx.snapshot());
                ctx.pop_scope();
                Ok(result)
            }

            HirNode::AgentInvoke { agent_ref, task_ref, .. } => {
                // Check permissions
                if !permissions.check(Some(agent_ref), &format!("agent.{}", task_ref)) {
                    return Err(format!("Permission denied: agent {}.{}", agent_ref, task_ref));
                }

                // Restore agent context
                ctx.restore_agent_context(agent_ref);
                ctx.push_scope(format!("agent:{}:{}", agent_ref, task_ref));

                // Simulate agent invocation
                let result = RuntimeValue::Map(ctx.snapshot());
                ctx.save_agent_context(agent_ref);
                ctx.pop_scope();
                Ok(result)
            }

            HirNode::ToolCall { tool_ref, method, inputs, .. } => {
                let method = method.as_deref().unwrap_or("?");
                // Check permissions
                let op = format!("{}.{}", tool_ref.to_lowercase(), method);
                if !permissions.check(agent_name.as_deref(), &op) {
                    return Err(format!("Permission denied: {}", op));
                }

                // Build args from inputs
                let mut args = HashMap::new();
                for (i, val) in inputs.iter().enumerate() {
                    args.insert(format!("arg{}", i), RuntimeValue::String(val.clone()));
                }
                // Also try to get named args from context
                for key in ctx.current.keys() {
                    if let Some(v) = ctx.get(&key) {
                        args.insert(key, v.clone());
                    }
                }

                tools.invoke(tool_ref, method, &args, ctx)
            }

            HirNode::LlmCall { model_ref, prompt_ref, .. } => {
                let prompt = prompt_ref.as_deref().unwrap_or("?");
                let options = CompleteOptions {
                    temperature: 0.7,
                    max_tokens: 4096,
                    stop_sequences: Vec::new(),
                    system_prompt: None,
                };

                let start_cost = 0.0;
                if let Some(ref cb) = self.audit_callback {
                    cb("llm_called", Some(node_id), Some(start_cost), Some(0), Some(model_ref));
                }

                let response = if let Some(ref router) = self.backend_router {
                    match router.route(model_ref) {
                        Some(backend) => {
                            match backend.complete(prompt, &options) {
                                Ok(text) => {
                                    let cost = backend.cost_per_1k_tokens();
                                    if let Some(ref cb) = self.audit_callback {
                                        cb("llm_completed", Some(node_id), Some(cost), Some(options.max_tokens as u64), Some(model_ref));
                                    }
                                    text
                                }
                                Err(e) => {
                                    if let Some(ref cb) = self.audit_callback {
                                        cb("llm_failed", Some(node_id), None, None, Some(&e));
                                    }
                                    return Err(format!(
                                        "LLM call failed (model={}, provider={}): {}",
                                        model_ref, backend.name(), e
                                    ));
                                }
                            }
                        }
                        None => {
                            let models = if let Some(ref r) = self.backend_router {
                                r.list_models().join(", ")
                            } else {
                                "(no backends configured)".to_string()
                            };
                            return Err(format!(
                                "No backend registered for model '{}'. Available models: {}",
                                model_ref, models
                            ));
                        }
                    }
                } else {
                    format!("[LLM response from {} for prompt '{}']", model_ref, prompt)
                };

                Ok(RuntimeValue::Probabilistic {
                    value: Box::new(RuntimeValue::String(response)),
                    confidence: 0.9,
                })
            }

            HirNode::Decision { condition, then_branch, else_branch, .. } => {
                // Evaluate condition from context
                let cond_val = ctx.get(condition)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Activate the chosen branch
                let branch_id = if cond_val { *then_branch } else { else_branch.unwrap_or(node_id) };

                // Mark the branch node as ready by clearing its dependencies
                if let Some(state) = self.nodes.get_mut(&branch_id) {
                    state.pending_inputs = 0;
                }

                Ok(RuntimeValue::Bool(cond_val))
            }

            HirNode::Parallel { branches, has_wait, .. } => {
                // Mark all branches as ready
                for &branch_id in branches {
                    if let Some(state) = self.nodes.get_mut(&branch_id) {
                        state.pending_inputs = 0;
                    }
                }
                if *has_wait {
                    // The join node will handle synchronization
                    Ok(RuntimeValue::Null)
                } else {
                    Ok(RuntimeValue::Null)
                }
            }

            HirNode::Join { .. } => {
                // Join node: all predecessors have completed
                Ok(RuntimeValue::Null)
            }

            HirNode::Loop { body, condition, .. } => {
                // Simple bounded loop (single iteration for now)
                ctx.push_scope("loop");
                // Execute body nodes
                for &body_id in body {
                    if let Some(state) = self.nodes.get_mut(&body_id) {
                        state.pending_inputs = 0;
                    }
                }
                ctx.pop_scope();
                Ok(RuntimeValue::Null)
            }

            HirNode::Let { name, value, .. } => {
                ctx.set(name.clone(), RuntimeValue::String(value.clone()));
                Ok(RuntimeValue::Null)
            }

            HirNode::Return { .. } => {
                Ok(RuntimeValue::Map(ctx.snapshot()))
            }

            HirNode::Goal { goal_ref, .. } => {
                ctx.push_scope(format!("goal:{}", goal_ref));
                Ok(RuntimeValue::Null)
            }

            HirNode::Workflow { workflow_ref, .. } => {
                ctx.push_scope(format!("workflow:{}", workflow_ref));
                Ok(RuntimeValue::Null)
            }

            HirNode::Approval { operation, .. } => {
                let op = operation.clone();
                if self.auto_approve || self.approved_gates.contains(&op) {
                    if let Some(ref cb) = self.audit_callback {
                        cb("approval_auto_approved", Some(node_id), None, None, Some(&op));
                    }
                    return Ok(RuntimeValue::Bool(true));
                }
                if let Some(ref cb) = self.approval_callback {
                    if cb(&op) {
                        if let Some(ref ac) = self.audit_callback {
                            ac("approval_granted", Some(node_id), None, None, Some(&op));
                        }
                        return Ok(RuntimeValue::Bool(true));
                    } else {
                        if let Some(ref ac) = self.audit_callback {
                            ac("approval_rejected", Some(node_id), None, None, Some(&op));
                        }
                        return Err(format!("Approval rejected for: {}", op));
                    }
                }
                // No callback set — default to approve
                Ok(RuntimeValue::Bool(true))
            }

            _ => {
                Ok(RuntimeValue::Null)
            }
        }
    }

    /// Check if all nodes have completed.
    pub fn all_completed(&self) -> bool {
        self.nodes.iter().all(|(id, state)| {
            matches!(state.status, NodeStatus::Completed | NodeStatus::Skipped)
                || self.failed_nodes.contains(id)
        })
    }

    /// Check if there's been a terminal failure.
    pub fn has_terminal_failure(&self) -> bool {
        self.terminal_failure
    }

    /// Get the output of a specific node.
    pub fn node_output(&self, node_id: usize) -> Option<&RuntimeValue> {
        self.nodes.get(&node_id).and_then(|s| s.output.as_ref())
    }

    /// Get all completed node outputs.
    pub fn outputs(&self) -> HashMap<usize, RuntimeValue> {
        self.completed_nodes.iter()
            .filter_map(|id| {
                self.node_output(*id).map(|v| (*id, v.clone()))
            })
            .collect()
    }
}

fn current_time_ms() -> u64 {
    static mut COUNTER: u64 = 0;
    unsafe {
        COUNTER = COUNTER.wrapping_add(1);
        COUNTER
    }
}
