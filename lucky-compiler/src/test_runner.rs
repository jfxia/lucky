//! Lucky test runner — discovers and executes *.test.lk test files.
//!
//! ## Test file format
//! ```lucky
//! test "test name" {
//!     let x = 1 + 1
//!     assert x == 2
//!     assert x != 3
//! }
//! ```
//!
//! Each test case is compiled through the Lucky compiler pipeline and executed
//! in an isolated runtime. Assertions are evaluated by a minimal recursive-descent
//! expression evaluator using runtime context values as the symbol table.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::ast::span::FileId;
use crate::hir::builder::HirBuilder;
use crate::runtime::executor::ExecutionEngine;
use crate::runtime::RuntimeValue;

// ─── Public data types ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<TestCase>,
    pub setup: Option<String>,
    pub teardown: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub source: String,
    pub expected_result: Option<String>,
    pub timeout_ms: u64,
    /// Internal: assertions extracted from the source.
    pub(crate) assertions: Vec<AssertionEntry>,
}

#[derive(Debug, Clone)]
pub(crate) enum AssertionEntry {
    Assert(String),
    AssertEq(String, String),
    AssertNe(String, String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestResult {
    Passed,
    Failed(String),
    Skipped(String),
    Error(String),
}

impl TestResult {
    pub fn is_passed(&self) -> bool {
        matches!(self, TestResult::Passed)
    }
}

#[derive(Debug, Clone)]
pub struct TestReport {
    pub suite_name: String,
    pub results: Vec<(TestCase, TestResult)>,
    pub duration_ms: u64,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

// ═════════════════════════════════════════════════════════════════════
// Expression evaluator for assertions
// ═════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
enum AssertExpr {
    Int(i64),
    Float(f64),
    Bool(bool),
    StringLit(String),
    Variable(String),
    FieldAccess(Box<AssertExpr>, String),
    BinaryOp(Box<AssertExpr>, BinOp, Box<AssertExpr>),
    LogicalOp(Box<AssertExpr>, LogOp, Box<AssertExpr>),
    UnaryOp(UnOp, Box<AssertExpr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BinOp {
    Eq, Ne, Lt, Gt, Le, Ge,
    Add, Sub, Mul, Div,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LogOp { And, Or }

#[derive(Debug, Clone, Copy, PartialEq)]
enum UnOp { Not, Neg }

// ─── Tokeniser ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Int(i64), Float(f64), Bool(bool), Str(String), Ident(String),
    Dot, Eq, Ne, Lt, Gt, Le, Ge,
    Plus, Minus, Star, Slash,
    LParen, RParen, And, Or, Not, Eof, Comma,
}

struct AssertLexer { chars: Vec<char>, pos: usize }

impl AssertLexer {
    fn new(source: &str) -> Self {
        Self { chars: source.chars().collect(), pos: 0 }
    }

    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn skip_spaces(&mut self) {
        while self.peek().map_or(false, |c| c.is_whitespace()) { self.advance(); }
    }

    fn read_number(&mut self, first: char) -> Tok {
        let mut s = String::new();
        s.push(first);
        while self.peek().map_or(false, |c| c.is_ascii_digit() || c == '.') {
            s.push(self.advance().unwrap());
        }
        if s.contains('.') { Tok::Float(s.parse().unwrap_or(0.0)) }
        else              { Tok::Int(s.parse().unwrap_or(0)) }
    }

    fn read_ident(&mut self, first: char) -> Tok {
        let mut s = String::new();
        s.push(first);
        while self.peek().map_or(false, |c| c.is_alphanumeric() || c == '_') {
            s.push(self.advance().unwrap());
        }
        match s.as_str() {
            "true"  => Tok::Bool(true),
            "false" => Tok::Bool(false),
            "and"   => Tok::And,
            "or"    => Tok::Or,
            "not"   => Tok::Not,
            _       => Tok::Ident(s),
        }
    }

    fn read_string(&mut self) -> Tok {
        let mut s = String::new();
        self.advance(); // skip opening "
        while let Some(c) = self.advance() {
            if c == '"' { break }
            if c == '\\' {
                match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some(other) => { s.push('\\'); s.push(other); }
                    None => break,
                }
            } else { s.push(c); }
        }
        Tok::Str(s)
    }

    fn next_token(&mut self) -> Tok {
        self.skip_spaces();
        let c = match self.advance() {
            Some(c) => c,
            None => return Tok::Eof,
        };
        match c {
            '+' => Tok::Plus,
            '-' => Tok::Minus,
            '*' => Tok::Star,
            '/' => Tok::Slash,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '.' => Tok::Dot,
            ',' => Tok::Comma,
            '=' => if self.peek() == Some('=') { self.advance(); Tok::Eq } else { Tok::Eof },
            '!' => if self.peek() == Some('=') { self.advance(); Tok::Ne } else { Tok::Not },
            '<' => if self.peek() == Some('=') { self.advance(); Tok::Le } else { Tok::Lt },
            '>' => if self.peek() == Some('=') { self.advance(); Tok::Ge } else { Tok::Gt },
            '"' => self.read_string(),
            c if c.is_ascii_digit() => self.read_number(c),
            c if c.is_alphabetic() || c == '_' => self.read_ident(c),
            _ => Tok::Eof,
        }
    }

    fn tokenize(&mut self) -> Vec<Tok> {
        let mut toks = Vec::new();
        loop {
            let t = self.next_token();
            let done = t == Tok::Eof;
            toks.push(t);
            if done { break }
        }
        toks
    }
}

// ─── Recursive-descent parser ────────────────────────────────────────

struct AssertParser { tokens: Vec<Tok>, pos: usize }

impl AssertParser {
    fn new(tokens: Vec<Tok>) -> Self { Self { tokens, pos: 0 } }

    fn peek(&self) -> &Tok { self.tokens.get(self.pos).unwrap_or(&Tok::Eof) }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn parse(&mut self) -> Result<AssertExpr, String> {
        let expr = self.or_expr()?;
        if *self.peek() != Tok::Eof {
            return Err(format!("Unexpected token after expression: {:?}", self.peek()));
        }
        Ok(expr)
    }

    fn or_expr(&mut self) -> Result<AssertExpr, String> {
        let mut left = self.and_expr()?;
        while *self.peek() == Tok::Or {
            self.advance();
            left = AssertExpr::LogicalOp(Box::new(left), LogOp::Or, Box::new(self.and_expr()?));
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> Result<AssertExpr, String> {
        let mut left = self.cmp_expr()?;
        while *self.peek() == Tok::And {
            self.advance();
            left = AssertExpr::LogicalOp(Box::new(left), LogOp::And, Box::new(self.cmp_expr()?));
        }
        Ok(left)
    }

    fn cmp_expr(&mut self) -> Result<AssertExpr, String> {
        let mut left = self.add_expr()?;
        while matches!(self.peek(), Tok::Eq | Tok::Ne | Tok::Lt | Tok::Gt | Tok::Le | Tok::Ge) {
            let op = self.peek().clone();
            self.advance();
            let right = self.add_expr()?;
            left = AssertExpr::BinaryOp(Box::new(left), tok_to_binop(&op), Box::new(right));
        }
        Ok(left)
    }

    fn add_expr(&mut self) -> Result<AssertExpr, String> {
        let mut left = self.mul_expr()?;
        while matches!(self.peek(), Tok::Plus | Tok::Minus) {
            let op = self.peek().clone();
            self.advance();
            let right = self.mul_expr()?;
            left = AssertExpr::BinaryOp(Box::new(left), tok_to_binop(&op), Box::new(right));
        }
        Ok(left)
    }

    fn mul_expr(&mut self) -> Result<AssertExpr, String> {
        let mut left = self.unary_expr()?;
        while matches!(self.peek(), Tok::Star | Tok::Slash) {
            let op = self.peek().clone();
            self.advance();
            let right = self.unary_expr()?;
            left = AssertExpr::BinaryOp(Box::new(left), tok_to_binop(&op), Box::new(right));
        }
        Ok(left)
    }

    fn unary_expr(&mut self) -> Result<AssertExpr, String> {
        match self.peek() {
            Tok::Not => { self.advance(); Ok(AssertExpr::UnaryOp(UnOp::Not, Box::new(self.unary_expr()?))) }
            Tok::Minus => { self.advance(); Ok(AssertExpr::UnaryOp(UnOp::Neg, Box::new(self.unary_expr()?))) }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<AssertExpr, String> {
        match self.peek().clone() {
            Tok::Int(n)    => { self.advance(); Ok(AssertExpr::Int(n)) }
            Tok::Float(f)  => { self.advance(); Ok(AssertExpr::Float(f)) }
            Tok::Bool(b)   => { self.advance(); Ok(AssertExpr::Bool(b)) }
            Tok::Str(ref s) => { let s = s.clone(); self.advance(); Ok(AssertExpr::StringLit(s)) }
            Tok::Ident(ref name) => {
                let name = name.clone();
                self.advance();
                let mut expr = AssertExpr::Variable(name);
                while *self.peek() == Tok::Dot {
                    self.advance();
                    if let Tok::Ident(ref field) = self.peek().clone() {
                        let field = field.clone();
                        self.advance();
                        expr = AssertExpr::FieldAccess(Box::new(expr), field);
                    } else {
                        return Err("Expected field name after '.'".to_string());
                    }
                }
                Ok(expr)
            }
            Tok::LParen => {
                self.advance();
                let expr = self.or_expr()?;
                if *self.peek() == Tok::RParen { self.advance(); }
                else { return Err("Expected ')'".to_string()); }
                Ok(expr)
            }
            ref t => Err(format!("Unexpected token: {:?}", t)),
        }
    }
}

fn tok_to_binop(tok: &Tok) -> BinOp {
    match tok {
        Tok::Eq => BinOp::Eq, Tok::Ne => BinOp::Ne,
        Tok::Lt => BinOp::Lt, Tok::Gt => BinOp::Gt,
        Tok::Le => BinOp::Le, Tok::Ge => BinOp::Ge,
        Tok::Plus => BinOp::Add, Tok::Minus => BinOp::Sub,
        Tok::Star => BinOp::Mul, Tok::Slash => BinOp::Div,
        _ => BinOp::Eq,
    }
}

// ─── Evaluator ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum EvalValue {
    Int(i64), Float(f64), Bool(bool), Str(String),
    Map(HashMap<String, EvalValue>),
}

impl EvalValue {
    fn from_runtime(v: &RuntimeValue) -> EvalValue {
        match v {
            RuntimeValue::Int(n)    => EvalValue::Int(*n),
            RuntimeValue::Float(f)  => EvalValue::Float(*f),
            RuntimeValue::Bool(b)   => EvalValue::Bool(*b),
            RuntimeValue::String(s) => EvalValue::Str(s.clone()),
            RuntimeValue::Map(m) => {
                let mut map = HashMap::new();
                for (k, val) in m { map.insert(k.clone(), EvalValue::from_runtime(val)); }
                EvalValue::Map(map)
            }
            RuntimeValue::Null => EvalValue::Str("null".to_string()),
            RuntimeValue::List(items) => {
                let mut map = HashMap::new();
                for (i, item) in items.iter().enumerate() { map.insert(i.to_string(), EvalValue::from_runtime(item)); }
                EvalValue::Map(map)
            }
            other => EvalValue::Str(format!("{}", other)),
        }
    }

    fn truthy(&self) -> bool {
        match self { EvalValue::Bool(b) => *b, EvalValue::Int(n) => *n != 0, _ => true }
    }

    fn show(&self) -> String {
        match self {
            EvalValue::Int(n) => n.to_string(),
            EvalValue::Float(f) => f.to_string(),
            EvalValue::Bool(b) => b.to_string(),
            EvalValue::Str(s) => format!("\"{}\"", s),
            EvalValue::Map(_) => "{...}".to_string(),
        }
    }

    fn as_number(&self) -> Option<f64> {
        match self { EvalValue::Int(n) => Some(*n as f64), EvalValue::Float(f) => Some(*f), _ => None }
    }

    fn as_int(&self) -> Option<i64> {
        match self { EvalValue::Int(n) => Some(*n), EvalValue::Float(f) => Some(*f as i64), _ => None }
    }
}

fn eval_expr(expr: &AssertExpr, ctx: &HashMap<String, EvalValue>) -> Result<EvalValue, String> {
    match expr {
        AssertExpr::Int(n)        => Ok(EvalValue::Int(*n)),
        AssertExpr::Float(f)      => Ok(EvalValue::Float(*f)),
        AssertExpr::Bool(b)       => Ok(EvalValue::Bool(*b)),
        AssertExpr::StringLit(s)   => Ok(EvalValue::Str(s.clone())),
        AssertExpr::Variable(name) => {
            ctx.get(name).cloned().ok_or_else(|| format!("Undefined variable '{}'", name))
        }
        AssertExpr::FieldAccess(base, field) => {
            let val = eval_expr(base, ctx)?;
            match val {
                EvalValue::Map(m) => m.get(field).cloned()
                    .ok_or_else(|| format!("Field '{}' not found", field)),
                _ => Err(format!("Cannot access field '{}' on non-map value", field)),
            }
        }
        AssertExpr::BinaryOp(left, op, right) => {
            let l = eval_expr(left, ctx)?;
            let r = eval_expr(right, ctx)?;
            eval_binary(&l, *op, &r)
        }
        AssertExpr::LogicalOp(left, op, right) => {
            let l = eval_expr(left, ctx)?;
            match op {
                LogOp::And => if !l.truthy() { Ok(EvalValue::Bool(false)) } else { eval_expr(right, ctx) },
                LogOp::Or  => if l.truthy() { Ok(EvalValue::Bool(true)) } else { eval_expr(right, ctx) },
            }
        }
        AssertExpr::UnaryOp(op, expr) => {
            let v = eval_expr(expr, ctx)?;
            match op {
                UnOp::Not => Ok(EvalValue::Bool(!v.truthy())),
                UnOp::Neg => match v {
                    EvalValue::Int(n)  => Ok(EvalValue::Int(-n)),
                    EvalValue::Float(f) => Ok(EvalValue::Float(-f)),
                    _ => Err("Cannot negate non-numeric value".to_string()),
                },
            }
        }
    }
}

fn eval_binary(l: &EvalValue, op: BinOp, r: &EvalValue) -> Result<EvalValue, String> {
    match op {
        BinOp::Eq => Ok(EvalValue::Bool(l.show() == r.show())),
        BinOp::Ne => Ok(EvalValue::Bool(l.show() != r.show())),
        BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
            let (ln, rn) = match (l.as_number(), r.as_number()) {
                (Some(a), Some(b)) => (a, b),
                _ => return Err("Cannot compare non-numeric values".to_string()),
            };
            Ok(EvalValue::Bool(match op {
                BinOp::Lt => ln < rn, BinOp::Gt => ln > rn,
                BinOp::Le => ln <= rn, BinOp::Ge => ln >= rn,
                _ => unreachable!(),
            }))
        }
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
            let (ln, rn) = match (l.as_number(), r.as_number()) {
                (Some(a), Some(b)) => (a, b),
                _ => return Err("Cannot perform arithmetic on non-numeric values".to_string()),
            };
            let result = match op {
                BinOp::Add => ln + rn,
                BinOp::Sub => ln - rn,
                BinOp::Mul => ln * rn,
                BinOp::Div => if rn == 0.0 { return Err("Division by zero".to_string()) } else { ln / rn },
                _ => unreachable!(),
            };
            if result.trunc() == result && l.as_int().is_some() && r.as_int().is_some() {
                Ok(EvalValue::Int(result as i64))
            } else {
                Ok(EvalValue::Float(result))
            }
        }
    }
}

// ═════════════════════════════════════════════════════════════════════
// Test file parser
// ═════════════════════════════════════════════════════════════════════

struct ParsedFile {
    setup: Option<String>,
    teardown: Option<String>,
    cases: Vec<ParsedCase>,
}

struct ParsedCase {
    name: String,
    body: String,
    assertions: Vec<AssertionEntry>,
}

fn parse_test_file(source: &str) -> ParsedFile {
    let mut setup: Option<String> = None;
    let mut teardown: Option<String> = None;
    let mut cases: Vec<ParsedCase> = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() || line.starts_with('#') { i += 1; continue; }

        if line.starts_with("setup ") || line.starts_with("setup{") {
            setup = Some(extract_block(&lines, &mut i));
        } else if line.starts_with("teardown ") || line.starts_with("teardown{") {
            teardown = Some(extract_block(&lines, &mut i));
        } else if line.starts_with("test ") {
            let rest = line.strip_prefix("test ").unwrap_or("").trim();
            if let Some(name) = extract_quoted_string(rest) {
                let body = extract_block(&lines, &mut i);
                let (stmts, assertions) = separate_assertions(&body);
                cases.push(ParsedCase { name, body: stmts, assertions });
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    ParsedFile { setup, teardown, cases }
}

/// Extract text between matching `{ ... }` braces. Advances `pos` past the closing `}`.
fn extract_block(lines: &[&str], pos: &mut usize) -> String {
    let _raw = lines[*pos];
    let mut out = String::new();
    let mut depth = 0i32;
    let mut started = false;

    loop {
        if *pos >= lines.len() { break; }
        let l = lines[*pos];
        for ch in l.chars() {
            match ch {
                '{' => { depth += 1; started = true; }
                '}' => {
                    depth -= 1;
                    if depth == 0 && started {
                        *pos += 1;
                        return out;
                    }
                }
                _ if depth > 0 => { out.push(ch); }
                _ => {}
            }
        }
        if depth > 0 { out.push('\n'); }
        *pos += 1;
    }
    out
}

/// Extract content of a double-quoted string literal. Returns `Some(content)` on success.
fn extract_quoted_string(text: &str) -> Option<String> {
    let t = text.trim();
    if !t.starts_with('"') { return None; }
    let mut s = String::new();
    let chars: Vec<char> = t[1..].chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && i + 1 < chars.len() {
            i += 1;
            match chars[i] { 'n' => s.push('\n'), 't' => s.push('\t'), '"' => s.push('"'), '\\' => s.push('\\'), c => { s.push('\\'); s.push(c); } }
        } else if ch == '"' {
            return Some(s);
        } else { s.push(ch); }
        i += 1;
    }
    Some(s)
}

/// Separate assertion lines from regular code.
fn separate_assertions(body: &str) -> (String, Vec<AssertionEntry>) {
    let mut code = String::new();
    let mut assertions = Vec::new();

    for line in body.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            code.push_str(line);
            code.push('\n');
            continue;
        }
        if let Some(rest) = t.strip_prefix("assert_eq ") {
            let parts: Vec<&str> = rest.splitn(2, ',').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                assertions.push(AssertionEntry::AssertEq(parts[0].to_string(), parts[1].to_string()));
            }
        } else if let Some(rest) = t.strip_prefix("assert_ne ") {
            let parts: Vec<&str> = rest.splitn(2, ',').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                assertions.push(AssertionEntry::AssertNe(parts[0].to_string(), parts[1].to_string()));
            }
        } else if let Some(rest) = t.strip_prefix("assert ") {
            assertions.push(AssertionEntry::Assert(rest.to_string()));
        } else {
            code.push_str(line);
            code.push('\n');
        }
    }

    (code, assertions)
}

// ═════════════════════════════════════════════════════════════════════
// Public API
// ═════════════════════════════════════════════════════════════════════

/// Discover tests by scanning a directory (or single file) for `*.test.lk` files.
pub fn discover_tests(path: &str) -> Result<TestSuite, String> {
    let p = Path::new(path);

    if p.is_file() {
        return discover_from_file(p);
    }
    if p.is_dir() {
        return discover_from_dir(p);
    }
    Err(format!("Path does not exist: {}", path))
}

fn discover_from_file(p: &Path) -> Result<TestSuite, String> {
    let source = fs::read_to_string(p)
        .map_err(|e| format!("Cannot read '{}': {}", p.display(), e))?;
    let parsed = parse_test_file(&source);
    let name = p.file_stem().and_then(|n| n.to_str()).unwrap_or("test").to_string();

    let cases: Vec<TestCase> = parsed.cases.into_iter().map(|c| TestCase {
        name: c.name,
        source: c.body,
        expected_result: None,
        timeout_ms: 5000,
        assertions: c.assertions,
    }).collect();

    if cases.is_empty() {
        return Err(format!("No test cases found in '{}'", p.display()));
    }

    Ok(TestSuite { name, tests: cases, setup: parsed.setup, teardown: parsed.teardown })
}

fn discover_from_dir(p: &Path) -> Result<TestSuite, String> {
    let entries = fs::read_dir(p)
        .map_err(|e| format!("Cannot read directory '{}': {}", p.display(), e))?;

    let mut all_cases: Vec<TestCase> = Vec::new();
    let mut suite_setup: Option<String> = None;
    let mut suite_teardown: Option<String> = None;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
        let fp = entry.path();
        if fp.extension().and_then(|e| e.to_str()) == Some("lk") {
            if let Some(fname) = fp.file_name().and_then(|n| n.to_str()) {
                if fname.ends_with(".test.lk") {
                    let source = fs::read_to_string(&fp)
                        .map_err(|e| format!("Cannot read '{}': {}", fp.display(), e))?;
                    let parsed = parse_test_file(&source);
                    if suite_setup.is_none() { suite_setup = parsed.setup; }
                    if suite_teardown.is_none() { suite_teardown = parsed.teardown; }
                    for c in parsed.cases {
                        all_cases.push(TestCase {
                            name: c.name,
                            source: c.body,
                            expected_result: None,
                            timeout_ms: 5000,
                            assertions: c.assertions,
                        });
                    }
                }
            }
        }
    }

    if all_cases.is_empty() {
        return Err(format!("No test cases found in '{}'", p.display()));
    }

    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("tests").to_string();
    Ok(TestSuite { name, tests: all_cases, setup: suite_setup, teardown: suite_teardown })
}

/// Compile and execute a single test case.
pub fn run_test(test: &TestCase) -> TestResult {
    let body = test.source.trim();
    if body.is_empty() && test.assertions.is_empty() {
        return TestResult::Failed("Empty test source".to_string());
    }

    let file_id = FileId(u32::MAX);

    // Build compilable Lucky source by wrapping the test body in a workflow
    let mut source = String::new();
    source.push_str("workflow __test_case {\n");
    if !body.is_empty() {
        source.push_str(body);
        source.push('\n');
    }
    source.push_str("}\n");

    // Step 1: Compile
    let (module, diagnostics) = crate::compile(&source, file_id);
    if diagnostics.has_errors() {
        let msgs: Vec<String> = diagnostics.diagnostics.iter()
            .filter(|d| d.severity == crate::diagnostics::Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        return TestResult::Error(format!("Compilation error: {}", msgs.join("; ")));
    }

    // Step 2: Build HIR
    let hir_graph = HirBuilder::new().build_module(&module);

    // Step 3: Execute in isolated runtime
    let mut engine = ExecutionEngine::new();
    engine.run(&hir_graph);

    if engine.summary().failed_nodes > 0 {
        return TestResult::Failed(format!(
            "Execution failed: {} node(s) failed", engine.summary().failed_nodes
        ));
    }

    // Step 4: Evaluate assertions against runtime context
    if test.assertions.is_empty() {
        return TestResult::Passed;
    }

    let ctx = engine.context.snapshot();
    let eval_ctx: HashMap<String, EvalValue> = ctx.iter()
        .map(|(k, v)| (k.clone(), EvalValue::from_runtime(v)))
        .collect();

    for a in &test.assertions {
        let r = eval_assertion(a, &eval_ctx);
        if !r.is_passed() { return r; }
    }

    TestResult::Passed
}

fn eval_assertion(a: &AssertionEntry, ctx: &HashMap<String, EvalValue>) -> TestResult {
    match a {
        AssertionEntry::Assert(expr_str) => {
            match AssertParser::new(AssertLexer::new(expr_str).tokenize()).parse() {
                Ok(expr) => match eval_expr(&expr, ctx) {
                    Ok(v) if v.truthy() => TestResult::Passed,
                    Ok(v) => TestResult::Failed(format!(
                        "assertion failed: `{}` evaluated to {}", expr_str, v.show()
                    )),
                    Err(e) => TestResult::Error(format!(
                        "Failed to evaluate `{}`: {}", expr_str, e
                    )),
                },
                Err(e) => TestResult::Error(format!(
                    "Failed to parse `{}`: {}", expr_str, e
                )),
            }
        }
        AssertionEntry::AssertEq(left_str, right_str) => {
            let l = eval_str(left_str, ctx);
            let r = eval_str(right_str, ctx);
            match (l, r) {
                (Ok(lv), Ok(rv)) => {
                    if lv.show() == rv.show() { TestResult::Passed }
                    else { TestResult::Failed(format!(
                        "assert_eq failed: `{} == {}`\n  left:  {}\n  right: {}",
                        left_str, right_str, lv.show(), rv.show()
                    ))}
                }
                (Err(e), _) => TestResult::Error(e),
                (_, Err(e)) => TestResult::Error(e),
            }
        }
        AssertionEntry::AssertNe(left_str, right_str) => {
            let l = eval_str(left_str, ctx);
            let r = eval_str(right_str, ctx);
            match (l, r) {
                (Ok(lv), Ok(rv)) => {
                    if lv.show() != rv.show() { TestResult::Passed }
                    else { TestResult::Failed(format!(
                        "assert_ne failed: `{} != {}`\n  both values: {}", left_str, right_str, lv.show()
                    ))}
                }
                (Err(e), _) => TestResult::Error(e),
                (_, Err(e)) => TestResult::Error(e),
            }
        }
    }
}

fn eval_str(s: &str, ctx: &HashMap<String, EvalValue>) -> Result<EvalValue, String> {
    let tokens = AssertLexer::new(s).tokenize();
    let expr = AssertParser::new(tokens)
        .parse()
        .map_err(|e| format!("Parse error in '{}': {}", s, e))?;
    eval_expr(&expr, ctx)
}

/// Run all tests in a suite and produce a report.
pub fn run_suite(suite: &TestSuite) -> TestReport {
    let start = Instant::now();
    let mut results = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for test in &suite.tests {
        let result = run_test(test);
        match &result {
            TestResult::Passed => passed += 1,
            TestResult::Failed(_) => failed += 1,
            TestResult::Skipped(_) => skipped += 1,
            TestResult::Error(_) => failed += 1,
        }
        results.push((test.clone(), result));
    }

    TestReport {
        suite_name: suite.name.clone(),
        duration_ms: start.elapsed().as_millis() as u64,
        results,
        passed,
        failed,
        skipped,
    }
}

/// Run all test suites from a list of paths.
pub fn run_all(paths: &[String]) -> Vec<TestReport> {
    let mut reports = Vec::new();
    for path in paths {
        match discover_tests(path) {
            Ok(suite) => reports.push(run_suite(&suite)),
            Err(e) => eprintln!("[ERROR] Failed to discover tests in '{}': {}", path, e),
        }
    }
    reports
}

// ═════════════════════════════════════════════════════════════════════
// Display helpers
// ═════════════════════════════════════════════════════════════════════

impl TestReport {
    pub fn print_summary(&self) {
        println!();
        println!("{}", "─".repeat(60));
        println!("Test Suite: {}", self.suite_name);
        println!("{}", "─".repeat(60));

        for (i, (test, result)) in self.results.iter().enumerate() {
            print!("  {}. {} ... ", i + 1, test.name);
            match result {
                TestResult::Passed  => println!("PASS"),
                TestResult::Failed(_) => println!("FAIL"),
                TestResult::Skipped(_) => println!("SKIP"),
                TestResult::Error(_) => println!("ERROR"),
            }
        }

        println!("{}", "─".repeat(60));
        println!(
            "Results: {} passed, {} failed, {} skipped ({}ms)",
            self.passed, self.failed, self.skipped, self.duration_ms
        );

        if self.failed > 0 {
            println!();
            println!("Failures:");
            for (test, result) in &self.results {
                match result {
                    TestResult::Failed(msg) | TestResult::Error(msg) => {
                        println!("  x {}", test.name);
                        for line in msg.lines() { println!("    {}", line); }
                    }
                    _ => {}
                }
            }
            println!();
        }
    }

    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }
}
