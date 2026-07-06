use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub package: Option<PackageInfo>,
    pub dependencies: HashMap<String, String>,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub name: String,
    pub provider: String,
    pub version: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub endpoint: Option<String>,
}

pub fn parse_manifest(path: &Path) -> Result<Manifest, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let mut current_section = String::new();
    let mut pkg_name = String::new();
    let mut pkg_version = String::new();
    let mut pkg_desc: Option<String> = None;
    let mut pkg_authors: Option<String> = None;
    let mut pkg_license: Option<String> = None;
    let mut dependencies = HashMap::new();
    let mut exports = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len() - 1].trim().to_string();
            continue;
        }

        let (key, value) = match split_key_value(trimmed) {
            Some(kv) => kv,
            None => continue,
        };

        match current_section.as_str() {
            "package" => match key.as_str() {
                "name" => pkg_name = unquote(&value).unwrap_or(value),
                "version" => pkg_version = unquote(&value).unwrap_or(value),
                "description" => pkg_desc = Some(unquote(&value).unwrap_or(value)),
                "authors" => pkg_authors = Some(stringify_toml_value(&value)),
                "license" => pkg_license = Some(unquote(&value).unwrap_or(value)),
                _ => {}
            },
            "dependencies" => {
                let dep_ver = unquote(&value).unwrap_or(value);
                dependencies.insert(key, dep_ver);
            }
            "exports" => {
                if key == "modules" {
                    exports = parse_toml_array(&value);
                }
            }
            _ => {}
        }
    }

    let package = if !pkg_name.is_empty() {
        Some(PackageInfo {
            name: pkg_name,
            version: pkg_version,
            description: pkg_desc,
            authors: pkg_authors,
            license: pkg_license,
        })
    } else {
        None
    };

    Ok(Manifest { package, dependencies, exports })
}

fn split_key_value(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    let eq_pos = line.find('=')?;
    let key = line[..eq_pos].trim().to_string();
    let value = line[eq_pos + 1..].trim().to_string();
    Some((key, value))
}

fn unquote(s: &str) -> Option<String> {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        Some(s[1..s.len() - 1].to_string())
    } else if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        Some(s[1..s.len() - 1].to_string())
    } else {
        Some(s.to_string())
    }
}

fn parse_toml_array(s: &str) -> Vec<String> {
    let s = s.trim();
    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len() - 1];
        if inner.trim().is_empty() {
            return Vec::new();
        }
        inner.split(',')
            .map(|item| {
                let item = item.trim();
                if item.len() >= 2 && item.starts_with('"') && item.ends_with('"') {
                    item[1..item.len() - 1].to_string()
                } else {
                    item.to_string()
                }
            })
            .filter(|item| !item.is_empty())
            .collect()
    } else {
        let val = unquote(s).unwrap_or_default();
        if val.is_empty() { Vec::new() } else { vec![val] }
    }
}

fn stringify_toml_value(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('[') && s.ends_with(']') {
        parse_toml_array(s).join(", ")
    } else {
        unquote(s).unwrap_or_else(|| s.to_string())
    }
}

pub fn parse_manifest_with_models(
    path: &Path,
) -> Result<(Manifest, HashMap<String, super::super::backends::ModelConfig>), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let mut current_section = String::new();
    let mut pkg_name = String::new();
    let mut pkg_version = String::new();
    let mut pkg_desc: Option<String> = None;
    let mut pkg_authors: Option<String> = None;
    let mut pkg_license: Option<String> = None;
    let mut dependencies = HashMap::new();
    let mut exports = Vec::new();

    let mut model_name = String::new();
    let mut model_provider = String::new();
    let mut model_temperature: Option<f64> = None;
    let mut model_max_tokens: Option<u32> = None;
    let mut model_endpoint: Option<String> = None;
    let mut models: HashMap<String, super::super::backends::ModelConfig> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = trimmed[1..trimmed.len() - 1].trim().to_string();

            if current_section.starts_with("models.") && !model_name.is_empty() {
                models.insert(
                    model_name.clone(),
                    super::super::backends::ModelConfig {
                        model_name: model_name.clone(),
                        provider: model_provider.clone(),
                        endpoint: model_endpoint.clone(),
                        temperature: model_temperature.unwrap_or(0.7),
                        max_tokens: model_max_tokens.unwrap_or(4096),
                    },
                );
            }

            if section.starts_with("models.") {
                model_name = section["models.".len()..].to_string();
                model_provider = String::new();
                model_temperature = None;
                model_max_tokens = None;
                model_endpoint = None;
                current_section = "models".to_string();
            } else {
                current_section = section;
            }
            continue;
        }

        let (key, value) = match split_key_value(trimmed) {
            Some(kv) => kv,
            None => continue,
        };

        match current_section.as_str() {
            "package" => match key.as_str() {
                "name" => pkg_name = unquote(&value).unwrap_or(value),
                "version" => pkg_version = unquote(&value).unwrap_or(value),
                "description" => pkg_desc = Some(unquote(&value).unwrap_or(value)),
                "authors" => pkg_authors = Some(stringify_toml_value(&value)),
                "license" => pkg_license = Some(unquote(&value).unwrap_or(value)),
                _ => {}
            },
            "dependencies" => {
                let dep_ver = unquote(&value).unwrap_or(value);
                dependencies.insert(key, dep_ver);
            }
            "exports" => {
                if key == "modules" {
                    exports = parse_toml_array(&value);
                }
            }
            "models" => {
                match key.as_str() {
                    "provider" => model_provider = unquote(&value).unwrap_or(value),
                    "temperature" => {
                        model_temperature = unquote(&value).unwrap_or(value).parse().ok();
                    }
                    "max_tokens" => {
                        model_max_tokens = unquote(&value).unwrap_or(value).parse().ok();
                    }
                    "endpoint" => model_endpoint = Some(unquote(&value).unwrap_or(value)),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    if current_section.starts_with("models.") || current_section == "models" {
        if !model_name.is_empty() {
            models.insert(
                model_name.clone(),
                super::super::backends::ModelConfig {
                    model_name: model_name.clone(),
                    provider: model_provider,
                    endpoint: model_endpoint,
                    temperature: model_temperature.unwrap_or(0.7),
                    max_tokens: model_max_tokens.unwrap_or(4096),
                },
            );
        }
    }

    let package = if !pkg_name.is_empty() {
        Some(PackageInfo {
            name: pkg_name,
            version: pkg_version,
            description: pkg_desc,
            authors: pkg_authors,
            license: pkg_license,
        })
    } else {
        None
    };

    Ok((Manifest { package, dependencies, exports }, models))
}
