//! Full LSP server implementation for the Lucky language.
//! Communicates via JSON-RPC 2.0 over stdin/stdout.
//! Uses only Rust stdlib.

use std::io::{self, BufRead, BufReader, Read, Write};

use super::{LspDiagnostic, LspSeverity, Position, Range};

// =============================================================================
// JSON Value Type & Parser (stdlib-only)
// =============================================================================

#[derive(Debug, Clone)]
enum JsonVal {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(Vec<(String, JsonVal)>),
}

impl JsonVal {
    fn parse(s: &str) -> Result<JsonVal, String> {
        let bytes = s.as_bytes();
        let mut pos: usize = 0;
        skip_ws(bytes, &mut pos);
        parse_value(bytes, &mut pos).ok_or_else(|| format!("parse error at byte {}", pos))
    }

    fn as_obj(&self) -> Option<&[(String, JsonVal)]> {
        match self { JsonVal::Obj(v) => Some(v), _ => None }
    }

    fn as_arr(&self) -> Option<&[JsonVal]> {
        match self { JsonVal::Arr(v) => Some(v), _ => None }
    }

    fn as_str(&self) -> Option<&str> {
        match self { JsonVal::Str(s) => Some(s), _ => None }
    }

    fn as_usize(&self) -> Option<usize> {
        match self { JsonVal::Num(n) => Some(*n as usize), _ => None }
    }

    fn get(&self, key: &str) -> Option<&JsonVal> {
        self.as_obj()?.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_str()
    }

    fn get_usize(&self, key: &str) -> Option<usize> {
        self.get(key)?.as_usize()
    }
}

fn skip_ws(bytes: &[u8], pos: &mut usize) {
    while *pos < bytes.len() {
        match bytes[*pos] {
            b' ' | b'\t' | b'\n' | b'\r' => *pos += 1,
            _ => break,
        }
    }
}

fn parse_value(bytes: &[u8], pos: &mut usize) -> Option<JsonVal> {
    skip_ws(bytes, pos);
    if *pos >= bytes.len() { return None; }
    match bytes[*pos] {
        b'{' => parse_object(bytes, pos).map(JsonVal::Obj),
        b'[' => parse_array(bytes, pos).map(JsonVal::Arr),
        b'"' => parse_string(bytes, pos).map(JsonVal::Str),
        b't' | b'f' => parse_bool(bytes, pos),
        b'n' => parse_null(bytes, pos),
        b'-' | b'0'..=b'9' => parse_number(bytes, pos).map(JsonVal::Num),
        _ => None,
    }
}

fn parse_object(bytes: &[u8], pos: &mut usize) -> Option<Vec<(String, JsonVal)>> {
    *pos += 1;
    let mut fields: Vec<(String, JsonVal)> = Vec::new();
    skip_ws(bytes, pos);
    if *pos < bytes.len() && bytes[*pos] == b'}' { *pos += 1; return Some(fields); }
    loop {
        skip_ws(bytes, pos);
        if *pos >= bytes.len() { return None; }
        if bytes[*pos] == b'}' { *pos += 1; return Some(fields); }
        let key = parse_string(bytes, pos)?;
        skip_ws(bytes, pos);
        if *pos >= bytes.len() || bytes[*pos] != b':' { return None; }
        *pos += 1;
        let val = parse_value(bytes, pos)?;
        fields.push((key, val));
        skip_ws(bytes, pos);
        if *pos < bytes.len() && bytes[*pos] == b',' { *pos += 1; }
    }
}

fn parse_array(bytes: &[u8], pos: &mut usize) -> Option<Vec<JsonVal>> {
    *pos += 1;
    let mut items: Vec<JsonVal> = Vec::new();
    skip_ws(bytes, pos);
    if *pos < bytes.len() && bytes[*pos] == b']' { *pos += 1; return Some(items); }
    loop {
        skip_ws(bytes, pos);
        if *pos >= bytes.len() { return None; }
        if bytes[*pos] == b']' { *pos += 1; return Some(items); }
        items.push(parse_value(bytes, pos)?);
        skip_ws(bytes, pos);
        if *pos < bytes.len() && bytes[*pos] == b',' { *pos += 1; }
    }
}

fn parse_string(bytes: &[u8], pos: &mut usize) -> Option<String> {
    *pos += 1;
    let mut s = String::new();
    while *pos < bytes.len() {
        let b = bytes[*pos];
        if b == b'"' { *pos += 1; return Some(s); }
        if b == b'\\' {
            *pos += 1;
            if *pos >= bytes.len() { return None; }
            match bytes[*pos] {
                b'"' => s.push('"'),
                b'\\' => s.push('\\'),
                b'/' => s.push('/'),
                b'b' => s.push('\x08'),
                b'f' => s.push('\x0c'),
                b'n' => s.push('\n'),
                b'r' => s.push('\r'),
                b't' => s.push('\t'),
                b'u' => {
                    *pos += 1;
                    let mut hex = String::with_capacity(4);
                    for _ in 0..4 {
                        if *pos >= bytes.len() { return None; }
                        hex.push(bytes[*pos] as char);
                        *pos += 1;
                    }
                    if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(cp) { s.push(c); }
                    }
                    continue;
                }
                _ => {}
            }
        } else {
            s.push(b as char);
        }
        *pos += 1;
    }
    None
}

fn parse_number(bytes: &[u8], pos: &mut usize) -> Option<f64> {
    let start = *pos;
    if *pos < bytes.len() && bytes[*pos] == b'-' { *pos += 1; }
    while *pos < bytes.len() && bytes[*pos].is_ascii_digit() { *pos += 1; }
    if *pos < bytes.len() && bytes[*pos] == b'.' {
        *pos += 1;
        while *pos < bytes.len() && bytes[*pos].is_ascii_digit() { *pos += 1; }
    }
    if *pos < bytes.len() && (bytes[*pos] == b'e' || bytes[*pos] == b'E') {
        *pos += 1;
        if *pos < bytes.len() && (bytes[*pos] == b'+' || bytes[*pos] == b'-') { *pos += 1; }
        while *pos < bytes.len() && bytes[*pos].is_ascii_digit() { *pos += 1; }
    }
    std::str::from_utf8(&bytes[start..*pos]).ok()?.parse::<f64>().ok()
}

fn parse_bool(bytes: &[u8], pos: &mut usize) -> Option<JsonVal> {
    if bytes[*pos..].starts_with(b"true") { *pos += 4; return Some(JsonVal::Bool(true)); }
    if bytes[*pos..].starts_with(b"false") { *pos += 5; return Some(JsonVal::Bool(false)); }
    None
}

fn parse_null(bytes: &[u8], pos: &mut usize) -> Option<JsonVal> {
    if bytes[*pos..].starts_with(b"null") { *pos += 4; return Some(JsonVal::Null); }
    None
}

// =============================================================================
// JSON Builder (stdlib-only)
// =============================================================================

struct JsonBuf {
    buf: String,
}

impl JsonBuf {
    fn new() -> Self { JsonBuf { buf: String::with_capacity(4096) } }

    fn obj_open(&mut self) { self.buf.push('{'); }
    fn obj_close(&mut self) { self.strip_trailing_comma(); self.buf.push('}'); }
    fn arr_open(&mut self) { self.buf.push('['); }
    fn arr_close(&mut self) { self.strip_trailing_comma(); self.buf.push(']'); }

    fn key(&mut self, k: &str) {
        self.str_val(k);
        self.buf.pop();
        self.buf.push(':');
    }

    fn str_val(&mut self, v: &str) {
        self.buf.push('"');
        escape_into(&mut self.buf, v);
        self.buf.push('"');
        self.buf.push(',');
    }

    fn num_val(&mut self, v: impl std::fmt::Display) {
        use std::fmt::Write;
        let _ = write!(self.buf, "{}", v);
        self.buf.push(',');
    }

    fn bool_val(&mut self, v: bool) {
        self.buf.push_str(if v { "true" } else { "false" });
        self.buf.push(',');
    }

    fn null(&mut self) { self.buf.push_str("null"); self.buf.push(','); }

    fn strip_trailing_comma(&mut self) {
        if self.buf.ends_with(',') { self.buf.pop(); }
    }

    fn into_string(mut self) -> String {
        self.strip_trailing_comma();
        self.buf
    }
}

fn escape_into(buf: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = std::fmt::Write::write_fmt(buf, format_args!("\\u{:04x}", c as u32));
            }
            _ => buf.push(c),
        }
    }
}

// =============================================================================
// Position / offset helpers (UTF-16 based for LSP)
// =============================================================================

fn byte_to_utf16_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset.min(text.len())].encode_utf16().count()
}

fn offset_to_lsp_position(offset: usize, source: &str) -> Position {
    let offset = offset.min(source.len());
    let text_up_to = &source[..offset];
    let line = text_up_to.chars().filter(|c| *c == '\n').count();
    let line_start = text_up_to.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = byte_to_utf16_offset(&source[line_start..], offset - line_start);
    Position { line, character: col }
}

fn span_to_lsp_range(span: crate::ast::span::Span, source: &str) -> Range {
    Range {
        start: offset_to_lsp_position(span.start, source),
        end: offset_to_lsp_position(span.end, source),
    }
}

fn lsp_pos_to_offset(source: &str, line: usize, character: usize) -> usize {
    let mut current_line: usize = 0;
    let mut line_start: usize = 0;
    for (i, c) in source.char_indices() {
        if current_line == line {
            let line_str = &source[line_start..i];
            let mut col: usize = 0;
            let mut ch_offset = line_start;
            for ch in line_str.chars() {
                let u16len = ch.len_utf16();
                if col + u16len > character { break; }
                col += u16len;
                ch_offset += ch.len_utf8();
            }
            return ch_offset;
        }
        if c == '\n' { current_line += 1; line_start = i + 1; }
    }
    if current_line == line { line_start } else { source.len() }
}

fn end_position(source: &str) -> Position {
    let lines = source.lines().count();
    if lines == 0 { return Position { line: 0, character: 0 }; }
    let last = source.lines().last().unwrap_or("");
    Position { line: lines.saturating_sub(1), character: last.encode_utf16().count() }
}

fn is_ident_byte(b: u8) -> bool { b.is_ascii_alphanumeric() || b == b'_' }

fn word_at_position(source: &str, line: usize, character: usize) -> String {
    let offset = lsp_pos_to_offset(source, line, character);
    let bytes = source.as_bytes();
    let end = bytes.len().min(offset);
    if end == 0 { return String::new(); }
    let start = (0..end).rev().take_while(|&i| is_ident_byte(bytes[i])).last().unwrap_or(end);
    let end = ((end.min(bytes.len()))..bytes.len())
        .take_while(|&i| is_ident_byte(bytes[i]))
        .last().map(|i| i + 1).unwrap_or(end);
    source[start..end].to_string()
}

fn word_range_at_position(source: &str, line: usize, character: usize, word: &str) -> Range {
    let offset = lsp_pos_to_offset(source, line, character);
    let bytes = source.as_bytes();
    let end = bytes.len().min(offset);
    let start_byte = (0..end).rev().take_while(|&i| is_ident_byte(bytes[i])).last().unwrap_or(end);
    let end_byte = start_byte + word.len();
    Range {
        start: offset_to_lsp_position(start_byte, source),
        end: offset_to_lsp_position(end_byte, source),
    }
}

// =============================================================================
// JSON output helpers
// =============================================================================

fn write_range(buf: &mut JsonBuf, range: &Range) {
    buf.obj_open();
    buf.key("start"); buf.obj_open();
    buf.key("line"); buf.num_val(range.start.line);
    buf.key("character"); buf.num_val(range.start.character);
    buf.obj_close();
    buf.key("end"); buf.obj_open();
    buf.key("line"); buf.num_val(range.end.line);
    buf.key("character"); buf.num_val(range.end.character);
    buf.obj_close();
    buf.obj_close();
}

fn write_json_val(buf: &mut JsonBuf, val: &JsonVal) {
    match val {
        JsonVal::Null => buf.null(),
        JsonVal::Bool(b) => buf.bool_val(*b),
        JsonVal::Num(n) => {
            if *n == (*n as i64) as f64 { buf.num_val(*n as i64); }
            else { buf.null(); }
        }
        JsonVal::Str(s) => buf.str_val(s),
        JsonVal::Arr(arr) => {
            buf.arr_open();
            for item in arr { write_json_val(buf, item); }
            buf.arr_close();
        }
        JsonVal::Obj(obj) => {
            buf.obj_open();
            for (k, v) in obj { buf.key(k); write_json_val(buf, v); }
            buf.obj_close();
        }
    }
}

fn write_json_opt(buf: &mut JsonBuf, val: &Option<JsonVal>) {
    match val {
        Some(v) => write_json_val(buf, v),
        None => buf.null(),
    }
}

fn completion_kind(s: &str) -> u32 {
    match s {
        "keyword" => 14, "class" => 7, "function" => 3,
        "method" => 6, "variable" => 6, "field" => 5, "struct" => 22, _ => 1,
    }
}

fn symbol_kind(s: &str) -> u32 {
    match s {
        "module" => 2, "class" => 5, "function" => 12, "method" => 6,
        "variable" => 13, "struct" => 23, "interface" => 11, "property" => 7, _ => 1,
    }
}

// =============================================================================
// I/O
// =============================================================================

fn read_content_length(reader: &mut BufReader<io::StdinLock>) -> io::Result<Option<usize>> {
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 { return Ok(None); }
        let lower = line.trim().to_lowercase();
        if lower.is_empty() { return Ok(None); }
        if let Some(val) = lower.strip_prefix("content-length:") {
            if let Ok(n) = val.trim().parse::<usize>() { return Ok(Some(n)); }
        }
    }
}

fn send_json(out: &mut io::StdoutLock, json: &str) -> io::Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", json.len());
    out.write_all(header.as_bytes())?;
    out.write_all(json.as_bytes())?;
    out.flush()
}

// =============================================================================
// Document Manager
// =============================================================================

struct Doc {
    uri: String,
    text: String,
}

pub(crate) struct TextDocumentContentChangeEvent {
    range: Option<Range>,
    text: Option<String>,
}

pub struct DocumentManager {
    docs: Vec<Doc>,
}

impl DocumentManager {
    pub fn new() -> Self { DocumentManager { docs: Vec::new() } }

    pub fn open(&mut self, uri: &str, text: &str) {
        self.close(uri);
        self.docs.push(Doc { uri: uri.to_string(), text: text.to_string() });
    }

    pub fn update(&mut self, uri: &str, changes: &[TextDocumentContentChangeEvent]) {
        if let Some(doc) = self.docs.iter_mut().find(|d| d.uri == uri) {
            for change in changes {
                if let Some(ref full_text) = change.text {
                    doc.text = full_text.clone();
                } else if let Some(ref range) = change.range {
                    let start = lsp_pos_to_offset(&doc.text, range.start.line, range.start.character);
                    let end = lsp_pos_to_offset(&doc.text, range.end.line, range.end.character);
                    let mut new_text = String::with_capacity(
                        start + change.text.as_ref().map_or(0, |s| s.len()) + doc.text.len().saturating_sub(end)
                    );
                    new_text.push_str(&doc.text[..start]);
                    if let Some(ref t) = change.text { new_text.push_str(t); }
                    new_text.push_str(&doc.text[end..]);
                    doc.text = new_text;
                }
            }
        }
    }

    pub fn close(&mut self, uri: &str) { self.docs.retain(|d| d.uri != uri); }

    pub fn get(&self, uri: &str) -> Option<&str> {
        self.docs.iter().find(|d| d.uri == uri).map(|d| d.text.as_str())
    }

    pub fn parse(&self, uri: &str) -> Option<crate::ast::Module> {
        let source = self.get(uri)?;
        use crate::ast::span::FileId;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        let file_id = FileId(0);
        let mut lexer = Lexer::new(source, file_id);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens, file_id);
        Some(parser.parse_module())
    }
}

// =============================================================================
// Completion Engine
// =============================================================================

pub struct CompletionEngine;

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: u32,
    pub insert_text: Option<String>,
}

const TOP_LEVEL_KEYWORDS: &[&str] = &[
    "agent", "task", "workflow", "goal", "memory", "tool", "model",
    "prompt", "policy", "type", "context", "permissions", "approval",
    "project", "import", "pub",
];

const AGENT_BODY_KEYWORDS: &[&str] = &[
    "model", "memory", "tools", "permissions", "policy", "prompt", "task",
];

const TASK_BODY_KEYWORDS: &[&str] = &[
    "input", "output", "context", "policy", "steps", "rollback",
];

const STATEMENT_KEYWORDS: &[&str] = &[
    "let", "const", "if", "match", "loop", "for", "parallel", "await",
    "when", "return", "break", "continue", "attempt", "swarm", "retry",
    "use", "ask", "reason",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context { TopLevel, AgentBody, TaskBody, Statement }

impl CompletionEngine {
    pub fn completions(
        &self,
        source: &str,
        line: usize,
        character: usize,
        module: Option<&crate::ast::Module>,
    ) -> Vec<CompletionItem> {
        let offset = lsp_pos_to_offset(source, line, character);
        let prefix = &source[..offset.min(source.len())];
        let partial = word_before(prefix);
        let ctx = infer_context(source, offset);
        let mut items: Vec<CompletionItem> = Vec::new();

        let keywords = match ctx {
            Context::TopLevel => TOP_LEVEL_KEYWORDS,
            Context::AgentBody => AGENT_BODY_KEYWORDS,
            Context::TaskBody => TASK_BODY_KEYWORDS,
            Context::Statement => STATEMENT_KEYWORDS,
        };

        for kw in keywords {
            if partial.is_empty() || kw.starts_with(partial) {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    detail: Some("keyword".into()),
                    kind: completion_kind("keyword"),
                    insert_text: Some(kw.to_string()),
                });
            }
        }

        if let Some(m) = module {
            for item in &m.items {
                let (name, kind_str) = match item {
                    crate::ast::ModuleItem::Agent(a) => (a.name.as_str(), "agent"),
                    crate::ast::ModuleItem::Task(t) => (t.name.as_str(), "task"),
                    crate::ast::ModuleItem::Workflow(w) => (w.name.as_str(), "workflow"),
                    crate::ast::ModuleItem::Goal(g) => (g.name.as_str(), "goal"),
                    crate::ast::ModuleItem::Memory(mem) => (mem.name.as_str(), "memory"),
                    crate::ast::ModuleItem::Tool(tl) => (tl.name.as_str(), "tool"),
                    crate::ast::ModuleItem::Model(md) => (md.name.as_str(), "model"),
                    crate::ast::ModuleItem::Prompt(p) => (p.name.as_str(), "prompt"),
                    crate::ast::ModuleItem::Policy(p) => (p.name.as_str(), "policy"),
                    crate::ast::ModuleItem::Type(t) => (t.name.as_str(), "type"),
                    _ => continue,
                };
                if partial.is_empty() || name.starts_with(partial) {
                    items.push(CompletionItem {
                        label: name.to_string(),
                        detail: Some(kind_str.into()),
                        kind: completion_kind("class"),
                        insert_text: Some(name.to_string()),
                    });
                }
            }
        }
        items
    }
}

fn word_before(text: &str) -> &str {
    let trimmed = text.trim_end();
    if let Some(pos) = trimmed.rfind(|c: char| {
        c.is_whitespace() || matches!(c, '(' | ')' | '{' | '}' | '[' | ']' | ',' | ':' | '.' | '=' | '>' | '<' | '+' | '-' | '*' | '/' | '|')
    }) {
        &trimmed[pos + 1..]
    } else {
        trimmed
    }
}

fn infer_context(source: &str, offset: usize) -> Context {
    let text_before = &source[..offset.min(source.len())];
    let lines: Vec<&str> = text_before.lines().collect();
    let mut in_agent = false;
    let mut in_task = false;
    let mut indent_level = 0usize;

    for line in &lines {
        let trimmed = line.trim();
        let line_indent = line.len() - trimmed.len();
        if line_indent == 0 {
            in_agent = false; in_task = false;
            if trimmed.starts_with("agent ") { in_agent = true; }
            if trimmed.starts_with("task ") { in_task = true; }
            indent_level = 0;
        } else {
            indent_level = line_indent;
        }
        if trimmed.starts_with("agent ") { in_agent = true; in_task = false; }
        else if trimmed.starts_with("task ") {
            if in_agent || indent_level == 0 { in_task = true; in_agent = false; }
        }
    }

    let last = lines.last().map(|s| s.trim()).unwrap_or("");
    if in_task && indent_level > 0 {
        let stmt_starters = ["let ", "const ", "if ", "match ", "loop ", "for ",
            "return ", "await ", "attempt ", "swarm "];
        if stmt_starters.iter().any(|s| last.starts_with(s))
            || last.contains("steps") || last.contains("rollback")
        {
            Context::Statement
        } else {
            Context::TaskBody
        }
    } else if in_agent && indent_level > 0 {
        Context::AgentBody
    } else if indent_level > 0 {
        Context::Statement
    } else {
        Context::TopLevel
    }
}

// =============================================================================
// Diagnostic Engine
// =============================================================================

pub struct DiagnosticEngine;

impl DiagnosticEngine {
    pub fn check(&self, source: &str) -> Vec<LspDiagnostic> {
        use crate::ast::span::FileId;
        use crate::diagnostics::diagnostic::Severity;
        use crate::diagnostics::DiagnosticBag;
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let file_id = FileId(0);
        let mut lexer = Lexer::new(source, file_id);
        let tokens = lexer.tokenize();
        let mut diags = DiagnosticBag::new();
        let mut parser = Parser::new(tokens, file_id);
        let _module = parser.parse_module();
        diags.extend(parser.diagnostics);

        let mut lsp_diags: Vec<LspDiagnostic> = Vec::new();
        for diag in &diags.diagnostics {
            let severity = match diag.severity {
                Severity::Error => LspSeverity::Error,
                Severity::Warning => LspSeverity::Warning,
                Severity::Note => LspSeverity::Information,
            };
            let range = if let Some(label) = diag.labels.first() {
                span_to_lsp_range(label.span, source)
            } else {
                Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } }
            };
            lsp_diags.push(LspDiagnostic {
                range, severity,
                code: diag.code.clone(),
                message: diag.message.clone(),
            });
        }
        lsp_diags
    }
}

// =============================================================================
// LSP Server
// =============================================================================

const SEMANTIC_TOKEN_TYPES: &[&str] = &[
    "namespace", "type", "class", "enum", "struct", "typeParameter",
    "parameter", "variable", "property", "enumMember", "event",
    "function", "method", "macro", "keyword", "modifier",
    "comment", "string", "number", "regexp", "operator", "decorator",
];

const SEMANTIC_TOKEN_MODIFIERS: &[&str] = &[
    "declaration", "definition", "readonly", "static",
    "deprecated", "abstract", "async", "modification", "documentation", "defaultLibrary",
];

pub struct LspServer {
    documents: DocumentManager,
    completion_engine: CompletionEngine,
    diagnostic_engine: DiagnosticEngine,
    shutdown_requested: bool,
}

impl LspServer {
    pub fn new() -> Self {
        LspServer {
            documents: DocumentManager::new(),
            completion_engine: CompletionEngine,
            diagnostic_engine: DiagnosticEngine,
            shutdown_requested: false,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        let stdout = io::stdout();
        let mut out = stdout.lock();

        loop {
            let content_length = match read_content_length(&mut reader) {
                Ok(Some(n)) => n,
                Ok(None) => continue,
                Err(e) => { eprintln!("[lucky-lsp] header read error: {}", e); break; }
            };

            let mut body = vec![0u8; content_length];
            if content_length > 0 {
                if let Err(e) = reader.read_exact(&mut body) {
                    eprintln!("[lucky-lsp] body read error: {}", e);
                    break;
                }
            }

            let body_str = String::from_utf8_lossy(&body);
            let req = match JsonVal::parse(&body_str) {
                Ok(v) => v,
                Err(e) => { eprintln!("[lucky-lsp] json parse error: {}", e); continue; }
            };

            let method = req.get_str("method").unwrap_or("");
            let id_val = req.get("id").cloned();

            match method {
                "initialize" => self.handle_initialize(&id_val, &mut out)?,
                "initialized" => {},
                "textDocument/didOpen" => self.handle_did_open(&req, &mut out)?,
                "textDocument/didChange" => self.handle_did_change(&req, &mut out)?,
                "textDocument/didClose" => self.handle_did_close(&req),
                "textDocument/completion" => self.handle_completion(&id_val, &req, &mut out)?,
                "textDocument/hover" => self.handle_hover(&id_val, &req, &mut out)?,
                "textDocument/definition" => self.handle_definition(&id_val, &req, &mut out)?,
                "textDocument/references" => self.handle_references(&id_val, &req, &mut out)?,
                "textDocument/formatting" => self.handle_formatting(&id_val, &req, &mut out)?,
                "textDocument/documentSymbol" => self.handle_document_symbol(&id_val, &req, &mut out)?,
                "textDocument/semanticTokens/full" => self.handle_semantic_tokens(&id_val, &req, &mut out)?,
                "shutdown" => { self.handle_shutdown(&id_val, &mut out)?; self.shutdown_requested = true; }
                "exit" => return Ok(()),
                _ => {
                    if id_val.is_some() {
                        let msg = format!("Method not found: {}", method);
                        self.send_error(&id_val, -32601, &msg, &mut out)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn respond(&self, id: &Option<JsonVal>, build: impl FnOnce(&mut JsonBuf), out: &mut io::StdoutLock) -> io::Result<()> {
        let mut buf = JsonBuf::new();
        buf.obj_open();
        buf.key("jsonrpc"); buf.str_val("2.0");
        buf.key("id"); write_json_opt(&mut buf, id);
        buf.key("result");
        build(&mut buf);
        buf.obj_close();
        send_json(out, &buf.into_string())
    }

    fn handle_initialize(&self, id: &Option<JsonVal>, out: &mut io::StdoutLock) -> io::Result<()> {
        self.respond(id, |buf| {
            buf.obj_open();
            buf.key("capabilities"); buf.obj_open();

            buf.key("textDocumentSync"); buf.num_val(1);

            buf.key("completionProvider"); buf.obj_open();
            buf.key("triggerCharacters"); buf.arr_open(); buf.str_val("."); buf.arr_close();
            buf.obj_close();

            buf.key("hoverProvider"); buf.bool_val(true);
            buf.key("definitionProvider"); buf.bool_val(true);
            buf.key("referencesProvider"); buf.bool_val(true);
            buf.key("documentSymbolProvider"); buf.bool_val(true);
            buf.key("documentFormattingProvider"); buf.bool_val(true);

            buf.key("semanticTokensProvider"); buf.obj_open();
            buf.key("legend"); buf.obj_open();
            buf.key("tokenTypes"); buf.arr_open();
            for tt in SEMANTIC_TOKEN_TYPES { buf.str_val(tt); }
            buf.arr_close();
            buf.key("tokenModifiers"); buf.arr_open();
            for tm in SEMANTIC_TOKEN_MODIFIERS { buf.str_val(tm); }
            buf.arr_close();
            buf.obj_close();
            buf.key("full"); buf.bool_val(true);
            buf.obj_close();

            buf.obj_close(); // capabilities

            buf.key("serverInfo"); buf.obj_open();
            buf.key("name"); buf.str_val("lucky-lsp");
            buf.key("version"); buf.str_val("0.1.0");
            buf.obj_close();

            buf.obj_close();
        }, out)
    }

    fn handle_did_open(&mut self, req: &JsonVal, out: &mut io::StdoutLock) -> io::Result<()> {
        let params = req.get("params");
        let td = params.and_then(|p| p.get("textDocument"));
        let uri = td.and_then(|t| t.get_str("uri")).unwrap_or("");
        let text = td.and_then(|t| t.get_str("text")).unwrap_or("");
        self.documents.open(uri, text);
        self.publish_diagnostics(uri, out)
    }

    fn handle_did_change(&mut self, req: &JsonVal, out: &mut io::StdoutLock) -> io::Result<()> {
        let params = match req.get("params") { Some(p) => p, None => return Ok(()), };
        let td = match params.get("textDocument") { Some(t) => t, None => return Ok(()), };
        let uri = td.get_str("uri").unwrap_or("");

        let mut changes: Vec<TextDocumentContentChangeEvent> = Vec::new();
        if let Some(arr) = params.get("contentChanges").and_then(|c| c.as_arr()) {
            for ch in arr {
                let range = ch.get("range").map(|r| Range {
                    start: Position {
                        line: r.get("start").and_then(|s| s.get_usize("line")).unwrap_or(0),
                        character: r.get("start").and_then(|s| s.get_usize("character")).unwrap_or(0),
                    },
                    end: Position {
                        line: r.get("end").and_then(|e| e.get_usize("line")).unwrap_or(0),
                        character: r.get("end").and_then(|e| e.get_usize("character")).unwrap_or(0),
                    },
                });
                let text = ch.get_str("text").map(|s| s.to_string());
                changes.push(TextDocumentContentChangeEvent { range, text });
            }
        }
        self.documents.update(uri, &changes);
        self.publish_diagnostics(uri, out)
    }

    fn handle_did_close(&mut self, req: &JsonVal) {
        let uri = req.get("params")
            .and_then(|p| p.get("textDocument"))
            .and_then(|t| t.get_str("uri"))
            .unwrap_or("");
        self.documents.close(uri);
    }

    fn handle_completion(&self, id: &Option<JsonVal>, req: &JsonVal, out: &mut io::StdoutLock) -> io::Result<()> {
        let params = match req.get("params") { Some(p) => p, None => return Ok(()), };
        let td = match params.get("textDocument") { Some(t) => t, None => return Ok(()), };
        let uri = td.get_str("uri").unwrap_or("");
        let pos = params.get("position");
        let line = pos.and_then(|p| p.get_usize("line")).unwrap_or(0);
        let ch = pos.and_then(|p| p.get_usize("character")).unwrap_or(0);
        let source = self.documents.get(uri).unwrap_or("");
        let module = self.documents.parse(uri);
        let items = self.completion_engine.completions(source, line, ch, module.as_ref());

        self.respond(id, |buf| {
            buf.obj_open();
            buf.key("isIncomplete"); buf.bool_val(false);
            buf.key("items"); buf.arr_open();
            for item in &items {
                buf.obj_open();
                buf.key("label"); buf.str_val(&item.label);
                if let Some(ref d) = item.detail { buf.key("detail"); buf.str_val(d); }
                buf.key("kind"); buf.num_val(item.kind);
                if let Some(ref ins) = item.insert_text { buf.key("insertText"); buf.str_val(ins); }
                buf.obj_close();
            }
            buf.arr_close();
            buf.obj_close();
        }, out)
    }

    fn handle_hover(&self, id: &Option<JsonVal>, req: &JsonVal, out: &mut io::StdoutLock) -> io::Result<()> {
        if id.is_none() { return Ok(()); }
        let params = match req.get("params") { Some(p) => p, None => { self.send_null(id, out)?; return Ok(()); } };
        let td = match params.get("textDocument") { Some(t) => t, None => { self.send_null(id, out)?; return Ok(()); } };
        let uri = td.get_str("uri").unwrap_or("");
        let pos = params.get("position");
        let line = pos.and_then(|p| p.get_usize("line")).unwrap_or(0);
        let ch = pos.and_then(|p| p.get_usize("character")).unwrap_or(0);
        let source = self.documents.get(uri).unwrap_or("");
        let word = word_at_position(source, line, ch);
        let module = self.documents.parse(uri);

        if let Some(ref m) = module {
            if let Some(info) = find_hover_info(m, &word) {
                let range = word_range_at_position(source, line, ch, &word);
                let info_copy = info.clone();
                let r = Range { start: range.start, end: range.end };
                return self.respond(id, |buf| {
                    buf.obj_open();
                    buf.key("contents"); buf.obj_open();
                    buf.key("kind"); buf.str_val("markdown");
                    buf.key("value"); buf.str_val(&info_copy);
                    buf.obj_close();
                    buf.key("range"); write_range(buf, &r);
                    buf.obj_close();
                }, out);
            }
        }
        self.send_null(id, out)
    }

    fn handle_definition(&self, id: &Option<JsonVal>, req: &JsonVal, out: &mut io::StdoutLock) -> io::Result<()> {
        if id.is_none() { return Ok(()); }
        let params = match req.get("params") { Some(p) => p, None => { self.send_null(id, out)?; return Ok(()); } };
        let td = match params.get("textDocument") { Some(t) => t, None => { self.send_null(id, out)?; return Ok(()); } };
        let uri = td.get_str("uri").unwrap_or("");
        let pos = params.get("position");
        let line = pos.and_then(|p| p.get_usize("line")).unwrap_or(0);
        let ch = pos.and_then(|p| p.get_usize("character")).unwrap_or(0);
        let source = self.documents.get(uri).unwrap_or("");
        let word = word_at_position(source, line, ch);
        let module = self.documents.parse(uri);

        if let Some(ref m) = module {
            if let Some(range) = find_definition_range(m, source, &word) {
                let own_uri = uri.to_string();
                return self.respond(id, |buf| {
                    buf.obj_open();
                    buf.key("uri"); buf.str_val(&own_uri);
                    buf.key("range"); write_range(buf, &range);
                    buf.obj_close();
                }, out);
            }
        }
        self.send_null(id, out)
    }

    fn handle_references(&self, id: &Option<JsonVal>, req: &JsonVal, out: &mut io::StdoutLock) -> io::Result<()> {
        if id.is_none() { return Ok(()); }
        let params = match req.get("params") { Some(p) => p, None => { self.send_empty(id, out)?; return Ok(()); } };
        let td = match params.get("textDocument") { Some(t) => t, None => { self.send_empty(id, out)?; return Ok(()); } };
        let uri = td.get_str("uri").unwrap_or("");
        let pos = params.get("position");
        let line = pos.and_then(|p| p.get_usize("line")).unwrap_or(0);
        let ch = pos.and_then(|p| p.get_usize("character")).unwrap_or(0);
        let source = self.documents.get(uri).unwrap_or("");
        let word = word_at_position(source, line, ch);
        let refs = find_references(source, &word, uri);
        let own_uri = uri.to_string();

        self.respond(id, |buf| {
            buf.arr_open();
            for (ruri, r) in &refs {
                buf.obj_open();
                buf.key("uri"); buf.str_val(if ruri.is_empty() { &own_uri } else { ruri });
                buf.key("range"); write_range(buf, r);
                buf.obj_close();
            }
            buf.arr_close();
        }, out)
    }

    fn handle_formatting(&self, id: &Option<JsonVal>, req: &JsonVal, out: &mut io::StdoutLock) -> io::Result<()> {
        if id.is_none() { return Ok(()); }
        let params = match req.get("params") { Some(p) => p, None => { self.send_empty(id, out)?; return Ok(()); } };
        let td = match params.get("textDocument") { Some(t) => t, None => { self.send_empty(id, out)?; return Ok(()); } };
        let uri = td.get_str("uri").unwrap_or("");
        let source = self.documents.get(uri).unwrap_or("");
        let tab_size = params.get("options").and_then(|o| o.get_usize("tabSize")).unwrap_or(4);
        let formatted = format_lucky(source, tab_size);
        let full_range = Range { start: Position { line: 0, character: 0 }, end: end_position(source) };

        self.respond(id, |buf| {
            buf.arr_open();
            buf.obj_open();
            buf.key("range"); write_range(buf, &full_range);
            buf.key("newText"); buf.str_val(&formatted);
            buf.obj_close();
            buf.arr_close();
        }, out)
    }

    fn handle_document_symbol(&self, id: &Option<JsonVal>, req: &JsonVal, out: &mut io::StdoutLock) -> io::Result<()> {
        if id.is_none() { return Ok(()); }
        let params = match req.get("params") { Some(p) => p, None => { self.send_empty(id, out)?; return Ok(()); } };
        let td = match params.get("textDocument") { Some(t) => t, None => { self.send_empty(id, out)?; return Ok(()); } };
        let uri = td.get_str("uri").unwrap_or("");
        let source = self.documents.get(uri).unwrap_or("");
        let module = self.documents.parse(uri);
        let symbols = if let Some(ref m) = module { build_document_symbols(m, source) } else { Vec::new() };

        self.respond(id, |buf| { write_symbols_array(buf, &symbols); }, out)
    }

    fn handle_semantic_tokens(&self, id: &Option<JsonVal>, req: &JsonVal, out: &mut io::StdoutLock) -> io::Result<()> {
        if id.is_none() { return Ok(()); }
        let params = match req.get("params") { Some(p) => p, None => { self.send_null(id, out)?; return Ok(()); } };
        let td = match params.get("textDocument") { Some(t) => t, None => { self.send_null(id, out)?; return Ok(()); } };
        let uri = td.get_str("uri").unwrap_or("");
        let source = self.documents.get(uri).unwrap_or("");
        let tokens = compute_semantic_tokens(source);

        self.respond(id, |buf| {
            buf.obj_open();
            buf.key("data"); buf.arr_open();
            for t in &tokens { buf.num_val(*t); }
            buf.arr_close();
            buf.obj_close();
        }, out)
    }

    fn handle_shutdown(&self, id: &Option<JsonVal>, out: &mut io::StdoutLock) -> io::Result<()> {
        self.send_null(id, out)
    }

    fn publish_diagnostics(&self, uri: &str, out: &mut io::StdoutLock) -> io::Result<()> {
        let source = self.documents.get(uri).unwrap_or("");
        let diags = self.diagnostic_engine.check(source);
        let mut buf = JsonBuf::new();
        buf.obj_open();
        buf.key("jsonrpc"); buf.str_val("2.0");
        buf.key("method"); buf.str_val("textDocument/publishDiagnostics");
        buf.key("params"); buf.obj_open();
        buf.key("uri"); buf.str_val(uri);
        buf.key("diagnostics"); buf.arr_open();
        for d in &diags {
            buf.obj_open();
            buf.key("range"); buf.obj_open();
            buf.key("start"); buf.obj_open();
            buf.key("line"); buf.num_val(d.range.start.line);
            buf.key("character"); buf.num_val(d.range.start.character);
            buf.obj_close();
            buf.key("end"); buf.obj_open();
            buf.key("line"); buf.num_val(d.range.end.line);
            buf.key("character"); buf.num_val(d.range.end.character);
            buf.obj_close();
            buf.obj_close();
            buf.key("severity"); buf.num_val(d.severity as u8);
            buf.key("message"); buf.str_val(&d.message);
            if let Some(ref code) = d.code { buf.key("code"); buf.str_val(code); }
            buf.obj_close();
        }
        buf.arr_close();
        buf.obj_close();
        buf.obj_close();
        send_json(out, &buf.into_string())
    }

    fn send_null(&self, id: &Option<JsonVal>, out: &mut io::StdoutLock) -> io::Result<()> {
        let mut buf = JsonBuf::new();
        buf.obj_open();
        buf.key("jsonrpc"); buf.str_val("2.0");
        buf.key("id"); write_json_opt(&mut buf, id);
        buf.key("result"); buf.null();
        buf.obj_close();
        send_json(out, &buf.into_string())
    }

    fn send_empty(&self, id: &Option<JsonVal>, out: &mut io::StdoutLock) -> io::Result<()> {
        let mut buf = JsonBuf::new();
        buf.obj_open();
        buf.key("jsonrpc"); buf.str_val("2.0");
        buf.key("id"); write_json_opt(&mut buf, id);
        buf.key("result"); buf.arr_open(); buf.arr_close();
        buf.obj_close();
        send_json(out, &buf.into_string())
    }

    fn send_error(&self, id: &Option<JsonVal>, code: i32, msg: &str, out: &mut io::StdoutLock) -> io::Result<()> {
        let m = msg.to_string();
        let mut buf = JsonBuf::new();
        buf.obj_open();
        buf.key("jsonrpc"); buf.str_val("2.0");
        buf.key("id"); write_json_opt(&mut buf, id);
        buf.key("error"); buf.obj_open();
        buf.key("code"); buf.num_val(code);
        buf.key("message"); buf.str_val(&m);
        buf.obj_close();
        buf.obj_close();
        send_json(out, &buf.into_string())
    }
}

// =============================================================================
// Hover info
// =============================================================================

fn find_hover_info(module: &crate::ast::Module, word: &str) -> Option<String> {
    use crate::ast::ModuleItem;
    for item in &module.items {
        match item {
            ModuleItem::Agent(a) if a.name == word => {
                let mut info = format!("```lucky\nagent {}\n```\n\nAn AI agent that can execute tasks.", a.name);
                if let Some(ref m) = a.model { info.push_str(&format!("\n\n**Model:** `{}`", m.to_string())); }
                if !a.tools.is_empty() {
                    let ts: Vec<String> = a.tools.iter().map(|t| format!("`{}`", t.to_string())).collect();
                    info.push_str(&format!("\n\n**Tools:** {}", ts.join(", ")));
                }
                if !a.tasks.is_empty() {
                    let ts: Vec<String> = a.tasks.iter().map(|t| format!("`{}`", t.name)).collect();
                    info.push_str(&format!("\n\n**Tasks:** {}", ts.join(", ")));
                }
                return Some(info);
            }
            ModuleItem::Task(t) if t.name == word => {
                let mut info = format!("```lucky\ntask {}", t.name);
                if t.inputs.is_empty() && t.outputs.is_empty() {
                    info.push_str("\n```");
                } else {
                    info.push_str("(\n");
                    for inp in &t.inputs {
                        info.push_str(&format!("    input {}: {}\n", inp.name, fmt_type_opt(inp.typ.as_deref())));
                    }
                    for outp in &t.outputs {
                        info.push_str(&format!("    output {}: {}\n", outp.name, fmt_type_opt(outp.typ.as_deref())));
                    }
                    info.push_str(")\n```");
                }
                if t.is_stateful { info.push_str("\n\n*(stateful)*"); }
                return Some(info);
            }
            ModuleItem::Workflow(w) if w.name == word => {
                return Some(format!("```lucky\nworkflow {}\n```\n\nA workflow orchestrating multiple tasks.", w.name));
            }
            ModuleItem::Goal(g) if g.name == word => {
                let cs: Vec<String> = g.success_criteria.iter().map(|c| format!("- {}", c)).collect();
                return Some(format!("```lucky\ngoal {}\n```\n\n**Success criteria:**\n{}", g.name, cs.join("\n")));
            }
            ModuleItem::Memory(m) if m.name == word => {
                let scope = m.scope.as_deref().unwrap_or("agent");
                let backend = m.backend.as_deref().unwrap_or("default");
                return Some(format!("```lucky\nmemory {}\n```\n\n**Scope:** {}  \n**Backend:** {}", m.name, scope, backend));
            }
            ModuleItem::Model(m) if m.name == word => {
                return Some(format!("```lucky\nmodel {}\n```\n\nA language model configuration.", m.name));
            }
            ModuleItem::Tool(t) if t.name == word => {
                let mut info = format!("```lucky\ntool {}\n```\n\nAn external tool integration.", t.name);
                if !t.config.is_empty() {
                    info.push_str("\n\n**Config:**");
                    for (k, _) in &t.config { info.push_str(&format!("\n- `{}`", k)); }
                }
                return Some(info);
            }
            ModuleItem::Type(td) if td.name == word => {
                return Some(format!("```lucky\ntype {} = {}\n```\n\nA user-defined type alias.", td.name, fmt_type(&td.typ)));
            }
            _ => {}
        }
    }
    None
}

fn fmt_type_opt(typ: Option<&crate::ast::types::TypeExpr>) -> String {
    typ.map(fmt_type).unwrap_or_else(|| "Any".into())
}

fn fmt_type(typ: &crate::ast::types::TypeExpr) -> String {
    use crate::ast::types::TypeExpr;
    match typ {
        TypeExpr::Primitive { name, .. } => name.clone(),
        TypeExpr::Named { name, .. } => name.clone(),
        TypeExpr::Nullable { inner, .. } => format!("{}?", fmt_type(inner)),
        TypeExpr::Optional { inner, .. } => format!("{}!", fmt_type(inner)),
        TypeExpr::Union { left, right, .. } => format!("{} | {}", fmt_type(left), fmt_type(right)),
        TypeExpr::List { element, .. } => format!("List<{}>", fmt_type(element)),
        TypeExpr::Set { element, .. } => format!("Set<{}>", fmt_type(element)),
        TypeExpr::Map { key, value, .. } => format!("Map<{}, {}>", fmt_type(key), fmt_type(value)),
        TypeExpr::Tuple { elements, .. } => {
            let parts: Vec<String> = elements.iter().map(fmt_type).collect();
            format!("({})", parts.join(", "))
        }
        TypeExpr::Function { params, returns, .. } => {
            let p: Vec<String> = params.iter().map(fmt_type).collect();
            let r: Vec<String> = returns.iter().map(fmt_type).collect();
            format!("fn({}) -> {}", p.join(", "), r.join(", "))
        }
        TypeExpr::Qualified { path, .. } => path.join("."),
        TypeExpr::Paren { inner, .. } => format!("({})", fmt_type(inner)),
        TypeExpr::Error { .. } => "?".into(),
    }
}

// =============================================================================
// Go-to-definition
// =============================================================================

fn find_definition_range(module: &crate::ast::Module, source: &str, word: &str) -> Option<Range> {
    use crate::ast::ModuleItem;
    for item in &module.items {
        let (name, span) = match item {
            ModuleItem::Agent(a) => (&a.name, a.span),
            ModuleItem::Task(t) => (&t.name, t.span),
            ModuleItem::Workflow(w) => (&w.name, w.span),
            ModuleItem::Goal(g) => (&g.name, g.span),
            ModuleItem::Memory(m) => (&m.name, m.span),
            ModuleItem::Tool(t) => (&t.name, t.span),
            ModuleItem::Model(m) => (&m.name, m.span),
            ModuleItem::Prompt(p) => (&p.name, p.span),
            ModuleItem::Policy(p) => (&p.name, p.span),
            ModuleItem::Type(t) => (&t.name, t.span),
            _ => continue,
        };
        if name == word { return Some(span_to_lsp_range(span, source)); }
    }
    None
}

// =============================================================================
// References
// =============================================================================

fn find_references(source: &str, word: &str, uri: &str) -> Vec<(String, Range)> {
    if word.is_empty() { return Vec::new(); }
    let mut refs: Vec<(String, Range)> = Vec::new();
    let mut start: usize = 0;
    while let Some(pos) = source[start..].find(word) {
        let abs_pos = start + pos;
        let end_pos = abs_pos + word.len();
        let before_ok = abs_pos == 0 || !is_ident_byte(source.as_bytes()[abs_pos - 1]);
        let after_ok = end_pos >= source.len() || !is_ident_byte(source.as_bytes()[end_pos]);
        if before_ok && after_ok {
            refs.push((uri.to_string(), Range {
                start: offset_to_lsp_position(abs_pos, source),
                end: offset_to_lsp_position(end_pos, source),
            }));
        }
        start = abs_pos + 1;
    }
    refs
}

// =============================================================================
// Document Symbols
// =============================================================================

struct DocSymbol {
    name: String,
    kind: u32,
    range: Range,
    selection_range: Range,
    children: Vec<DocSymbol>,
}

fn build_document_symbols(module: &crate::ast::Module, source: &str) -> Vec<DocSymbol> {
    use crate::ast::ModuleItem;
    let mut symbols: Vec<DocSymbol> = Vec::new();

    for item in &module.items {
        match item {
            ModuleItem::Agent(a) => {
                let range = span_to_lsp_range(a.span, source);
                let children: Vec<DocSymbol> = a.tasks.iter().map(|t| {
                    let r = span_to_lsp_range(t.span, source);
                    DocSymbol { name: t.name.clone(), kind: symbol_kind("method"), range: r, selection_range: r, children: Vec::new() }
                }).collect();
                symbols.push(DocSymbol { name: a.name.clone(), kind: symbol_kind("class"), range, selection_range: range, children });
            }
            ModuleItem::Task(t) => {
                let r = span_to_lsp_range(t.span, source);
                symbols.push(DocSymbol { name: t.name.clone(), kind: symbol_kind("method"), range: r, selection_range: r, children: Vec::new() });
            }
            ModuleItem::Workflow(w) => {
                let r = span_to_lsp_range(w.span, source);
                symbols.push(DocSymbol { name: w.name.clone(), kind: symbol_kind("function"), range: r, selection_range: r, children: Vec::new() });
            }
            ModuleItem::Goal(g) => {
                let r = span_to_lsp_range(g.span, source);
                symbols.push(DocSymbol { name: g.name.clone(), kind: symbol_kind("interface"), range: r, selection_range: r, children: Vec::new() });
            }
            ModuleItem::Type(td) => {
                let r = span_to_lsp_range(td.span, source);
                symbols.push(DocSymbol { name: td.name.clone(), kind: symbol_kind("struct"), range: r, selection_range: r, children: Vec::new() });
            }
            ModuleItem::Memory(m) => {
                let r = span_to_lsp_range(m.span, source);
                symbols.push(DocSymbol { name: m.name.clone(), kind: symbol_kind("property"), range: r, selection_range: r, children: Vec::new() });
            }
            _ => {}
        }
    }
    symbols
}

fn write_symbols_array(buf: &mut JsonBuf, symbols: &[DocSymbol]) {
    buf.arr_open();
    for sym in symbols {
        buf.obj_open();
        buf.key("name"); buf.str_val(&sym.name);
        buf.key("kind"); buf.num_val(sym.kind);
        buf.key("range"); write_range(buf, &sym.range);
        buf.key("selectionRange"); write_range(buf, &sym.selection_range);
        if !sym.children.is_empty() {
            buf.key("children");
            write_symbols_array(buf, &sym.children);
        }
        buf.obj_close();
    }
    buf.arr_close();
}

// =============================================================================
// Semantic Tokens
// =============================================================================

fn compute_semantic_tokens(source: &str) -> Vec<usize> {
    use crate::ast::span::FileId;
    use crate::lexer::token::TokenKind;
    use crate::lexer::Lexer;

    let file_id = FileId(0);
    let mut lexer = Lexer::new(source, file_id);
    let tokens = lexer.tokenize();

    let mut data: Vec<usize> = Vec::new();
    let mut prev_line: usize = 0;
    let mut prev_col: usize = 0;

    for token in &tokens {
        let pos = offset_to_lsp_position(token.span.start, source);
        let end_pos = offset_to_lsp_position(token.span.end, source);

        let token_type = match token.kind {
            TokenKind::Keyword => {
                match token.text.as_str() { "pub" | "stateful" => 15, _ => 14 }
            }
            TokenKind::StringLit => 16,
            TokenKind::IntLit | TokenKind::FloatLit | TokenKind::BoolLit | TokenKind::NullLit => 17,
            TokenKind::Comment | TokenKind::DocComment => 15,
            TokenKind::Ident => 7,
            _ => continue,
        };

        let len = if token.text.is_empty() {
            end_pos.character.saturating_sub(pos.character)
        } else {
            token.text.encode_utf16().count()
        };
        if len == 0 { continue; }

        let delta_line = pos.line.saturating_sub(prev_line);
        let delta_col = if delta_line == 0 { pos.character.saturating_sub(prev_col) } else { pos.character };

        data.push(delta_line);
        data.push(delta_col);
        data.push(len);
        data.push(token_type);
        data.push(0);

        prev_line = pos.line;
        prev_col = pos.character;
    }
    data
}

// =============================================================================
// Formatting
// =============================================================================

fn format_lucky(source: &str, tab_size: usize) -> String {
    let spaces = " ".repeat(tab_size);
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() { return String::new(); }

    let mut use_tabs = false;
    for line in &lines {
        if line.starts_with('\t') { use_tabs = true; break; }
    }
    let indent_str: &str = if use_tabs { "\t" } else { &spaces };

    let mut result = String::with_capacity(source.len() + 1024);
    let mut prev_indent: usize = 0;
    let mut prev_was_empty = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() { prev_was_empty = true; continue; }
        if i > 0 && !prev_was_empty && is_top_level(trimmed) && prev_indent == 0 {
            result.push('\n');
        }
        let indent = compute_indent(line, &lines, i);
        result.push_str(&indent_str.repeat(indent));
        result.push_str(trimmed);
        result.push('\n');
        prev_indent = indent;
        prev_was_empty = false;
    }

    while result.ends_with('\n') { result.pop(); }
    result.push('\n');
    result
}

fn is_top_level(line: &str) -> bool {
    ["agent", "task", "workflow", "goal", "memory", "tool", "model",
     "prompt", "policy", "type", "context", "permissions", "approval",
     "project", "import", "pub"].iter().any(|d| line.starts_with(d))
}

fn compute_indent(line: &str, lines: &[&str], idx: usize) -> usize {
    let leading = line.len() - line.trim_start().len();
    if leading == 0 {
        if idx > 0 {
            let prev = lines[idx - 1].trim();
            let prev_indent = lines[idx - 1].len() - prev.len();
            if prev.ends_with(':') { return (prev_indent / 4) + 1; }
        }
        0
    } else {
        leading / 4
    }
}

// =============================================================================
// Entry point
// =============================================================================

/// Start the LSP server, reading from stdin and writing to stdout.
/// Blocks until the client sends `exit`.
pub fn run_lsp_server() -> io::Result<()> {
    let mut server = LspServer::new();
    server.run()
}
