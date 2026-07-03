use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{Debugger, DebugState, PauseReason};

static NEXT_SEQ: AtomicU64 = AtomicU64::new(1);

fn next_seq() -> u64 {
    NEXT_SEQ.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        match self {
            JsonValue::Number(n) => Some(*n as usize),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(o) => o.get(key),
            _ => None,
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }

    pub fn get_usize(&self, key: &str) -> Option<usize> {
        self.get(key).and_then(|v| v.as_usize())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_array(&self, key: &str) -> Option<&[JsonValue]> {
        self.get(key).and_then(|v| v.as_array())
    }
}

pub fn parse_json(input: &str) -> Result<JsonValue, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;
    skip_whitespace(&chars, &mut pos);
    let val = parse_value(&chars, &mut pos)?;
    Ok(val)
}

fn skip_whitespace(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn parse_value(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    skip_whitespace(chars, pos);
    if *pos >= chars.len() {
        return Err("unexpected end of input".into());
    }
    match chars[*pos] {
        'n' => parse_null(chars, pos),
        't' => parse_true(chars, pos),
        'f' => parse_false(chars, pos),
        '"' => parse_string(chars, pos),
        '[' => parse_array(chars, pos),
        '{' => parse_object(chars, pos),
        c if c.is_ascii_digit() || c == '-' => parse_number(chars, pos),
        c => Err(format!("unexpected character '{}' at pos {}", c, pos)),
    }
}

fn parse_null(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    if chars[*pos..].starts_with(&['n', 'u', 'l', 'l']) {
        *pos += 4;
        Ok(JsonValue::Null)
    } else {
        Err("expected null".into())
    }
}

fn parse_true(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    if chars[*pos..].starts_with(&['t', 'r', 'u', 'e']) {
        *pos += 4;
        Ok(JsonValue::Bool(true))
    } else {
        Err("expected true".into())
    }
}

fn parse_false(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    if chars[*pos..].starts_with(&['f', 'a', 'l', 's', 'e']) {
        *pos += 5;
        Ok(JsonValue::Bool(false))
    } else {
        Err("expected false".into())
    }
}

fn parse_string(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    *pos += 1;
    let mut s = String::new();
    while *pos < chars.len() {
        let c = chars[*pos];
        *pos += 1;
        if c == '"' {
            return Ok(JsonValue::String(s));
        }
        if c == '\\' && *pos < chars.len() {
            let esc = chars[*pos];
            *pos += 1;
            match esc {
                '"' => s.push('"'),
                '\\' => s.push('\\'),
                '/' => s.push('/'),
                'b' => s.push('\x08'),
                'f' => s.push('\x0c'),
                'n' => s.push('\n'),
                'r' => s.push('\r'),
                't' => s.push('\t'),
                'u' => {
                    let mut hex = String::new();
                    for _ in 0..4 {
                        if *pos >= chars.len() {
                            return Err("unterminated unicode escape".into());
                        }
                        hex.push(chars[*pos]);
                        *pos += 1;
                    }
                    let code = u32::from_str_radix(&hex, 16)
                        .map_err(|_| format!("invalid unicode escape \\u{}", hex))?;
                    if let Some(c) = char::from_u32(code) {
                        s.push(c);
                    }
                }
                _ => s.push(esc),
            }
        } else {
            s.push(c);
        }
    }
    Err("unterminated string".into())
}

fn parse_number(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    let start = *pos;
    if *pos < chars.len() && chars[*pos] == '-' {
        *pos += 1;
    }
    while *pos < chars.len() && chars[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if *pos < chars.len() && chars[*pos] == '.' {
        *pos += 1;
        while *pos < chars.len() && chars[*pos].is_ascii_digit() {
            *pos += 1;
        }
    }
    if *pos < chars.len() && (chars[*pos] == 'e' || chars[*pos] == 'E') {
        *pos += 1;
        if *pos < chars.len() && (chars[*pos] == '+' || chars[*pos] == '-') {
            *pos += 1;
        }
        while *pos < chars.len() && chars[*pos].is_ascii_digit() {
            *pos += 1;
        }
    }
    let num_str: String = chars[start..*pos].iter().collect();
    num_str
        .parse::<f64>()
        .map(JsonValue::Number)
        .map_err(|_| format!("invalid number {}", num_str))
}

fn parse_array(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    *pos += 1;
    let mut arr = Vec::new();
    skip_whitespace(chars, pos);
    if *pos < chars.len() && chars[*pos] == ']' {
        *pos += 1;
        return Ok(JsonValue::Array(arr));
    }
    loop {
        let val = parse_value(chars, pos)?;
        arr.push(val);
        skip_whitespace(chars, pos);
        if *pos >= chars.len() {
            return Err("unterminated array".into());
        }
        if chars[*pos] == ']' {
            *pos += 1;
            return Ok(JsonValue::Array(arr));
        }
        if chars[*pos] != ',' {
            return Err(format!("expected ',' or ']' in array at pos {}", pos));
        }
        *pos += 1;
    }
}

fn parse_object(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    *pos += 1;
    let mut obj = HashMap::new();
    skip_whitespace(chars, pos);
    if *pos < chars.len() && chars[*pos] == '}' {
        *pos += 1;
        return Ok(JsonValue::Object(obj));
    }
    loop {
        skip_whitespace(chars, pos);
        if *pos >= chars.len() || chars[*pos] != '"' {
            return Err("expected string key in object".into());
        }
        let key = match parse_string(chars, pos)? {
            JsonValue::String(s) => s,
            _ => return Err("expected string key".into()),
        };
        skip_whitespace(chars, pos);
        if *pos >= chars.len() || chars[*pos] != ':' {
            return Err("expected ':' in object".into());
        }
        *pos += 1;
        let val = parse_value(chars, pos)?;
        obj.insert(key, val);
        skip_whitespace(chars, pos);
        if *pos >= chars.len() {
            return Err("unterminated object".into());
        }
        if chars[*pos] == '}' {
            *pos += 1;
            return Ok(JsonValue::Object(obj));
        }
        if chars[*pos] != ',' {
            return Err(format!("expected ',' or '}}' in object at pos {}", pos));
        }
        *pos += 1;
    }
}

fn json_to_string(value: &JsonValue, buf: &mut String) {
    match value {
        JsonValue::Null => buf.push_str("null"),
        JsonValue::Bool(b) => buf.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => {
            if *n == (*n as i64) as f64 && n.is_finite() {
                buf.push_str(&format!("{}", *n as i64));
            } else {
                buf.push_str(&format!("{}", n));
            }
        }
        JsonValue::String(s) => {
            buf.push('"');
            for c in s.chars() {
                match c {
                    '"' => buf.push_str("\\\""),
                    '\\' => buf.push_str("\\\\"),
                    '\x08' => buf.push_str("\\b"),
                    '\x0c' => buf.push_str("\\f"),
                    '\n' => buf.push_str("\\n"),
                    '\r' => buf.push_str("\\r"),
                    '\t' => buf.push_str("\\t"),
                    c if c.is_control() => {
                        buf.push_str(&format!("\\u{:04x}", c as u32));
                    }
                    _ => buf.push(c),
                }
            }
            buf.push('"');
        }
        JsonValue::Array(arr) => {
            buf.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                json_to_string(v, buf);
            }
            buf.push(']');
        }
        JsonValue::Object(obj) => {
            buf.push('{');
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                buf.push('"');
                for c in k.chars() {
                    match c {
                        '"' => buf.push_str("\\\""),
                        '\\' => buf.push_str("\\\\"),
                        c if c.is_control() => buf.push_str(&format!("\\u{:04x}", c as u32)),
                        _ => buf.push(c),
                    }
                }
                buf.push('"');
                buf.push(':');
                json_to_string(obj.get(*k).unwrap(), buf);
            }
            buf.push('}');
        }
    }
}

pub fn serialize_json(value: &JsonValue) -> String {
    let mut buf = String::new();
    json_to_string(value, &mut buf);
    buf
}

#[derive(Debug, Clone)]
pub struct DapMessage {
    pub seq: u64,
    pub msg_type: String,
    pub command: Option<String>,
    pub event: Option<String>,
    pub request_seq: Option<u64>,
    pub success: Option<bool>,
    pub arguments: Option<JsonValue>,
    pub body: Option<JsonValue>,
    pub message: Option<String>,
}

impl DapMessage {
    pub fn from_json(raw: &str) -> Result<Self, String> {
        let val = parse_json(raw)?;
        let obj = match &val {
            JsonValue::Object(o) => o,
            _ => return Err("expected JSON object".into()),
        };

        Ok(DapMessage {
            seq: obj.get("seq").and_then(|v| v.as_i64()).unwrap_or(0) as u64,
            msg_type: obj
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            command: obj.get("command").and_then(|v| v.as_str()).map(String::from),
            event: obj.get("event").and_then(|v| v.as_str()).map(String::from),
            request_seq: obj.get("request_seq").and_then(|v| v.as_i64()).map(|n| n as u64),
            success: obj.get("success").and_then(|v| v.as_bool()),
            arguments: obj.get("arguments").cloned(),
            body: obj.get("body").cloned(),
            message: obj.get("message").and_then(|v| v.as_str()).map(String::from),
        })
    }

    pub fn to_json(&self) -> JsonValue {
        let mut obj = HashMap::new();
        obj.insert("seq".into(), JsonValue::Number(self.seq as f64));
        obj.insert("type".into(), JsonValue::String(self.msg_type.clone()));

        if let Some(ref cmd) = self.command {
            obj.insert("command".into(), JsonValue::String(cmd.clone()));
        }
        if let Some(ref ev) = self.event {
            obj.insert("event".into(), JsonValue::String(ev.clone()));
        }
        if let Some(rs) = self.request_seq {
            obj.insert("request_seq".into(), JsonValue::Number(rs as f64));
        }
        if let Some(s) = self.success {
            obj.insert("success".into(), JsonValue::Bool(s));
        }
        if let Some(ref body) = self.body {
            obj.insert("body".into(), body.clone());
        }
        if let Some(ref msg) = self.message {
            obj.insert("message".into(), JsonValue::String(msg.clone()));
        }

        JsonValue::Object(obj)
    }

    pub fn request(seq: u64, command: &str, arguments: Option<JsonValue>) -> Self {
        DapMessage {
            seq,
            msg_type: "request".into(),
            command: Some(command.into()),
            event: None,
            request_seq: None,
            success: None,
            arguments,
            body: None,
            message: None,
        }
    }

    pub fn response(request_seq: u64, command: &str, success: bool, body: Option<JsonValue>) -> Self {
        DapMessage {
            seq: next_seq(),
            msg_type: "response".into(),
            command: Some(command.into()),
            event: None,
            request_seq: Some(request_seq),
            success: Some(success),
            arguments: None,
            body,
            message: if success {
                None
            } else {
                Some("error".into())
            },
        }
    }

    pub fn event(event: &str, body: Option<JsonValue>) -> Self {
        DapMessage {
            seq: next_seq(),
            msg_type: "event".into(),
            command: None,
            event: Some(event.into()),
            request_seq: None,
            success: None,
            arguments: None,
            body,
            message: None,
        }
    }
}

fn make_obj(pairs: &[(&str, JsonValue)]) -> JsonValue {
    let mut m = HashMap::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v.clone());
    }
    JsonValue::Object(m)
}

fn json_number(n: i64) -> JsonValue {
    JsonValue::Number(n as f64)
}

fn json_bool(b: bool) -> JsonValue {
    JsonValue::Bool(b)
}

fn json_str(s: &str) -> JsonValue {
    JsonValue::String(s.into())
}

fn json_null() -> JsonValue {
    JsonValue::Null
}

fn json_array(items: &[JsonValue]) -> JsonValue {
    JsonValue::Array(items.to_vec())
}

pub struct DapServer {
    debugger: Debugger,
    client_lines_start_at_1: bool,
    client_columns_start_at_1: bool,
    supports_variable_type: bool,
    supports_variable_paging: bool,
    thread_id: u64,
}

impl DapServer {
    pub fn new() -> Self {
        Self {
            debugger: Debugger::new(),
            client_lines_start_at_1: true,
            client_columns_start_at_1: true,
            supports_variable_type: false,
            supports_variable_paging: false,
            thread_id: 1,
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        self.run_with_streams(stdin.lock(), stdout.lock())
    }

    pub fn run_with_streams<R: Read, W: Write>(
        &mut self,
        reader: R,
        mut writer: W,
    ) -> Result<(), String> {
        let mut buf_reader = BufReader::new(reader);
        loop {
            let msg = match self.read_message(&mut buf_reader) {
                Ok(Some(m)) => m,
                Ok(None) => break,
                Err(e) => {
                    eprintln!("DAP read error: {}", e);
                    break;
                }
            };

            let response = self.handle_message(&msg);
            for resp in response {
                let json = resp.to_json();
                let body = serialize_json(&json);
                let header = format!("Content-Length: {}\r\n\r\n", body.len());
                if writer.write_all(header.as_bytes()).is_err() {
                    return Ok(());
                }
                if writer.write_all(body.as_bytes()).is_err() {
                    return Ok(());
                }
                if writer.flush().is_err() {
                    return Ok(());
                }
            }

            if let Some(ref cmd) = msg.command {
                if cmd == "disconnect" {
                    break;
                }
            }
        }
        Ok(())
    }

    fn read_message<R: Read>(&self, reader: &mut BufReader<R>) -> Result<Option<DapMessage>, String> {
        let mut content_length: Option<usize> = None;

        loop {
            let mut line = String::new();
            let n = reader
                .read_line(&mut line)
                .map_err(|e| format!("read error: {}", e))?;
            if n == 0 {
                return Ok(None);
            }
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            if line.is_empty() {
                break;
            }
            if let Some(len_str) = line.strip_prefix("Content-Length:") {
                content_length = Some(
                    len_str
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| format!("invalid Content-Length: {}", len_str))?,
                );
            }
        }

        let len = content_length.ok_or("missing Content-Length header")?;

        let mut body = vec![0u8; len];
        reader
            .read_exact(&mut body)
            .map_err(|e| format!("read body error: {}", e))?;

        let body_str =
            String::from_utf8(body).map_err(|e| format!("invalid UTF-8: {}", e))?;

        Ok(Some(DapMessage::from_json(&body_str)?))
    }

    fn handle_message(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        if msg.msg_type != "request" {
            return vec![];
        }

        let cmd = match &msg.command {
            Some(c) => c.as_str(),
            None => return vec![],
        };

        match cmd {
            "initialize" => self.handle_initialize(msg),
            "launch" => self.handle_launch(msg),
            "attach" => self.handle_attach(msg),
            "setBreakpoints" => self.handle_set_breakpoints(msg),
            "setFunctionBreakpoints" => self.handle_set_function_breakpoints(msg),
            "configurationDone" => self.handle_configuration_done(msg),
            "threads" => self.handle_threads(msg),
            "stackTrace" => self.handle_stack_trace(msg),
            "scopes" => self.handle_scopes(msg),
            "variables" => self.handle_variables(msg),
            "continue" => self.handle_continue(msg),
            "next" => self.handle_next(msg),
            "stepIn" => self.handle_step_in(msg),
            "stepOut" => self.handle_step_out(msg),
            "evaluate" => self.handle_evaluate(msg),
            "pause" => self.handle_pause(msg),
            "disconnect" => self.handle_disconnect(msg),
            _ => {
                let seq = msg.seq;
                vec![DapMessage::response(seq, cmd, true, None)]
            }
        }
    }

    fn handle_initialize(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        if let Some(ref args) = msg.arguments {
            self.client_lines_start_at_1 = args
                .get_bool("linesStartAt1")
                .unwrap_or(true);
            self.client_columns_start_at_1 = args
                .get_bool("columnsStartAt1")
                .unwrap_or(true);
            self.supports_variable_type = args
                .get_bool("supportsVariableType")
                .unwrap_or(false);
            self.supports_variable_paging = args
                .get_bool("supportsVariablePaging")
                .unwrap_or(false);
        }

        let body = make_obj(&[
            ("supportsConfigurationDoneRequest", json_bool(true)),
            ("supportsFunctionBreakpoints", json_bool(false)),
            ("supportsConditionalBreakpoints", json_bool(true)),
            ("supportsHitConditionalBreakpoints", json_bool(false)),
            ("supportsEvaluateForHovers", json_bool(true)),
            ("supportsStepBack", json_bool(false)),
            ("supportsSetVariable", json_bool(false)),
            ("supportsRestartFrame", json_bool(false)),
            ("supportsGotoTargetsRequest", json_bool(false)),
            ("supportsStepInTargetsRequest", json_bool(false)),
            ("supportsCompletionsRequest", json_bool(false)),
            ("supportsModulesRequest", json_bool(false)),
            ("supportsExceptionOptions", json_bool(false)),
            ("supportsValueFormattingOptions", json_bool(false)),
            ("supportsExceptionInfoRequest", json_bool(false)),
            ("supportTerminateDebuggee", json_bool(false)),
            ("supportsDelayedStackTraceLoading", json_bool(false)),
            ("supportsLoadedSourcesRequest", json_bool(false)),
            ("supportsLogPoints", json_bool(false)),
            ("supportsTerminateThreadsRequest", json_bool(false)),
            ("supportsSetExpression", json_bool(false)),
            ("supportsTerminateRequest", json_bool(false)),
            ("supportsDataBreakpoints", json_bool(false)),
            ("supportsReadMemoryRequest", json_bool(false)),
            ("supportsWriteMemoryRequest", json_bool(false)),
            ("supportsDisassembleRequest", json_bool(false)),
            ("supportsCancelRequest", json_bool(false)),
            ("supportsBreakpointLocationsRequest", json_bool(false)),
            ("supportsClipboardContext", json_bool(false)),
            ("supportsSteppingGranularity", json_bool(false)),
            ("supportsInstructionBreakpoints", json_bool(false)),
            ("supportsExceptionFilterOptions", json_bool(false)),
            ("supportsSingleThreadExecutionRequests", json_bool(false)),
        ]);

        let mut responses = vec![DapMessage::response(msg.seq, "initialize", true, Some(body))];

        responses.push(DapMessage::event("initialized", None));

        responses
    }

    fn handle_launch(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        if let Some(ref args) = msg.arguments {
            if let Some(program) = args.get_str("program") {
                self.debugger.set_source(program.to_string(), String::new());
            }
        }

        self.debugger.session.state = DebugState::Stopped;

        let mut responses = vec![DapMessage::response(msg.seq, "launch", true, None)];

        responses.push(DapMessage::event(
            "stopped",
            Some(make_obj(&[
                ("reason", json_str("entry")),
                ("threadId", json_number(self.thread_id as i64)),
                ("allThreadsStopped", json_bool(true)),
            ])),
        ));

        responses
    }

    fn handle_attach(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        vec![DapMessage::response(msg.seq, "attach", true, None)]
    }

    fn handle_set_breakpoints(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        let source_path = msg
            .arguments
            .as_ref()
            .and_then(|a| a.get("source"))
            .and_then(|s| s.get_str("path"))
            .unwrap_or("unknown")
            .to_string();

        let lines: Vec<usize> = msg
            .arguments
            .as_ref()
            .and_then(|a| a.get_array("breakpoints"))
            .map(|arr| {
                arr.iter()
                    .filter_map(|bp| bp.get_usize("line"))
                    .collect()
            })
            .unwrap_or_default();

        let breakpoint_ids = self.debugger.set_breakpoints_for_file(source_path.clone(), &lines);

        let mut bp_list = Vec::new();
        for (i, &line) in lines.iter().enumerate() {
            let id = breakpoint_ids.get(i).copied().unwrap_or(0);
            bp_list.push(make_obj(&[
                ("id", json_number(id as i64)),
                ("verified", json_bool(true)),
                ("line", json_number(line as i64)),
                (
                    "source",
                    make_obj(&[("path", json_str(&source_path))]),
                ),
            ]));
        }

        let body = make_obj(&[("breakpoints", json_array(&bp_list))]);

        vec![DapMessage::response(msg.seq, "setBreakpoints", true, Some(body))]
    }

    fn handle_set_function_breakpoints(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        let body = make_obj(&[("breakpoints", json_array(&[]))]);
        vec![DapMessage::response(
            msg.seq,
            "setFunctionBreakpoints",
            true,
            Some(body),
        )]
    }

    fn handle_configuration_done(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        vec![DapMessage::response(
            msg.seq,
            "configurationDone",
            true,
            None,
        )]
    }

    fn handle_threads(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        let thread = make_obj(&[
            ("id", json_number(self.thread_id as i64)),
            ("name", json_str("main")),
        ]);

        let body = make_obj(&[("threads", json_array(&[thread]))]);

        vec![DapMessage::response(msg.seq, "threads", true, Some(body))]
    }

    fn handle_stack_trace(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        self.debugger.sync_state();

        let frames: Vec<JsonValue> = self
            .debugger
            .get_stack_frames()
            .iter()
            .map(|f| {
                make_obj(&[
                    ("id", json_number(f.id as i64)),
                    ("name", json_str(&f.name)),
                    (
                        "source",
                        make_obj(&[("path", json_str(&f.file))]),
                    ),
                    ("line", json_number(f.line as i64)),
                    ("column", json_number(f.column as i64)),
                ])
            })
            .collect();

        let body = make_obj(&[
            ("stackFrames", json_array(&frames)),
            ("totalFrames", json_number(frames.len() as i64)),
        ]);

        vec![DapMessage::response(
            msg.seq,
            "stackTrace",
            true,
            Some(body),
        )]
    }

    fn handle_scopes(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        let frame_id = msg
            .arguments
            .as_ref()
            .and_then(|a| a.get_usize("frameId"))
            .unwrap_or(0);

        let mut scopes_list = Vec::new();

        let locals_ref = (frame_id + 1) * 1000;

        scopes_list.push(make_obj(&[
            ("name", json_str("Locals")),
            ("variablesReference", json_number(locals_ref as i64)),
            ("namedVariables", json_number(0)),
            ("indexedVariables", json_number(0)),
            ("expensive", json_bool(false)),
        ]));

        scopes_list.push(make_obj(&[
            ("name", json_str("Globals")),
            ("variablesReference", json_number((locals_ref + 1) as i64)),
            ("namedVariables", json_number(0)),
            ("indexedVariables", json_number(0)),
            ("expensive", json_bool(true)),
        ]));

        let body = make_obj(&[("scopes", json_array(&scopes_list))]);

        vec![DapMessage::response(msg.seq, "scopes", true, Some(body))]
    }

    fn handle_variables(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        let vars_ref = msg
            .arguments
            .as_ref()
            .and_then(|a| a.get_usize("variablesReference"))
            .unwrap_or(0);

        let frame_id = if vars_ref >= 1000 {
            (vars_ref / 1000).saturating_sub(1)
        } else {
            0
        };

        let variables = self.debugger.get_variables(frame_id);

        let var_jsons: Vec<JsonValue> = variables
            .iter()
            .map(|v| {
                let mut props = vec![
                    ("name", json_str(&v.name)),
                    ("value", json_str(&v.value)),
                    ("variablesReference", json_number(v.variables_reference as i64)),
                ];
                if self.supports_variable_type {
                    props.push(("type", json_str(&v.type_name)));
                }
                make_obj(&props)
            })
            .collect();

        let body = make_obj(&[("variables", json_array(&var_jsons))]);

        vec![DapMessage::response(
            msg.seq,
            "variables",
            true,
            Some(body),
        )]
    }

    fn handle_continue(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        self.debugger.continue_exec();

        loop {
            let has_more = self.debugger.execute_one_step();
            if !has_more {
                break;
            }
            if let DebugState::Paused { .. } = self.debugger.session.state {
                break;
            }
        }

        let mut responses = vec![DapMessage::response(
            msg.seq,
            "continue",
            true,
            Some(make_obj(&[("allThreadsContinued", json_bool(true))])),
        )];

        if matches!(self.debugger.session.state, DebugState::Stopped) {
            responses.push(DapMessage::event(
                "terminated",
                None,
            ));
        }

        responses
    }

    fn handle_next(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        self.debugger.step_over();

        loop {
            let has_more = self.debugger.execute_one_step();
            if !has_more {
                break;
            }
            if let DebugState::Paused { .. } = self.debugger.session.state {
                break;
            }
        }

        let mut responses = vec![DapMessage::response(msg.seq, "next", true, None)];

        if matches!(self.debugger.session.state, DebugState::Stopped) {
            responses.push(DapMessage::event("terminated", None));
        } else if let DebugState::Paused { reason } = self.debugger.session.state {
            responses.push(DapMessage::event(
                "stopped",
                Some(make_obj(&[
                    ("reason", json_str(reason_to_str(reason))),
                    ("threadId", json_number(self.thread_id as i64)),
                    ("allThreadsStopped", json_bool(true)),
                ])),
            ));
        }

        responses
    }

    fn handle_step_in(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        self.debugger.step_in();

        loop {
            let has_more = self.debugger.execute_one_step();
            if !has_more {
                break;
            }
            if let DebugState::Paused { .. } = self.debugger.session.state {
                break;
            }
        }

        let mut responses = vec![DapMessage::response(msg.seq, "stepIn", true, None)];

        if matches!(self.debugger.session.state, DebugState::Stopped) {
            responses.push(DapMessage::event("terminated", None));
        } else if let DebugState::Paused { reason } = self.debugger.session.state {
            responses.push(DapMessage::event(
                "stopped",
                Some(make_obj(&[
                    ("reason", json_str(reason_to_str(reason))),
                    ("threadId", json_number(self.thread_id as i64)),
                    ("allThreadsStopped", json_bool(true)),
                ])),
            ));
        }

        responses
    }

    fn handle_step_out(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        self.debugger.step_out();

        loop {
            let has_more = self.debugger.execute_one_step();
            if !has_more {
                break;
            }
            if let DebugState::Paused { .. } = self.debugger.session.state {
                break;
            }
        }

        let mut responses = vec![DapMessage::response(msg.seq, "stepOut", true, None)];

        if matches!(self.debugger.session.state, DebugState::Stopped) {
            responses.push(DapMessage::event("terminated", None));
        } else if let DebugState::Paused { reason } = self.debugger.session.state {
            responses.push(DapMessage::event(
                "stopped",
                Some(make_obj(&[
                    ("reason", json_str(reason_to_str(reason))),
                    ("threadId", json_number(self.thread_id as i64)),
                    ("allThreadsStopped", json_bool(true)),
                ])),
            ));
        }

        responses
    }

    fn handle_evaluate(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        let expression = msg
            .arguments
            .as_ref()
            .and_then(|a| a.get_str("expression"))
            .unwrap_or("");

        let frame_id = msg
            .arguments
            .as_ref()
            .and_then(|a| a.get_usize("frameId"))
            .unwrap_or(0);

        let result = self.debugger.evaluate_expression(expression, frame_id);

        let body = match result {
            Some(val) => make_obj(&[
                ("result", json_str(&format!("{}", val))),
                ("type", json_str(val.type_name())),
                ("variablesReference", json_number(0)),
            ]),
            None => make_obj(&[
                ("result", json_str("undefined")),
                ("type", json_str("undefined")),
                ("variablesReference", json_number(0)),
            ]),
        };

        vec![DapMessage::response(msg.seq, "evaluate", true, Some(body))]
    }

    fn handle_pause(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        self.debugger.pause();

        let mut responses = vec![DapMessage::response(msg.seq, "pause", true, None)];

        responses.push(DapMessage::event(
            "stopped",
            Some(make_obj(&[
                ("reason", json_str("pause")),
                ("threadId", json_number(self.thread_id as i64)),
                ("allThreadsStopped", json_bool(true)),
            ])),
        ));

        responses
    }

    fn handle_disconnect(&mut self, msg: &DapMessage) -> Vec<DapMessage> {
        vec![DapMessage::response(
            msg.seq,
            "disconnect",
            true,
            None,
        )]
    }
}

fn reason_to_str(reason: PauseReason) -> &'static str {
    match reason {
        PauseReason::Breakpoint => "breakpoint",
        PauseReason::Step => "step",
        PauseReason::Pause => "pause",
        PauseReason::Exception => "exception",
        PauseReason::Entry => "entry",
    }
}
