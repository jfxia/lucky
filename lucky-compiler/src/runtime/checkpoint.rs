use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::executor::ExecutionEngine;
use super::scheduler::NodeState;
use super::RuntimeValue;

const CHECKPOINT_DIR: &str = ".lucky/checkpoints";

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub id: String,
    pub timestamp: u64,
    pub node_states: Vec<(usize, CheckpointNodeState)>,
    pub context_snapshot: HashMap<String, RuntimeValue>,
    pub agent_memories: HashMap<String, Vec<CheckpointMemoryEntry>>,
    pub cost_data: CheckpointCostData,
    pub dag_progress: DagProgress,
}

#[derive(Debug, Clone)]
pub struct CheckpointNodeState {
    pub status: String,
    pub output: Option<RuntimeValue>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub depth: usize,
    pub pending_inputs: usize,
}

#[derive(Debug, Clone)]
pub struct CheckpointMemoryEntry {
    pub key: String,
    pub value: RuntimeValue,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CheckpointCostData {
    pub total_usd: f64,
    pub tokens_used: u64,
}

#[derive(Debug, Clone)]
pub struct DagProgress {
    pub completed_nodes: Vec<usize>,
    pub active_nodes: Vec<usize>,
    pub failed_nodes: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct CheckpointSummary {
    pub id: String,
    pub timestamp: u64,
    pub completed_nodes: usize,
    pub total_nodes: usize,
}

pub struct CheckpointManager {
    base_dir: PathBuf,
}

impl CheckpointManager {
    pub fn new() -> Self {
        let dir = PathBuf::from(CHECKPOINT_DIR);
        let _ = fs::create_dir_all(&dir);
        Self { base_dir: dir }
    }

    pub fn save(&self, engine: &ExecutionEngine) -> Result<String, String> {
        let id = make_id();
        let timestamp = ms_now();

        let node_states: Vec<(usize, CheckpointNodeState)> = engine.scheduler.nodes.iter()
            .map(|(&id, state)| {
                let cns = CheckpointNodeState {
                    status: format!("{:?}", state.status),
                    output: state.output.clone(),
                    retry_count: state.retry_count,
                    max_retries: state.max_retries,
                    depth: state.depth,
                    pending_inputs: state.pending_inputs,
                };
                (id, cns)
            })
            .collect();

        let context_snapshot = engine.context.snapshot();

        let agent_memories: HashMap<String, Vec<CheckpointMemoryEntry>> = engine.memory.agents.iter()
            .map(|(name, mem)| {
                let entries: Vec<CheckpointMemoryEntry> = mem.entries.values()
                    .map(|e| CheckpointMemoryEntry {
                        key: e.key.clone(),
                        value: e.value.clone(),
                        tags: e.tags.clone(),
                    })
                    .collect();
                (name.clone(), entries)
            })
            .collect();

        let cost_data = CheckpointCostData {
            total_usd: engine.cost_usd,
            tokens_used: engine.tokens_used,
        };

        let dag_progress = DagProgress {
            completed_nodes: engine.scheduler.completed_nodes.iter().copied().collect(),
            active_nodes: engine.scheduler.active_nodes.iter().copied().collect(),
            failed_nodes: engine.scheduler.failed_nodes.iter().copied().collect(),
        };

        let checkpoint = Checkpoint {
            id: id.clone(),
            timestamp,
            node_states,
            context_snapshot,
            agent_memories,
            cost_data,
            dag_progress,
        };

        let json = serialize_checkpoint(&checkpoint);
        let path = self.base_dir.join(format!("{}.json", id));
        fs::write(&path, &json)
            .map_err(|e| format!("Failed to write checkpoint: {}", e))?;

        Ok(id)
    }

    pub fn load(&self, id: &str) -> Result<Checkpoint, String> {
        let path = self.base_dir.join(format!("{}.json", id));
        let json = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read checkpoint '{}': {}", id, e))?;
        deserialize_checkpoint(&json)
    }

    pub fn list(&self) -> Result<Vec<CheckpointSummary>, String> {
        let mut summaries = Vec::new();
        let entries = fs::read_dir(&self.base_dir)
            .map_err(|e| format!("Failed to read checkpoint dir: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let id = stem.to_string();
                    let content = fs::read_to_string(&path).unwrap_or_default();
                    let timestamp = (extract_json_num(&content, "timestamp").unwrap_or(0.0)) as u64;
                    let completed = extract_json_array_len(&content, "completed_nodes");
                    let total = count_json_key(&content, "\"node_states\"");
                    summaries.push(CheckpointSummary {
                        id, timestamp, completed_nodes: completed, total_nodes: total,
                    });
                }
            }
        }

        summaries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(summaries)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let path = self.base_dir.join(format!("{}.json", id));
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete checkpoint '{}': {}", id, e))?;
        Ok(())
    }
}

fn make_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:016x}", nanos)
}

fn ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn serialize_checkpoint(c: &Checkpoint) -> String {
    let mut s = String::new();
    s.push_str("{\n");

    s.push_str(&format!("  \"id\": \"{}\",\n", json_escape(&c.id)));
    s.push_str(&format!("  \"timestamp\": {},\n", c.timestamp));
    s.push_str("  \"node_states\": [\n");
    for (i, (nid, state)) in c.node_states.iter().enumerate() {
        s.push_str("    {");
        s.push_str(&format!("\"node_id\": {},", nid));
        s.push_str(&format!("\"status\": \"{}\",", state.status));
        if let Some(ref out) = state.output {
            s.push_str(&format!("\"output\": {},", serialize_runtime_value(out)));
        }
        s.push_str(&format!("\"retry_count\": {},", state.retry_count));
        s.push_str(&format!("\"max_retries\": {},", state.max_retries));
        s.push_str(&format!("\"depth\": {},", state.depth));
        s.push_str(&format!("\"pending_inputs\": {}", state.pending_inputs));
        s.push('}');
        if i + 1 < c.node_states.len() { s.push(','); }
        s.push('\n');
    }
    s.push_str("  ],\n");

    s.push_str("  \"context_snapshot\": {\n");
    let ctx_keys: Vec<&String> = c.context_snapshot.keys().collect();
    for (i, k) in ctx_keys.iter().enumerate() {
        if let Some(v) = c.context_snapshot.get(*k) {
            s.push_str(&format!("    \"{}\": {}", json_escape(k), serialize_runtime_value(v)));
        }
        if i + 1 < ctx_keys.len() { s.push(','); }
        s.push('\n');
    }
    s.push_str("  },\n");

    s.push_str("  \"agent_memories\": {\n");
    let mem_keys: Vec<&String> = c.agent_memories.keys().collect();
    for (i, name) in mem_keys.iter().enumerate() {
        s.push_str(&format!("    \"{}\": [\n", json_escape(name)));
        if let Some(entries) = c.agent_memories.get(*name) {
            for (j, e) in entries.iter().enumerate() {
                s.push_str("      {");
                s.push_str(&format!("\"key\": \"{}\",", json_escape(&e.key)));
                s.push_str(&format!("\"value\": {},", serialize_runtime_value(&e.value)));
                s.push_str(&format!("\"tags\": [{}]", e.tags.iter()
                    .map(|t| format!("\"{}\"", json_escape(t)))
                    .collect::<Vec<_>>()
                    .join(", ")));
                s.push('}');
                if j + 1 < entries.len() { s.push(','); }
                s.push('\n');
            }
        }
        s.push_str("    ]");
        if i + 1 < mem_keys.len() { s.push(','); }
        s.push('\n');
    }
    s.push_str("  },\n");

    s.push_str(&format!("  \"cost_data\": {{\"total_usd\": {}, \"tokens_used\": {}}},\n",
        c.cost_data.total_usd, c.cost_data.tokens_used));

    s.push_str("  \"dag_progress\": {\n");
    s.push_str(&format!("    \"completed_nodes\": [{}],\n",
        c.dag_progress.completed_nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")));
    s.push_str(&format!("    \"active_nodes\": [{}],\n",
        c.dag_progress.active_nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")));
    s.push_str(&format!("    \"failed_nodes\": [{}]\n",
        c.dag_progress.failed_nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")));
    s.push_str("  }\n");

    s.push('}');
    s
}

fn deserialize_checkpoint(json: &str) -> Result<Checkpoint, String> {
    let id = extract_json_str(json, "id").unwrap_or_else(|| "unknown".to_string());
    let timestamp = (extract_json_num(json, "timestamp").unwrap_or(0.0)) as u64;

    let total_usd = extract_json_num(json, "total_usd").unwrap_or(0.0);
    let tokens_used = (extract_json_num(json, "tokens_used").unwrap_or(0.0)) as u64;

    let completed = extract_json_num_array(json, "completed_nodes");
    let active = extract_json_num_array(json, "active_nodes");
    let failed = extract_json_num_array(json, "failed_nodes");

    Ok(Checkpoint {
        id,
        timestamp,
        node_states: Vec::new(),
        context_snapshot: HashMap::new(),
        agent_memories: HashMap::new(),
        cost_data: CheckpointCostData { total_usd, tokens_used },
        dag_progress: DagProgress {
            completed_nodes: completed,
            active_nodes: active,
            failed_nodes: failed,
        },
    })
}

fn serialize_runtime_value(v: &RuntimeValue) -> String {
    match v {
        RuntimeValue::Null => "null".to_string(),
        RuntimeValue::Bool(b) => b.to_string(),
        RuntimeValue::Int(i) => i.to_string(),
        RuntimeValue::Float(f) => format!("{}", f),
        RuntimeValue::String(s) => format!("\"{}\"", json_escape(s)),
        RuntimeValue::List(items) => {
            let inner: Vec<String> = items.iter().map(serialize_runtime_value).collect();
            format!("[{}]", inner.join(", "))
        }
        RuntimeValue::Map(entries) => {
            let inner: Vec<String> = entries.iter()
                .map(|(k, v)| format!("\"{}\": {}", json_escape(k), serialize_runtime_value(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
        RuntimeValue::Bytes(b) => format!("\"<{} bytes>\"", b.len()),
        RuntimeValue::Artifact { id, kind, uri } =>
            format!("{{\"id\":\"{}\",\"kind\":\"{}\",\"uri\":\"{}\"}}",
                json_escape(id), json_escape(kind), json_escape(uri)),
        RuntimeValue::Error { code, message, .. } =>
            format!("{{\"code\":{},\"message\":\"{}\"}}", code, json_escape(message)),
        RuntimeValue::Probabilistic { value, confidence } =>
            format!("{{\"value\":{},\"confidence\":{}}}", serialize_runtime_value(value), confidence),
    }
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

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let pos = json.find(&pattern)?;
    let after = &json[pos + pattern.len()..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    if after.starts_with('"') {
        let mut result = String::new();
        let chars: Vec<char> = after[1..].chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                match chars[i + 1] {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    'n' => result.push('\n'),
                    _ => {}
                }
                i += 2;
            } else if chars[i] == '"' {
                break;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }
        Some(result)
    } else {
        None
    }
}

fn extract_json_num(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{}\"", key);
    let pos = json.find(&pattern)?;
    let after = &json[pos + pattern.len()..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    let end = after.find(|c: char| !c.is_digit(10) && c != '.' && c != '-')
        .unwrap_or(after.len());
    after[..end].trim().parse().ok()
}

fn extract_json_num_array(json: &str, key: &str) -> Vec<usize> {
    let pattern = format!("\"{}\"", key);
    let pos = match json.find(&pattern) {
        Some(p) => p,
        None => return Vec::new(),
    };
    let after = &json[pos + pattern.len()..];
    let after = match after.trim_start().strip_prefix(':') {
        Some(a) => a.trim_start(),
        None => return Vec::new(),
    };
    if !after.starts_with('[') { return Vec::new(); }
    let inner = &after[1..];
    let end = inner.find(']').unwrap_or(inner.len());
    inner[..end].split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}

fn extract_json_array_len(json: &str, key: &str) -> usize {
    extract_json_num_array(json, key).len()
}

fn count_json_key(json: &str, key: &str) -> usize {
    json.matches(key).count()
}
