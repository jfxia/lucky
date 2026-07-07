use std::io::Write;
use std::net::TcpStream;

use super::http;
use super::{Backend, CompleteOptions};

pub struct OllamaBackend {
    endpoint: String,
}

impl OllamaBackend {
    pub fn new(endpoint: Option<String>, _api_key: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| "http://localhost:11434/api/generate".to_string());
        Self { endpoint }
    }

    fn host_port(&self) -> (&str, u16) {
        if self.endpoint.starts_with("http://") {
            let rest = &self.endpoint[7..];
            (rest.split('/').next().unwrap_or("localhost"), 80)
        } else { ("localhost", 11434) }
    }

    fn path(&self) -> &str {
        if let Some(pos) = self.endpoint.find("://") {
            let rest = &self.endpoint[pos + 3..];
            if let Some(slash) = rest.find('/') { &rest[slash..] } else { "/api/generate" }
        } else { "/api/generate" }
    }

    fn build_request_body(&self, prompt: &str, options: &CompleteOptions, stream: bool) -> String {
        let sys = options.system_prompt.as_deref().unwrap_or("");
        let opts = format!(r#"{{"temperature":{},"num_predict":{}}}"#, options.temperature, options.max_tokens);
        format!(r#"{{"model":"llama3","prompt":"{}","system":"{}","stream":{},"options":{}}}"#,
            http::json_escape(prompt), http::json_escape(sys), stream, opts)
    }

    fn parse_response(body: &str) -> Result<String, String> {
        if let Some(response) = body.lines().last() {
            if let Some(content) = http::extract_json_string(response, "response") {
                return Ok(content);
            }
        }
        http::extract_json_string(body, "response")
            .ok_or_else(|| format!("Ollama: no response. Body: {}", &body[..body.len().min(200)]))
    }
}

impl Backend for OllamaBackend {
    fn name(&self) -> &'static str { "ollama" }

    fn complete(&self, prompt: &str, options: &CompleteOptions) -> Result<String, String> {
        let body = self.build_request_body(prompt, options, false);
        let (host, port) = self.host_port();
        let path = self.path();
        let mut stream = TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| format!("Ollama connect: {}", e))?;
        http::send_http_request(&mut stream, host, path, "", &body)?;
        let response = http::read_http_response(&mut stream)?;
        Self::parse_response(&response)
    }

    fn complete_stream(&self, prompt: &str, options: &CompleteOptions) -> Result<Box<dyn FnMut() -> Option<String>>, String> {
        let body = self.build_request_body(prompt, options, true);
        let (host, port) = self.host_port();
        let path = self.path();
        let mut stream = TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| format!("Ollama connect: {}", e))?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(300)))
            .map_err(|e| format!("set timeout: {}", e))?;
        http::send_http_request(&mut stream, host, path, "", &body)?;
        let chunks = http::read_sse_stream(&mut stream)?;
        let mut iter = chunks.into_iter();
        Ok(Box::new(move || loop {
            match iter.next() {
                Some(chunk) => {
                    if let Some(content) = http::extract_json_string(&chunk, "response") {
                        if !content.is_empty() { return Some(content); }
                    }
                }
                None => return None,
            }
        }))
    }

    fn health_check(&self) -> bool { false }
    fn cost_per_1k_tokens(&self) -> f64 { 0.0 }
}
