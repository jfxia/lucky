use crate::ast::*;
use crate::ast::stmt::Stmt;
use crate::ast::expr::Expr;

#[derive(Debug, Clone)]
pub struct TypeCheckDiagnostic {
    pub message: String,
    pub span: crate::ast::span::Span,
    pub is_error: bool,
}

#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    pub diagnostics: Vec<TypeCheckDiagnostic>,
}

impl TypeCheckResult {
    pub fn new() -> Self {
        Self { diagnostics: Vec::new() }
    }

    pub fn error(&mut self, msg: impl Into<String>, span: crate::ast::span::Span) {
        self.diagnostics.push(TypeCheckDiagnostic {
            message: msg.into(), span, is_error: true,
        });
    }

    pub fn warning(&mut self, msg: impl Into<String>, span: crate::ast::span::Span) {
        self.diagnostics.push(TypeCheckDiagnostic {
            message: msg.into(), span, is_error: false,
        });
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error)
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

pub struct TypeChecker {
    result: TypeCheckResult,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self { result: TypeCheckResult::new() }
    }

    pub fn check(mut self, module: &Module) -> TypeCheckResult {
        for item in &module.items {
            self.check_item(item);
        }
        self.result
    }

    fn check_item(&mut self, item: &ModuleItem) {
        match item {
            ModuleItem::Agent(d) => self.check_agent(d),
            ModuleItem::Task(d) => self.check_task(d),
            ModuleItem::Workflow(d) => self.check_workflow(d),
            ModuleItem::Goal(d) => self.check_goal(d),
            ModuleItem::Model(d) => self.check_model(d),
            ModuleItem::Memory(d) => self.check_memory(d),
            ModuleItem::Tool(d) => self.check_tool(d),
            ModuleItem::Prompt(d) => self.check_prompt(d),
            ModuleItem::Policy(d) => self.check_policy(d),
            ModuleItem::Type(d) => self.check_type_decl(d),
            ModuleItem::Context(d) => self.check_context(d),
            ModuleItem::Permission(d) => self.check_permission(d),
            ModuleItem::Approval(d) => self.check_approval(d),
            ModuleItem::Import(_) => {}
            _ => {}
        }
    }

    fn check_agent(&mut self, decl: &AgentDecl) {
        if decl.model.is_none() {
            self.result.warning(
                format!("Agent '{}' has no model reference", decl.name),
                decl.span,
            );
        }
        if decl.tools.is_empty() {
            self.result.warning(
                format!("Agent '{}' has no tools", decl.name),
                decl.span,
            );
        }
        for task in &decl.tasks {
            self.check_task(task);
        }
    }

    fn check_task(&mut self, decl: &TaskDecl) {
        if decl.inputs.is_empty() && decl.outputs.is_empty() {
            self.result.warning(
                format!("Task '{}' has no inputs or outputs", decl.name),
                decl.span,
            );
        }
        for input in &decl.inputs {
            if input.typ.is_none() {
                self.result.warning(
                    format!("Task '{}' input '{}' has no type annotation", decl.name, input.name),
                    input.span,
                );
            }
        }
        for output in &decl.outputs {
            if output.typ.is_none() {
                self.result.warning(
                    format!("Task '{}' output '{}' has no type annotation", decl.name, output.name),
                    output.span,
                );
            }
        }
        if let Some(ref steps) = decl.steps {
            for stmt in &steps.stmts {
                self.check_stmt(stmt);
            }
        }
        if let Some(ref rollback) = decl.rollback {
            for stmt in &rollback.stmts {
                self.check_stmt(stmt);
            }
        }
    }

    fn check_workflow(&mut self, decl: &WorkflowDecl) {
        for stmt in &decl.body.stmts {
            self.check_stmt(stmt);
        }
    }

    fn check_goal(&mut self, decl: &GoalDecl) {
        if decl.success_criteria.is_empty() {
            self.result.warning(
                format!("Goal '{}' has no success criteria", decl.name),
                decl.span,
            );
        }
        if decl.workflows.is_empty() {
            self.result.warning(
                format!("Goal '{}' has no workflows", decl.name),
                decl.span,
            );
        }
    }

    fn check_model(&mut self, decl: &ModelDecl) {
        if decl.config.is_empty() {
            self.result.warning(
                format!("Model '{}' has no configuration", decl.name),
                decl.span,
            );
        }
    }

    fn check_memory(&mut self, decl: &MemoryDecl) {
        if decl.scope.is_none() {
            self.result.warning(
                format!("Memory '{}' has no scope", decl.name),
                decl.span,
            );
        }
    }

    fn check_tool(&mut self, _decl: &ToolDecl) {}

    fn check_prompt(&mut self, decl: &PromptDecl) {
        if decl.sections.is_empty() {
            self.result.warning(
                format!("Prompt '{}' has no sections", decl.name),
                decl.span,
            );
        }
    }

    fn check_policy(&mut self, decl: &PolicyDecl) {
        if decl.entries.is_empty() {
            self.result.warning(
                format!("Policy '{}' has no entries", decl.name),
                decl.span,
            );
        }
    }

    fn check_type_decl(&mut self, _decl: &TypeDecl) {}

    fn check_context(&mut self, decl: &ContextDecl) {
        for entry in &decl.entries {
            if entry.typ.is_none() {
                self.result.warning(
                    format!("Context entry '{}' has no type annotation", entry.name),
                    entry.span,
                );
            }
        }
    }

    fn check_permission(&mut self, decl: &PermissionDecl) {
        for entry in &decl.allow {
            if entry.path.is_empty() {
                self.result.warning(
                    "Permission allow entry has empty path".to_string(),
                    entry.span,
                );
            }
        }
        for entry in &decl.deny {
            if entry.path.is_empty() {
                self.result.warning(
                    "Permission deny entry has empty path".to_string(),
                    entry.span,
                );
            }
        }
    }

    fn check_approval(&mut self, decl: &ApprovalDecl) {
        if decl.gates.is_empty() {
            self.result.warning(
                "Approval declaration has no gates".to_string(),
                decl.span,
            );
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { typ, .. } => {
                if typ.is_none() {
                    self.result.warning(
                        "Let binding has no type annotation".to_string(),
                        stmt.span(),
                    );
                }
            }
            Stmt::ExprStmt { expr, .. } => {
                self.check_expr(expr);
            }
            Stmt::If { branches, else_body, .. } => {
                for branch in branches {
                    for s in &branch.body.stmts {
                        self.check_stmt(s);
                    }
                }
                if let Some(b) = else_body {
                    for s in &b.stmts {
                        self.check_stmt(s);
                    }
                }
            }
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    for s in &arm.body.stmts {
                        self.check_stmt(s);
                    }
                }
            }
            Stmt::Loop { body, .. } | Stmt::Parallel { body, .. } => {
                for s in &body.stmts {
                    self.check_stmt(s);
                }
            }
            Stmt::For { body, .. } => {
                for s in &body.stmts {
                    self.check_stmt(s);
                }
            }
            Stmt::Attempt { body, .. } => {
                for s in &body.stmts {
                    self.check_stmt(s);
                }
            }
            Stmt::Return { .. } | Stmt::Assign { .. } | Stmt::Const { .. }
            | Stmt::Pipeline { .. } | Stmt::Await { .. }
            | Stmt::When { .. } | Stmt::Break { .. } | Stmt::Continue { .. }
            | Stmt::Swarm { .. } => {}
            _ => {}
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call { callee, args, .. } => {
                self.check_expr(callee);
                for arg in args {
                    self.check_expr(&arg.value);
                }
            }
            Expr::BinaryOp { lhs, rhs, .. } => {
                self.check_expr(lhs);
                self.check_expr(rhs);
            }
            Expr::Var { .. } | Expr::Lit { .. } => {}
            _ => {}
        }
    }
}
