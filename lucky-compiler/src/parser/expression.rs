use crate::ast::expr::*;
use crate::ast::literal::Literal;
use crate::ast::span::Span;
use crate::ast::stmt::Block;
use crate::lexer::token::TokenKind;

use super::parser::Parser;

const RANGE_PREC: u8 = 3;

impl Parser {
    /// Parse an expression using Pratt parsing.
    pub fn parse_expr(&mut self) -> Expr {
        self.parse_expr_prec(0)
    }

    fn parse_expr_prec(&mut self, min_prec: u8) -> Expr {
        let mut lhs = self.parse_prefix();

        loop {
            // Check for multi-line pipeline: NEWLINE + [INDENT] + Pipe
            if self.kind() == TokenKind::Newline
                && (self.peek_kind(1) == TokenKind::Pipe
                    || (self.peek_kind(1) == TokenKind::Indent && self.peek_kind(2) == TokenKind::Pipe))
            {
                lhs = self.parse_pipeline(lhs);
                continue;
            }

            // Check for pipeline operator (same-line)
            if self.kind() == TokenKind::Pipe {
                lhs = self.parse_pipeline(lhs);
                continue;
            }

            // Check for binary operators
            let op = match self.kind() {
                TokenKind::Plus if !self.is_unary_context_plus() => Some(BinOp::Add),
                TokenKind::Minus if !self.is_unary_context_minus() => Some(BinOp::Sub),
                TokenKind::Star => Some(BinOp::Mul),
                TokenKind::Slash => Some(BinOp::Div),
                TokenKind::Percent => Some(BinOp::Rem),
                TokenKind::EqEq => Some(BinOp::Eq),
                TokenKind::Neq => Some(BinOp::Neq),
                TokenKind::Lt => Some(BinOp::Lt),
                TokenKind::Gt => Some(BinOp::Gt),
                TokenKind::Le => Some(BinOp::Le),
                TokenKind::Ge => Some(BinOp::Ge),
                TokenKind::Keyword if self.text() == "and" => Some(BinOp::And),
                TokenKind::Keyword if self.text() == "or" => Some(BinOp::Or),
                _ => None,
            };

            if let Some(op) = op {
                let prec = op.precedence();
                if prec < min_prec {
                    break;
                }
                let start = self.span();
                self.bump(); // consume operator
                let rhs = self.parse_expr_prec(prec + 1);
                let span = start.merge(rhs.span());
                lhs = Expr::BinaryOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
                continue;
            }

            // Check for nullable coalesce
            if self.kind() == TokenKind::NullCoalesce {
                let start = self.span();
                self.bump();
                let rhs = self.parse_expr_prec(1);
                let span = start.merge(rhs.span());
                lhs = Expr::NullCoalesce { expr: Box::new(lhs), default: Box::new(rhs), span };
                continue;
            }

            // Check for range expression: lhs..rhs or lhs..=rhs
            if self.kind() == TokenKind::DotDot || self.kind() == TokenKind::DotDotEq {
                let inclusive = self.kind() == TokenKind::DotDotEq;
                let token_span = self.span();
                self.bump();
                let rhs = self.parse_expr_prec(RANGE_PREC);
                let span = lhs.span().merge(rhs.span());
                lhs = Expr::Range { start: Some(Box::new(lhs)), end: Some(Box::new(rhs)), inclusive, span };
                continue;
            }

            // Check for confidence expression
            if self.kind() == TokenKind::Keyword && self.text() == "confidence" {
                lhs = self.parse_confidence(lhs);
                continue;
            }

            break;
        }

        lhs
    }

    fn is_unary_context_plus(&self) -> bool {
        // '+' after an expression start is binary, otherwise it's unary.
        // After operands, ')', ']', '}', identifiers, literals -> binary.
        // Otherwise -> unary.
        false // '+' as unary is a no-op, so we just treat it as binary
    }

    fn is_unary_context_minus(&self) -> bool {
        // '-' at the start of an expression or after '(' / '[' / ',' / operator is unary
        true // Simplify: always check in prefix parsing
    }

    // ---- Prefix expressions ----

    fn parse_prefix(&mut self) -> Expr {
        match self.kind() {
            TokenKind::IntLit => self.parse_int_lit(),
            TokenKind::FloatLit => self.parse_float_lit(),
            TokenKind::StringLit => self.parse_string_lit(),
            TokenKind::BoolLit => self.parse_bool_lit(),
            TokenKind::NullLit => self.parse_null_lit(),
            TokenKind::UnknownLit => self.parse_unknown_lit(),
            TokenKind::Ident | TokenKind::Keyword => self.parse_ident_or_keyword_prefix(),
            TokenKind::Minus => self.parse_unary(UnaryOp::Neg),
            TokenKind::LParen => self.parse_paren_or_tuple(),
            TokenKind::LBrack => self.parse_list(),
            TokenKind::LBrace => self.parse_set_or_map(),
            TokenKind::Colon => self.parse_lambda_prefix(),
            TokenKind::DotDot | TokenKind::DotDotEq => self.parse_range_prefix(),
            _ => {
                let span = self.span();
                self.error(format!("Unexpected token '{}' at start of expression", self.text()));
                self.bump();
                Expr::Error { span }
            }
        }
    }

    fn parse_int_lit(&mut self) -> Expr {
        let text = self.text().to_string();
        let span = self.span();
        self.bump();
        match text.parse::<i64>() {
            Ok(v) => Expr::Lit { value: Literal::Int(v), span },
            Err(_) => {
                self.error(format!("Invalid integer literal: {}", text));
                Expr::Error { span }
            }
        }
    }

    fn parse_float_lit(&mut self) -> Expr {
        let text = self.text().to_string();
        let span = self.span();
        self.bump();
        match text.parse::<f64>() {
            Ok(v) => Expr::Lit { value: Literal::Float(v), span },
            Err(_) => {
                self.error(format!("Invalid float literal: {}", text));
                Expr::Error { span }
            }
        }
    }

    fn parse_string_lit(&mut self) -> Expr {
        let text = self.text().to_string();
        let span = self.span();
        self.bump();
        Expr::Lit { value: Literal::String(text), span }
    }

    fn parse_bool_lit(&mut self) -> Expr {
        let b = self.text() == "true";
        let span = self.span();
        self.bump();
        Expr::Lit { value: Literal::Bool(b), span }
    }

    fn parse_null_lit(&mut self) -> Expr {
        let span = self.span();
        self.bump();
        Expr::Lit { value: Literal::Null, span }
    }

    fn parse_unknown_lit(&mut self) -> Expr {
        let span = self.span();
        self.bump();
        Expr::Lit { value: Literal::Unknown, span }
    }

    fn parse_ident_or_keyword_prefix(&mut self) -> Expr {
        let text = self.text().to_string();
        let start = self.span();

        // Check for special keyword expressions
        match text.as_str() {
            "fn" => return self.parse_lambda(),
            "if" => return self.parse_if_expr(),
            "match" => return self.parse_match_expr(),
            "ask" => return self.parse_ask_expr(),
            "reason" => return self.parse_reason_expr(),
            "use" => return self.parse_use_expr(),
            "not" => return self.parse_unary(UnaryOp::Not),
            "true" | "false" | "null" | "unknown" => {
                // These should have been tokenized as literals, but handle gracefully
                return self.parse_prefix();
            }
            _ => {}
        }

        self.bump();
        let mut expr = Expr::Var { name: QualifiedName::simple(&text, start), span: start };

        // Check for postfix: field access, call, index
        loop {
            match self.kind() {
                TokenKind::Dot => {
                    let dot_span = self.span();
                    self.bump();
                    if let Some((field, _)) = self.expect_ident("field access") {
                        let span = start.merge(self.span());
                        expr = Expr::FieldAccess { base: Box::new(expr), field, span };
                    }
                }
                TokenKind::NullableAccess => {
                    self.bump();
                    if let Some((field, _)) = self.expect_ident("nullable field access") {
                        let span = start.merge(self.span());
                        expr = Expr::NullableFieldAccess { base: Box::new(expr), field, span };
                    }
                }
                TokenKind::LParen => {
                    expr = self.parse_call(expr, start);
                }
                TokenKind::LBrack => {
                    expr = self.parse_index(expr, start, false);
                }
                TokenKind::NullableIndex => {
                    expr = self.parse_index_nullable(expr, start);
                }
                TokenKind::Question => {
                    self.bump();
                    let span = start.merge(self.span());
                    // Just the ? suffix for nullable �?needs type system
                    // For now, treat as passthrough
                }
                _ => break,
            }
        }

        expr
    }

    fn parse_call(&mut self, callee: Expr, start: Span) -> Expr {
        self.bump(); // consume '('
        let mut args = Vec::new();

        while !self.is_eof() && self.kind() != TokenKind::RParen {
            if self.kind() == TokenKind::Newline || self.kind() == TokenKind::Dedent
                || self.kind() == TokenKind::Indent {
                self.bump();
                continue;
            }

            // Check for named argument: `name = value`
            if self.is_ident() && self.peek_kind(1) == TokenKind::Eq {
                let name = self.text().to_string();
                let name_span = self.span();
                self.bump(); // ident
                self.bump(); // =
                let value = self.parse_expr();
                args.push(Arg { name: Some(name), value: Box::new(value), span: name_span });
            } else {
                let value = self.parse_expr();
                let span = value.span();
                args.push(Arg { name: None, value: Box::new(value), span });
            }

            if self.kind() == TokenKind::Comma {
                self.bump();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RParen, "function call arguments");
        let span = start.merge(self.span());
        Expr::Call { callee: Box::new(callee), args, span }
    }

    fn parse_index(&mut self, base: Expr, start: Span, _nullable: bool) -> Expr {
        self.bump(); // consume '['
        let index = self.parse_expr();
        self.expect(TokenKind::RBrack, "index expression");
        let span = start.merge(self.span());
        Expr::Index { base: Box::new(base), index: Box::new(index), span }
    }

    fn parse_index_nullable(&mut self, base: Expr, start: Span) -> Expr {
        self.bump(); // consume '?['
        let index = self.parse_expr();
        self.expect(TokenKind::RBrack, "nullable index expression");
        let span = start.merge(self.span());
        Expr::NullableIndex { base: Box::new(base), index: Box::new(index), span }
    }

    fn parse_unary(&mut self, op: UnaryOp) -> Expr {
        let start = self.span();
        self.bump();
        let expr = self.parse_expr_prec(6); // unary binds tightest
        let span = start.merge(expr.span());
        Expr::UnaryOp { op, expr: Box::new(expr), span }
    }

    fn parse_paren_or_tuple(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // '('
        if self.kind() == TokenKind::RParen {
            let span = start.merge(self.span());
            self.bump();
            return Expr::Paren { expr: Box::new(Expr::Error { span }), span };
        }

        let expr = self.parse_expr();
        if self.kind() == TokenKind::Comma {
            // Tuple
            let mut elements = vec![expr];
            while self.kind() == TokenKind::Comma {
                self.bump();
                if self.kind() == TokenKind::RParen { break; }
                elements.push(self.parse_expr());
            }
            self.expect(TokenKind::RParen, "tuple literal");
            let span = start.merge(self.span());
            // Tuples are represented as lists for now
            Expr::List { elements, span }
        } else {
            self.expect(TokenKind::RParen, "parenthesized expression");
            let span = start.merge(self.span());
            Expr::Paren { expr: Box::new(expr), span }
        }
    }

    fn parse_list(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // '['
        let mut elements = Vec::new();

        while !self.is_eof() && self.kind() != TokenKind::RBrack {
            if self.at_stmt_end() { break; }
            elements.push(self.parse_expr());
            if self.kind() == TokenKind::Comma {
                self.bump();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RBrack, "list literal");
        let span = start.merge(self.span());
        Expr::List { elements, span }
    }

    fn parse_set_or_map(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // '{'
        // Skip whitespace/indent after {
        while self.kind() == TokenKind::Newline || self.kind() == TokenKind::Indent
            || self.kind() == TokenKind::Dedent {
            self.bump();
        }
        let mut elements = Vec::new();
        let mut entries = Vec::new();
        let mut is_map = false;

        // Check if empty
        if self.kind() == TokenKind::RBrace {
            let span = start.merge(self.span());
            self.bump();
            // Empty {} could be set or map; default to Map (user can annotate)
            return Expr::Map { entries: vec![], span };
        }

        // Peek ahead to determine if this is a map or set
        // Map entry has form: expr : expr
        let first = self.parse_expr();
        if self.kind() == TokenKind::Colon {
            is_map = true;
            self.bump(); // ':'
            let value = self.parse_expr();
            entries.push((first, value));

            while self.kind() == TokenKind::Comma || self.kind() == TokenKind::Newline
                || self.kind() == TokenKind::Indent || self.kind() == TokenKind::Dedent {
                if self.kind() == TokenKind::Comma { self.bump(); }
                while self.kind() == TokenKind::Newline || self.kind() == TokenKind::Indent
                    || self.kind() == TokenKind::Dedent {
                    self.bump();
                }
                if self.kind() == TokenKind::RBrace { break; }
                let key = self.parse_expr();
                self.expect(TokenKind::Colon, "map entry");
                let value = self.parse_expr();
                entries.push((key, value));
            }
        } else {
            // Set
            elements.push(first);
            while self.kind() == TokenKind::Comma || self.kind() == TokenKind::Newline
                || self.kind() == TokenKind::Indent || self.kind() == TokenKind::Dedent {
                if self.kind() == TokenKind::Comma { self.bump(); }
                while self.kind() == TokenKind::Newline || self.kind() == TokenKind::Indent
                    || self.kind() == TokenKind::Dedent {
                    self.bump();
                }
                if self.kind() == TokenKind::RBrace { break; }
                elements.push(self.parse_expr());
            }
        }

        while self.kind() == TokenKind::Newline || self.kind() == TokenKind::Indent
            || self.kind() == TokenKind::Dedent {
            self.bump();
        }
        self.expect(TokenKind::RBrace, if is_map { "map literal" } else { "set literal" });
        let span = start.merge(self.span());

        if is_map {
            Expr::Map { entries, span }
        } else {
            Expr::Set { elements, span }
        }
    }

    fn parse_lambda(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // 'fn'
        let mut params = Vec::new();

        if self.kind() == TokenKind::LParen {
            self.bump();
            while !self.is_eof() && self.kind() != TokenKind::RParen {
                if let Some((name, name_span)) = self.expect_ident("lambda parameter") {
                    let typ = if self.kind() == TokenKind::Colon {
                        self.bump();
                        Some(Box::new(self.parse_type_expr()))
                    } else {
                        None
                    };
                    params.push(crate::ast::types::TypedIdent { name, typ, span: name_span });
                }
                if self.kind() == TokenKind::Comma { self.bump(); }
            }
            self.expect(TokenKind::RParen, "lambda parameters");
        } else {
            // Single parameter without parens
            if let Some((name, name_span)) = self.expect_ident("lambda parameter") {
                params.push(crate::ast::types::TypedIdent { name, typ: None, span: name_span });
            }
        }

        self.expect(TokenKind::FatArrow, "lambda arrow");
        let body = self.parse_expr();
        let span = start.merge(body.span());
        Expr::Lambda { params, body: Box::new(body), span }
    }

    fn parse_lambda_prefix(&mut self) -> Expr {
        // ':' at start means a lambda with no params? No - ':' is for type annotations.
        // This shouldn't really happen at expression start. Fall through to error.
        let span = self.span();
        self.error("Unexpected ':' at start of expression".to_string());
        self.bump();
        Expr::Error { span }
    }

    fn parse_range_prefix(&mut self) -> Expr {
        let start = self.span();
        let inclusive = self.kind() == TokenKind::DotDotEq;
        self.bump(); // '..' or '..='
        let end = self.parse_expr_prec(1);
        let span = start.merge(end.span());
        Expr::Range { start: None, end: Some(Box::new(end)), inclusive, span }
    }

    fn parse_pipeline(&mut self, first: Expr) -> Expr {
        let start = first.span();
        let mut stages = vec![first];

        // If called from the multi-line check in parse_expr_prec, we're still before NEWLINE
        // If called from the same-line check, we're at Pipe directly
        loop {
            // Skip NEWLINE and optional INDENT before Pipe
            if self.kind() == TokenKind::Newline {
                self.bump();
                if self.kind() == TokenKind::Indent {
                    self.bump();
                }
            }
            if self.kind() != TokenKind::Pipe {
                break;
            }
            self.bump(); // '|>'
            let stage = self.parse_expr_prec(1);
            stages.push(stage);
        }

        // Consume the DEDENT that closes the pipeline's sub-indentation
        if self.kind() == TokenKind::Dedent {
            self.bump();
        }

        let span = start.merge(stages.last().unwrap().span());
        Expr::Pipeline { stages, span }
    }

    fn parse_confidence(&mut self, expr: Expr) -> Expr {
        let start = self.span();
        self.bump(); // 'confidence'
        let op = match self.kind() {
            TokenKind::EqEq => CmpOp::Eq,
            TokenKind::Neq => CmpOp::Neq,
            TokenKind::Lt => CmpOp::Lt,
            TokenKind::Gt => CmpOp::Gt,
            TokenKind::Le => CmpOp::Le,
            TokenKind::Ge => CmpOp::Ge,
            _ => CmpOp::Gt,
        };
        self.bump();
        let threshold = self.parse_expr_prec(1);
        let span = start.merge(threshold.span());
        Expr::Confidence { expr: Box::new(expr), op, threshold: Box::new(threshold), span }
    }

    fn parse_if_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // 'if'
        let cond = self.parse_expr();
        self.expect_keyword("then", "if expression");
        let then = self.parse_expr();
        self.expect_keyword("else", "if expression");
        let else_ = self.parse_expr();
        let span = start.merge(else_.span());
        Expr::IfExpr { cond: Box::new(cond), then: Box::new(then), else_: Box::new(else_), span }
    }

    fn parse_match_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // 'match'
        let scrutinee = self.parse_expr();
        while self.kind() == TokenKind::Newline { self.bump(); }
        self.expect_indent();
        let mut arms = Vec::new();

        while !self.is_eof() && !self.at_dedent() {
            while self.kind() == TokenKind::Newline { self.bump(); }
            if self.at_dedent() { break; }

            let arm_start = self.span();
            let pattern = self.parse_pattern();
            let guard = if self.is_keyword("if") {
                self.bump();
                Some(self.parse_expr())
            } else {
                None
            };
            // Expect '=>'
            self.expect(TokenKind::FatArrow, "match arm");

            // The body is a block (indented)
            let body = if self.kind() == TokenKind::Indent {
                self.parse_block()
            } else {
                // Single expression on same line
                let expr = self.parse_expr();
                let span = expr.span();
                Block::new(vec![crate::ast::stmt::Stmt::ExprStmt { expr, span }], span)
            };

            let span = arm_start.merge(body.span);
            arms.push(crate::ast::stmt::MatchArm { pattern, guard, body, span });
        }

        self.eat_dedent();
        let span = start.merge(self.span());
        Expr::MatchExpr { scrutinee: Box::new(scrutinee), arms, span }
    }

    fn parse_ask_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // 'ask'
        let model = self.text().to_string();
        self.bump(); // model name
        self.expect(TokenKind::Colon, "ask expression");

        if model == "human" {
            let body = self.parse_block();
            let body_text: Vec<String> = body.stmts.iter().filter_map(|s| {
                if let crate::ast::stmt::Stmt::ExprStmt { expr, .. } = s {
                    Some(format!("{:?}", expr)) // simplified
                } else {
                    None
                }
            }).collect();
            let span = start.merge(body.span);
            Expr::AskHuman { body: body_text, span }
        } else {
            let body = self.parse_block();
            let body_text = vec![format!("{:?}", body)]; // simplified
            let span = start.merge(body.span);
            Expr::Ask { model, body: body_text, span }
        }
    }

    fn parse_reason_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // 'reason'
        let mode = match self.text() {
            "deep" => ReasonMode::Deep,
            "fast" => ReasonMode::Fast,
            "none" => ReasonMode::None,
            _ => {
                self.error(format!("Expected 'deep', 'fast', or 'none' after 'reason', got '{}'", self.text()));
                ReasonMode::Fast
            }
        };
        let span = start.merge(self.span());
        self.bump();
        Expr::Reason { mode, span }
    }

    fn parse_use_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // 'use'
        let model = self.text().to_string();
        let span = start.merge(self.span());
        self.bump();
        Expr::Use { model, span }
    }

    // ---- Type expressions ----

    pub fn parse_type_expr(&mut self) -> crate::ast::types::TypeExpr {
        use crate::ast::types::TypeExpr;
        let start = self.span();
        let typ = self.parse_primary_type();

        // Check for union
        if self.kind() == TokenKind::Pipe || (self.kind() == TokenKind::Keyword && self.text() == "or") {
            self.bump(); // '|' or 'or' (in type context)
            let rhs = self.parse_type_expr();
            let span = start.merge(rhs.span());
            return TypeExpr::Union { left: Box::new(typ), right: Box::new(rhs), span };
        }

        // Check for nullable/optional
        match self.kind() {
            TokenKind::Question => {
                self.bump();
                let span = start.merge(self.span());
                return TypeExpr::Nullable { inner: Box::new(typ), span };
            }
            TokenKind::Exclamation => {
                self.bump();
                let span = start.merge(self.span());
                return TypeExpr::Optional { inner: Box::new(typ), span };
            }
            _ => {}
        }

        typ
    }

    fn parse_primary_type(&mut self) -> crate::ast::types::TypeExpr {
        use crate::ast::types::{TypeExpr, PRIMITIVE_TYPES, AI_TYPES};
        let start = self.span();

        if self.is_ident() || self.kind() == TokenKind::Keyword {
            let name = self.text().to_string();
            let name_span = self.span();
            self.bump();

            // Check for generic args: `List<Int>` or `Map<String, Int>`
            let mut args = Vec::new();
            if self.kind() == TokenKind::Lt {
                self.bump(); // '<'
                loop {
                    args.push(self.parse_type_expr());
                    if self.kind() == TokenKind::Comma { self.bump(); }
                    else { break; }
                }
                self.expect(TokenKind::Gt, "type arguments");
            }

            let span = start.merge(self.span());

            if PRIMITIVE_TYPES.contains(&name.as_str()) || AI_TYPES.contains(&name.as_str()) {
                if name == "List" && args.len() == 1 {
                    return TypeExpr::List { element: Box::new(args[0].clone()), span };
                }
                if name == "Set" && args.len() == 1 {
                    return TypeExpr::Set { element: Box::new(args[0].clone()), span };
                }
                if name == "Map" && args.len() == 2 {
                    return TypeExpr::Map { key: Box::new(args[0].clone()), value: Box::new(args[1].clone()), span };
                }
                if args.is_empty() {
                    return TypeExpr::Primitive { name, span };
                }
            }

            if args.is_empty() {
                TypeExpr::Named { name, args: vec![], span }
            } else {
                TypeExpr::Named { name, args, span }
            }
        } else if self.kind() == TokenKind::LParen {
            // Tuple type or parenthesized type
            self.bump();
            let mut elements = Vec::new();
            while !self.is_eof() && self.kind() != TokenKind::RParen {
                elements.push(self.parse_type_expr());
                if self.kind() == TokenKind::Comma { self.bump(); }
                else { break; }
            }
            self.expect(TokenKind::RParen, "type expression");
            let span = start.merge(self.span());
            if elements.len() == 1 {
                TypeExpr::Paren { inner: Box::new(elements[0].clone()), span }
            } else {
                TypeExpr::Tuple { elements, span }
            }
        } else if self.kind() == TokenKind::Keyword && self.text() == "fn" {
            self.bump();
            self.expect(TokenKind::LParen, "function type parameters");
            let mut params = Vec::new();
            while !self.is_eof() && self.kind() != TokenKind::RParen {
                params.push(self.parse_type_expr());
                if self.kind() == TokenKind::Comma { self.bump(); }
            }
            self.expect(TokenKind::RParen, "function type parameters");
            self.expect(TokenKind::Arrow, "function type return arrow");
            let returns = vec![self.parse_type_expr()];
            let span = start.merge(self.span());
            TypeExpr::Function { params, returns, span }
        } else {
            let span = self.span();
            self.error(format!("Expected type expression, found '{}'", self.text()));
            self.bump();
            TypeExpr::Error { span }
        }
    }

    // ---- Patterns ----

    pub fn parse_pattern(&mut self) -> crate::ast::pattern::Pattern {
        use crate::ast::pattern::Pattern;
        let start = self.span();

        match self.kind() {
            TokenKind::IntLit => {
                let text = self.text().to_string();
                let span = self.span();
                self.bump();
                Pattern::Literal { value: Literal::Int(text.parse().unwrap_or(0)), span }
            }
            TokenKind::StringLit => {
                let text = self.text().to_string();
                let span = self.span();
                self.bump();
                Pattern::Literal { value: Literal::String(text), span }
            }
            TokenKind::BoolLit => {
                let b = self.text() == "true";
                let span = self.span();
                self.bump();
                Pattern::Literal { value: Literal::Bool(b), span }
            }
            TokenKind::NullLit => {
                let span = self.span();
                self.bump();
                Pattern::Literal { value: Literal::Null, span }
            }
            TokenKind::Ident if self.text() == "_" => {
                let span = self.span();
                self.bump();
                Pattern::Wildcard { span }
            }
            TokenKind::Ident | TokenKind::Keyword => {
                let name = self.text().to_string();
                let span = self.span();
                self.bump();

                // Constructor pattern: `Name(fields...)`
                if self.kind() == TokenKind::LParen {
                    self.bump();
                    let mut fields = Vec::new();
                    while !self.is_eof() && self.kind() != TokenKind::RParen {
                        fields.push(self.parse_pattern());
                        if self.kind() == TokenKind::Comma { self.bump(); }
                    }
                    self.expect(TokenKind::RParen, "pattern constructor");
                    let span = start.merge(self.span());
                    Pattern::Constructor { name, fields, span }
                } else {
                    Pattern::Variable { name, span }
                }
            }
            TokenKind::LBrack => {
                self.bump();
                let mut elements = Vec::new();
                let mut rest = None;
                while !self.is_eof() && self.kind() != TokenKind::RBrack {
                    if self.kind() == TokenKind::DotDot {
                        self.bump();
                        if self.is_ident() {
                            rest = Some(self.text().to_string());
                            self.bump();
                        }
                        break;
                    }
                    elements.push(self.parse_pattern());
                    if self.kind() == TokenKind::Comma { self.bump(); }
                }
                self.expect(TokenKind::RBrack, "list pattern");
                let span = start.merge(self.span());
                Pattern::List { elements, rest, span }
            }
            TokenKind::LParen => {
                self.bump();
                let mut elements = Vec::new();
                while !self.is_eof() && self.kind() != TokenKind::RParen {
                    elements.push(self.parse_pattern());
                    if self.kind() == TokenKind::Comma { self.bump(); }
                }
                self.expect(TokenKind::RParen, "tuple pattern");
                let span = start.merge(self.span());
                Pattern::Tuple { elements, span }
            }
            _ => {
                let span = self.span();
                self.error(format!("Expected pattern, found '{}'", self.text()));
                self.bump();
                Pattern::Error { span }
            }
        }
    }
}

/// Desugar `=>` keyword. Since EBNF uses `=>` after patterns in match arms,
/// but the lexer tokenizes `=` as Eq and `>` as Gt separately, we need to handle this.
/// For now, we treat `=>` as just expecting the arm body after the pattern + guard.
impl Parser {
    /// Parse `>` after `=` to form `=>`.
    pub fn expect_fat_arrow(&mut self) -> bool {
        // In our current token setup, `=>` is not a single token.
        // We handle this in match arm parsing by looking for INDENT after the pattern+guard.
        true
    }
}
