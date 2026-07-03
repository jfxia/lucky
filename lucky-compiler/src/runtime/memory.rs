//! Memory Manager — agent-scoped key-value storage with vector similarity search.

use std::collections::{BTreeMap, HashMap};
use super::RuntimeValue;

/// Memory scope determines the lifetime of stored entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    Local,          // Duration of a single task
    Session,        // Duration of a user session
    Project,        // Lifetime of the project
    Organization,   // Shared across projects
    Global,         // Shared globally
}

impl MemoryScope {
    pub fn from_str(s: &str) -> Self {
        match s {
            "local" => MemoryScope::Local,
            "session" => MemoryScope::Session,
            "project" => MemoryScope::Project,
            "organization" => MemoryScope::Organization,
            "global" => MemoryScope::Global,
            _ => MemoryScope::Project,
        }
    }
}

/// A single entry in agent memory.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub key: String,
    pub value: RuntimeValue,
    pub embedding: Option<Vec<f64>>,
    pub tags: Vec<String>,
    pub created_at: u64,    // milliseconds since epoch
    pub updated_at: u64,
    pub ttl_ms: Option<u64>, // time-to-live in milliseconds
}

impl MemoryEntry {
    pub fn new(key: impl Into<String>, value: RuntimeValue) -> Self {
        let now = current_time_ms();
        Self {
            key: key.into(),
            value,
            embedding: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            ttl_ms: None,
        }
    }

    pub fn with_embedding(mut self, emb: Vec<f64>) -> Self {
        self.embedding = Some(emb);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_ms {
            current_time_ms() - self.updated_at > ttl
        } else {
            false
        }
    }
}

/// A simple embedding vector store using brute-force cosine similarity.
#[derive(Debug, Clone, Default)]
pub struct VectorIndex {
    entries: Vec<(Vec<f64>, String)>,  // (embedding, key)
}

impl VectorIndex {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn insert(&mut self, embedding: Vec<f64>, key: String) {
        // Remove old entry for the same key if it exists
        self.entries.retain(|(_, k)| k != &key);
        self.entries.push((embedding, key));
    }

    pub fn remove(&mut self, key: &str) {
        self.entries.retain(|(_, k)| k != key);
    }

    /// Find the K nearest neighbors by cosine similarity.
    pub fn search(&self, query: &[f64], k: usize) -> Vec<(String, f64)> {
        let mut scored: Vec<(String, f64)> = self.entries.iter()
            .map(|(emb, key)| (key.clone(), cosine_similarity(query, emb)))
            .filter(|(_, score)| !score.is_nan())
            .collect();

        // Sort by score descending (higher = more similar)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

/// Agent-specific memory store.
#[derive(Debug, Clone)]
pub struct AgentMemory {
    pub agent_name: String,
    pub scope: MemoryScope,
    pub entries: BTreeMap<String, MemoryEntry>,
    pub vector_index: VectorIndex,
}

impl AgentMemory {
    pub fn new(agent_name: impl Into<String>, scope: MemoryScope) -> Self {
        Self {
            agent_name: agent_name.into(),
            scope,
            entries: BTreeMap::new(),
            vector_index: VectorIndex::new(),
        }
    }

    /// Store or update an entry.
    pub fn remember(&mut self, key: impl Into<String>, value: RuntimeValue, embedding: Option<Vec<f64>>) {
        let key = key.into();
        let mut entry = MemoryEntry::new(key.clone(), value);
        if let Some(emb) = embedding {
            entry = entry.with_embedding(emb.clone());
            self.vector_index.insert(emb, key.clone());
        }
        self.entries.insert(key, entry);
    }

    /// Retrieve an entry by exact key.
    pub fn recall(&self, key: &str) -> Option<&RuntimeValue> {
        self.entries.get(key)
            .filter(|e| !e.is_expired())
            .map(|e| &e.value)
    }

    /// Forget an entry.
    pub fn forget(&mut self, key: &str) {
        self.entries.remove(key);
        self.vector_index.remove(key);
    }

    /// Find K nearest neighbors by embedding similarity.
    pub fn similar(&self, embedding: &[f64], limit: usize) -> Vec<(String, RuntimeValue, f64)> {
        let results = self.vector_index.search(embedding, limit);
        results.into_iter()
            .filter_map(|(key, score)| {
                self.entries.get(&key).map(|e| (key, e.value.clone(), score))
            })
            .collect()
    }

    /// Full-text search over keys and values.
    pub fn search(&self, query: &str, limit: usize) -> Vec<(String, RuntimeValue, f64)> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<(String, RuntimeValue, f64)> = self.entries.iter()
            .filter(|(k, e)| {
                if e.is_expired() { return false; }
                k.to_lowercase().contains(&query_lower)
                    || format!("{}", e.value).to_lowercase().contains(&query_lower)
            })
            .map(|(k, e)| (k.clone(), e.value.clone(), 0.5)) // simple text match score
            .collect();
        results.truncate(limit);
        results
    }

    /// List keys, optionally filtered by prefix or tags.
    pub fn list(&self, prefix: Option<&str>, tags: Option<&[String]>) -> Vec<String> {
        self.entries.iter()
            .filter(|(k, e)| {
                if e.is_expired() { return false; }
                if let Some(pref) = prefix {
                    if !k.starts_with(pref) { return false; }
                }
                if let Some(t) = tags {
                    if !t.iter().any(|tag| e.tags.contains(tag)) { return false; }
                }
                true
            })
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.entries.get(key)
            .map(|e| !e.is_expired())
            .unwrap_or(false)
    }

    /// Number of non-expired entries.
    pub fn count(&self) -> usize {
        self.entries.values().filter(|e| !e.is_expired()).count()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.vector_index = VectorIndex::new();
    }

    /// Export all entries as key-value pairs.
    pub fn export(&self) -> Vec<(String, RuntimeValue)> {
        self.entries.iter()
            .filter(|(_, e)| !e.is_expired())
            .map(|(k, e)| (k.clone(), e.value.clone()))
            .collect()
    }

    /// Import entries from key-value pairs.
    pub fn import(&mut self, entries: Vec<(String, RuntimeValue)>) {
        for (key, value) in entries {
            self.remember(key, value, None);
        }
    }
}

/// The memory manager holds all agent memories.
pub struct MemoryManager {
    pub agents: HashMap<String, AgentMemory>,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self { agents: HashMap::new() }
    }

    /// Get or create an agent's memory.
    pub fn get_or_create(&mut self, agent_name: &str, scope: MemoryScope) -> &mut AgentMemory {
        self.agents.entry(agent_name.to_string())
            .or_insert_with(|| AgentMemory::new(agent_name, scope))
    }

    /// Get an agent's memory if it exists.
    pub fn get(&self, agent_name: &str) -> Option<&AgentMemory> {
        self.agents.get(agent_name)
    }

    /// Get mutable access to an agent's memory.
    pub fn get_mut(&mut self, agent_name: &str) -> Option<&mut AgentMemory> {
        self.agents.get_mut(agent_name)
    }

    /// Forget all entries in a scope.
    pub fn clear_scope(&mut self, scope: MemoryScope) {
        for agent in self.agents.values_mut() {
            if agent.scope == scope {
                agent.clear();
            }
        }
    }
}

/// Get current time in milliseconds since Unix epoch.
fn current_time_ms() -> u64 {
    // Simple fallback: use a counter-based timestamp when system time is unavailable.
    // In production, this would use std::time::SystemTime.
    static mut COUNTER: u64 = 0;
    // For now, return incrementing counter
    unsafe {
        COUNTER = COUNTER.wrapping_add(1);
        COUNTER
    }
}
