use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use super::{Backend, CompleteOptions};

pub struct OllamaBackend {
    endpoint: String,
}

impl OllamaBackend {
    pub fn new(endpoint: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| {
            "http://localhost:11434/api/generate".to_string()
        });
        Self { endpoint }
    }

    fn host_port(&self) -> (&str, u16) {
        if self.endpoint.starts_with("http://") {
            let rest = &self.endpoint[7..];
            let host = rest.split('/').next().unwrap_or("localhost");
            (host, 80)
        } else {
            ("localhost", 11434)
        }
    }

    fn port_fallback(&self) -> u16 {
        if self.endpoint.contains(":11434") {
            11434
        } else if self.endpoint.starts_with("http://") {
            let rest = &self.endpoint[7..];
            if let Some(colon) = rest.find(':') {
                let after_host = &rest[colon + 1..];
                if let Some(slash) = after_host.find('/') {
                    after_host[..slash].parse().unwrap_or(11434)
                } else {
                    after_host.parse().unwrap_or(11434)
                }
            } else {
                80
            }
        } else {
            11434
        }
    }

    fn path(&self) -> &str {
        if let Some(pos) = self.endpoint.find("://") {
            let rest = &self.endpoint[pos + 3..];
            if let Some(slash) = rest.find('/') {
                &rest[slash..]
            } else {
                "/api/generate"
            }
        } else {
            "/api/generate"
        }
    }

    fn build_request_body(
        &self, prompt: &str, options: &CompleteOptions, stream: bool,
    ) -> String {
        let mut parts = Vec::new();
        parts.push(format!(r#""model":"llama3""#));
        parts.push(format!(r#""prompt":"{}""#, json_escape(prompt)));
        if let Some(ref sys) = options.system_prompt {
            parts.push(format!(r#""system":"{}""#, json_escape(sys)));
        }
        parts.push(format!(r#""stream":{}"#, stream));
        format!(r#"{{{}}}"#, parts.join(","))
    }

    fn parse_response(body: &str) -> Result<String, String> {
        if let Some(err) = extract_json_string(body, "error") {
            return Err(err);
        }
        extract_json_string(body, "response")
            .ok_or_else(|| format!("Ollama: no response field. Body: {}", &body[..body.len().min(500)]))
    }
}

impl Backend for OllamaBackend {
    fn name(&self) -> &'static str {
        "ollama"
    }

    fn complete(&self, prompt: &str, options: &CompleteOptions) -> Result<String, String> {
        let body = self.build_request_body(prompt, options, false);
        let (host, _) = self.host_port();
        let port = self.port_fallback();
        let path = self.path();

        let response = http_post_plain(host, port, path, &body)?;

        let status = parse_status(&response)?;
        match status {
            200 => Self::parse_response(&response),
            500..=599 => Err(format!("Ollama: server error ({})", status)),
            _ => Err(format!("Ollama: unexpected status {}", status)),
        }
    }

    fn complete_stream(
        &self, prompt: &str, options: &CompleteOptions,
    ) -> Result<Box<dyn FnMut() -> Option<String>>, String> {
        let body = self.build_request_body(prompt, options, true);
        let (host, _) = self.host_port();
        let port = self.port_fallback();
        let path = self.path();

        let chunks = http_post_stream_plain(host, port, path, &body)?;

        let mut iter = chunks.into_iter();
        Ok(Box::new(move || {
            iter.next().and_then(|chunk| {
                extract_json_string(&chunk, "response")
                    .filter(|s| !s.is_empty())
            })
        }))
    }

    fn health_check(&self) -> bool {
        match http_get_plain("localhost", 11434, "/api/tags") {
            Ok(resp) => resp.contains("200"),
            Err(_) => false,
        }
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        0.0
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            _ => out.push(c),
        }
    }
    out
}

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let pos = json.find(&pattern)?;
    let after = &json[pos + pattern.len()..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    if after.starts_with('"') {
        let mut result = String::new();
        let chars: Vec<char> = after[1..].chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                match chars[i + 1] {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    _ => {}
                }
                i += 2;
            } else if chars[i] == '"' {
                break;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }
        Some(result)
    } else {
        None
    }
}

fn parse_status(response: &str) -> Result<u16, String> {
    let first_line = response.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse().map_err(|_| format!("Bad status: {}", first_line))
    } else {
        Err(format!("No status line in: {}", &first_line[..first_line.len().min(100)]))
    }
}

fn http_post_plain(host: &str, port: u16, path: &str, body: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| format!("TCP connect to {}:{}: {}", host, port, e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(120)))
        .map_err(|e| format!("set timeout: {}", e))?;

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, port, body.len(), body
    );
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("HTTP write: {}", e))?;
    stream.flush()
        .map_err(|e| format!("HTTP flush: {}", e))?;

    read_http_response(&mut stream)
}

fn http_get_plain(host: &str, port: u16, path: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| format!("TCP connect: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| format!("set timeout: {}", e))?;

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
        path, host, port
    );
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("HTTP write: {}", e))?;
    stream.flush()
        .map_err(|e| format!("HTTP flush: {}", e))?;

    read_http_response(&mut stream)
}

fn http_post_stream_plain(
    host: &str, port: u16, path: &str, body: &str,
) -> Result<Vec<String>, String> {
    let mut stream = TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| format!("TCP connect: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(300)))
        .map_err(|e| format!("set timeout: {}", e))?;

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, port, body.len(), body
    );
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("HTTP write: {}", e))?;
    stream.flush()
        .map_err(|e| format!("HTTP flush: {}", e))?;

    let mut reader = BufReader::new(&mut stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line)
        .map_err(|e| format!("read status: {}", e))?;

    let parts: Vec<&str> = status_line.split_whitespace().collect();
    if parts.len() >= 2 {
        let code: u16 = parts[1].parse().unwrap_or(0);
        if code >= 400 {
            let mut error_body = String::new();
            reader.read_to_string(&mut error_body).ok();
            return Err(format!("HTTP {} error: {}", code, error_body));
        }
    }

    loop {
        let mut header = String::new();
        reader.read_line(&mut header)
            .map_err(|e| format!("read header: {}", e))?;
        if header.trim().is_empty() { break; }
    }

    let mut chunks = Vec::new();
    let mut buf = String::new();
    reader.read_to_string(&mut buf)
        .map_err(|e| format!("read stream body: {}", e))?;

    for line in buf.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if let Some(response) = extract_json_string(trimmed, "response") {
            chunks.push(response);
        } else if let Some(error) = extract_json_string(trimmed, "error") {
            return Err(error);
        }
    }

    if chunks.is_empty() {
        return Err("Ollama stream returned no data".into());
    }
    Ok(chunks)
}

fn read_http_response(stream: &mut dyn Read) -> Result<String, String> {
    let mut reader = BufReader::new(stream);
    let mut response = String::new();

    let mut status_line = String::new();
    reader.read_line(&mut status_line)
        .map_err(|e| format!("read status: {}", e))?;
    response.push_str(&status_line);

    let mut content_length: Option<usize> = None;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)
            .map_err(|e| format!("read header: {}", e))?;
        response.push_str(&header);
        if header.trim().is_empty() { break; }
        let lower = header.to_lowercase();
        if lower.starts_with("content-length:") {
            content_length = header.split(':').nth(1)
                .and_then(|v| v.trim().parse().ok());
        }
    }

    if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)
            .map_err(|e| format!("read body len={}: {}", len, e))?;
        response.push_str(&String::from_utf8_lossy(&buf));
    } else {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)
            .map_err(|e| format!("read body: {}", e))?;
        response.push_str(&String::from_utf8_lossy(&buf));
    }

    Ok(response)
}
