use super::http;
use super::{Backend, CompleteOptions};

pub struct OpenAiBackend {
    endpoint: String,
    api_key: String,
    model: String,
}

impl OpenAiBackend {
    pub fn new(endpoint: Option<String>, api_key: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());
        let api_key = api_key.filter(|k| !k.is_empty())
            .or_else(|| std::env::var("OPENAI_API_KEY").ok()).unwrap_or_default();
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
        Self { endpoint, api_key, model }
    }

    fn host_port(&self) -> (&str, u16) {
        if self.endpoint.starts_with("https://") { let rest = &self.endpoint[8..]; (rest.split('/').next().unwrap_or("api.openai.com"), 443) }
        else if self.endpoint.starts_with("http://") { let rest = &self.endpoint[7..]; (rest.split('/').next().unwrap_or("localhost"), 80) }
        else { ("api.openai.com", 443) }
    }

    fn path(&self) -> &str {
        if let Some(pos) = self.endpoint.find("://") { let rest = &self.endpoint[pos + 3..]; if let Some(slash) = rest.find('/') { &rest[slash..] } else { "/v1/chat/completions" } }
        else { "/v1/chat/completions" }
    }

    fn build_request_body(&self, prompt: &str, options: &CompleteOptions, stream: bool) -> String {
        let mut msgs = String::new();
        if let Some(ref sys) = options.system_prompt { msgs.push_str(&format!(r#"{{"role":"system","content":"{}"}},"#, http::json_escape(sys))); }
        msgs.push_str(&format!(r#"{{"role":"user","content":"{}"}}"#, http::json_escape(prompt)));
        format!(r#"{{"model":"{}","messages":[{}],"temperature":{},"max_tokens":{},"stream":{}}}"#, self.model, msgs, options.temperature, options.max_tokens, stream)
    }
}

impl Backend for OpenAiBackend {
    fn name(&self) -> &'static str { "openai" }

    fn complete(&self, prompt: &str, options: &CompleteOptions) -> Result<String, String> {
        if self.api_key.is_empty() { return Err("OPENAI_API_KEY not set.".to_string()); }
        let body = self.build_request_body(prompt, options, false);
        let (host, port) = self.host_port();
        let path = self.path();
        let is_https = self.endpoint.starts_with("https://");
        let auth = format!("Authorization: Bearer {}", self.api_key);
        let response = if is_https { http::http_post_tls(host, port, path, &auth, &body)? } else { http::http_post_plain(host, port, path, &auth, &body)? };
        let status = http::parse_status(&response)?;
        match status {
            200 => http::extract_json_string_nested(&response, "content").ok_or_else(|| format!("OpenAI: no content. Body: {}", &response[..response.len().min(500)])),
            401 => Err("OpenAI API: authentication failed (401).".into()),
            429 => Err("OpenAI API: rate limited (429).".into()),
            500..=599 => Err(format!("OpenAI API: server error ({})", status)),
            _ => Err(format!("OpenAI API: unexpected status {}", status)),
        }
    }

    fn complete_stream(&self, prompt: &str, options: &CompleteOptions) -> Result<Box<dyn FnMut() -> Option<String>>, String> {
        if self.api_key.is_empty() { return Err("OPENAI_API_KEY not set.".to_string()); }
        let body = self.build_request_body(prompt, options, true);
        let (host, port) = self.host_port();
        let path = self.path();
        let is_https = self.endpoint.starts_with("https://");
        let auth = format!("Authorization: Bearer {}", self.api_key);
        let chunks = if is_https { http::http_post_stream_tls(host, port, path, &auth, &body)? } else { http::http_post_stream_plain(host, port, path, &auth, &body)? };
        let mut iter = chunks.into_iter();
        Ok(Box::new(move || loop { match iter.next() {
            Some(chunk) => { if chunk == "[DONE]" { return None; } if let Some(content) = http::extract_json_string_nested(&chunk, "content") { if !content.is_empty() { return Some(content); } } }
            None => return None,
        }}))
    }

    fn health_check(&self) -> bool { self.api_key.is_empty() }
    fn cost_per_1k_tokens(&self) -> f64 { 0.01 }
}
