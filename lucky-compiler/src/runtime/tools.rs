//! Tool Registry & Execution — built-in and custom tool adapters.
//!
//! Each tool adapter implements the ToolAdapter trait. The ToolRegistry
//! dispatches tool calls to the appropriate adapter after permission checks.

use std::collections::HashMap;
use super::RuntimeValue;
use super::context::ContextManager;

/// Result of a tool invocation.
pub type ToolResult = Result<RuntimeValue, String>;

/// Trait for tool adapters that can be invoked by the runtime.
pub trait ToolAdapter {
    /// Unique name for this tool.
    fn name(&self) -> &str;

    /// List of methods this tool supports.
    fn methods(&self) -> Vec<&str>;

    /// Invoke a method on this tool with arguments and context.
    fn invoke(
        &mut self,
        method: &str,
        args: &HashMap<String, RuntimeValue>,
        context: &ContextManager,
    ) -> ToolResult;
}

/// Registry of all available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolAdapter>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    /// Register a tool adapter.
    pub fn register(&mut self, adapter: Box<dyn ToolAdapter>) {
        let name = adapter.name().to_string();
        self.tools.insert(name, adapter);
    }

    /// Check if a tool is registered.
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// List all registered tool names.
    pub fn list(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// List methods for a tool.
    pub fn methods(&self, name: &str) -> Option<Vec<&str>> {
        self.tools.get(name).map(|t| t.methods())
    }

    /// Invoke a tool method.
    pub fn invoke(
        &mut self,
        tool_name: &str,
        method: &str,
        args: &HashMap<String, RuntimeValue>,
        context: &ContextManager,
    ) -> ToolResult {
        match self.tools.get_mut(tool_name) {
            Some(adapter) => adapter.invoke(method, args, context),
            None => Err(format!("Tool '{}' not found", tool_name)),
        }
    }
}

// ─── Built-in Tool: Filesystem ──────────────────────────────────────

pub struct FilesystemAdapter {
    root: String,
}

impl FilesystemAdapter {
    pub fn new(root: impl Into<String>) -> Self {
        Self { root: root.into() }
    }

    fn resolve(&self, path: &str) -> Result<std::path::PathBuf, String> {
        let root = std::path::Path::new(&self.root);
        let resolved = root.join(path.trim_start_matches('/'));
        // Prevent path traversal
        let canonical = resolved.canonicalize().unwrap_or(resolved.clone());
        if !canonical.starts_with(root) {
            return Err(format!("Path '{}' escapes sandbox root", path));
        }
        Ok(canonical)
    }
}

impl ToolAdapter for FilesystemAdapter {
    fn name(&self) -> &str { "Filesystem" }

    fn methods(&self) -> Vec<&str> {
        vec!["read", "write", "exists", "list", "remove", "create_dir"]
    }

    fn invoke(&mut self, method: &str, args: &HashMap<String, RuntimeValue>, _ctx: &ContextManager) -> ToolResult {
        match method {
            "read" => {
                let path = get_string_arg(args, "path")?;
                let resolved = self.resolve(&path)?;
                match std::fs::read_to_string(&resolved) {
                    Ok(content) => Ok(RuntimeValue::String(content)),
                    Err(e) => Err(format!("Failed to read '{}': {}", path, e)),
                }
            }
            "write" => {
                let path = get_string_arg(args, "path")?;
                let content = get_string_arg(args, "content")?;
                let resolved = self.resolve(&path)?;
                if let Some(parent) = resolved.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                match std::fs::write(&resolved, &content) {
                    Ok(_) => Ok(RuntimeValue::Null),
                    Err(e) => Err(format!("Failed to write '{}': {}", path, e)),
                }
            }
            "exists" => {
                let path = get_string_arg(args, "path")?;
                let resolved = self.resolve(&path)?;
                Ok(RuntimeValue::Bool(resolved.exists()))
            }
            "list" => {
                let path = get_string_arg(args, "path").unwrap_or_else(|_| ".".to_string());
                let resolved = self.resolve(&path)?;
                match std::fs::read_dir(&resolved) {
                    Ok(entries) => {
                        let mut items = Vec::new();
                        for entry in entries.flatten() {
                            items.push(RuntimeValue::String(
                                entry.file_name().to_string_lossy().to_string()
                            ));
                        }
                        Ok(RuntimeValue::List(items))
                    }
                    Err(e) => Err(format!("Failed to list '{}': {}", path, e)),
                }
            }
            "remove" => {
                let path = get_string_arg(args, "path")?;
                let resolved = self.resolve(&path)?;
                match std::fs::remove_file(&resolved) {
                    Ok(_) => Ok(RuntimeValue::Null),
                    Err(e) => Err(format!("Failed to remove '{}': {}", path, e)),
                }
            }
            "create_dir" => {
                let path = get_string_arg(args, "path")?;
                let resolved = self.resolve(&path)?;
                match std::fs::create_dir_all(&resolved) {
                    Ok(_) => Ok(RuntimeValue::Null),
                    Err(e) => Err(format!("Failed to create dir '{}': {}", path, e)),
                }
            }
            _ => Err(format!("Filesystem: unknown method '{}'", method)),
        }
    }
}

// ─── Built-in Tool: Shell ────────────────────────────────────────────

pub struct ShellAdapter {
    allowed_commands: Vec<String>,
    workdir: String,
}

impl ShellAdapter {
    pub fn new(workdir: impl Into<String>) -> Self {
        Self {
            allowed_commands: vec![
                "ls".into(), "cat".into(), "grep".into(), "find".into(),
                "cargo".into(), "npm".into(), "python".into(), "node".into(),
                "go".into(), "make".into(), "git".into(), "echo".into(),
                "pwd".into(), "date".into(), "wc".into(), "sort".into(),
                "uniq".into(), "head".into(), "tail".into(),
            ],
            workdir: workdir.into(),
        }
    }
}

impl ToolAdapter for ShellAdapter {
    fn name(&self) -> &str { "Shell" }

    fn methods(&self) -> Vec<&str> {
        vec!["exec"]
    }

    fn invoke(&mut self, method: &str, args: &HashMap<String, RuntimeValue>, _ctx: &ContextManager) -> ToolResult {
        match method {
            "exec" => {
                let command = get_string_arg(args, "command")?;
                // Extract the base command name
                let base = command.split_whitespace().next().unwrap_or("");
                if !self.allowed_commands.iter().any(|c| c == base) {
                    return Err(format!("Shell: command '{}' is not in the allowed list", base));
                }
                // Execute via system shell
                let output = std::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
                    .arg(if cfg!(windows) { "/C" } else { "-c" })
                    .arg(&command)
                    .current_dir(&self.workdir)
                    .output();

                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        let mut result = HashMap::new();
                        result.insert("stdout".to_string(), RuntimeValue::String(stdout));
                        result.insert("stderr".to_string(), RuntimeValue::String(stderr));
                        result.insert("exit_code".to_string(), RuntimeValue::Int(out.status.code().unwrap_or(-1) as i64));
                        result.insert("success".to_string(), RuntimeValue::Bool(out.status.success()));
                        Ok(RuntimeValue::Map(result))
                    }
                    Err(e) => Err(format!("Shell: failed to execute: {}", e)),
                }
            }
            _ => Err(format!("Shell: unknown method '{}'", method)),
        }
    }
}

// ─── Built-in Tool: Git ──────────────────────────────────────────────

pub struct GitAdapter {
    workdir: String,
}

impl GitAdapter {
    pub fn new(workdir: impl Into<String>) -> Self {
        Self { workdir: workdir.into() }
    }

    fn run_git(&self, args: &[&str]) -> ToolResult {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(&self.workdir)
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                if out.status.success() {
                    Ok(RuntimeValue::String(stdout.trim().to_string()))
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    Err(format!("Git error: {}", stderr.trim()))
                }
            }
            Err(e) => Err(format!("Git: failed to run: {}", e)),
        }
    }
}

impl ToolAdapter for GitAdapter {
    fn name(&self) -> &str { "Git" }

    fn methods(&self) -> Vec<&str> {
        vec!["status", "log", "diff", "clone", "commit", "push", "pull",
             "branch", "checkout", "add", "current_branch", "has_changes"]
    }

    fn invoke(&mut self, method: &str, args: &HashMap<String, RuntimeValue>, _ctx: &ContextManager) -> ToolResult {
        match method {
            "status" => self.run_git(&["status", "--porcelain"]),
            "log" => {
                let count = get_int_arg(args, "count").unwrap_or(10);
                self.run_git(&["log", "--oneline", &format!("-{}", count)])
            }
            "diff" => self.run_git(&["diff"]),
            "clone" => {
                let url = get_string_arg(args, "url")?;
                self.run_git(&["clone", &url])
            }
            "commit" => {
                let message = get_string_arg(args, "message")?;
                self.run_git(&["commit", "-m", &message])
            }
            "push" => self.run_git(&["push"]),
            "pull" => self.run_git(&["pull"]),
            "branch" => {
                let name = get_string_arg(args, "name")?;
                self.run_git(&["branch", &name])
            }
            "checkout" => {
                let branch = get_string_arg(args, "branch")?;
                self.run_git(&["checkout", &branch])
            }
            "add" => {
                let files = get_string_arg(args, "files").unwrap_or_else(|_| ".".to_string());
                self.run_git(&["add", &files])
            }
            "current_branch" => self.run_git(&["branch", "--show-current"]),
            "has_changes" => {
                let result = self.run_git(&["status", "--porcelain"]);
                match result {
                    Ok(RuntimeValue::String(s)) => Ok(RuntimeValue::Bool(!s.is_empty())),
                    _ => Ok(RuntimeValue::Bool(false)),
                }
            }
            _ => Err(format!("Git: unknown method '{}'", method)),
        }
    }
}

// ─── Built-in Tool: HTTP ─────────────────────────────────────────────

pub struct HttpAdapter;

impl HttpAdapter {
    pub fn new() -> Self { Self }
}

impl ToolAdapter for HttpAdapter {
    fn name(&self) -> &str { "HTTP" }

    fn methods(&self) -> Vec<&str> {
        vec!["get", "post"]
    }

    fn invoke(&mut self, method: &str, args: &HashMap<String, RuntimeValue>, _ctx: &ContextManager) -> ToolResult {
        match method {
            "get" => {
                let url = get_string_arg(args, "url")?;
                // Stub: real implementation would use an HTTP client
                let mut result = HashMap::new();
                result.insert("url".to_string(), RuntimeValue::String(url));
                result.insert("status".to_string(), RuntimeValue::Int(200));
                result.insert("body".to_string(), RuntimeValue::String("[HTTP stub response]".to_string()));
                Ok(RuntimeValue::Map(result))
            }
            "post" => {
                let url = get_string_arg(args, "url")?;
                let mut result = HashMap::new();
                result.insert("url".to_string(), RuntimeValue::String(url));
                result.insert("status".to_string(), RuntimeValue::Int(201));
                result.insert("body".to_string(), RuntimeValue::String("[HTTP stub response]".to_string()));
                Ok(RuntimeValue::Map(result))
            }
            _ => Err(format!("HTTP: unknown method '{}'", method)),
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────

fn get_string_arg(args: &HashMap<String, RuntimeValue>, key: &str) -> Result<String, String> {
    match args.get(key) {
        Some(RuntimeValue::String(s)) => Ok(s.clone()),
        Some(v) => Ok(format!("{}", v)),
        None => Err(format!("Missing required argument '{}'", key)),
    }
}

fn get_int_arg(args: &HashMap<String, RuntimeValue>, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_int())
}

/// Register all built-in tools into a tool registry.
pub fn register_builtin_tools(registry: &mut ToolRegistry) {
    registry.register(Box::new(FilesystemAdapter::new(".")));
    registry.register(Box::new(ShellAdapter::new(".")));
    registry.register(Box::new(GitAdapter::new(".")));
    registry.register(Box::new(HttpAdapter::new()));
}
