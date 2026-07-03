use std::collections::HashMap;
use crate::runtime::executor::ExecutionEngine;
use crate::runtime::{NodeStatus, RunStatus, RuntimeValue};
use crate::hir::{HirGraph, HirNode};
use super::Breakpoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    None,
    StepIn,
    StepOver { depth: usize },
    StepOut { depth: usize },
}

pub struct DebugExecutor {
    pub engine: ExecutionEngine,
    pub source_files: HashMap<String, String>,
    pub current_node_id: Option<usize>,
    pub step_mode: StepMode,
    pub paused: bool,
    loaded_graph: bool,
}

impl DebugExecutor {
    pub fn new() -> Self {
        Self {
            engine: ExecutionEngine::new(),
            source_files: HashMap::new(),
            current_node_id: None,
            step_mode: StepMode::None,
            paused: false,
            loaded_graph: false,
        }
    }

    pub fn load_graph(&mut self, graph: &HirGraph) {
        self.engine.load(graph);
        self.loaded_graph = true;
        self.current_node_id = None;
        self.paused = false;
        self.step_mode = StepMode::None;
    }

    pub fn load_graph_with_source(
        &mut self,
        graph: &HirGraph,
        file_path: String,
        source: String,
    ) {
        self.source_files.insert(file_path, source);
        self.load_graph(graph);
    }

    pub fn step_execution(&mut self, breakpoints: &[Breakpoint]) -> bool {
        if !self.loaded_graph {
            return false;
        }

        if self.engine.status != RunStatus::Running
            && self.engine.status != RunStatus::Created
            && self.engine.status != RunStatus::Paused
        {
            return false;
        }

        if self.paused {
            return false;
        }

        self.engine.status = RunStatus::Running;

        let has_more = self.engine.step();

        if !has_more {
            return false;
        }

        if let Some(node_id) = self.find_last_executed_node() {
            self.current_node_id = Some(node_id);

            let hit = self.check_breakpoint(node_id, breakpoints);

            match self.step_mode {
                StepMode::StepIn => {
                    self.paused = true;
                    self.step_mode = StepMode::None;
                }
                StepMode::StepOver { depth } => {
                    let current_depth = self
                        .engine
                        .scheduler
                        .nodes
                        .get(&node_id)
                        .map(|s| s.depth)
                        .unwrap_or(0);
                    if current_depth <= depth {
                        self.paused = true;
                        self.step_mode = StepMode::None;
                    }
                }
                StepMode::StepOut { depth } => {
                    let current_depth = self
                        .engine
                        .scheduler
                        .nodes
                        .get(&node_id)
                        .map(|s| s.depth)
                        .unwrap_or(0);
                    if current_depth < depth {
                        self.paused = true;
                        self.step_mode = StepMode::None;
                    }
                }
                StepMode::None => {
                    if hit {
                        self.paused = true;
                    }
                }
            }
        }

        !self.paused && has_more
    }

    fn find_last_executed_node(&self) -> Option<usize> {
        self.engine.scheduler.nodes.iter()
            .filter(|(_, state)| {
                state.status == NodeStatus::Completed
                    || state.status == NodeStatus::Running
            })
            .max_by_key(|(_, state)| state.completed_at_ms)
            .map(|(id, _)| *id)
    }

    fn check_breakpoint(&self, node_id: usize, breakpoints: &[Breakpoint]) -> bool {
        let node = match self.engine.scheduler.hir_nodes.get(&node_id) {
            Some(n) => n,
            None => return false,
        };

        let span = node.span();
        let file_name = format!("file_{}", span.file_id.0);

        let source = match self.source_files.get(&file_name) {
            Some(s) => s,
            None => return false,
        };

        let line = source[..span.start.min(source.len())]
            .chars()
            .filter(|&c| c == '\n')
            .count()
            + 1;

        for bp in breakpoints {
            if bp.file == file_name && bp.line == line {
                if let Some(ref cond) = bp.condition {
                    return self.evaluate_condition(cond);
                }
                return true;
            }
        }

        false
    }

    fn evaluate_condition(&self, _condition: &str) -> bool {
        true
    }

    pub fn get_all_variables(&self) -> HashMap<String, RuntimeValue> {
        self.engine.context.snapshot()
    }

    pub fn get_variable(&self, name: &str) -> Option<&RuntimeValue> {
        self.engine.context.get(name)
    }

    pub fn node_source_location(&self, node_id: usize) -> Option<(String, usize)> {
        let node = self.engine.scheduler.hir_nodes.get(&node_id)?;
        let span = node.span();
        let file_name = format!("file_{}", span.file_id.0);
        let source = self.source_files.get(&file_name)?;

        let line = source[..span.start.min(source.len())]
            .chars()
            .filter(|&c| c == '\n')
            .count()
            + 1;

        Some((file_name, line))
    }

    pub fn current_node_info(&self) -> Option<(usize, String, String, usize)> {
        let node_id = self.current_node_id?;
        let node = self.engine.scheduler.hir_nodes.get(&node_id)?;

        let name = match node {
            HirNode::Goal { goal_ref, .. } => goal_ref.clone(),
            HirNode::Workflow { workflow_ref, .. } => workflow_ref.clone(),
            HirNode::Task { task_ref, .. } => task_ref.clone(),
            HirNode::AgentInvoke { agent_ref, task_ref, .. } =>
                format!("{}.{}", agent_ref, task_ref),
            HirNode::ToolCall { tool_ref, method, .. } =>
                format!("{}.{}", tool_ref, method.as_deref().unwrap_or("?")),
            HirNode::LlmCall { model_ref, .. } => format!("LLM:{}", model_ref),
            HirNode::Decision { condition, .. } => format!("if {}", condition),
            HirNode::Let { name, .. } => format!("let {}", name),
            HirNode::Return { .. } => "return".into(),
            _ => format!("{:?}", std::mem::discriminant(node)),
        };

        let span = node.span();
        let file_name = format!("file_{}", span.file_id.0);
        let line = self.source_files.get(&file_name)
            .map(|src| {
                src[..span.start.min(src.len())]
                    .chars()
                    .filter(|&c| c == '\n')
                    .count() + 1
            })
            .unwrap_or(1);

        Some((node_id, name, file_name, line))
    }

    pub fn is_running(&self) -> bool {
        self.engine.status == RunStatus::Running
            || self.engine.status == RunStatus::Created
    }

    pub fn is_completed(&self) -> bool {
        matches!(
            self.engine.status,
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled
        )
    }
}
