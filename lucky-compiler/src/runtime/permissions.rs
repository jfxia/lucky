//! Permission Enforcer — capability-security model for the Lucky runtime.

/// A permission entry can be a simple path like `filesystem.read`
/// or a wildcard like `filesystem.*` or `git.push(main)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionEntry {
    pub segments: Vec<String>,
    pub is_deny: bool,
}

impl PermissionEntry {
    pub fn parse(raw: &str, is_deny: bool) -> Self {
        let segments: Vec<String> = raw.split('.').map(|s| s.to_string()).collect();
        Self { segments, is_deny }
    }

    /// Check if this permission entry matches a requested operation.
    /// Operation is dot-separated: `filesystem.read`
    pub fn matches(&self, operation: &str) -> bool {
        let op_segments: Vec<&str> = operation.split('.').collect();
        self.matches_segments(&op_segments)
    }

    fn matches_segments(&self, op: &[&str]) -> bool {
        if self.segments.len() > op.len() && self.segments.last().map(|s| s.as_str()) != Some("*") {
            return false;
        }

        for (i, seg) in self.segments.iter().enumerate() {
            if i >= op.len() { return seg == "*"; }
            if seg == "*" { return true; }
            if seg == "**" { return true; }
            if seg != op[i] && !seg_matches_glob(seg, op[i]) {
                return false;
            }
        }

        // If permission has fewer segments than operation, it's a prefix match
        // e.g., "filesystem" matches "filesystem.read"
        if self.segments.len() < op.len() {
            return true;
        }

        true
    }
}

/// Simple glob matching for a single segment.
fn seg_matches_glob(pattern: &str, value: &str) -> bool {
    if pattern == "*" || pattern == "**" { return true; }
    if !pattern.contains('*') && !pattern.contains('?') {
        return pattern == value;
    }
    // Very simple glob: * matches any sequence, ? matches any single char
    let mut pi = 0;
    let mut vi = 0;
    let pchars: Vec<char> = pattern.chars().collect();
    let vchars: Vec<char> = value.chars().collect();

    while pi < pchars.len() {
        match pchars[pi] {
            '*' => {
                // Consume all stars
                while pi < pchars.len() && pchars[pi] == '*' { pi += 1; }
                if pi == pchars.len() { return true; }
                // Find the next non-star character in value
                while vi < vchars.len() {
                    if pchars[pi] == '?' || pchars[pi] == vchars[vi] {
                        break;
                    }
                    vi += 1;
                }
                if vi == vchars.len() { return false; }
            }
            '?' => {
                if vi >= vchars.len() { return false; }
                pi += 1; vi += 1;
            }
            c => {
                if vi >= vchars.len() || c != vchars[vi] { return false; }
                pi += 1; vi += 1;
            }
        }
    }
    vi == vchars.len()
}

/// A permission set with allow and deny entries.
#[derive(Debug, Clone)]
pub struct PermissionSet {
    pub allows: Vec<PermissionEntry>,
    pub denies: Vec<PermissionEntry>,
}

impl PermissionSet {
    pub fn new() -> Self {
        Self { allows: Vec::new(), denies: Vec::new() }
    }

    /// Allow a permission pattern.
    pub fn allow(&mut self, pattern: &str) {
        self.allows.push(PermissionEntry::parse(pattern, false));
    }

    /// Deny a permission pattern.
    pub fn deny(&mut self, pattern: &str) {
        self.denies.push(PermissionEntry::parse(pattern, true));
    }

    /// Check if an operation is allowed.
    /// Denies take precedence over allows. If nothing matches, default is deny.
    pub fn is_allowed(&self, operation: &str) -> bool {
        // Check denies first (they take precedence)
        for deny in &self.denies {
            if deny.matches(operation) {
                return false;
            }
        }
        // Check allows
        for allow in &self.allows {
            if allow.matches(operation) {
                return true;
            }
        }
        // Default deny
        false
    }

    /// Merge another permission set into this one (restrictive: denies propagate).
    pub fn merge(&mut self, other: &PermissionSet) {
        self.allows.extend(other.allows.clone());
        self.denies.extend(other.denies.clone());
    }
}

/// The permission enforcer manages permission sets per agent and per session.
pub struct PermissionEnforcer {
    /// Agent-level permissions: agent_name -> PermissionSet
    agent_permissions: HashMap<String, PermissionSet>,
    /// Session-level default permissions.
    session_permissions: PermissionSet,
}

use std::collections::HashMap;

impl PermissionEnforcer {
    pub fn new() -> Self {
        let mut session = PermissionSet::new();
        // Default safe permissions
        session.allow("filesystem.read");
        session.allow("git.clone");
        session.allow("git.status");
        session.allow("git.log");
        session.allow("browser.search");
        session.allow("http.get");
        session.deny("filesystem.delete");
        session.deny("shell.exec");
        session.deny("git.force_push");

        Self {
            agent_permissions: HashMap::new(),
            session_permissions: session,
        }
    }

    /// Set permissions for an agent.
    pub fn set_agent_permissions(&mut self, agent_name: &str, permissions: PermissionSet) {
        self.agent_permissions.insert(agent_name.to_string(), permissions);
    }

    /// Get the effective permission set for an agent.
    pub fn get_effective(&self, agent_name: Option<&str>) -> PermissionSet {
        let mut effective = self.session_permissions.clone();
        if let Some(name) = agent_name {
            if let Some(agent_perms) = self.agent_permissions.get(name) {
                effective.merge(agent_perms);
            }
        }
        effective
    }

    /// Check if a specific operation is allowed for an agent.
    pub fn check(&self, agent_name: Option<&str>, operation: &str) -> bool {
        self.get_effective(agent_name).is_allowed(operation)
    }

    /// Add a global allow rule.
    pub fn allow_global(&mut self, pattern: &str) {
        self.session_permissions.allow(pattern);
    }

    /// Add a global deny rule.
    pub fn deny_global(&mut self, pattern: &str) {
        self.session_permissions.deny(pattern);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_matching() {
        let mut ps = PermissionSet::new();
        ps.allow("filesystem.read");
        ps.allow("git.*");
        ps.deny("git.force_push");

        assert!(ps.is_allowed("filesystem.read"));
        assert!(ps.is_allowed("git.clone"));
        assert!(ps.is_allowed("git.commit"));
        assert!(!ps.is_allowed("git.force_push"));
        assert!(!ps.is_allowed("filesystem.delete"));
        assert!(!ps.is_allowed("shell.exec"));
    }

    #[test]
    fn test_wildcard_allow() {
        let mut ps = PermissionSet::new();
        ps.allow("filesystem.*");
        assert!(ps.is_allowed("filesystem.read"));
        assert!(ps.is_allowed("filesystem.write"));
        assert!(ps.is_allowed("filesystem.delete"));
        assert!(!ps.is_allowed("git.clone"));
    }

    #[test]
    fn test_glob_matching() {
        assert!(seg_matches_glob("*.py", "main.py"));
        assert!(seg_matches_glob("test_*", "test_foo"));
        assert!(!seg_matches_glob("*.rs", "main.py"));
    }
}
