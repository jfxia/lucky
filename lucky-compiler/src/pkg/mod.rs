use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub mod manifest;
pub mod registry;
pub mod resolver;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn parse(s: &str) -> Option<Version> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.is_empty() || parts.len() > 3 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        Some(Version { major, minor, patch })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::cmp::PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major.cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

#[derive(Debug, Clone)]
pub enum VersionConstraint {
    Exact(Version),
    Caret(Version),
    Tilde(Version),
    Range {
        min: Option<Version>,
        max: Option<Version>,
        min_inclusive: bool,
        max_inclusive: bool,
    },
    Any,
    Raw(String),
}

impl VersionConstraint {
    pub fn parse(s: &str) -> VersionConstraint {
        let s = s.trim();
        if s.is_empty() || s == "*" {
            return VersionConstraint::Any;
        }
        if s.starts_with('^') && s.len() > 1 {
            if let Some(v) = Version::parse(&s[1..]) {
                return VersionConstraint::Caret(v);
            }
        }
        if s.starts_with('~') && s.len() > 1 {
            if let Some(v) = Version::parse(&s[1..]) {
                return VersionConstraint::Tilde(v);
            }
        }
        if s.contains(">=") || s.contains("<=") || s.contains('>') || s.contains('<') {
            let mut min: Option<Version> = None;
            let mut max: Option<Version> = None;
            let mut min_inclusive = true;
            let mut max_inclusive = false;

            for part in s.split_whitespace() {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                if let Some(ver_str) = part.strip_prefix(">=") {
                    min = Version::parse(ver_str);
                    min_inclusive = true;
                } else if let Some(ver_str) = part.strip_prefix("<=") {
                    max = Version::parse(ver_str);
                    max_inclusive = true;
                } else if let Some(ver_str) = part.strip_prefix('>') {
                    min = Version::parse(ver_str);
                    min_inclusive = false;
                } else if let Some(ver_str) = part.strip_prefix('<') {
                    max = Version::parse(ver_str);
                    max_inclusive = false;
                }
            }

            return VersionConstraint::Range { min, max, min_inclusive, max_inclusive };
        }
        if let Some(v) = Version::parse(s) {
            return VersionConstraint::Exact(v);
        }
        VersionConstraint::Raw(s.to_string())
    }

    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::Caret(v) => {
                if v.major == 0 {
                    if v.minor == 0 {
                        let max = Version { major: 0, minor: 0, patch: v.patch.saturating_add(1) };
                        version >= v && version < &max
                    } else {
                        let max = Version { major: 0, minor: v.minor + 1, patch: 0 };
                        version >= v && version < &max
                    }
                } else {
                    let max = Version { major: v.major + 1, minor: 0, patch: 0 };
                    version >= v && version < &max
                }
            }
            VersionConstraint::Tilde(v) => {
                let max = Version { major: v.major, minor: v.minor + 1, patch: 0 };
                version >= v && version < &max
            }
            VersionConstraint::Range { min, max, min_inclusive, max_inclusive } => {
                if let Some(min_v) = min {
                    let ok = if *min_inclusive { version >= min_v } else { version > min_v };
                    if !ok {
                        return false;
                    }
                }
                if let Some(max_v) = max {
                    let ok = if *max_inclusive { version <= max_v } else { version < max_v };
                    if !ok {
                        return false;
                    }
                }
                true
            }
            VersionConstraint::Any => true,
            VersionConstraint::Raw(_) => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub author: String,
    pub dependencies: HashMap<String, String>,
    pub exports: Vec<String>,
    pub license: String,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LockedPackage {
    pub name: String,
    pub version: Version,
    pub source: String,
    pub integrity: String,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Lockfile {
    pub packages: Vec<LockedPackage>,
    pub checksum: String,
}

pub fn install_package(name: &str, version: Option<&str>) -> Result<(), String> {
    let reg = registry::LocalRegistry::new("./lucky-packages");

    let pkg = if let Some(ver) = version {
        reg.fetch_package(name, ver)?
    } else {
        let results = reg.search_packages(name)?;
        let mut matching: Vec<&Package> = results.iter()
            .filter(|p| p.name == name)
            .collect();
        if matching.is_empty() {
            return Err(format!("Package '{}' not found in registry", name));
        }
        matching.sort_by(|a, b| b.version.cmp(&a.version));
        matching[0].clone()
    };

    let mut r = resolver::Resolver::new();
    let resolved = r.resolve(&pkg, &reg)?;

    println!("Installing {} v{}", pkg.name, pkg.version);
    for dep in &resolved {
        if dep.name != pkg.name {
            println!("  - {} v{}", dep.name, dep.version);
        }
    }

    let locked: Vec<LockedPackage> = resolved.iter().map(|p| {
        LockedPackage {
            name: p.name.clone(),
            version: p.version.clone(),
            source: reg.package_path(&p.name, &p.version),
            integrity: compute_integrity(p),
            dependencies: p.dependencies.clone(),
        }
    }).collect();

    let checksum = compute_lockfile_checksum(&locked);
    let lockfile = Lockfile { packages: locked, checksum };
    write_lockfile(&lockfile, "lucky.lock")?;

    println!("Lockfile written to lucky.lock ({} packages)", r.resolved_count());
    Ok(())
}

pub fn publish_package(path: &str) -> Result<(), String> {
    let manifest_path = if path.ends_with("lucky.toml") {
        Path::new(path).to_path_buf()
    } else {
        Path::new(path).join("lucky.toml")
    };

    if !manifest_path.exists() {
        return Err(format!("lucky.toml not found at '{}'", manifest_path.display()));
    }

    let manifest = manifest::parse_manifest(&manifest_path)?;
    let pkg = manifest_to_package(&manifest, path)?;

    let reg = registry::LocalRegistry::new("./lucky-packages");
    reg.publish_package(&pkg, path)?;

    println!("Published {} v{}", pkg.name, pkg.version);
    Ok(())
}

pub fn resolve_dependencies(pkg: &Package) -> Result<Vec<Package>, String> {
    let reg = registry::LocalRegistry::new("./lucky-packages");
    let mut r = resolver::Resolver::new();
    r.resolve(pkg, &reg)
}

pub fn read_lockfile(path: &str) -> Result<Lockfile, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read lockfile '{}': {}", path, e))?;

    let mut packages = Vec::new();
    let mut checksum = String::new();
    let mut current_pkg: Option<LockedPackage> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(val) = trimmed.strip_prefix("checksum = ") {
            checksum = unquote_str(val.trim()).unwrap_or_default();
            continue;
        }

        if trimmed == "[[package]]" {
            if let Some(p) = current_pkg.take() {
                packages.push(p);
            }
            current_pkg = Some(LockedPackage {
                name: String::new(),
                version: Version { major: 0, minor: 0, patch: 0 },
                source: String::new(),
                integrity: String::new(),
                dependencies: HashMap::new(),
            });
            continue;
        }

        if let Some(ref mut pkg) = current_pkg {
            if let Some((key, value)) = split_kv(trimmed) {
                match key.as_str() {
                    "name" => pkg.name = unquote_str(&value).unwrap_or(value),
                    "version" => {
                        let v = unquote_str(&value).unwrap_or(value);
                        if let Some(parsed) = Version::parse(&v) {
                            pkg.version = parsed;
                        }
                    }
                    "source" => pkg.source = unquote_str(&value).unwrap_or(value),
                    "integrity" => pkg.integrity = unquote_str(&value).unwrap_or(value),
                    "dependencies" => {
                        pkg.dependencies = parse_inline_table(&value);
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(p) = current_pkg.take() {
        packages.push(p);
    }

    Ok(Lockfile { packages, checksum })
}

pub fn write_lockfile(lockfile: &Lockfile, path: &str) -> Result<(), String> {
    let mut content = String::new();
    content.push_str("# This file is automatically generated by lucky. Do not edit.\n");
    content.push_str(&format!("checksum = \"{}\"\n\n", lockfile.checksum));

    for pkg in &lockfile.packages {
        content.push_str("[[package]]\n");
        content.push_str(&format!("name = \"{}\"\n", pkg.name));
        content.push_str(&format!("version = \"{}\"\n", pkg.version));
        content.push_str(&format!("source = \"{}\"\n", pkg.source));
        content.push_str(&format!("integrity = \"{}\"\n", pkg.integrity));
        if !pkg.dependencies.is_empty() {
            let deps: Vec<String> = pkg.dependencies.iter()
                .map(|(k, v)| format!("{} = \"{}\"", k, v))
                .collect();
            content.push_str(&format!("dependencies = {{{}}}\n", deps.join(", ")));
        }
        content.push('\n');
    }

    fs::write(path, content)
        .map_err(|e| format!("Failed to write lockfile '{}': {}", path, e))?;

    Ok(())
}

pub(crate) fn unquote_str(s: &str) -> Option<String> {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        Some(s[1..s.len() - 1].to_string())
    } else if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        Some(s[1..s.len() - 1].to_string())
    } else {
        Some(s.to_string())
    }
}

pub(crate) fn split_kv(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    let eq_pos = line.find('=')?;
    let key = line[..eq_pos].trim().to_string();
    let value = line[eq_pos + 1..].trim().to_string();
    Some((key, value))
}

fn parse_inline_table(value: &str) -> HashMap<String, String> {
    let value = value.trim();
    let mut map = HashMap::new();
    if value.starts_with('{') && value.ends_with('}') {
        let inner = &value[1..value.len() - 1];
        if inner.trim().is_empty() {
            return map;
        }
        for part in inner.split(',') {
            let part = part.trim();
            if let Some(eq) = part.find('=') {
                let key = part[..eq].trim().to_string();
                let val = unquote_str(&part[eq + 1..].trim()).unwrap_or_default();
                if !key.is_empty() {
                    map.insert(key, val);
                }
            }
        }
    }
    map
}

fn compute_integrity(pkg: &Package) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    pkg.name.hash(&mut h);
    pkg.version.to_string().hash(&mut h);
    format!("sha256-{:x}", h.finish())
}

fn compute_lockfile_checksum(packages: &[LockedPackage]) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for pkg in packages {
        pkg.name.hash(&mut h);
        pkg.version.to_string().hash(&mut h);
        pkg.integrity.hash(&mut h);
    }
    format!("{:x}", h.finish())
}

fn manifest_to_package(manifest: &manifest::Manifest, source_path: &str) -> Result<Package, String> {
    let info = manifest.package.as_ref()
        .ok_or_else(|| "Missing [package] section in manifest".to_string())?;

    let version = Version::parse(&info.version)
        .ok_or_else(|| format!("Invalid version '{}' in [package]", info.version))?;

    Ok(Package {
        name: info.name.clone(),
        version,
        description: info.description.clone().unwrap_or_default(),
        author: info.authors.clone().unwrap_or_default(),
        dependencies: manifest.dependencies.clone(),
        exports: manifest.exports.clone(),
        license: info.license.clone().unwrap_or_default(),
        source_path: Some(source_path.to_string()),
    })
}
