use crate::ast::expr::{Expr, InterpolatedPart, QueryOp, QualifiedName};
use crate::ast::pattern::Pattern;
use crate::ast::stmt::{Block, IfBranch, MatchArm, PipelineStage, RecoveryAction, Stmt};
use crate::ast::types::TypeExpr;
use crate::ast::{
    AgentDecl, ApprovalDecl, CheckpointTrigger, ContextDecl, GoalDecl,
    ImportDecl, MemoryDecl, ModelDecl, Module, ModuleItem, PermissionDecl,
    PolicyDecl, PolicyEntry, PromptDecl, PromptSection, TaskDecl, ToolDecl, ToolMethod,
    TypeDecl, WorkflowDecl,
};

use super::{ResolvedModule, SymbolKind, SymbolTable};

pub struct NameResolver {
    symbols: SymbolTable,
}

impl NameResolver {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
        }
    }

    pub fn resolve_module(&mut self, module: Module) -> ResolvedModule {
        for item in &module.items {
            self.define_top_level(item);
        }

        for item in &module.items {
            self.resolve_item(item);
        }

        ResolvedModule::new(module, self.symbols.clone())
    }

    fn define_top_level(&mut self, item: &ModuleItem) {
        match item {
            ModuleItem::Agent(d) => {
                self.symbols.define(d.name.clone(), SymbolKind::Agent, d.span);
            }
            ModuleItem::Task(d) => {
                self.symbols.define(d.name.clone(), SymbolKind::Task, d.span);
            }
            ModuleItem::Workflow(d) => {
                self.symbols.define(d.name.clone(), SymbolKind::Workflow, d.span);
            }
            ModuleItem::Goal(d) => {
                self.symbols.define(d.name.clone(), SymbolKind::Goal, d.span);
            }
            ModuleItem::Memory(d) => {
                self.symbols.define(d.name.clone(), SymbolKind::Memory, d.span);
            }
            ModuleItem::Tool(d) => {
                self.symbols.define(d.name.clone(), SymbolKind::Tool, d.span);
            }
            ModuleItem::Model(d) => {
                self.symbols.define(d.name.clone(), SymbolKind::Model, d.span);
            }
            ModuleItem::Prompt(d) => {
                self.symbols.define(d.name.clone(), SymbolKind::Prompt, d.span);
            }
            ModuleItem::Policy(d) => {
                self.symbols.define(d.name.clone(), SymbolKind::Policy, d.span);
            }
            ModuleItem::Type(d) => {
                self.symbols.define(d.name.clone(), SymbolKind::Type, d.span);
            }
            _ => {}
        }
    }

    fn resolve_item(&mut self, item: &ModuleItem) {
        match item {
            ModuleItem::Import(d) => self.resolve_import(d),
            ModuleItem::Agent(d) => self.resolve_agent(d),
            ModuleItem::Task(d) => self.resolve_task_decl(d),
            ModuleItem::Workflow(d) => self.resolve_workflow(d),
            ModuleItem::Goal(d) => self.resolve_goal(d),
            ModuleItem::Memory(d) => self.resolve_memory(d),
            ModuleItem::Tool(d) => self.resolve_tool(d),
            ModuleItem::Model(d) => self.resolve_model(d),
            ModuleItem::Prompt(d) => self.resolve_prompt(d),
            ModuleItem::Policy(d) => self.resolve_policy(d),
            ModuleItem::Type(d) => self.resolve_type_alias(d),
            ModuleItem::Context(d) => self.resolve_context(d),
            ModuleItem::Permission(d) => self.resolve_permission(d),
            ModuleItem::Approval(d) => self.resolve_approval(d),
            ModuleItem::Error { .. } => {}
        }
    }

    // ---- Module-level resolvers ----

    fn resolve_import(&mut self, _decl: &ImportDecl) {}

    pub fn resolve_agent(&mut self, decl: &AgentDecl) {
        self.symbols.enter_scope();

        for param in &decl.tools {
            self.resolve_qualified_name(param);
        }
        if let Some(model) = &decl.model {
            self.resolve_qualified_name(model);
        }
        if let Some(memory) = &decl.memory {
            self.resolve_qualified_name(memory);
        }
        if let Some(policy) = &decl.policy {
            self.resolve_qualified_name(policy);
        }
        if let Some(prompt) = &decl.prompt {
            self.resolve_qualified_name(prompt);
        }
        if let Some(permissions) = &decl.permissions {
            self.resolve_permission(permissions);
        }

        for task in &decl.tasks {
            self.resolve_task_decl(task);
        }

        self.symbols.exit_scope();
    }

    fn resolve_task_decl(&mut self, decl: &TaskDecl) {
        self.symbols.define(decl.name.clone(), SymbolKind::Task, decl.span);

        self.symbols.enter_scope();

        for input in &decl.inputs {
            self.symbols.define(
                input.name.clone(),
                SymbolKind::Parameter,
                input.span,
            );
        }
        for output in &decl.outputs {
            self.symbols.define(
                output.name.clone(),
                SymbolKind::Parameter,
                output.span,
            );
        }
        for ctx in &decl.context {
            self.symbols.define(
                ctx.name.clone(),
                SymbolKind::Parameter,
                ctx.span,
            );
        }

        for input in &decl.inputs {
            if let Some(typ) = &input.typ {
                self.resolve_type_expr(typ);
            }
        }
        for output in &decl.outputs {
            if let Some(typ) = &output.typ {
                self.resolve_type_expr(typ);
            }
        }
        for ctx in &decl.context {
            if let Some(typ) = &ctx.typ {
                self.resolve_type_expr(typ);
            }
        }

        if let Some(policy) = &decl.policy {
            self.resolve_qualified_name(policy);
        }
        if let Some(steps) = &decl.steps {
            self.resolve_block(steps);
        }
        if let Some(rollback) = &decl.rollback {
            self.resolve_block(rollback);
        }

        self.symbols.exit_scope();
    }

    pub fn resolve_workflow(&mut self, decl: &WorkflowDecl) {
        self.symbols.enter_scope();

        for ctx in &decl.context {
            self.symbols.define(
                ctx.name.clone(),
                SymbolKind::Parameter,
                ctx.span,
            );
        }
        for ctx in &decl.context {
            if let Some(typ) = &ctx.typ {
                self.resolve_type_expr(typ);
            }
        }

        self.resolve_block(&decl.body);

        self.symbols.exit_scope();
    }

    pub fn resolve_goal(&mut self, decl: &GoalDecl) {
        for workflow_name in &decl.workflows {
            self.symbols.lookup(workflow_name);
        }
    }

    fn resolve_memory(&mut self, decl: &MemoryDecl) {
        for (_key, value) in &decl.config {
            self.resolve_expr(value);
        }
    }

    fn resolve_tool(&mut self, decl: &ToolDecl) {
        self.symbols.enter_scope();

        for (_key, value) in &decl.config {
            self.resolve_expr(value);
        }

        for method in &decl.methods {
            self.resolve_tool_method(method);
        }

        self.symbols.exit_scope();
    }

    fn resolve_tool_method(&mut self, method: &ToolMethod) {
        self.symbols.enter_scope();

        for param in &method.params {
            self.symbols.define(
                param.name.clone(),
                SymbolKind::Parameter,
                param.span,
            );
        }
        for param in &method.params {
            if let Some(typ) = &param.typ {
                self.resolve_type_expr(typ);
            }
        }
        if let Some(ret) = &method.return_type {
            self.resolve_type_expr(ret);
        }

        self.symbols.exit_scope();
    }

    fn resolve_model(&mut self, decl: &ModelDecl) {
        for (_key, value) in &decl.config {
            self.resolve_expr(value);
        }
    }

    fn resolve_prompt(&mut self, decl: &PromptDecl) {
        for section in &decl.sections {
            self.resolve_prompt_section(section);
        }
    }

    fn resolve_prompt_section(&mut self, _section: &PromptSection) {}

    fn resolve_policy(&mut self, decl: &PolicyDecl) {
        for entry in &decl.entries {
            self.resolve_policy_entry(entry);
        }
    }

    fn resolve_policy_entry(&mut self, entry: &PolicyEntry) {
        match entry {
            PolicyEntry::Retry {
                max_delay, ..
            } => {
                if let Some(expr) = max_delay {
                    self.resolve_expr(expr);
                }
            }
            PolicyEntry::Timeout { duration, .. } => {
                self.resolve_expr(duration);
            }
            PolicyEntry::Checkpoint {
                trigger, ..
            } => {
                self.resolve_checkpoint_trigger(trigger);
            }
            PolicyEntry::Cache { ttl, .. } => {
                if let Some(expr) = ttl {
                    self.resolve_expr(expr);
                }
            }
            PolicyEntry::CostLimit { amount, .. } => {
                self.resolve_expr(amount);
            }
            _ => {}
        }
    }

    fn resolve_checkpoint_trigger(&mut self, trigger: &CheckpointTrigger) {
        if let CheckpointTrigger::Interval(expr) = trigger {
            self.resolve_expr(expr);
        }
    }

    fn resolve_type_alias(&mut self, decl: &TypeDecl) {
        self.resolve_type_expr(&decl.typ);
    }

    fn resolve_context(&mut self, decl: &ContextDecl) {
        for entry in &decl.entries {
            self.symbols.define(
                entry.name.clone(),
                SymbolKind::Variable,
                entry.span,
            );
        }
        for entry in &decl.entries {
            if let Some(typ) = &entry.typ {
                self.resolve_type_expr(typ);
            }
        }
    }

    fn resolve_permission(&mut self, _decl: &PermissionDecl) {}

    fn resolve_approval(&mut self, decl: &ApprovalDecl) {
        for gate in &decl.gates {
            if let Some(timeout) = &gate.timeout {
                self.resolve_expr(timeout);
            }
        }
    }

    // ---- Block and statement resolution ----

    fn resolve_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.resolve_stmt(stmt);
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                name,
                typ,
                value,
                span,
            } => {
                self.symbols
                    .define(name.clone(), SymbolKind::Variable, *span);
                if let Some(typ) = typ {
                    self.resolve_type_expr(typ);
                }
                self.resolve_expr(value);
            }
            Stmt::Const {
                name,
                typ,
                value,
                span,
            } => {
                self.symbols
                    .define(name.clone(), SymbolKind::Variable, *span);
                if let Some(typ) = typ {
                    self.resolve_type_expr(typ);
                }
                self.resolve_expr(value);
            }
            Stmt::Assign {
                target,
                value,
                ..
            } => {
                self.resolve_expr(target);
                self.resolve_expr(value);
            }
            Stmt::ExprStmt { expr, .. } => {
                self.resolve_expr(expr);
            }
            Stmt::If {
                branches,
                else_body,
                ..
            } => {
                for branch in branches {
                    self.resolve_if_branch(branch);
                }
                if let Some(body) = else_body {
                    self.resolve_block(body);
                }
            }
            Stmt::Match {
                scrutinee,
                arms,
                ..
            } => {
                self.resolve_expr(scrutinee);
                for arm in arms {
                    self.resolve_match_arm(arm);
                }
            }
            Stmt::Loop { body, .. } => {
                self.resolve_block(body);
            }
            Stmt::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                self.resolve_pattern_bindings(pattern);
                self.resolve_expr(iterable);
                self.resolve_block(body);
            }
            Stmt::Parallel { body, .. } => {
                self.resolve_block(body);
            }
            Stmt::Await { expr, .. } => {
                self.resolve_expr(expr);
            }
            Stmt::When {
                conditions,
                body,
                ..
            } => {
                for cond in conditions {
                    self.resolve_expr(cond);
                }
                self.resolve_block(body);
            }
            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    self.resolve_expr(expr);
                }
            }
            Stmt::Break { value, .. } => {
                if let Some(expr) = value {
                    self.resolve_expr(expr);
                }
            }
            Stmt::Continue { .. } => {}
            Stmt::Attempt {
                body,
                recovery_blocks,
                ..
            } => {
                self.resolve_block(body);
                for block in recovery_blocks {
                    for action in block {
                        self.resolve_recovery_action(action);
                    }
                }
            }
            Stmt::Swarm {
                count,
                target,
                ..
            } => {
                self.resolve_expr(count);
                self.resolve_expr(target);
            }
            Stmt::Pipeline { stages, .. } => {
                for stage in stages {
                    self.resolve_pipeline_stage(stage);
                }
            }
            Stmt::Error { .. } => {}
        }
    }

    fn resolve_if_branch(&mut self, branch: &IfBranch) {
        self.resolve_expr(&branch.condition);
        self.resolve_block(&branch.body);
    }

    fn resolve_match_arm(&mut self, arm: &MatchArm) {
        self.symbols.enter_scope();
        self.resolve_pattern_bindings(&arm.pattern);
        if let Some(guard) = &arm.guard {
            self.resolve_expr(guard);
        }
        self.resolve_block(&arm.body);
        self.symbols.exit_scope();
    }

    fn resolve_pattern_bindings(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Variable { name, span } => {
                self.symbols
                    .define(name.clone(), SymbolKind::Variable, *span);
            }
            Pattern::Constructor { fields, .. } => {
                for field in fields {
                    self.resolve_pattern_bindings(field);
                }
            }
            Pattern::List {
                elements, rest, span
            } => {
                for elem in elements {
                    self.resolve_pattern_bindings(elem);
                }
                if let Some(name) = rest {
                    self.symbols
                        .define(name.clone(), SymbolKind::Variable, *span);
                }
            }
            Pattern::Map { entries, .. } => {
                for (key, value) in entries {
                    self.resolve_pattern_bindings(key);
                    self.resolve_pattern_bindings(value);
                }
            }
            _ => {}
        }
    }

    fn resolve_recovery_action(&mut self, action: &RecoveryAction) {
        match action {
            RecoveryAction::Retry {
                max_delay,
                ..
            } => {
                if let Some(expr) = max_delay {
                    self.resolve_expr(expr);
                }
            }
            RecoveryAction::Fallback { task, .. } => {
                self.resolve_expr(task);
            }
            _ => {}
        }
    }

    fn resolve_pipeline_stage(&mut self, stage: &PipelineStage) {
        for arg in &stage.args {
            self.resolve_expr(arg);
        }
    }

    // ---- Expression resolution ----

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Lit { .. } => {}
            Expr::Var { name, .. } => {
                self.resolve_qualified_name(name);
            }
            Expr::Call {
                callee,
                args,
                ..
            } => {
                self.resolve_expr(callee);
                for arg in args {
                    if let Some(name) = &arg.name {
                        self.symbols.lookup(name);
                    }
                    self.resolve_expr(&arg.value);
                }
            }
            Expr::Index {
                base,
                index,
                ..
            } => {
                self.resolve_expr(base);
                self.resolve_expr(index);
            }
            Expr::FieldAccess { base, .. } => {
                self.resolve_expr(base);
            }
            Expr::NullableFieldAccess { base, .. } => {
                self.resolve_expr(base);
            }
            Expr::NullableIndex {
                base,
                index,
                ..
            } => {
                self.resolve_expr(base);
                self.resolve_expr(index);
            }
            Expr::BinaryOp {
                lhs,
                rhs,
                ..
            } => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            Expr::UnaryOp { expr, .. } => {
                self.resolve_expr(expr);
            }
            Expr::Lambda {
                params,
                body,
                ..
            } => {
                self.symbols.enter_scope();
                for param in params {
                    self.symbols.define(
                        param.name.clone(),
                        SymbolKind::Parameter,
                        param.span,
                    );
                }
                self.resolve_expr(body);
                self.symbols.exit_scope();
            }
            Expr::Pipeline { stages, .. } => {
                for stage in stages {
                    self.resolve_expr(stage);
                }
            }
            Expr::Query {
                source,
                ops,
                ..
            } => {
                self.resolve_expr(source);
                for op in ops {
                    match op {
                        QueryOp::Where(expr) => self.resolve_expr(expr),
                        QueryOp::Select(expr) => self.resolve_expr(expr),
                        QueryOp::OrderBy { expr, .. } => self.resolve_expr(expr),
                        QueryOp::GroupBy(expr) => self.resolve_expr(expr),
                        QueryOp::Limit(expr) => self.resolve_expr(expr),
                        QueryOp::Skip(expr) => self.resolve_expr(expr),
                    }
                }
            }
            Expr::List { elements, .. } => {
                for elem in elements {
                    self.resolve_expr(elem);
                }
            }
            Expr::Set { elements, .. } => {
                for elem in elements {
                    self.resolve_expr(elem);
                }
            }
            Expr::Map { entries, .. } => {
                for (key, value) in entries {
                    self.resolve_expr(key);
                    self.resolve_expr(value);
                }
            }
            Expr::InterpolatedString { parts, .. } => {
                for part in parts {
                    if let InterpolatedPart::Expr(expr) = part {
                        self.resolve_expr(expr);
                    }
                }
            }
            Expr::Ask { .. } => {}
            Expr::AskHuman { .. } => {}
            Expr::Reason { .. } => {}
            Expr::Use { .. } => {}
            Expr::Confidence {
                expr,
                threshold,
                ..
            } => {
                self.resolve_expr(expr);
                self.resolve_expr(threshold);
            }
            Expr::NullCoalesce {
                expr,
                default,
                ..
            } => {
                self.resolve_expr(expr);
                self.resolve_expr(default);
            }
            Expr::Range {
                start,
                end,
                ..
            } => {
                if let Some(s) = start {
                    self.resolve_expr(s);
                }
                if let Some(e) = end {
                    self.resolve_expr(e);
                }
            }
            Expr::Paren { expr, .. } => {
                self.resolve_expr(expr);
            }
            Expr::IfExpr {
                cond,
                then,
                else_,
                ..
            } => {
                self.resolve_expr(cond);
                self.resolve_expr(then);
                self.resolve_expr(else_);
            }
            Expr::MatchExpr {
                scrutinee,
                arms,
                ..
            } => {
                self.resolve_expr(scrutinee);
                for arm in arms {
                    self.resolve_match_arm(arm);
                }
            }
            Expr::Error { .. } => {}
        }
    }

    fn resolve_type_expr(&mut self, typ: &TypeExpr) {
        match typ {
            TypeExpr::Primitive { .. } => {}
            TypeExpr::Named {
                name,
                args,
                ..
            } => {
                self.symbols.lookup(name);
                for arg in args {
                    self.resolve_type_expr(arg);
                }
            }
            TypeExpr::Nullable { inner, .. } => {
                self.resolve_type_expr(inner);
            }
            TypeExpr::Optional { inner, .. } => {
                self.resolve_type_expr(inner);
            }
            TypeExpr::Union {
                left,
                right,
                ..
            } => {
                self.resolve_type_expr(left);
                self.resolve_type_expr(right);
            }
            TypeExpr::List { element, .. } => {
                self.resolve_type_expr(element);
            }
            TypeExpr::Set { element, .. } => {
                self.resolve_type_expr(element);
            }
            TypeExpr::Map {
                key,
                value,
                ..
            } => {
                self.resolve_type_expr(key);
                self.resolve_type_expr(value);
            }
            TypeExpr::Tuple { elements, .. } => {
                for elem in elements {
                    self.resolve_type_expr(elem);
                }
            }
            TypeExpr::Function {
                params,
                returns,
                ..
            } => {
                for param in params {
                    self.resolve_type_expr(param);
                }
                for ret in returns {
                    self.resolve_type_expr(ret);
                }
            }
            TypeExpr::Qualified {
                path,
                args,
                ..
            } => {
                if let Some(first) = path.first() {
                    self.symbols.lookup(first);
                }
                for arg in args {
                    self.resolve_type_expr(arg);
                }
            }
            TypeExpr::Paren { inner, .. } => {
                self.resolve_type_expr(inner);
            }
            TypeExpr::Error { .. } => {}
        }
    }

    fn resolve_qualified_name(&self, name: &QualifiedName) {
        if let Some(first) = name.parts.first() {
            self.symbols.lookup(first);
        }
    }
}

impl Default for NameResolver {
    fn default() -> Self {
        Self::new()
    }
}
