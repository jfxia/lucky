use crate::ast;

use super::{HirEdgeKind, HirGraph, HirNode, NodeId};

pub struct HirBuilder {
    graph: HirGraph,
}

impl HirBuilder {
    pub fn new() -> Self {
        Self {
            graph: HirGraph::new(),
        }
    }

    pub fn build_module(mut self, module: &ast::Module) -> HirGraph {
        // TODO: replace Module with crate::resolver::ResolvedModule once name
        // resolution is implemented.

        for item in &module.items {
            match item {
                ast::ModuleItem::Goal(d) => {
                    self.build_goal(d);
                }
                ast::ModuleItem::Task(d) => {
                    self.build_task(d);
                }
                ast::ModuleItem::Workflow(d) => {
                    self.build_workflow(d);
                }
                ast::ModuleItem::Agent(d) => {
                    self.build_agent(d);
                }
                _ => {
                    let id = self.graph.add_node(HirNode::Noop {
                        span: item.span(),
                    });
                    self.graph.set_entry(id);
                }
            }
        }

        self.graph
    }

    fn build_goal(&mut self, decl: &ast::GoalDecl) -> NodeId {
        // TODO: resolve subgoals from decl.workflows and recursively build them.
        // TODO: wire subgoal nodes with Data edges and entry_point.

        let mut subgoals: Vec<NodeId> = Vec::new();

        for _wf_name in &decl.workflows {
            subgoals.push(self.graph.add_node(HirNode::Noop {
                span: decl.span,
            }));
        }

        let id = self.graph.add_node(HirNode::Goal {
            goal_ref: decl.name.clone(),
            success_criteria: decl.success_criteria.clone(),
            subgoals: subgoals.clone(),
            span: decl.span,
        });

        for &child in &subgoals {
            self.graph.add_edge(id, child, HirEdgeKind::Data);
        }

        self.graph.set_entry(id);
        id
    }

    fn build_workflow(&mut self, decl: &ast::WorkflowDecl) -> NodeId {
        // TODO: walk decl.body.stmts and convert each Stmt into the appropriate
        // HirNode variant (Let, If->Decision, Match, Loop, For, Parallel,
        // Attempt, Pipeline, ExprStmt->AgentInvoke/ToolCall/LlmCall, etc.).
        // TODO: wire sequential steps with Control edges.
        // TODO: handle context bindings as data-flow inputs.

        let mut body_ids: Vec<NodeId> = Vec::new();

        for _stmt in &decl.body.stmts {
            body_ids.push(self.graph.add_node(HirNode::Noop {
                span: decl.span,
            }));
        }

        let context: Vec<(String, String)> = decl
            .context
            .iter()
            .map(|ti| (ti.name.clone(), "<type>".to_string()))
            .collect();

        let id = self.graph.add_node(HirNode::Workflow {
            workflow_ref: decl.name.clone(),
            context,
            body: body_ids.clone(),
            span: decl.span,
        });

        for pair in body_ids.windows(2) {
            self.graph.add_edge(pair[0], pair[1], HirEdgeKind::Control);
        }

        self.graph.set_entry(id);
        id
    }

    fn build_task(&mut self, decl: &ast::TaskDecl) -> NodeId {
        // TODO: walk decl.steps (Option<Block>) and convert each Stmt into the
        // appropriate HirNode variant.
        // TODO: wire sequential steps with Control edges.
        // TODO: handle decl.inputs / decl.outputs / decl.context as typed
        // bindings on the task node.
        // TODO: handle decl.rollback block.

        let mut step_ids: Vec<NodeId> = Vec::new();
        let mut rollback_ids: Vec<NodeId> = Vec::new();

        if let Some(ref steps) = decl.steps {
            for _stmt in &steps.stmts {
                step_ids.push(self.graph.add_node(HirNode::Noop {
                    span: decl.span,
                }));
            }
        }

        if let Some(ref rollback) = decl.rollback {
            for _stmt in &rollback.stmts {
                rollback_ids.push(self.graph.add_node(HirNode::Noop {
                    span: decl.span,
                }));
            }
        }

        let inputs: Vec<(String, String)> = decl
            .inputs
            .iter()
            .map(|ti| (ti.name.clone(), "<type>".to_string()))
            .collect();
        let outputs: Vec<(String, String)> = decl
            .outputs
            .iter()
            .map(|ti| (ti.name.clone(), "<type>".to_string()))
            .collect();
        let context: Vec<(String, String)> = decl
            .context
            .iter()
            .map(|ti| (ti.name.clone(), "<type>".to_string()))
            .collect();

        let id = self.graph.add_node(HirNode::Task {
            task_ref: decl.name.clone(),
            inputs,
            outputs,
            context,
            steps: step_ids.clone(),
            rollback: rollback_ids,
            span: decl.span,
        });

        for pair in step_ids.windows(2) {
            self.graph.add_edge(pair[0], pair[1], HirEdgeKind::Control);
        }

        self.graph.set_entry(id);
        id
    }

    fn build_agent(&mut self, decl: &ast::AgentDecl) -> NodeId {
        // TODO: for each task in decl.tasks, build_task() and add an
        // AgentInvoke node wired to the task's entry.
        // TODO: wire AgentInvoke nodes with Control edges from the agent
        // entry.
        // TODO: wire model/memory/tools/permissions as Resource or Data edges.

        let mut task_nodes: Vec<NodeId> = Vec::new();

        for task_decl in &decl.tasks {
            let task_id = self.build_task(task_decl);
            let invoke_id = self.graph.add_node(HirNode::AgentInvoke {
                agent_ref: decl.name.clone(),
                task_ref: task_decl.name.clone(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                span: decl.span,
            });

            self.graph.add_edge(invoke_id, task_id, HirEdgeKind::Control);
            task_nodes.push(invoke_id);
        }

        let entry_id = self.graph.add_node(HirNode::Noop {
            span: decl.span,
        });

        self.graph.set_entry(entry_id);

        for &task_id in &task_nodes {
            self.graph.add_edge(entry_id, task_id, HirEdgeKind::Control);
        }

        entry_id
    }
}

impl Default for HirBuilder {
    fn default() -> Self {
        Self::new()
    }
}
