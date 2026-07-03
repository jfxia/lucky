use crate::ast;

pub mod builder;

pub type NodeId = usize;

#[derive(Debug, Clone, PartialEq)]
pub struct HirGraph {
    pub nodes: Vec<HirNode>,
    pub edges: Vec<HirEdge>,
    pub entry_points: Vec<NodeId>,
}

impl HirGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            entry_points: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: HirNode) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, kind: HirEdgeKind) {
        self.edges.push(HirEdge { from, to, kind });
    }

    pub fn set_entry(&mut self, id: NodeId) {
        self.entry_points.push(id);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirNode {
    Goal {
        goal_ref: String,
        success_criteria: Vec<String>,
        subgoals: Vec<NodeId>,
        span: ast::span::Span,
    },

    Workflow {
        workflow_ref: String,
        context: Vec<(String, String)>,
        body: Vec<NodeId>,
        span: ast::span::Span,
    },

    Task {
        task_ref: String,
        inputs: Vec<(String, String)>,
        outputs: Vec<(String, String)>,
        context: Vec<(String, String)>,
        steps: Vec<NodeId>,
        rollback: Vec<NodeId>,
        span: ast::span::Span,
    },

    AgentInvoke {
        agent_ref: String,
        task_ref: String,
        inputs: Vec<String>,
        outputs: Vec<String>,
        span: ast::span::Span,
    },

    ToolCall {
        tool_ref: String,
        method: Option<String>,
        inputs: Vec<String>,
        outputs: Vec<String>,
        span: ast::span::Span,
    },

    LlmCall {
        model_ref: String,
        prompt_ref: Option<String>,
        inputs: Vec<String>,
        outputs: Vec<String>,
        span: ast::span::Span,
    },

    Decision {
        condition: String,
        then_branch: NodeId,
        else_branch: Option<NodeId>,
        span: ast::span::Span,
    },

    Match {
        scrutinee: String,
        arms: Vec<(String, NodeId)>,
        default: Option<NodeId>,
        span: ast::span::Span,
    },

    Parallel {
        branches: Vec<NodeId>,
        join: Option<NodeId>,
        has_wait: bool,
        span: ast::span::Span,
    },

    Join {
        sources: Vec<NodeId>,
        span: ast::span::Span,
    },

    Loop {
        body: Vec<NodeId>,
        condition: Option<String>,
        span: ast::span::Span,
    },

    ForEach {
        binding: String,
        iterable: String,
        body: Vec<NodeId>,
        span: ast::span::Span,
    },

    Pipeline {
        stages: Vec<NodeId>,
        span: ast::span::Span,
    },

    Attempt {
        body: Vec<NodeId>,
        recovery_blocks: Vec<Vec<NodeId>>,
        span: ast::span::Span,
    },

    Approval {
        gate_ref: String,
        operation: String,
        timeout: Option<String>,
        span: ast::span::Span,
    },

    Let {
        name: String,
        value: String,
        typ: Option<String>,
        span: ast::span::Span,
    },

    Return {
        value: Option<String>,
        span: ast::span::Span,
    },

    Noop {
        span: ast::span::Span,
    },
}

impl HirNode {
    pub fn span(&self) -> ast::span::Span {
        match self {
            HirNode::Goal { span, .. } => *span,
            HirNode::Workflow { span, .. } => *span,
            HirNode::Task { span, .. } => *span,
            HirNode::AgentInvoke { span, .. } => *span,
            HirNode::ToolCall { span, .. } => *span,
            HirNode::LlmCall { span, .. } => *span,
            HirNode::Decision { span, .. } => *span,
            HirNode::Match { span, .. } => *span,
            HirNode::Parallel { span, .. } => *span,
            HirNode::Join { span, .. } => *span,
            HirNode::Loop { span, .. } => *span,
            HirNode::ForEach { span, .. } => *span,
            HirNode::Pipeline { span, .. } => *span,
            HirNode::Attempt { span, .. } => *span,
            HirNode::Approval { span, .. } => *span,
            HirNode::Let { span, .. } => *span,
            HirNode::Return { span, .. } => *span,
            HirNode::Noop { span } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: HirEdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HirEdgeKind {
    Control,
    Data,
    Resource,
    Condition,
    Approval,
    Error,
}
