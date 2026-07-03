use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Package, Version, VersionConstraint};

pub struct LocalRegistry {
    root: PathBuf,
}

impl LocalRegistry {
    pub fn new(root_path: &str) -> Self {
        LocalRegistry {
            root: PathBuf::from(root_path),
        }
    }

    pub fn root_dir(&self) -> &Path {
        &self.root
    }

    pub fn package_path(&self, name: &str, version: &Version) -> String {
        format!("{}/{}-{}.lkpkg", self.root.display(), name, version)
    }

    pub fn search_packages(&self, query: &str) -> Result<Vec<Package>, String> {
        let query_lower = query.to_lowercase();
        let mut packages = Vec::new();

        if !self.root.exists() {
            return Ok(packages);
        }

        let entries = fs::read_dir(&self.root)
            .map_err(|e| format!("Failed to read registry directory '{}': {}", self.root.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("lkpkg") {
                if let Ok(pkg) = self.load_package_file(&path) {
                    if pkg.name.to_lowercase().contains(&query_lower) {
                        packages.push(pkg);
                    }
                }
            }
        }

        Ok(packages)
    }

    pub fn fetch_package(&self, name: &str, constraint: &str) -> Result<Package, String> {
        let all = self.search_packages(name)?;
        let version_constraint = VersionConstraint::parse(constraint);

        let mut matching: Vec<&Package> = all.iter()
            .filter(|p| p.name == name && version_constraint.matches(&p.version))
            .collect();

        if matching.is_empty() {
            return Err(format!(
                "No version of '{}' matches constraint '{}'",
                name, constraint
            ));
        }

        matching.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(matching[0].clone())
    }

    pub fn publish_package(&self, pkg: &Package, _source_path: &str) -> Result<(), String> {
        if !self.root.exists() {
            fs::create_dir_all(&self.root)
                .map_err(|e| format!("Failed to create registry directory '{}': {}", self.root.display(), e))?;
        }

        let file_path = self.root.join(format!("{}-{}.lkpkg", pkg.name, pkg.version));

        let json = package_to_json(pkg);
        fs::write(&file_path, json)
            .map_err(|e| format!("Failed to write package to '{}': {}", file_path.display(), e))?;

        Ok(())
    }

    fn load_package_file(&self, path: &Path) -> Result<Package, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;

        json_to_package(&content)
    }
}

fn package_to_json(pkg: &Package) -> String {
    let mut s = String::from("{\n");
    s.push_str(&format!("  \"name\": \"{}\",\n", escape_json(&pkg.name)));
    s.push_str(&format!("  \"version\": \"{}\",\n", pkg.version));
    s.push_str(&format!("  \"description\": \"{}\",\n", escape_json(&pkg.description)));
    s.push_str(&format!("  \"author\": \"{}\",\n", escape_json(&pkg.author)));
    s.push_str(&format!("  \"license\": \"{}\",\n", escape_json(&pkg.license)));

    s.push_str("  \"dependencies\": {");
    let deps: Vec<String> = pkg.dependencies.iter()
        .map(|(k, v)| format!("\"{}\": \"{}\"", escape_json(k), escape_json(v)))
        .collect();
    s.push_str(&deps.join(", "));
    s.push_str("},\n");

    s.push_str("  \"exports\": [");
    let exps: Vec<String> = pkg.exports.iter()
        .map(|e| format!("\"{}\"", escape_json(e)))
        .collect();
    s.push_str(&exps.join(", "));
    s.push_str("]\n");

    s.push('}');
    s
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn json_to_package(json: &str) -> Result<Package, String> {
    let mut parser = JsonParser::new(json);
    let root_value = parser.parse_value()?;

    let obj = match root_value {
        JsonValue::Object(o) => o,
        _ => return Err("Expected JSON object at top level".to_string()),
    };

    let name = json_require_string(&obj, "name")?;
    let version_str = json_require_string(&obj, "version")?;
    let version = Version::parse(&version_str)
        .ok_or_else(|| format!("Invalid version '{}'", version_str))?;
    let description = json_opt_string(&obj, "description").unwrap_or_default();
    let author = json_opt_string(&obj, "author").unwrap_or_default();
    let license = json_opt_string(&obj, "license").unwrap_or_default();

    let mut dependencies = HashMap::new();
    if let Some(JsonValue::Object(deps)) = obj.get("dependencies") {
        for (k, v) in deps {
            if let JsonValue::String(ver) = v {
                dependencies.insert(k.clone(), ver.clone());
            }
        }
    }

    let mut exports = Vec::new();
    if let Some(JsonValue::Array(arr)) = obj.get("exports") {
        for item in arr {
            if let JsonValue::String(s) = item {
                exports.push(s.clone());
            }
        }
    }

    Ok(Package {
        name,
        version,
        description,
        author,
        dependencies,
        exports,
        license,
        source_path: None,
    })
}

fn json_require_string(obj: &HashMap<String, JsonValue>, key: &str) -> Result<String, String> {
    match obj.get(key) {
        Some(JsonValue::String(s)) => Ok(s.clone()),
        Some(_) => Err(format!("Field '{}' must be a string", key)),
        None => Err(format!("Missing required field '{}'", key)),
    }
}

fn json_opt_string(obj: &HashMap<String, JsonValue>, key: &str) -> Option<String> {
    match obj.get(key) {
        Some(JsonValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

#[derive(Debug, Clone)]
enum JsonValue {
    Null,
    Bool(bool),
    String(String),
    Number(f64),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

struct JsonParser {
    chars: Vec<char>,
    pos: usize,
}

impl JsonParser {
    fn new(input: &str) -> Self {
        JsonParser {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_ws();
        match self.peek() {
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('t') | Some('f') => self.parse_bool(),
            Some('n') => self.parse_null(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(format!("Unexpected character '{}' at position {}", c, self.pos)),
            None => Err("Unexpected end of input".to_string()),
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => return Ok(s),
                Some('\\') => {
                    match self.advance() {
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some('/') => s.push('/'),
                        Some('n') => s.push('\n'),
                        Some('r') => s.push('\r'),
                        Some('t') => s.push('\t'),
                        Some(c) => {
                            s.push('\\');
                            s.push(c);
                        }
                        None => return Err("Unexpected end of string escape".to_string()),
                    }
                }
                Some(c) => s.push(c),
                None => return Err("Unterminated string".to_string()),
            }
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.expect('{')?;
        self.skip_ws();

        let mut map = HashMap::new();

        if self.peek() == Some('}') {
            self.advance();
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_ws();
            let key = match self.parse_value()? {
                JsonValue::String(s) => s,
                _ => return Err("Object keys must be strings".to_string()),
            };

            self.skip_ws();
            self.expect(':')?;
            self.skip_ws();

            let value = self.parse_value()?;
            map.insert(key, value);

            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some('}') => {
                    self.advance();
                    break;
                }
                Some(c) => return Err(format!("Expected ',' or '}}', found '{}'", c)),
                None => return Err("Unterminated object".to_string()),
            }
        }

        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.expect('[')?;
        self.skip_ws();

        let mut arr = Vec::new();

        if self.peek() == Some(']') {
            self.advance();
            return Ok(JsonValue::Array(arr));
        }

        loop {
            self.skip_ws();
            arr.push(self.parse_value()?);

            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some(']') => {
                    self.advance();
                    break;
                }
                Some(c) => return Err(format!("Expected ',' or ']', found '{}'", c)),
                None => return Err("Unterminated array".to_string()),
            }
        }

        Ok(JsonValue::Array(arr))
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.match_literal("true") {
            Ok(JsonValue::Bool(true))
        } else if self.match_literal("false") {
            Ok(JsonValue::Bool(false))
        } else {
            Err("Expected 'true' or 'false'".to_string())
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.match_literal("null") {
            Ok(JsonValue::Null)
        } else {
            Err("Expected 'null'".to_string())
        }
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;

        if self.peek() == Some('-') {
            self.pos += 1;
        }

        if self.pos < self.chars.len() && self.chars[self.pos] == '0' {
            self.pos += 1;
        } else {
            if self.pos >= self.chars.len() || !self.chars[self.pos].is_ascii_digit() {
                return Err("Expected digit".to_string());
            }
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        if self.peek() == Some('.') {
            self.pos += 1;
            if self.pos >= self.chars.len() || !self.chars[self.pos].is_ascii_digit() {
                return Err("Expected digit after decimal point".to_string());
            }
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        if self.peek() == Some('e') || self.peek() == Some('E') {
            self.pos += 1;
            if self.peek() == Some('+') || self.peek() == Some('-') {
                self.pos += 1;
            }
            if self.pos >= self.chars.len() || !self.chars[self.pos].is_ascii_digit() {
                return Err("Expected digit in exponent".to_string());
            }
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        let num_str: String = self.chars[start..self.pos].iter().collect();
        let num = num_str.parse::<f64>()
            .map_err(|_| format!("Invalid number '{}'", num_str))?;
        Ok(JsonValue::Number(num))
    }

    fn match_literal(&mut self, expected: &str) -> bool {
        let expected_chars: Vec<char> = expected.chars().collect();
        if self.pos + expected_chars.len() > self.chars.len() {
            return false;
        }
        if self.chars[self.pos..self.pos + expected_chars.len()] == expected_chars[..] {
            self.pos += expected_chars.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.advance() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(format!("Expected '{}' but found '{}' at position {}", expected, c, self.pos - 1)),
            None => Err(format!("Expected '{}' but reached end of input", expected)),
        }
    }
}
