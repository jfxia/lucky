use std::collections::HashSet;

use super::Package;
use super::registry::LocalRegistry;

pub struct Resolver {
    resolved: Vec<Package>,
    resolved_names: HashSet<String>,
    resolving_stack: Vec<String>,
}

impl Resolver {
    pub fn new() -> Self {
        Resolver {
            resolved: Vec::new(),
            resolved_names: HashSet::new(),
            resolving_stack: Vec::new(),
        }
    }

    pub fn resolve(
        &mut self,
        root: &Package,
        registry: &LocalRegistry,
    ) -> Result<Vec<Package>, String> {
        self.resolved.clear();
        self.resolved_names.clear();
        self.resolving_stack.clear();

        self.resolve_pkg(root, registry)?;

        Ok(self.resolved.clone())
    }

    fn resolve_pkg(
        &mut self,
        pkg: &Package,
        registry: &LocalRegistry,
    ) -> Result<(), String> {
        if self.resolved_names.contains(&pkg.name) {
            if let Some(existing) = self.resolved.iter().find(|p| p.name == pkg.name) {
                if existing.version != pkg.version {
                    return Err(format!(
                        "Version conflict for '{}': requires {} but {} is already resolved",
                        pkg.name, pkg.version, existing.version
                    ));
                }
            }
            return Ok(());
        }

        if self.resolving_stack.contains(&pkg.name) {
            let cycle_start = self.resolving_stack.iter()
                .position(|n| n == &pkg.name)
                .unwrap_or(0);
            let cycle: Vec<&str> = self.resolving_stack[cycle_start..]
                .iter()
                .map(|s| s.as_str())
                .collect();
            return Err(format!(
                "Circular dependency detected: {} -> {}",
                cycle.join(" -> "),
                pkg.name
            ));
        }

        self.resolving_stack.push(pkg.name.clone());

        for (dep_name, dep_constraint) in &pkg.dependencies {
            let dep_pkg = registry.fetch_package(dep_name, dep_constraint)?;
            self.resolve_pkg(&dep_pkg, registry)?;
        }

        self.resolving_stack.retain(|n| n != &pkg.name);
        self.resolved_names.insert(pkg.name.clone());
        self.resolved.push(pkg.clone());

        Ok(())
    }

    pub fn resolved_count(&self) -> usize {
        self.resolved.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{Version, VersionConstraint};

    #[test]
    fn test_version_constraint_exact() {
        let c = VersionConstraint::parse("1.2.3");
        assert!(c.matches(&Version::parse("1.2.3").unwrap()));
        assert!(!c.matches(&Version::parse("1.2.4").unwrap()));
    }

    #[test]
    fn test_version_constraint_caret() {
        let c = VersionConstraint::parse("^1.2.3");
        assert!(c.matches(&Version::parse("1.2.3").unwrap()));
        assert!(c.matches(&Version::parse("1.9.9").unwrap()));
        assert!(!c.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_version_constraint_caret_zero() {
        let c = VersionConstraint::parse("^0.2.3");
        assert!(c.matches(&Version::parse("0.2.3").unwrap()));
        assert!(c.matches(&Version::parse("0.2.9").unwrap()));
        assert!(!c.matches(&Version::parse("0.3.0").unwrap()));
    }

    #[test]
    fn test_version_constraint_tilde() {
        let c = VersionConstraint::parse("~1.2.3");
        assert!(c.matches(&Version::parse("1.2.3").unwrap()));
        assert!(c.matches(&Version::parse("1.2.9").unwrap()));
        assert!(!c.matches(&Version::parse("1.3.0").unwrap()));
    }

    #[test]
    fn test_version_constraint_range() {
        let c = VersionConstraint::parse(">=1.0 <2.0");
        assert!(c.matches(&Version::parse("1.0.0").unwrap()));
        assert!(c.matches(&Version::parse("1.5.0").unwrap()));
        assert!(!c.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!c.matches(&Version::parse("0.9.0").unwrap()));
    }

    #[test]
    fn test_version_constraint_any() {
        let c = VersionConstraint::parse("*");
        assert!(c.matches(&Version::parse("0.0.0").unwrap()));
        assert!(c.matches(&Version::parse("99.99.99").unwrap()));
    }

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_parse_partial() {
        let v = Version::parse("1.2").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 0);

        let v = Version::parse("1").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_version_ordering() {
        let a = Version::parse("1.0.0").unwrap();
        let b = Version::parse("1.0.1").unwrap();
        let c = Version::parse("2.0.0").unwrap();
        assert!(a < b);
        assert!(b < c);
        assert!(a < c);
    }

    #[test]
    fn test_resolver_no_deps() {
        let pkg = Package {
            name: "root".to_string(),
            version: Version::parse("1.0.0").unwrap(),
            description: String::new(),
            author: String::new(),
            dependencies: std::collections::HashMap::new(),
            exports: Vec::new(),
            license: String::new(),
            source_path: None,
        };

        let registry = LocalRegistry::new("./test-registry");
        let mut resolver = Resolver::new();
        let result = resolver.resolve(&pkg, &registry).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "root");
    }

    #[test]
    fn test_resolver_cycle_detection() {
        let mut deps_a = std::collections::HashMap::new();
        deps_a.insert("b".to_string(), "*".to_string());

        let pkg_a = Package {
            name: "a".to_string(),
            version: Version::parse("1.0.0").unwrap(),
            description: String::new(),
            author: String::new(),
            dependencies: deps_a,
            exports: Vec::new(),
            license: String::new(),
            source_path: None,
        };

        let registry = LocalRegistry::new("./test-registry");
        let mut resolver = Resolver::new();

        let result = resolver.resolve(&pkg_a, &registry);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No version"));
    }
}
