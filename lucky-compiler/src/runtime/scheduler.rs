//! Task Scheduler — priority-based topological traversal of the execution DAG.

use std::collections::{HashMap, HashSet, VecDeque};
use crate::hir::{HirGraph, HirNode, HirEdgeKind};
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
    /// Number of unsatisfied incoming dependencies.
    pub pending_inputs: usize,
    /// Depth in the DAG from the entry point.
    pub depth: usize,
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
        // Pick the highest-priority ready node
        let node_id = match self.ready_queue.pop_front() {
            Some(id) => id,
            None => return false,
        };

        self.active_nodes.insert(node_id);
        if let Some(state) = self.nodes.get_mut(&node_id) {
            state.status = NodeStatus::Running;
            state.started_at_ms = current_time_ms();
        }

        // Execute the node
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
                // Signal successors
                self.signal_successors(node_id);
            }
            Err(err) => {
                if let Some(state) = self.nodes.get_mut(&node_id) {
                    if state.retry_count < state.max_retries
                        && err.contains("transient") || err.contains("retry") {
                        state.retry_count += 1;
                        state.status = NodeStatus::Ready;
                        state.pending_inputs = 0;
                        // Re-queue with lower priority
                        self.ready_queue.push_back(node_id);
                    } else {
                        state.status = NodeStatus::Failed;
                        state.output = Some(RuntimeValue::Error {
                            code: 1,
                            message: err.clone(),
                            recoverable: false,
                        });
                        self.failed_nodes.insert(node_id);
                        // Don't signal successors on failure unless recovery is configured
                        self.terminal_failure = true;
                    }
                }
            }
        }

        // Refresh ready queue after state changes
        self.refresh_ready_queue();

        true
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
                // Stub: in production this would call the LLM backend
                let prompt = prompt_ref.as_deref().unwrap_or("?");
                Ok(RuntimeValue::Probabilistic {
                    value: Box::new(RuntimeValue::String(
                        format!("[LLM response from {} for prompt '{}']", model_ref, prompt)
                    )),
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
                // In a full implementation, this would suspend for human approval.
                // For now, auto-approve.
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
