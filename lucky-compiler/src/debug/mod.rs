pub mod dap;
pub mod executor;

use std::collections::HashMap;
use crate::runtime::RuntimeValue;

pub type Value = RuntimeValue;

#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub id: usize,
    pub file: String,
    pub line: usize,
    pub condition: Option<String>,
    pub verified: bool,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub id: usize,
    pub name: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub variables: HashMap<String, Value>,
    pub scope_id: usize,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub type_name: String,
    pub variables_reference: usize,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub id: usize,
    pub name: String,
    pub variables: HashMap<String, Value>,
    pub variables_reference: usize,
    pub expensive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugState {
    Running,
    Paused { reason: PauseReason },
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseReason {
    Breakpoint,
    Step,
    Pause,
    Exception,
    Entry,
}

#[derive(Debug, Clone)]
pub struct DebugSession {
    pub breakpoints: Vec<Breakpoint>,
    pub stack_frames: Vec<StackFrame>,
    pub scopes: Vec<Scope>,
    pub state: DebugState,
    next_breakpoint_id: usize,
    next_frame_id: usize,
    next_scope_id: usize,
}

impl DebugSession {
    pub fn new() -> Self {
        Self {
            breakpoints: Vec::new(),
            stack_frames: Vec::new(),
            scopes: Vec::new(),
            state: DebugState::Stopped,
            next_breakpoint_id: 1,
            next_frame_id: 0,
            next_scope_id: 0,
        }
    }
}

pub struct Debugger {
    pub session: DebugSession,
    pub executor: executor::DebugExecutor,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            session: DebugSession::new(),
            executor: executor::DebugExecutor::new(),
        }
    }

    pub fn add_breakpoint(&mut self, file: String, line: usize) -> usize {
        let id = self.session.next_breakpoint_id;
        self.session.next_breakpoint_id += 1;
        self.session.breakpoints.push(Breakpoint {
            id,
            file,
            line,
            condition: None,
            verified: true,
        });
        id
    }

    pub fn add_breakpoint_conditional(
        &mut self,
        file: String,
        line: usize,
        condition: String,
    ) -> usize {
        let id = self.session.next_breakpoint_id;
        self.session.next_breakpoint_id += 1;
        self.session.breakpoints.push(Breakpoint {
            id,
            file,
            line,
            condition: Some(condition),
            verified: true,
        });
        id
    }

    pub fn remove_breakpoint(&mut self, id: usize) -> bool {
        let len_before = self.session.breakpoints.len();
        self.session.breakpoints.retain(|bp| bp.id != id);
        self.session.breakpoints.len() < len_before
    }

    pub fn step_in(&mut self) {
        self.executor.step_mode = executor::StepMode::StepIn;
        self.executor.paused = false;
        self.session.state = DebugState::Running;
    }

    pub fn step_over(&mut self) {
        let depth = self.executor_step_depth();
        self.executor.step_mode = executor::StepMode::StepOver { depth };
        self.executor.paused = false;
        self.session.state = DebugState::Running;
    }

    pub fn step_out(&mut self) {
        let depth = self.executor_step_depth();
        self.executor.step_mode = executor::StepMode::StepOut { depth };
        self.executor.paused = false;
        self.session.state = DebugState::Running;
    }

    fn executor_step_depth(&self) -> usize {
        self.executor.engine.scheduler.nodes.values()
            .filter(|s| s.status == crate::runtime::NodeStatus::Running)
            .map(|s| s.depth)
            .next()
            .unwrap_or(0)
    }

    pub fn continue_exec(&mut self) {
        self.executor.step_mode = executor::StepMode::None;
        self.executor.paused = false;
        self.session.state = DebugState::Running;
    }

    pub fn pause(&mut self) {
        self.executor.paused = true;
        self.session.state = DebugState::Paused { reason: PauseReason::Pause };
    }

    pub fn execute_one_step(&mut self) -> bool {
        let has_more = self.executor.step_execution(&self.session.breakpoints);
        self.sync_state();
        has_more
    }

    pub fn sync_state(&mut self) {
        use crate::runtime::RunStatus;

        match self.executor.engine.status {
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled => {
                self.session.state = DebugState::Stopped;
            }
            _ => {
                if self.executor.paused {
                    let reason = match self.executor.step_mode {
                        executor::StepMode::None => PauseReason::Breakpoint,
                        _ => PauseReason::Step,
                    };
                    self.session.state = DebugState::Paused { reason };
                } else {
                    self.session.state = DebugState::Running;
                }
            }
        }

        self.session.stack_frames = self.build_stack_frames();
    }

    fn build_stack_frames(&self) -> Vec<StackFrame> {
        let mut frames = Vec::new();

        let current_node_id = match self.executor.current_node_id {
            Some(id) => id,
            None => return frames,
        };

        if let Some(node) = self.executor.engine.scheduler.hir_nodes.get(&current_node_id) {
            let span = node.span();
            let (file_name, line) = self.source_location(&span);
            let name = self.node_label(node);

            let vars = self.executor.engine.context.snapshot();

            let scope_id = self.session.next_scope_id;

            frames.push(StackFrame {
                id: self.session.next_frame_id,
                name,
                file: file_name,
                line,
                column: 0,
                variables: vars,
                scope_id,
            });
        }

        frames
    }

    fn node_label(&self, node: &crate::hir::HirNode) -> String {
        match node {
            crate::hir::HirNode::Goal { goal_ref, .. } => goal_ref.clone(),
            crate::hir::HirNode::Workflow { workflow_ref, .. } => workflow_ref.clone(),
            crate::hir::HirNode::Task { task_ref, .. } => task_ref.clone(),
            crate::hir::HirNode::AgentInvoke { agent_ref, task_ref, .. } =>
                format!("{}.{}", agent_ref, task_ref),
            crate::hir::HirNode::ToolCall { tool_ref, method, .. } =>
                format!("{}.{}", tool_ref, method.as_deref().unwrap_or("?")),
            crate::hir::HirNode::LlmCall { model_ref, .. } => format!("LLM:{}", model_ref),
            crate::hir::HirNode::Decision { condition, .. } => format!("if {}", condition),
            crate::hir::HirNode::Let { name, .. } => format!("let {}", name),
            crate::hir::HirNode::Return { .. } => "return".into(),
            _ => format!("{:?}", std::mem::discriminant(node)),
        }
    }

    fn source_location(&self, span: &crate::ast::span::Span) -> (String, usize) {
        let file_name = format!("file_{}", span.file_id.0);
        let source = self.executor.source_files.get(&file_name);

        match source {
            Some(src) => {
                let line = src[..span.start.min(src.len())]
                    .chars()
                    .filter(|&c| c == '\n')
                    .count() + 1;
                (file_name, line)
            }
            None => (file_name, 1),
        }
    }

    pub fn evaluate_expression(&self, expr: &str, _frame_id: usize) -> Option<Value> {
        let trimmed = expr.trim();

        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            return Some(Value::String(trimmed[1..trimmed.len() - 1].to_string()));
        }

        if let Ok(i) = trimmed.parse::<i64>() {
            return Some(Value::Int(i));
        }

        if let Ok(f) = trimmed.parse::<f64>() {
            return Some(Value::Float(f));
        }

        if trimmed == "true" {
            return Some(Value::Bool(true));
        }
        if trimmed == "false" {
            return Some(Value::Bool(false));
        }
        if trimmed == "null" {
            return Some(Value::Null);
        }

        let ctx = &self.executor.engine.context;
        if let Some(val) = ctx.get(trimmed) {
            return Some(val.clone());
        }

        None
    }

    pub fn get_variables(&self, frame_id: usize) -> Vec<Variable> {
        let mut vars = Vec::new();

        if let Some(frame) = self.session.stack_frames.get(frame_id) {
            for (name, value) in &frame.variables {
                vars.push(Variable {
                    name: name.clone(),
                    value: format!("{}", value),
                    type_name: value.type_name().to_string(),
                    variables_reference: 0,
                });
            }
        }

        for scope in &self.session.scopes {
            for (name, value) in &scope.variables {
                vars.push(Variable {
                    name: name.clone(),
                    value: format!("{}", value),
                    type_name: value.type_name().to_string(),
                    variables_reference: scope.variables_reference,
                });
            }
        }

        vars
    }

    pub fn get_stack_frames(&self) -> &[StackFrame] {
        &self.session.stack_frames
    }

    pub fn get_breakpoints_for_file(&self, file: &str) -> Vec<&Breakpoint> {
        self.session.breakpoints.iter()
            .filter(|bp| bp.file == file)
            .collect()
    }

    pub fn set_breakpoints_for_file(&mut self, file: String, lines: &[usize]) -> Vec<usize> {
        self.session.breakpoints.retain(|bp| bp.file != file);
        let mut ids = Vec::new();
        for &line in lines {
            let id = self.add_breakpoint(file.clone(), line);
            ids.push(id);
        }
        ids
    }

    pub fn set_source(&mut self, file_path: String, source: String) {
        self.executor.source_files.insert(file_path, source);
    }
}
