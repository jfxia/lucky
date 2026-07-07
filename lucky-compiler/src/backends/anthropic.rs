use super::http;
use super::{Backend, CompleteOptions};

pub struct AnthropicBackend {
    endpoint: String,
    api_key: String,
    model: String,
}

impl AnthropicBackend {
    pub fn new(endpoint: Option<String>, api_key: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| {
            "https://api.anthropic.com/v1/messages".to_string()
        });
        let api_key = api_key
            .filter(|k| !k.is_empty())
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .unwrap_or_default();
        let model = std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
        Self { endpoint, api_key, model }
    }

    fn host_port(&self) -> (&str, u16) {
        if self.endpoint.starts_with("https://") {
            let rest = &self.endpoint[8..];
            let host = rest.split('/').next().unwrap_or("api.anthropic.com");
            (host, 443)
        } else if self.endpoint.starts_with("http://") {
            let rest = &self.endpoint[7..];
            let host = rest.split('/').next().unwrap_or("localhost");
            (host, 80)
        } else {
            ("api.anthropic.com", 443)
        }
    }

    fn path(&self) -> &str {
        if let Some(pos) = self.endpoint.find("://") {
            let rest = &self.endpoint[pos + 3..];
            if let Some(slash) = rest.find('/') {
                &rest[slash..]
            } else {
                "/v1/messages"
            }
        } else {
            "/v1/messages"
        }
    }

    fn build_request_body(&self, prompt: &str, options: &CompleteOptions, stream: bool) -> String {
        let mut msgs = String::new();
        if let Some(ref sys) = options.system_prompt {
            msgs.push_str(&format!(
                r#"{{"type":"text","text":"{}"}}"#,
                http::json_escape(sys)
            ));
        }
        msgs.push_str(&format!(
            r#"{{"role":"user","content":"{}"}}"#,
            http::json_escape(prompt)
        ));

        format!(
            r#"{{"model":"{}","max_tokens":{},"messages":[{}],"stream":{}}}"#,
            self.model, options.max_tokens, msgs, stream
        )
    }
}

impl Backend for AnthropicBackend {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn complete(&self, prompt: &str, options: &CompleteOptions) -> Result<String, String> {
        if self.api_key.is_empty() {
            return Err(
                "ANTHROPIC_API_KEY environment variable not set. Set it to your Anthropic API key."
                    .to_string(),
            );
        }

        let body = self.build_request_body(prompt, options, false);
        let (host, port) = self.host_port();
        let path = self.path();
        let is_https = self.endpoint.starts_with("https://");
        let auth = format!("x-api-key: {}", self.api_key);

        let response = if is_https {
            http::http_post_tls(host, port, path, &auth, &body)?
        } else {
            http::http_post_plain(host, port, path, &auth, &body)?
        };

        let status = http::parse_status(&response)?;
        match status {
            200 => {
                if let Some(content) = http::extract_json_string_nested(&response, "text") {
                    Ok(content)
                } else {
                    Err(format!("Anthropic: no content in response. Body: {}",
                        &response[..response.len().min(500)]))
                }
            }
            401 => Err("Anthropic API: authentication failed (401). Check your ANTHROPIC_API_KEY.".into()),
            429 => Err("Anthropic API: rate limited (429). Wait and retry.".into()),
            500..=599 => Err(format!("Anthropic API: server error ({})", status)),
            _ => Err(format!("Anthropic API: unexpected status {}", status)),
        }
    }

    fn complete_stream(
        &self, prompt: &str, options: &CompleteOptions,
    ) -> Result<Box<dyn FnMut() -> Option<String>>, String> {
        if self.api_key.is_empty() {
            return Err("ANTHROPIC_API_KEY not set.".to_string());
        }

        let body = self.build_request_body(prompt, options, true);
        let (host, port) = self.host_port();
        let path = self.path();
        let is_https = self.endpoint.starts_with("https://");
        let auth = format!("x-api-key: {}", self.api_key);

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
                        if chunk == "[DONE]" || chunk.contains("\"type\":\"message_stop\"") {
                            return None;
                        }
                        if let Some(content) = http::extract_json_string_nested(&chunk, "text") {
                            if !content.is_empty() {
                                return Some(content);
                            }
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
        0.003
    }
}
