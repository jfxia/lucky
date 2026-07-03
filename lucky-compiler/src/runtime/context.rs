//! Context Manager — immutable layered key-value propagation through the execution DAG.

use std::collections::HashMap;
use super::RuntimeValue;

/// A context layer — a scope-level set of key-value bindings.
#[derive(Debug, Clone)]
pub struct ContextLayer {
    pub scope_id: String,
    pub entries: HashMap<String, RuntimeValue>,
    pub parent: Option<Box<ContextLayer>>,
}

impl ContextLayer {
    pub fn new(scope_id: impl Into<String>) -> Self {
        Self {
            scope_id: scope_id.into(),
            entries: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(scope_id: impl Into<String>, parent: ContextLayer) -> Self {
        Self {
            scope_id: scope_id.into(),
            entries: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Get a value by key, walking up parent layers.
    pub fn get(&self, key: &str) -> Option<&RuntimeValue> {
        if let Some(v) = self.entries.get(key) {
            return Some(v);
        }
        if let Some(ref parent) = self.parent {
            return parent.get(key);
        }
        None
    }

    /// Check if a key exists in any layer.
    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
            || self.parent.as_ref().map(|p| p.contains(key)).unwrap_or(false)
    }

    /// Insert a value into this layer (shadows parent layers).
    pub fn insert(&mut self, key: impl Into<String>, value: RuntimeValue) {
        self.entries.insert(key.into(), value);
    }

    /// Get all keys visible from this layer.
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.entries.keys().cloned().collect();
        if let Some(ref parent) = self.parent {
            for k in parent.keys() {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }
        keys
    }

    /// Create a snapshot of all visible entries.
    pub fn to_map(&self) -> HashMap<String, RuntimeValue> {
        let mut result = HashMap::new();
        if let Some(ref parent) = self.parent {
            result = parent.to_map();
        }
        for (k, v) in &self.entries {
            result.insert(k.clone(), v.clone());
        }
        result
    }
}

/// The context manager maintains the context stack during execution.
pub struct ContextManager {
    /// The current context layer (top of the stack).
    pub current: ContextLayer,
    /// Named agent contexts that persist across task invocations.
    agent_contexts: HashMap<String, ContextLayer>,
    /// The global/project-level context.
    global_context: ContextLayer,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            current: ContextLayer::new("root"),
            agent_contexts: HashMap::new(),
            global_context: ContextLayer::new("global"),
        }
    }

    /// Push a new scope layer onto the context stack.
    pub fn push_scope(&mut self, scope_id: impl Into<String>) {
        let parent = std::mem::replace(
            &mut self.current,
            ContextLayer::new("temp"),
        );
        self.current = ContextLayer::with_parent(scope_id, parent);
    }

    /// Pop the current scope, returning to the parent.
    pub fn pop_scope(&mut self) {
        if let Some(parent) = self.current.parent.take() {
            self.current = *parent;
        }
    }

    /// Set a value in the current context layer.
    pub fn set(&mut self, key: impl Into<String>, value: RuntimeValue) {
        self.current.insert(key, value);
    }

    /// Get a value from the context (walks up the layer chain).
    pub fn get(&self, key: &str) -> Option<&RuntimeValue> {
        self.current.get(key)
    }

    /// Store an agent's context for later reuse.
    pub fn save_agent_context(&mut self, agent_name: &str) {
        self.agent_contexts.insert(
            agent_name.to_string(),
            self.current.clone(),
        );
    }

    /// Restore an agent's context.
    pub fn restore_agent_context(&mut self, agent_name: &str) -> bool {
        if let Some(ctx) = self.agent_contexts.get(agent_name) {
            self.current = ctx.clone();
            true
        } else {
            false
        }
    }

    /// Get a snapshot of the current context.
    pub fn snapshot(&self) -> HashMap<String, RuntimeValue> {
        self.current.to_map()
    }

    /// Set global context (available to all scopes).
    pub fn set_global(&mut self, key: impl Into<String>, value: RuntimeValue) {
        self.global_context.insert(key, value);
    }

    /// Initialize context for a workflow entry.
    pub fn init_workflow_context(&mut self, workflow_name: &str, params: &HashMap<String, RuntimeValue>) {
        self.push_scope(format!("workflow:{}", workflow_name));
        for (k, v) in params {
            self.set(k.clone(), v.clone());
        }
    }
}
