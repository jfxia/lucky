use crate::ast::expr::Expr;
use crate::ast::span::Span;
use crate::ast::stmt::*;
use crate::ast::pattern::Pattern;
use crate::ast::types::TypedIdent;
use crate::lexer::token::TokenKind;

use super::parser::Parser;

impl Parser {
    /// Parse a block: INDENT { statement } DEDENT.
    pub fn parse_block(&mut self) -> Block {
        let start = self.span();
        while self.kind() == TokenKind::Newline { self.bump(); }
        let has_indent = self.expect_indent();
        let mut stmts = Vec::new();

        if has_indent {
            while !self.is_eof() && !self.at_dedent() {
                // Skip comments at block level
                if self.kind() == TokenKind::Comment || self.kind() == TokenKind::DocComment {
                    self.bump();
                    continue;
                }
                if let Some(stmt) = self.parse_stmt() {
                    stmts.push(stmt);
                }
                // Skip any extra newlines between statements
                while self.kind() == TokenKind::Newline {
                    self.bump();
                }
            }
            self.eat_dedent();
        }

        let span = start.merge(self.span());
        Block::new(stmts, span)
    }

    /// Parse a workflow body after INDENT has already been consumed.
    pub fn parse_workflow_body_from_indent(&mut self, start: Span) -> Block {
        let mut stmts = Vec::new();

        while !self.is_eof() && !self.at_dedent() {
            // Handle sub-indented arrow markers
            if self.kind() == TokenKind::Indent {
                self.bump();
                if self.kind() == TokenKind::Arrow {
                    self.bump();
                    while self.kind() == TokenKind::Newline { self.bump(); }
                    if self.at_dedent() { self.bump(); }
                }
                continue;
            }

            if self.kind() == TokenKind::Arrow {
                self.bump();
                while self.kind() == TokenKind::Newline { self.bump(); }
                continue;
            }

            if self.at_dedent() { break; }

            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else {
                self.bump();
            }

            while self.kind() == TokenKind::Newline {
                self.bump();
            }
        }
        self.eat_dedent();

        let span = start.merge(self.span());
        Block::new(stmts, span)
    }

    /// Parse a workflow body: INDENT { step [-> step]* } DEDENT.
    /// Handles arrow-separated chains spread across lines.
    pub fn parse_workflow_body(&mut self) -> Block {
        let start = self.span();
        let has_indent = self.expect_indent();
        let mut stmts = Vec::new();

        if has_indent {
            while !self.is_eof() && !self.at_dedent() {
                // Handle sub-indented arrow markers: `    ->` between steps
                if self.kind() == TokenKind::Indent {
                    self.bump();
                    if self.kind() == TokenKind::Arrow {
                        self.bump(); // consume Arrow
                        while self.kind() == TokenKind::Newline { self.bump(); }
                        // Consume the DEDENT that closes this sub-indent
                        if self.at_dedent() { self.bump(); }
                    }
                    continue;
                }

                if self.kind() == TokenKind::Arrow {
                    self.bump();
                    while self.kind() == TokenKind::Newline { self.bump(); }
                    continue;
                }

                if self.at_dedent() { break; }

                if let Some(stmt) = self.parse_stmt() {
                    stmts.push(stmt);
                } else {
                    self.bump();
                }

                while self.kind() == TokenKind::Newline {
                    self.bump();
                }
            }
            self.eat_dedent();
        }

        let span = start.merge(self.span());
        Block::new(stmts, span)
    }

    /// Parse a single statement.
    pub fn parse_stmt(&mut self) -> Option<Stmt> {
        if self.is_eof() || self.at_dedent() {
            return None;
        }

        // Skip standalone comments at statement level
        if self.kind() == TokenKind::Comment || self.kind() == TokenKind::DocComment {
            self.bump();
            return None;
        }

        match self.kind() {
            TokenKind::Keyword => self.parse_keyword_stmt(),
            _ => {
                // Expression statement, assignment, or pipeline
                let start = self.span();
                let expr = self.parse_expr();

                // Check for assignment
                if self.kind() == TokenKind::Eq {
                    self.bump();
                    let value = self.parse_expr();
                    let span = start.merge(value.span());
                    Some(Stmt::Assign { target: expr, value, span })
                } else if self.kind() == TokenKind::Pipe {
                    // Pipeline continuation
                    let stages = self.collect_pipeline_stages(expr);
                    let span = start.merge(self.span());
                    Some(Stmt::Pipeline { stages, span })
                } else {
                    let span = start.merge(expr.span());
                    Some(Stmt::ExprStmt { expr, span })
                }
            }
        }
    }

    fn parse_keyword_stmt(&mut self) -> Option<Stmt> {
        match self.text() {
            "let" => self.parse_let_stmt(),
            "const" => self.parse_const_stmt(),
            "if" => self.parse_if_stmt(),
            "match" => self.parse_match_stmt(),
            "loop" => self.parse_loop_stmt(),
            "for" => self.parse_for_stmt(),
            "parallel" => self.parse_parallel_stmt(),
            "await" => self.parse_await_stmt(),
            "when" => self.parse_when_stmt(),
            "return" => self.parse_return_stmt(),
            "break" => self.parse_break_stmt(),
            "continue" => self.parse_continue_stmt(),
            "attempt" => self.parse_attempt_stmt(),
            "swarm" => self.parse_swarm_stmt(),
            _ => {
                let start = self.span();
                let expr = self.parse_expr();
                let span = start.merge(expr.span());
                Some(Stmt::ExprStmt { expr, span })
            }
        }
    }

    fn parse_let_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'let'
        let (name, _) = self.expect_ident("let binding")?;
        let typ = if self.kind() == TokenKind::Colon {
            self.bump();
            Some(Box::new(self.parse_type_expr()))
        } else {
            None
        };
        self.expect(TokenKind::Eq, "let binding");
        let value = self.parse_expr();
        let span = start.merge(value.span());
        Some(Stmt::Let { name, typ, value, span })
    }

    fn parse_const_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'const'
        let (name, _) = self.expect_ident("const binding")?;
        let typ = if self.kind() == TokenKind::Colon {
            self.bump();
            Some(Box::new(self.parse_type_expr()))
        } else {
            None
        };
        self.expect(TokenKind::Eq, "const binding");
        let value = self.parse_expr();
        let span = start.merge(value.span());
        Some(Stmt::Const { name, typ, value, span })
    }

    fn parse_if_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'if'
        let cond = self.parse_expr();
        let then_body = self.parse_block();

        let mut branches = vec![IfBranch { condition: cond, body: then_body }];
        let mut else_body = None;

        // Parse elif/else chains
        while self.is_keyword("else") {
            self.bump();
            if self.is_keyword("if") {
                self.bump();
                let cond = self.parse_expr();
                let body = self.parse_block();
                branches.push(IfBranch { condition: cond, body });
            } else {
                else_body = Some(self.parse_block());
                break;
            }
        }

        let span = start.merge(self.span());
        Some(Stmt::If { branches, else_body, span })
    }

    fn parse_match_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'match'
        let scrutinee = self.parse_expr();
        let mut arms = Vec::new();

        if self.kind() == TokenKind::Indent {
            self.bump(); // INDENT
            while !self.is_eof() && !self.at_dedent() {
                let arm_start = self.span();
                let pattern = self.parse_pattern();

                // Optional guard
                let guard = if self.is_keyword("if") {
                    self.bump();
                    Some(self.parse_expr())
                } else {
                    None
                };

                // The body is an indented block
                let body = self.parse_block();
                let span = arm_start.merge(body.span);
                arms.push(MatchArm { pattern, guard, body, span });
            }
            self.eat_dedent();
        }

        let span = start.merge(self.span());
        Some(Stmt::Match { scrutinee, arms, span })
    }

    fn parse_loop_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'loop'
        let body = self.parse_block();
        let span = start.merge(body.span);
        Some(Stmt::Loop { body, span })
    }

    fn parse_for_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'for'
        let pattern = self.parse_pattern();
        self.expect_keyword("in", "for loop");
        let iterable = self.parse_expr();
        let body = self.parse_block();
        let span = start.merge(body.span);
        Some(Stmt::For { pattern, iterable, body, span })
    }

    fn parse_parallel_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'parallel'
        let body = self.parse_block();
        let has_wait = self.is_keyword("wait");
        if has_wait {
            self.bump();
        }
        let span = start.merge(self.span());
        Some(Stmt::Parallel { body, has_wait, span })
    }

    fn parse_await_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'await'
        let expr = self.parse_expr();
        let span = start.merge(expr.span());
        Some(Stmt::Await { expr, span })
    }

    fn parse_when_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'when'
        let mut conditions = Vec::new();

        while !self.is_eof() && !self.is_keyword("run") {
            conditions.push(self.parse_expr());
            // Skip newlines between conditions
            while self.kind() == TokenKind::Newline { self.bump(); }
        }

        self.expect_keyword("run", "when block");
        let body = self.parse_block();
        let span = start.merge(body.span);
        Some(Stmt::When { conditions, body, span })
    }

    fn parse_return_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'return'
        let value = if self.at_stmt_end() {
            None
        } else {
            Some(self.parse_expr())
        };
        let span = start.merge(self.span());
        Some(Stmt::Return { value, span })
    }

    fn parse_break_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'break'
        let (label, value) = if !self.at_stmt_end() && self.is_ident() {
            let name = self.text().to_string();
            self.bump();
            if !self.at_stmt_end() {
                (Some(name), Some(self.parse_expr()))
            } else {
                (Some(name), None)
            }
        } else if !self.at_stmt_end() {
            (None, Some(self.parse_expr()))
        } else {
            (None, None)
        };
        let span = start.merge(self.span());
        Some(Stmt::Break { label, value, span })
    }

    fn parse_continue_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'continue'
        let label = if !self.at_stmt_end() && self.is_ident() {
            let name = self.text().to_string();
            self.bump();
            Some(name)
        } else {
            None
        };
        let span = start.merge(self.span());
        Some(Stmt::Continue { label, span })
    }

    fn parse_attempt_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'attempt'
        let body = self.parse_block();

        let mut recovery_blocks = Vec::new();

        while self.is_keyword("recover") {
            self.bump();
            let recovery = self.parse_block();
            let mut actions = Vec::new();

            for stmt in &recovery.stmts {
                match stmt {
                    Stmt::ExprStmt { expr, .. } => {
                        // Determine recovery action from expression text
                        if let Expr::Var { name, .. } = expr {
                            match name.last() {
                                "retry" => actions.push(RecoveryAction::Retry {
                                    count: None, backoff: None, max_delay: None,
                                    span: name.span,
                                }),
                                "fallback" => actions.push(RecoveryAction::Fallback {
                                    task: Expr::Error { span: name.span },
                                    span: name.span,
                                }),
                                "human" => actions.push(RecoveryAction::Human {
                                    message: None, span: name.span,
                                }),
                                "abort" => actions.push(RecoveryAction::Abort {
                                    span: name.span,
                                }),
                                "skip" => actions.push(RecoveryAction::Skip {
                                    span: name.span,
                                }),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }

            recovery_blocks.push(actions);
        }

        let span = start.merge(self.span());
        Some(Stmt::Attempt { body, recovery_blocks, span })
    }

    fn parse_swarm_stmt(&mut self) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // 'swarm'
        let count = self.parse_expr();
        let target = self.parse_expr();
        let span = start.merge(target.span());
        Some(Stmt::Swarm { count, target, span })
    }

    fn collect_pipeline_stages(&mut self, _first: Expr) -> Vec<PipelineStage> {
        let mut stages = Vec::new();
        // After a pipeline operator, collect remaining stages
        while self.kind() == TokenKind::Pipe {
            self.bump();
            if self.is_ident() {
                let name = self.text().to_string();
                let span = self.span();
                self.bump();
                let mut args = Vec::new();
                // Parse arguments until newline
                while !self.at_stmt_end() {
                    args.push(self.parse_expr());
                }
                stages.push(PipelineStage { operation: name, args, span });
            }
        }
        stages
    }
}
