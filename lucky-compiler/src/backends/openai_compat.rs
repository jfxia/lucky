use super::http;
use super::{Backend, CompleteOptions};

/// Configuration for an OpenAI-compatible LLM provider.
/// All providers listed here use the `/v1/chat/completions` JSON format.
pub struct OpenAiCompatConfig {
    pub name: &'static str,
    pub default_endpoint: &'static str,
    pub env_var: &'static str,
    pub default_model: &'static str,
}

pub struct OpenAiCompatBackend {
    pub config: &'static OpenAiCompatConfig,
    endpoint: String,
    api_key: String,
    model: String,
}

impl OpenAiCompatBackend {
    pub fn new(
        config: &'static OpenAiCompatConfig,
        endpoint: Option<String>,
        api_key: Option<String>,
    ) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| config.default_endpoint.to_string());
        let api_key = api_key
            .filter(|k| !k.is_empty())
            .or_else(|| std::env::var(config.env_var).ok())
            .unwrap_or_default();
        let model = std::env::var(format!("{}_MODEL", config.env_var))
            .unwrap_or_else(|_| config.default_model.to_string());
        Self { config, endpoint, api_key, model }
    }

    fn host_port(&self) -> (&str, u16) {
        if self.endpoint.starts_with("https://") {
            let rest = &self.endpoint[8..];
            let host = rest.split('/').next().unwrap_or("api.openai.com");
            (host, 443)
        } else if self.endpoint.starts_with("http://") {
            let rest = &self.endpoint[7..];
            let host = rest.split('/').next().unwrap_or("localhost");
            (host, 80)
        } else {
            ("api.openai.com", 443)
        }
    }

    fn path(&self) -> &str {
        if let Some(pos) = self.endpoint.find("://") {
            let rest = &self.endpoint[pos + 3..];
            if let Some(slash) = rest.find('/') {
                &rest[slash..]
            } else {
                "/v1/chat/completions"
            }
        } else {
            "/v1/chat/completions"
        }
    }

    fn build_request_body(&self, prompt: &str, options: &CompleteOptions, stream: bool) -> String {
        let mut msgs = String::new();
        if let Some(ref sys) = options.system_prompt {
            msgs.push_str(&format!(
                r#"{{"role":"system","content":"{}"}},"#,
                http::json_escape(sys)
            ));
        }
        msgs.push_str(&format!(
            r#"{{"role":"user","content":"{}"}}"#,
            http::json_escape(prompt)
        ));

        format!(
            r#"{{"model":"{}","messages":[{}],"temperature":{},"max_tokens":{},"stream":{}}}"#,
            self.model, msgs, options.temperature, options.max_tokens, stream
        )
    }
}

impl Backend for OpenAiCompatBackend {
    fn name(&self) -> &'static str {
        self.config.name
    }

    fn complete(&self, prompt: &str, options: &CompleteOptions) -> Result<String, String> {
        if self.api_key.is_empty() {
            return Err(format!(
                "{} API key not set. Set {} environment variable or configure in lucky.toml.",
                self.config.name, self.config.env_var
            ));
        }

        let body = self.build_request_body(prompt, options, false);
        let (host, port) = self.host_port();
        let path = self.path();
        let is_https = self.endpoint.starts_with("https://");
        let auth = format!("Authorization: Bearer {}", self.api_key);

        let response = if is_https {
            http::http_post_tls(host, port, path, &auth, &body)?
        } else {
            http::http_post_plain(host, port, path, &auth, &body)?
        };

        let status = http::parse_status(&response)?;
        match status {
            200 => {
                if let Some(content) = http::extract_json_string_nested(&response, "content") {
                    Ok(content)
                } else {
                    Err(format!("{}: no content in response. Body: {}",
                        self.config.name, &response[..response.len().min(500)]))
                }
            }
            401 => Err(format!("{} API: authentication failed (401). Check your {}.", self.config.name, self.config.env_var)),
            429 => Err(format!("{} API: rate limited (429). Wait and retry.", self.config.name)),
            500..=599 => Err(format!("{} API: server error ({})", self.config.name, status)),
            _ => Err(format!("{} API: unexpected status {}", self.config.name, status)),
        }
    }

    fn complete_stream(
        &self, prompt: &str, options: &CompleteOptions,
    ) -> Result<Box<dyn FnMut() -> Option<String>>, String> {
        if self.api_key.is_empty() {
            return Err(format!("{} API key not set.", self.config.name));
        }

        let body = self.build_request_body(prompt, options, true);
        let (host, port) = self.host_port();
        let path = self.path();
        let is_https = self.endpoint.starts_with("https://");
        let auth = format!("Authorization: Bearer {}", self.api_key);

        let chunks = if is_https {
            http::http_post_stream_tls(host, port, path, &auth, &body)?
        } else {
            http::http_post_stream_plain(host, port, path, &auth, &body)?
        };

        let mut iter = chunks.into_iter();
        Ok(Box::new(move || {
            loop {
                match iter.next() {
                    Some(chunk) => {
                        if chunk == "[DONE]" { return None; }
                        if let Some(content) = http::extract_json_string_nested(&chunk, "content") {
                            if !content.is_empty() { return Some(content); }
                        }
                    }
                    None => return None,
                }
            }
        }))
    }

    fn health_check(&self) -> bool {
        self.api_key.is_empty()
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        0.001
    }
}

pub static OPENAI_CONFIG: OpenAiCompatConfig = OpenAiCompatConfig {
    name: "openai",
    default_endpoint: "https://api.openai.com/v1/chat/completions",
    env_var: "OPENAI_API_KEY",
    default_model: "gpt-4o",
};

pub static DEEPSEEK_CONFIG: OpenAiCompatConfig = OpenAiCompatConfig {
    name: "deepseek",
    default_endpoint: "https://api.deepseek.com/chat/completions",
    env_var: "DEEPSEEK_API_KEY",
    default_model: "deepseek-chat",
};

pub static KIMI_CONFIG: OpenAiCompatConfig = OpenAiCompatConfig {
    name: "kimi",
    default_endpoint: "https://api.moonshot.cn/v1/chat/completions",
    env_var: "KIMI_API_KEY",
    default_model: "kimi-latest",
};

pub static QWEN_CONFIG: OpenAiCompatConfig = OpenAiCompatConfig {
    name: "qwen",
    default_endpoint: "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions",
    env_var: "QWEN_API_KEY",
    default_model: "qwen-max",
};

pub static DOUBAO_CONFIG: OpenAiCompatConfig = OpenAiCompatConfig {
    name: "doubao",
    default_endpoint: "https://ark.cn-beijing.volces.com/api/v3/chat/completions",
    env_var: "DOUBAO_API_KEY",
    default_model: "doubao-pro-32k",
};

pub static GLM_CONFIG: OpenAiCompatConfig = OpenAiCompatConfig {
    name: "glm",
    default_endpoint: "https://open.bigmodel.cn/api/paas/v4/chat/completions",
    env_var: "GLM_API_KEY",
    default_model: "glm-4-plus",
};

pub static GOOGLE_CONFIG: OpenAiCompatConfig = OpenAiCompatConfig {
    name: "google",
    default_endpoint: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
    env_var: "GOOGLE_API_KEY",
    default_model: "gemini-2.0-flash",
};
