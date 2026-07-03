pub mod resolver;

use crate::ast::span::Span;
use crate::ast::Module;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Agent,
    Task,
    Workflow,
    Goal,
    Memory,
    Tool,
    Model,
    Prompt,
    Policy,
    Type,
    Variable,
    Parameter,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Agent => "agent",
            SymbolKind::Task => "task",
            SymbolKind::Workflow => "workflow",
            SymbolKind::Goal => "goal",
            SymbolKind::Memory => "memory",
            SymbolKind::Tool => "tool",
            SymbolKind::Model => "model",
            SymbolKind::Prompt => "prompt",
            SymbolKind::Policy => "policy",
            SymbolKind::Type => "type",
            SymbolKind::Variable => "variable",
            SymbolKind::Parameter => "parameter",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub defined_at: Span,
    pub scope_level: usize,
}

impl Symbol {
    pub fn new(name: String, kind: SymbolKind, defined_at: Span, scope_level: usize) -> Self {
        Self {
            name,
            kind,
            defined_at,
            scope_level,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define(
        &mut self,
        name: String,
        kind: SymbolKind,
        defined_at: Span,
    ) -> Option<Symbol> {
        let scope_level = self.scopes.len() - 1;
        let symbol = Symbol::new(name.clone(), kind, defined_at, scope_level);
        let current = self.scopes.last_mut().expect("at least one scope");
        current.insert(name, symbol)
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }

    pub fn current_scope_depth(&self) -> usize {
        self.scopes.len()
    }

    pub fn all_symbols(&self) -> Vec<&Symbol> {
        self.scopes.iter().flat_map(|scope| scope.values()).collect()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub module: Module,
    pub symbols: SymbolTable,
}

impl ResolvedModule {
    pub fn new(module: Module, symbols: SymbolTable) -> Self {
        Self { module, symbols }
    }
}
