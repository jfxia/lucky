use crate::ast::*;
use crate::ast::span::Span;
use crate::ast::types::TypedIdent;
use crate::lexer::token::TokenKind;

use super::parser::Parser;

impl Parser {
    /// Parse a complete module.
    pub fn parse_module(&mut self) -> Module {
        let start = self.span();
        let project = if self.is_keyword("project") {
            Some(self.parse_project_decl())
        } else {
            None
        };

        let mut items = Vec::new();

        while !self.is_eof() {
            // Skip newlines between declarations
            while self.kind() == TokenKind::Newline { self.bump(); }
            if self.is_eof() { break; }

            match self.parse_module_item() {
                Some(item) => items.push(item),
                None => {
                    // Error recovery: skip to next declaration keyword
                    self.recover_to_decl();
                }
            }
        }

        let span = start.merge(self.span());
        Module { project, items, span }
    }

    fn recover_to_decl(&mut self) {
        let decl_keywords = [
            "import", "agent", "task", "workflow", "goal", "memory",
            "tool", "model", "prompt", "policy", "type", "context",
            "permissions", "approval", "pub", "project",
        ];
        while !self.is_eof() {
            if self.kind() == TokenKind::Keyword && decl_keywords.contains(&self.text()) {
                return;
            }
            self.bump();
        }
    }

    fn parse_module_item(&mut self) -> Option<ModuleItem> {
        // Handle `pub` visibility modifier
        let is_pub = self.is_keyword("pub");
        if is_pub { self.bump(); }

        let result = if !self.is_keyword("") && self.kind() == TokenKind::Keyword {
            match self.text() {
                "import" => Some(ModuleItem::Import(self.parse_import_decl()?)),
                "agent" => Some(ModuleItem::Agent(self.parse_agent_decl()?)),
                "task" => Some(ModuleItem::Task(self.parse_task_decl()?)),
                "workflow" => Some(ModuleItem::Workflow(self.parse_workflow_decl()?)),
                "goal" => Some(ModuleItem::Goal(self.parse_goal_decl()?)),
                "memory" => Some(ModuleItem::Memory(self.parse_memory_decl()?)),
                "tool" => Some(ModuleItem::Tool(self.parse_tool_decl()?)),
                "model" => Some(ModuleItem::Model(self.parse_model_decl()?)),
                "prompt" => Some(ModuleItem::Prompt(self.parse_prompt_decl()?)),
                "policy" => Some(ModuleItem::Policy(self.parse_policy_decl()?)),
                "type" => Some(ModuleItem::Type(self.parse_type_decl()?)),
                "context" => Some(ModuleItem::Context(self.parse_context_decl()?)),
                "permissions" => Some(ModuleItem::Permission(self.parse_permission_decl()?)),
                "approval" => Some(ModuleItem::Approval(self.parse_approval_decl()?)),
                _ => {
                    self.error(format!("Unexpected keyword '{}' at module level", self.text()));
                    self.bump();
                    None
                }
            }
        } else {
            None
        };

        if is_pub && result.is_none() {
            self.error("Expected declaration after 'pub'".to_string());
        }

        result
    }

    // --- Project ---

    fn parse_project_decl(&mut self) -> ProjectDecl {
        let start = self.span();
        self.bump(); // 'project'
        let (name, _) = self.expect_ident("project name").unwrap_or_else(|| ("unknown".into(), start));
        let span = start.merge(self.span());
        ProjectDecl { name, span }
    }

    // --- Import ---

    fn parse_import_decl(&mut self) -> Option<ImportDecl> {
        let start = self.span();
        self.bump(); // 'import'
        let path = self.parse_qualified_name()?;
        let select = if self.kind() == TokenKind::LBrace {
            self.bump();
            let mut names = Vec::new();
            while !self.is_eof() && self.kind() != TokenKind::RBrace {
                if let Some((name, _)) = self.expect_ident("import name") {
                    names.push(name);
                }
                if self.kind() == TokenKind::Comma { self.bump(); }
            }
            self.expect(TokenKind::RBrace, "import select");
            ImportSelect::Named(names)
        } else if self.kind() == TokenKind::Dot && self.peek_kind(1) == TokenKind::Star {
            self.bump(); // '.'
            self.bump(); // '*'
            ImportSelect::All
        } else {
            ImportSelect::Nothing
        };

        let alias = if self.is_keyword("as") {
            self.bump();
            self.expect_ident("import alias").map(|(name, _)| name)
        } else {
            None
        };

        let span = start.merge(self.span());
        Some(ImportDecl { path, select, alias, span })
    }

    // --- Agent ---

    fn parse_agent_decl(&mut self) -> Option<AgentDecl> {
        let start = self.span();
        self.bump(); // 'agent'
        let (name, _) = self.expect_ident("agent name")?;

        let mut model = None;
        let mut memory_ref = None;
        let mut tools = Vec::new();
        let mut permissions = None;
        let mut policy = None;
        let mut prompt = None;
        let mut tasks = Vec::new();

        if self.kind() == TokenKind::Indent {
            self.bump(); // INDENT
            while !self.is_eof() && !self.at_dedent() {
                // Check for top-level declarations at the same level (end of agent body)
                if self.kind() == TokenKind::Keyword && !self.is_keyword("model")
                    && !self.is_keyword("memory") && !self.is_keyword("tools")
                    && !self.is_keyword("permissions") && !self.is_keyword("policy")
                    && !self.is_keyword("prompt") && !self.is_keyword("task") {
                    break;
                }
                if self.is_keyword("model") {
                    self.bump();
                    model = self.expect_ident("model name").map(|(n, s)| QualifiedName::simple(&n, s));
                } else if self.is_keyword("memory") {
                    self.bump();
                    memory_ref = self.expect_ident("memory name").map(|(n, s)| QualifiedName::simple(&n, s));
                } else if self.is_keyword("tools") {
                    self.bump();
                    // Parse tool references: `Tools, Tool2` possibly across lines
                    loop {
                        while self.kind() == TokenKind::Newline || self.kind() == TokenKind::Comma {
                            self.bump();
                        }
                        if self.at_dedent() || !self.is_ident() && self.kind() != TokenKind::Keyword {
                            break;
                        }
                        if self.is_keyword("model") || self.is_keyword("memory")
                            || self.is_keyword("permissions") || self.is_keyword("policy")
                            || self.is_keyword("prompt") || self.is_keyword("task") {
                            break;
                        }
                        let tname = self.text().to_string();
                        let tspan = self.span();
                        self.bump();
                        tools.push(QualifiedName::simple(&tname, tspan));
                    }
                } else if self.is_keyword("permissions") {
                    permissions = Some(self.parse_permission_decl()?);
                } else if self.is_keyword("policy") {
                    self.bump();
                    if let Some((n, s)) = self.expect_ident("policy name") {
                        policy = Some(QualifiedName::simple(&n, s));
                    }
                } else if self.is_keyword("prompt") {
                    self.bump();
                    if let Some((n, s)) = self.expect_ident("prompt name") {
                        prompt = Some(QualifiedName::simple(&n, s));
                    }
                } else if self.is_keyword("task") {
                    if let Some(task) = self.parse_task_decl() {
                        tasks.push(task);
                    }
                } else {
                    // Unknown token in agent body - skip to next recognizable keyword or dedent
                    self.error(format!("Unexpected token '{}' in agent body", self.text()));
                    self.bump();
                    while !self.is_eof() && !self.at_dedent()
                        && !self.is_keyword("model") && !self.is_keyword("memory")
                        && !self.is_keyword("tools") && !self.is_keyword("permissions")
                        && !self.is_keyword("policy") && !self.is_keyword("prompt")
                        && !self.is_keyword("task") {
                        self.bump();
                    }
                }
                while self.kind() == TokenKind::Newline { self.bump(); }
            }
            self.eat_dedent();
        }

        let span = start.merge(self.span());
        Some(AgentDecl { name, model, memory: memory_ref, tools, permissions, policy, prompt, tasks, span })
    }

    // --- Task ---

    fn parse_task_decl(&mut self) -> Option<TaskDecl> {
        let start = self.span();
        self.bump(); // 'task'
        let (name, _) = self.expect_ident("task name")?;

        let mut is_stateful = false;
        // Check for (stateful) modifier
        if self.kind() == TokenKind::LParen {
            self.bump();
            if self.is_keyword("stateful") {
                is_stateful = true;
                self.bump();
            }
            self.expect(TokenKind::RParen, "task modifier");
        }

        let mut type_params = Vec::new();
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut context = Vec::new();
        let mut policy = None;
        let mut steps = None;
        let mut rollback = None;

        if self.kind() == TokenKind::Indent {
            self.bump(); // INDENT
            while !self.is_eof() && !self.at_dedent() {
                // If we encounter a top-level keyword, we've exited this scope
                let is_top_level = self.is_keyword("agent") || self.is_keyword("task")
                    || self.is_keyword("workflow") || self.is_keyword("goal")
                    || self.is_keyword("memory") || self.is_keyword("tool")
                    || self.is_keyword("model") || self.is_keyword("prompt")
                    || self.is_keyword("policy") || self.is_keyword("permissions")
                    || self.is_keyword("project") || self.is_keyword("import")
                    || self.is_keyword("type") || self.is_keyword("context")
                    || self.is_keyword("approval") || self.is_keyword("pub");
                if is_top_level { break; }
                match self.text() {
                    "input" => {
                        self.bump();
                        self.parse_typed_idents(&mut inputs);
                    }
                    "output" => {
                        self.bump();
                        self.parse_typed_idents(&mut outputs);
                    }
                    "context" => {
                        self.bump();
                        self.parse_typed_idents(&mut context);
                    }
                    "policy" => {
                        self.bump();
                        if let Some((n, s)) = self.expect_ident("policy name") {
                            policy = Some(QualifiedName::simple(&n, s));
                        }
                    }
                    "steps" => {
                        self.bump();
                        while self.kind() == TokenKind::Newline { self.bump(); }
                        steps = Some(self.parse_block());
                    }
                    "rollback" => {
                        self.bump();
                        while self.kind() == TokenKind::Newline { self.bump(); }
                        rollback = Some(self.parse_block());
                    }
                    _ => {
                        self.error(format!("Unexpected '{}' in task body", self.text()));
                        // Skip to next section keyword or end of task
                        while !self.is_eof() && !self.at_dedent()
                            && !self.is_keyword("input") && !self.is_keyword("output")
                            && !self.is_keyword("context") && !self.is_keyword("policy")
                            && !self.is_keyword("steps") && !self.is_keyword("rollback") {
                            self.bump();
                        }
                    }
                }
                while self.kind() == TokenKind::Newline { self.bump(); }
            }
            if self.kind() == TokenKind::Dedent { self.bump(); }
        }

        let span = start.merge(self.span());
        Some(TaskDecl { name, is_stateful, type_params, inputs, outputs, context, policy, steps, rollback, span })
    }

    fn parse_typed_idents(&mut self, list: &mut Vec<TypedIdent>) {
        while self.is_ident() || (self.kind() == TokenKind::Keyword && !self.at_dedent()) {
            if let Some((name, name_span)) = self.expect_ident("typed identifier") {
                let typ = if self.kind() == TokenKind::Colon {
                    self.bump();
                    Some(Box::new(self.parse_type_expr()))
                } else {
                    None
                };
                // Check for default value
                if self.kind() == TokenKind::Eq {
                    self.bump();
                    let _default = self.parse_expr(); // consume but don't store in AST yet
                }
                list.push(TypedIdent { name, typ, span: name_span });
            }
            if self.kind() == TokenKind::Comma { self.bump(); }
            while self.kind() == TokenKind::Newline { self.bump(); }
            if self.at_dedent() || self.is_keyword("input") || self.is_keyword("output")
                || self.is_keyword("context") || self.is_keyword("policy")
                || self.is_keyword("steps") || self.is_keyword("rollback") {
                break;
            }
        }
    }

    // --- Workflow ---

    fn parse_workflow_decl(&mut self) -> Option<WorkflowDecl> {
        let start = self.span();
        self.bump(); // 'workflow'
        let (name, _) = self.expect_ident("workflow name")?;
        let mut context = Vec::new();

        // Skip any intervening newlines before the body
        while self.kind() == TokenKind::Newline { self.bump(); }

        let body = self.parse_block();
        let span = start.merge(body.span);
        Some(WorkflowDecl { name, context, body, span })
    }

    // --- Goal ---

    fn parse_goal_decl(&mut self) -> Option<GoalDecl> {
        let start = self.span();
        self.bump(); // 'goal'
        let (name, _) = self.expect_ident("goal name")?;
        let mut success_criteria = Vec::new();
        let mut workflows = Vec::new();

        if self.kind() == TokenKind::Indent {
            self.bump();
            while !self.is_eof() && !self.at_dedent() {
                if self.is_keyword("success") {
                    self.bump();
                    while !self.is_eof() && !self.at_dedent() && !self.is_keyword("workflow") {
                        if self.is_ident() {
                            success_criteria.push(self.text().to_string());
                        }
                        self.bump();
                        // If we hit EOF, break out
                        if self.is_eof() { break; }
                    }
                } else if self.is_keyword("workflow") {
                    self.bump();
                    if let Some((n, _)) = self.expect_ident("workflow name") {
                        workflows.push(n);
                    }
                } else {
                    self.bump();
                }
                // Skip newlines; break if EOF
                while self.kind() == TokenKind::Newline { self.bump(); }
                if self.is_eof() { break; }
            }
            if self.kind() == TokenKind::Dedent { self.bump(); }
        }

        let span = start.merge(self.span());
        Some(GoalDecl { name, success_criteria, workflows, span })
    }

    // --- Memory ---

    fn parse_memory_decl(&mut self) -> Option<MemoryDecl> {
        let start = self.span();
        self.bump(); // 'memory'
        let (name, _) = self.expect_ident("memory name")?;
        let mut scope = None;
        let mut backend = None;
        let mut config = Vec::new();

        if self.kind() == TokenKind::Indent {
            self.bump();
            while !self.is_eof() && !self.at_dedent() {
                if self.at_dedent() { break; }
                let key = if self.is_ident() || self.kind() == TokenKind::Keyword {
                    let t = self.text().to_string();
                    self.bump();
                    t
                } else {
                    self.bump();
                    continue;
                };

                // Skip to next line if key doesn't have a value on the same line
                if self.kind() == TokenKind::Newline {
                    // Key without value, like `scope` alone
                    continue;
                }

                let value = if self.kind() == TokenKind::Eq {
                    self.bump();
                    self.parse_expr()
                } else {
                    // Value on same line
                    let v = if self.is_ident() || self.kind() == TokenKind::Keyword {
                        let t = self.text().to_string();
                        self.bump();
                        t
                    } else {
                        self.bump();
                        continue;
                    };
                    // Create a dummy Var expression as value
                    Expr::Var { name: QualifiedName::simple(&v, Span::DUMMY), span: Span::DUMMY }
                };

                match key.as_str() {
                    "scope" => {
                        if let Expr::Var { name, .. } = &value { scope = Some(name.last().to_string()); }
                    }
                    "backend" => {
                        if let Expr::Var { name, .. } = &value { backend = Some(name.last().to_string()); }
                    }
                    _ => { config.push((key, value)); }
                }
                while self.kind() == TokenKind::Newline { self.bump(); }
            }
            self.eat_dedent();
        }

        let span = start.merge(self.span());
        Some(MemoryDecl { name, scope, backend, config, span })
    }

    // --- Tool ---

    fn parse_tool_decl(&mut self) -> Option<ToolDecl> {
        let start = self.span();
        self.bump(); // 'tool'
        let (name, _) = self.expect_ident("tool name")?;
        let mut config = Vec::new();
        let mut methods = Vec::new();

        // Parse tool config params: `tool Git(workdir = "./repo")`
        if self.kind() == TokenKind::LParen {
            self.bump();
            while !self.is_eof() && self.kind() != TokenKind::RParen {
                if let Some((key, _)) = self.expect_ident("tool param key") {
                    if self.kind() == TokenKind::Eq {
                        self.bump();
                        let value = self.parse_expr();
                        config.push((key, value));
                    }
                }
                if self.kind() == TokenKind::Comma { self.bump(); }
            }
            self.expect(TokenKind::RParen, "tool parameters");
        }

        let span = start.merge(self.span());
        Some(ToolDecl { name, config, methods, span })
    }

    // --- Model ---

    fn parse_model_decl(&mut self) -> Option<ModelDecl> {
        let start = self.span();
        self.bump(); // 'model'
        let (name, _) = self.expect_ident("model name")?;
        let mut config = Vec::new();

        if self.kind() == TokenKind::LParen {
            self.bump();
            while !self.is_eof() && self.kind() != TokenKind::RParen {
                if let Some((key, _)) = self.expect_ident("model config key") {
                    if self.kind() == TokenKind::Eq {
                        self.bump();
                        let value = self.parse_expr();
                        config.push((key, value));
                    }
                }
                if self.kind() == TokenKind::Comma { self.bump(); }
            }
            self.expect(TokenKind::RParen, "model parameters");
        }

        let span = start.merge(self.span());
        Some(ModelDecl { name, config, span })
    }

    // --- Prompt ---

    fn parse_prompt_decl(&mut self) -> Option<PromptDecl> {
        let start = self.span();
        self.bump(); // 'prompt'
        let (name, _) = self.expect_ident("prompt name")?;
        let mut sections = Vec::new();

        if self.kind() == TokenKind::Indent {
            self.bump();
            while !self.is_eof() && !self.at_dedent() {
                if self.is_keyword("role") {
                    self.bump();
                    let text = self.collect_text_block();
                    sections.push(PromptSection::Role { text, span: start });
                } else if self.is_keyword("rules") {
                    self.bump();
                    let mut items = Vec::new();
                    while !self.at_dedent() && self.kind() != TokenKind::Newline {
                        items.push(self.collect_text_block());
                    }
                    sections.push(PromptSection::Rules { items, span: start });
                } else if self.is_keyword("context") {
                    self.bump();
                    let text = self.collect_text_block();
                    sections.push(PromptSection::Context { text, span: start });
                } else if self.is_keyword("examples") {
                    self.bump();
                    // Collect example pairs (input/output)
                    let mut pairs = Vec::new();
                    sections.push(PromptSection::Examples { pairs, span: start });
                } else if self.is_keyword("format") {
                    self.bump();
                    let text = self.collect_text_block();
                    sections.push(PromptSection::Format { text, span: start });
                } else {
                    self.bump();
                }
                while self.kind() == TokenKind::Newline { self.bump(); }
            }
            self.eat_dedent();
        }

        let span = start.merge(self.span());
        Some(PromptDecl { name, sections, span })
    }

    fn collect_text_block(&mut self) -> String {
        let mut lines = Vec::new();
        while !self.is_eof() && !self.at_dedent() && !self.is_keyword("role")
            && !self.is_keyword("rules") && !self.is_keyword("context")
            && !self.is_keyword("examples") && !self.is_keyword("format") {
            if self.kind() == TokenKind::Newline {
                self.bump();
                continue;
            }
            lines.push(self.text().to_string());
            self.bump();
        }
        lines.join(" ")
    }

    // --- Policy ---

    fn parse_policy_decl(&mut self) -> Option<PolicyDecl> {
        let start = self.span();
        self.bump(); // 'policy'
        let (name, _) = self.expect_ident("policy name")?;
        let mut entries = Vec::new();

        if self.kind() == TokenKind::Indent {
            self.bump();
            while !self.is_eof() && !self.at_dedent() && self.is_ident() {
                let key = self.text().to_string();
                let key_span = self.span();
                self.bump();
                let value = self.parse_expr();

                let entry = match key.as_str() {
                    "retry" => PolicyEntry::Retry { count: 3, backoff: None, max_delay: None, span: key_span },
                    "timeout" => PolicyEntry::Timeout { duration: value, span: key_span },
                    "checkpoint" => PolicyEntry::Checkpoint { trigger: CheckpointTrigger::AfterEachTask, span: key_span },
                    "cache" => PolicyEntry::Cache { ttl: None, span: key_span },
                    "sandbox" => PolicyEntry::Sandbox { enabled: true, span: key_span },
                    "model" => {
                        if let Expr::Var { name, .. } = &value {
                            PolicyEntry::Model { name: name.last().to_string(), span: key_span }
                        } else {
                            PolicyEntry::Other { key, value: format!("{:?}", value), span: key_span }
                        }
                    }
                    "cost_limit" => PolicyEntry::CostLimit { amount: value, span: key_span },
                    "priority" => PolicyEntry::Priority { level: "normal".into(), span: key_span },
                    _ => PolicyEntry::Other { key, value: format!("{:?}", value), span: key_span },
                };
                entries.push(entry);
                while self.kind() == TokenKind::Newline { self.bump(); }
            }
            self.eat_dedent();
        }

        let span = start.merge(self.span());
        Some(PolicyDecl { name, entries, span })
    }

    // --- Type ---

    fn parse_type_decl(&mut self) -> Option<TypeDecl> {
        let start = self.span();
        self.bump(); // 'type'
        let (name, _) = self.expect_ident("type name")?;
        let mut type_params = Vec::new();

        if self.kind() == TokenKind::Lt {
            self.bump();
            while !self.is_eof() && self.kind() != TokenKind::Gt {
                if let Some((n, _)) = self.expect_ident("type parameter") {
                    type_params.push(n);
                }
                if self.kind() == TokenKind::Comma { self.bump(); }
            }
            self.expect(TokenKind::Gt, "type parameters");
        }

        self.expect(TokenKind::Eq, "type definition");
        let typ = Box::new(self.parse_type_expr());
        let span = start.merge(typ.span());
        Some(TypeDecl { name, type_params, typ, span })
    }

    // --- Context ---

    fn parse_context_decl(&mut self) -> Option<ContextDecl> {
        let start = self.span();
        self.bump(); // 'context'
        let mut entries = Vec::new();
        self.parse_typed_idents(&mut entries);
        let span = start.merge(self.span());
        Some(ContextDecl { entries, span })
    }

    // --- Permissions ---

    fn parse_permission_decl(&mut self) -> Option<PermissionDecl> {
        let start = self.span();
        let is_permissions = self.text() == "permissions";
        if is_permissions { self.bump(); }

        let mut allow = Vec::new();
        let mut deny = Vec::new();

        if self.kind() == TokenKind::Indent {
            self.bump();
            let mut allow_entries = Vec::new();
            let mut deny_entries = Vec::new();
            let mut current_is_allow = true;
            while !self.is_eof() && !self.at_dedent() {
                if self.is_keyword("allow") {
                    self.bump();
                    current_is_allow = true;
                    while self.kind() != TokenKind::Newline && !self.at_dedent() {
                        let entry = self.parse_permission_entry();
                        allow_entries.push(entry);
                        if self.kind() == TokenKind::Comma { self.bump(); }
                    }
                } else if self.is_keyword("deny") {
                    self.bump();
                    current_is_allow = false;
                    while self.kind() != TokenKind::Newline && !self.at_dedent() {
                        let entry = self.parse_permission_entry();
                        deny_entries.push(entry);
                        if self.kind() == TokenKind::Comma { self.bump(); }
                    }
                } else {
                    let entry = self.parse_permission_entry();
                    if current_is_allow {
                        allow_entries.push(entry);
                    } else {
                        deny_entries.push(entry);
                    }
                    if self.kind() == TokenKind::Comma { self.bump(); }
                }
                while self.kind() == TokenKind::Newline { self.bump(); }
            }
            self.eat_dedent();
            allow = allow_entries;
            deny = deny_entries;
        }

        let span = start.merge(self.span());
        Some(PermissionDecl { allow, deny, span })
    }

    fn parse_permission_entry(&mut self) -> PermissionEntry {
        let start = self.span();
        let mut path = Vec::new();

        while self.is_ident() {
            path.push(self.text().to_string());
            self.bump();
            if self.kind() == TokenKind::Dot {
                self.bump();
            } else {
                break;
            }
        }

        let span = start.merge(self.span());
        PermissionEntry { path, span }
    }

    // --- Approval ---

    fn parse_approval_decl(&mut self) -> Option<ApprovalDecl> {
        let start = self.span();
        self.bump(); // 'approval'
        let mut gates = Vec::new();

        if self.kind() == TokenKind::Indent {
            self.bump();
            while !self.is_eof() && !self.at_dedent() {
                if self.is_keyword("before") {
                    self.bump();
                    let mut op_parts = Vec::new();
                    while !self.at_dedent() && self.is_ident() {
                        op_parts.push(self.text().to_string());
                        self.bump();
                    }
                    let operation = op_parts.join(" ");
                    gates.push(ApprovalGate { operation, timeout: None, escalation: None, span: start });
                }
                while self.kind() == TokenKind::Newline { self.bump(); }
            }
            self.eat_dedent();
        }

        let span = start.merge(self.span());
        Some(ApprovalDecl { gates, span })
    }

    // --- Qualified names ---

    fn parse_qualified_name(&mut self) -> Option<QualifiedName> {
        let start = self.span();
        let (first, _) = self.expect_ident("qualified name")?;
        let mut parts = vec![first];

        while self.kind() == TokenKind::Dot {
            self.bump();
            if let Some((name, _)) = self.expect_ident("qualified name segment") {
                parts.push(name);
            } else {
                break;
            }
        }

        let span = start.merge(self.span());
        Some(QualifiedName::new(parts, span))
    }
}
