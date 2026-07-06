use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;

pub struct AuditLogger {
    writer: Option<Mutex<BufWriter<File>>>,
}

#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub timestamp: u64,
    pub event_type: String,
    pub node_id: Option<usize>,
    pub agent_name: Option<String>,
    pub cost: Option<f64>,
    pub tokens: Option<u64>,
    pub error: Option<String>,
    pub detail: Option<String>,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self { writer: None }
    }

    pub fn open(path: &str) -> Result<Self, String> {
        let file = File::create(path)
            .map_err(|e| format!("Failed to create audit file '{}': {}", path, e))?;
        let writer = BufWriter::new(file);
        Ok(Self {
            writer: Some(Mutex::new(writer)),
        })
    }

    pub fn log(&self, event: AuditEvent) {
        if let Some(ref writer) = self.writer {
            if let Ok(mut w) = writer.lock() {
                let line = format_audit_event(&event);
                let _ = writeln!(w, "{}", line);
                let _ = w.flush();
            }
        }
    }

    pub fn log_event(&self, event_type: &str) {
        self.log(AuditEvent {
            timestamp: ms_now(),
            event_type: event_type.to_string(),
            node_id: None,
            agent_name: None,
            cost: None,
            tokens: None,
            error: None,
            detail: None,
        });
    }

    pub fn log_node_event(
        &self, event_type: &str, node_id: usize, agent_name: Option<&str>,
    ) {
        self.log(AuditEvent {
            timestamp: ms_now(),
            event_type: event_type.to_string(),
            node_id: Some(node_id),
            agent_name: agent_name.map(|s| s.to_string()),
            cost: None,
            tokens: None,
            error: None,
            detail: None,
        });
    }

    pub fn log_node_error(
        &self, event_type: &str, node_id: usize, error: &str, agent_name: Option<&str>,
    ) {
        self.log(AuditEvent {
            timestamp: ms_now(),
            event_type: event_type.to_string(),
            node_id: Some(node_id),
            agent_name: agent_name.map(|s| s.to_string()),
            cost: None,
            tokens: None,
            error: Some(error.to_string()),
            detail: None,
        });
    }

    pub fn log_cost(&self, total_usd: f64, tokens_used: u64) {
        self.log(AuditEvent {
            timestamp: ms_now(),
            event_type: "cost_updated".to_string(),
            node_id: None,
            agent_name: None,
            cost: Some(total_usd),
            tokens: Some(tokens_used),
            error: None,
            detail: None,
        });
    }
}

fn format_audit_event(e: &AuditEvent) -> String {
    let mut parts = Vec::new();
    parts.push(format!("\"timestamp\": {}", e.timestamp));
    parts.push(format!("\"event_type\": \"{}\"", json_escape(&e.event_type)));

    if let Some(nid) = e.node_id {
        parts.push(format!("\"node_id\": {}", nid));
    }
    if let Some(ref name) = e.agent_name {
        parts.push(format!("\"agent_name\": \"{}\"", json_escape(name)));
    }
    if let Some(c) = e.cost {
        parts.push(format!("\"cost\": {}", c));
    }
    if let Some(t) = e.tokens {
        parts.push(format!("\"tokens\": {}", t));
    }
    if let Some(ref err) = e.error {
        parts.push(format!("\"error\": \"{}\"", json_escape(err)));
    }
    if let Some(ref detail) = e.detail {
        parts.push(format!("\"detail\": \"{}\"", json_escape(detail)));
    }

    format!("{{{}}}", parts.join(", "))
}

fn ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}
