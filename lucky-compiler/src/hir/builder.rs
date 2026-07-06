use crate::ast;
use crate::ast::expr::Expr;
use crate::ast::stmt::Stmt;
use crate::ast::pattern::Pattern;

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
        for item in &module.items {
            match item {
                ast::ModuleItem::Goal(d) => { self.build_goal(d); }
                ast::ModuleItem::Task(d) => { self.build_task(d); }
                ast::ModuleItem::Workflow(d) => { self.build_workflow(d); }
                ast::ModuleItem::Agent(d) => { self.build_agent(d); }
                ast::ModuleItem::Model(d) => { self.build_decl_placeholder(&d.name, d.span); }
                ast::ModuleItem::Memory(d) => { self.build_decl_placeholder(&d.name, d.span); }
                ast::ModuleItem::Prompt(d) => { self.build_decl_placeholder(&d.name, d.span); }
                ast::ModuleItem::Policy(d) => { self.build_decl_placeholder(&d.name, d.span); }
                ast::ModuleItem::Tool(d) => { self.build_decl_placeholder(&d.name, d.span); }
                ast::ModuleItem::Type(d) => { self.build_decl_placeholder(&d.name, d.span); }
                ast::ModuleItem::Context(_) => {}
                ast::ModuleItem::Permission(_) => {}
                ast::ModuleItem::Approval(_) => {}
                ast::ModuleItem::Import(_) => {}
                _ => {}
            }
        }
        self.graph
    }

    fn build_decl_placeholder(&mut self, name: &str, span: ast::span::Span) -> NodeId {
        let id = self.graph.add_node(HirNode::Noop { span });
        self.graph.set_entry(id);
        id
    }

    fn build_goal(&mut self, decl: &ast::GoalDecl) -> NodeId {
        let mut subgoals: Vec<NodeId> = Vec::new();
        for _wf_name in &decl.workflows {
            let sid = self.graph.add_node(HirNode::Noop { span: decl.span });
            subgoals.push(sid);
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
        let mut body_ids: Vec<NodeId> = Vec::new();
        for stmt in &decl.body.stmts {
            let prev_len = self.graph.nodes.len();
            self.build_stmt(stmt);
            for nid in prev_len..self.graph.nodes.len() {
                body_ids.push(nid);
            }
        }
        let context: Vec<(String, String)> = decl
            .context
            .iter()
            .map(|ti| (ti.name.clone(), type_str(ti.typ.as_deref())))
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
        let mut step_ids: Vec<NodeId> = Vec::new();
        let mut rollback_ids: Vec<NodeId> = Vec::new();
        if let Some(ref steps) = decl.steps {
            for stmt in &steps.stmts {
                let prev = self.graph.nodes.len();
                self.build_stmt(stmt);
                for nid in prev..self.graph.nodes.len() {
                    step_ids.push(nid);
                }
            }
        }
        if let Some(ref rollback) = decl.rollback {
            for stmt in &rollback.stmts {
                let prev = self.graph.nodes.len();
                self.build_stmt(stmt);
                for nid in prev..self.graph.nodes.len() {
                    rollback_ids.push(nid);
                }
            }
        }
        let inputs: Vec<(String, String)> = decl
            .inputs.iter()
            .map(|ti| (ti.name.clone(), type_str(ti.typ.as_deref())))
            .collect();
        let outputs: Vec<(String, String)> = decl
            .outputs.iter()
            .map(|ti| (ti.name.clone(), type_str(ti.typ.as_deref())))
            .collect();
        let context: Vec<(String, String)> = decl
            .context.iter()
            .map(|ti| (ti.name.clone(), type_str(ti.typ.as_deref())))
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
        let model_ref = decl.model.as_ref().map(|q| q.last().to_string());
        let memory_ref = decl.memory.as_ref().map(|q| q.last().to_string());
        let tool_refs: Vec<String> = decl.tools.iter().map(|q| q.last().to_string()).collect();
        let policy_ref = decl.policy.as_ref().map(|q| q.last().to_string());
        let prompt_ref = decl.prompt.as_ref().map(|q| q.last().to_string());

        let entry_id = self.graph.add_node(HirNode::Noop { span: decl.span });
        self.graph.set_entry(entry_id);
        let mut prev_id = entry_id;

        for task_decl in &decl.tasks {
            let task_id = self.build_task(task_decl);
            let invoke_id = self.graph.add_node(HirNode::AgentInvoke {
                agent_ref: decl.name.clone(),
                task_ref: task_decl.name.clone(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                model_ref: model_ref.clone(),
                memory_ref: memory_ref.clone(),
                tool_refs: tool_refs.clone(),
                policy_ref: policy_ref.clone(),
                prompt_ref: prompt_ref.clone(),
                span: decl.span,
            });
            self.graph.add_edge(prev_id, invoke_id, HirEdgeKind::Control);
            self.graph.add_edge(invoke_id, task_id, HirEdgeKind::Control);
            prev_id = invoke_id;
        }
        entry_id
    }

    fn build_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, value, typ, span } => {
                let val_str = expr_to_string(value);
                let typ_str = typ.as_ref().map(|t| format!("{:?}", t));
                self.graph.add_node(HirNode::Let {
                    name: name.clone(), value: val_str, typ: typ_str, span: *span,
                });
            }
            Stmt::Const { name, value, typ, span } => {
                let val_str = expr_to_string(value);
                let typ_str = typ.as_ref().map(|t| format!("{:?}", t));
                self.graph.add_node(HirNode::Let {
                    name: name.clone(), value: val_str, typ: typ_str, span: *span,
                });
            }
            Stmt::Return { value, span } => {
                let val_str = value.as_ref().map(|e| expr_to_string(e));
                self.graph.add_node(HirNode::Return { value: val_str, span: *span });
            }
            Stmt::ExprStmt { expr, span } => {
                self.build_expr(expr, *span);
            }
            Stmt::If { branches, else_body, span } => {
                for branch in branches {
                    let cond_str = expr_to_string(&branch.condition);
                    let then_start = self.graph.nodes.len();
                    for s in &branch.body.stmts {
                        self.build_stmt(s);
                    }
                    let then_end = self.graph.nodes.len();
                    let then_id = if then_end > then_start { then_start } else {
                        self.graph.add_node(HirNode::Noop { span: branch.body.span })
                    };
                    let else_id = else_body.as_ref().map(|b| {
                        let s = self.graph.nodes.len();
                        for st in &b.stmts { self.build_stmt(st); }
                        if self.graph.nodes.len() > s { s } else {
                            self.graph.add_node(HirNode::Noop { span: b.span })
                        }
                    });
                    let did = self.graph.add_node(HirNode::Decision {
                        condition: cond_str,
                        then_branch: then_id,
                        else_branch: else_id,
                        span: *span,
                    });
                    for i in then_start..then_end {
                        if i > then_start {
                            self.graph.add_edge(i - 1, i, HirEdgeKind::Control);
                        }
                    }
                }
            }
            Stmt::Match { scrutinee, arms, span } => {
                let scr_str = expr_to_string(scrutinee);
                let default_id = self.graph.add_node(HirNode::Noop { span: *span });
                let mut arm_nodes = Vec::new();
                for arm in arms {
                    let body_start = self.graph.nodes.len();
                    for s in &arm.body.stmts { self.build_stmt(s); }
                    let body_end = self.graph.nodes.len();
                    let body_id = if body_end > body_start { body_start } else {
                        self.graph.add_node(HirNode::Noop { span: arm.body.span })
                    };
                    arm_nodes.push((pattern_to_string(&arm.pattern), body_id));
                }
                self.graph.add_node(HirNode::Match {
                    scrutinee: scr_str, arms: arm_nodes, default: Some(default_id), span: *span,
                });
            }
            Stmt::Loop { body, span } => {
                let body_start = self.graph.nodes.len();
                for s in &body.stmts { self.build_stmt(s); }
                let body_end = self.graph.nodes.len();
                let body_ids: Vec<NodeId> = (body_start..body_end).collect();
                self.graph.add_node(HirNode::Loop {
                    body: body_ids, condition: None, span: *span,
                });
            }
            Stmt::For { pattern, iterable, body, span } => {
                let body_start = self.graph.nodes.len();
                for s in &body.stmts { self.build_stmt(s); }
                let body_end = self.graph.nodes.len();
                let body_ids: Vec<NodeId> = (body_start..body_end).collect();
                let iter_str = expr_to_string(iterable);
                self.graph.add_node(HirNode::ForEach {
                    binding: pattern_to_string(pattern),
                    iterable: iter_str,
                    body: body_ids,
                    span: *span,
                });
            }
            Stmt::Parallel { body, has_wait, span } => {
                let branch_start = self.graph.nodes.len();
                for s in &body.stmts { self.build_stmt(s); }
                let branch_end = self.graph.nodes.len();
                let branch_ids: Vec<NodeId> = (branch_start..branch_end).collect();
                let join_id = if *has_wait {
                    Some(self.graph.add_node(HirNode::Join {
                        sources: branch_ids.clone(), span: *span,
                    }))
                } else { None };
                self.graph.add_node(HirNode::Parallel {
                    branches: branch_ids, join: join_id, has_wait: *has_wait, span: *span,
                });
            }
            Stmt::Attempt { body, recovery_blocks, span } => {
                let body_start = self.graph.nodes.len();
                for s in &body.stmts { self.build_stmt(s); }
                let body_end = self.graph.nodes.len();
                let body_ids: Vec<NodeId> = (body_start..body_end).collect();
                let recovery_ids: Vec<Vec<NodeId>> = recovery_blocks.iter().map(|_| {
                    let rid = self.graph.add_node(HirNode::Noop { span: *span });
                    vec![rid]
                }).collect();
                self.graph.add_node(HirNode::Attempt {
                    body: body_ids, recovery_blocks: recovery_ids, span: *span,
                });
            }
            Stmt::Pipeline { stages, span } => {
                let stage_ids: Vec<NodeId> = stages.iter().map(|_| {
                    self.graph.add_node(HirNode::Noop { span: *span })
                }).collect();
                self.graph.add_node(HirNode::Pipeline {
                    stages: stage_ids, span: *span,
                });
            }
            _ => {
                self.graph.add_node(HirNode::Noop { span: stmt.span() });
            }
        }
    }

    fn build_expr(&mut self, expr: &Expr, span: ast::span::Span) {
        match expr {
            Expr::Var { name, .. } => {
                let name_str = name.last();
                if name_str.contains('.') || is_upper_first(name_str) {
                    self.graph.add_node(HirNode::AgentInvoke {
                        agent_ref: name_str.to_string(),
                        task_ref: String::new(),
                        inputs: Vec::new(),
                        outputs: Vec::new(),
                        model_ref: None, memory_ref: None, tool_refs: Vec::new(),
                        policy_ref: None, prompt_ref: None, span,
                    });
                } else {
                    self.graph.add_node(HirNode::LlmCall {
                        model_ref: name_str.to_string(),
                        prompt_ref: Some(name_str.to_string()),
                        inputs: Vec::new(), outputs: Vec::new(), span,
                    });
                }
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Var { name, .. } = callee.as_ref() {
                    let name_str = name.last();
                    let input_strs: Vec<String> = args.iter()
                        .map(|a| expr_to_string(&a.value))
                        .collect();
                    if name_str.contains('.') {
                        let parts: Vec<&str> = name_str.splitn(2, '.').collect();
                        self.graph.add_node(HirNode::ToolCall {
                            tool_ref: parts[0].to_string(),
                            method: Some(parts[1].to_string()),
                            inputs: input_strs, outputs: Vec::new(), span,
                        });
                    } else if is_upper_first(name_str) {
                        self.graph.add_node(HirNode::AgentInvoke {
                            agent_ref: name_str.to_string(),
                            task_ref: String::new(),
                            inputs: input_strs, outputs: Vec::new(),
                            model_ref: None, memory_ref: None, tool_refs: Vec::new(),
                            policy_ref: None, prompt_ref: None, span,
                        });
                    } else {
                        self.graph.add_node(HirNode::LlmCall {
                            model_ref: name_str.to_string(),
                            prompt_ref: Some(name_str.to_string()),
                            inputs: input_strs, outputs: Vec::new(), span,
                        });
                    }
                } else {
                    self.graph.add_node(HirNode::Noop { span });
                }
            }
            Expr::Lit { value, .. } => {
                self.graph.add_node(HirNode::Let {
                    name: String::new(), value: format!("{}", value),
                    typ: None, span,
                });
            }
            Expr::BinaryOp { lhs, rhs, .. } => {
                let lhs_str = expr_to_string(lhs);
                let rhs_str = expr_to_string(rhs);
                self.graph.add_node(HirNode::Let {
                    name: String::new(),
                    value: format!("{} {} {}", lhs_str, "?", rhs_str),
                    typ: None, span,
                });
            }
            _ => {
                self.graph.add_node(HirNode::Noop { span });
            }
        }
    }
}

impl Default for HirBuilder {
    fn default() -> Self { Self::new() }
}

fn is_upper_first(s: &str) -> bool {
    s.chars().next().map_or(false, |c| c.is_uppercase())
}

fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Var { name, .. } => name.last().to_string(),
        Expr::Lit { value, .. } => format!("{}", value),
        Expr::Call { callee, args, .. } => {
            let c = expr_to_string(callee);
            let a: Vec<String> = args.iter().map(|a| expr_to_string(&a.value)).collect();
            format!("{}({})", c, a.join(", "))
        }
        Expr::BinaryOp { lhs, rhs, .. } => {
            format!("({} ? {})", expr_to_string(lhs), expr_to_string(rhs))
        }
        _ => "<?>".to_string(),
    }
}

fn pattern_to_string(pat: &Pattern) -> String {
    match pat {
        Pattern::Wildcard { .. } => "_".to_string(),
        Pattern::Variable { name, .. } => name.clone(),
        Pattern::Literal { value, .. } => format!("{}", value),
        Pattern::Constructor { name, fields, .. } => {
            let a: Vec<String> = fields.iter().map(pattern_to_string).collect();
            format!("{}({})", name, a.join(", "))
        }
        Pattern::List { elements, rest, .. } => {
            let mut e: Vec<String> = elements.iter().map(|p| pattern_to_string(p)).collect();
            if let Some(r) = rest { e.push(format!("..{}", r)); }
            format!("[{}]", e.join(", "))
        }
        Pattern::Map { .. } => "{{..}}".to_string(),
        Pattern::Error { .. } => "<error>".to_string(),
    }
}

fn type_str(typ: Option<&ast::types::TypeExpr>) -> String {
    match typ {
        Some(t) => format!("{:?}", t),
        None => "<type>".to_string(),
    }
}
