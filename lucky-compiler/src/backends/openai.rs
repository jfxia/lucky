use std::io::{BufRead, BufReader, Read, Write};

use super::tls::TlsStream;
use super::{Backend, CompleteOptions};

pub struct OpenAiBackend {
    endpoint: String,
    api_key: String,
    model: String,
}

impl OpenAiBackend {
    pub fn new(endpoint: Option<String>, api_key: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| {
            "https://api.openai.com/v1/chat/completions".to_string()
        });
        let api_key = api_key
            .filter(|k| !k.is_empty())
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .unwrap_or_default();
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
        Self { endpoint, api_key, model }
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

    fn build_request_body(
        &self, prompt: &str, options: &CompleteOptions, stream: bool,
    ) -> String {
        let mut msgs = String::new();
        if let Some(ref sys) = options.system_prompt {
            msgs.push_str(&format!(
                r#"{{"role":"system","content":"{}"}},"#,
                json_escape(sys)
            ));
        }
        msgs.push_str(&format!(
            r#"{{"role":"user","content":"{}"}}"#,
            json_escape(prompt)
        ));

        format!(
            r#"{{"model":"{}","messages":[{}],"temperature":{},"max_tokens":{},"stream":{}}}"#,
            self.model, msgs, options.temperature, options.max_tokens, stream
        )
    }
}

impl Backend for OpenAiBackend {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn complete(&self, prompt: &str, options: &CompleteOptions) -> Result<String, String> {
        if self.api_key.is_empty() {
            return Err(
                "OPENAI_API_KEY environment variable not set. Set it to your OpenAI API key."
                    .to_string(),
            );
        }

        let body = self.build_request_body(prompt, options, false);
        let (host, port) = self.host_port();
        let path = self.path();
        let is_https = self.endpoint.starts_with("https://");

        let response = if is_https {
            http_post_tls(host, port, path, &self.api_key, &body)?
        } else {
            http_post_plain(host, port, path, &self.api_key, &body)?
        };

        let status = parse_status(&response)?;
        match status {
            200 => {
                if let Some(content) = extract_json_string_nested(&response, "content") {
                    Ok(content)
                } else {
                    Err(format!("OpenAI: no content in response. Body: {}",
                        &response[..response.len().min(500)]))
                }
            }
            401 => Err("OpenAI API: authentication failed (401). Check your OPENAI_API_KEY.".into()),
            429 => Err("OpenAI API: rate limited (429). Wait and retry.".into()),
            500..=599 => Err(format!("OpenAI API: server error ({})", status)),
            _ => Err(format!("OpenAI API: unexpected status {}", status)),
        }
    }

    fn complete_stream(
        &self, prompt: &str, options: &CompleteOptions,
    ) -> Result<Box<dyn FnMut() -> Option<String>>, String> {
        if self.api_key.is_empty() {
            return Err("OPENAI_API_KEY environment variable not set.".to_string());
        }

        let body = self.build_request_body(prompt, options, true);
        let (host, port) = self.host_port();
        let path = self.path();
        let is_https = self.endpoint.starts_with("https://");

        let chunks = if is_https {
            http_post_stream_tls(host, port, path, &self.api_key, &body)?
        } else {
            http_post_stream_plain(host, port, path, &self.api_key, &body)?
        };

        let mut iter = chunks.into_iter();
        Ok(Box::new(move || {
            loop {
                match iter.next() {
                    Some(chunk) => {
                        if chunk == "[DONE]" {
                            return None;
                        }
                        if let Some(content) = extract_json_string_nested(&chunk, "content") {
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
        0.01
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

fn extract_json_string_nested(json: &str, key: &str) -> Option<String> {
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

fn http_post_tls(
    host: &str, port: u16, path: &str, api_key: &str, body: &str,
) -> Result<String, String> {
    let mut stream = TlsStream::connect(host, port)?;
    send_http_request(&mut stream, host, path, api_key, body)?;
    read_http_response(&mut stream)
}

fn http_post_plain(
    host: &str, port: u16, path: &str, api_key: &str, body: &str,
) -> Result<String, String> {
    let mut stream = std::net::TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| format!("TCP connect: {}", e))?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(120)))
        .map_err(|e| format!("set timeout: {}", e))?;
    send_http_request(&mut stream, host, path, api_key, body)?;
    read_http_response(&mut stream)
}

fn send_http_request(
    stream: &mut dyn Write, host: &str, path: &str, api_key: &str, body: &str,
) -> Result<(), String> {
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, api_key, body.len(), body
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("HTTP write: {}", e))?;
    stream.flush().map_err(|e| format!("HTTP flush: {}", e))?;
    Ok(())
}

fn read_http_response(stream: &mut dyn Read) -> Result<String, String> {
    let mut reader = BufReader::new(stream);
    let mut response = String::new();

    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .map_err(|e| format!("read status: {}", e))?;
    response.push_str(&status_line);

    let mut content_length: Option<usize> = None;
    let mut is_chunked = false;
    loop {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|e| format!("read header: {}", e))?;
        response.push_str(&header);
        if header.trim().is_empty() {
            break;
        }
        let lower = header.to_lowercase();
        if lower.starts_with("content-length:") {
            content_length = header
                .split(':')
                .nth(1)
                .and_then(|v| v.trim().parse().ok());
        }
        if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
            is_chunked = true;
        }
    }

    if is_chunked {
        let body = read_chunked_body(&mut reader)?;
        response.push_str(&body);
    } else if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        reader
            .read_exact(&mut buf)
            .map_err(|e| format!("read body len={}: {}", len, e))?;
        response.push_str(&String::from_utf8_lossy(&buf));
    } else {
        let mut buf = Vec::new();
        reader
            .read_to_end(&mut buf)
            .map_err(|e| format!("read body: {}", e))?;
        response.push_str(&String::from_utf8_lossy(&buf));
    }

    Ok(response)
}

fn http_post_stream_tls(
    host: &str, port: u16, path: &str, api_key: &str, body: &str,
) -> Result<Vec<String>, String> {
    let mut stream = TlsStream::connect(host, port)?;
    send_http_request(&mut stream, host, path, api_key, body)?;
    read_sse_stream(&mut stream)
}

fn http_post_stream_plain(
    host: &str, port: u16, path: &str, api_key: &str, body: &str,
) -> Result<Vec<String>, String> {
    let mut stream = std::net::TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| format!("TCP connect: {}", e))?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(300)))
        .map_err(|e| format!("set timeout: {}", e))?;
    send_http_request(&mut stream, host, path, api_key, body)?;
    read_sse_stream(&mut stream)
}

fn read_sse_stream(stream: &mut dyn Read) -> Result<Vec<String>, String> {
    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
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

    let mut is_chunked = false;
    loop {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|e| format!("read header: {}", e))?;
        if header.trim().is_empty() {
            break;
        }
        if header.to_lowercase().starts_with("transfer-encoding:")
            && header.to_lowercase().contains("chunked")
        {
            is_chunked = true;
        }
    }

    let mut chunks = Vec::new();
    if is_chunked {
        loop {
            let mut size_line = String::new();
            if reader.read_line(&mut size_line).is_err() {
                break;
            }
            let size_str = size_line.trim();
            if size_str.is_empty() {
                continue;
            }
            let chunk_size = usize::from_str_radix(size_str, 16).unwrap_or(0);
            if chunk_size == 0 {
                reader.read_line(&mut String::new()).ok();
                break;
            }
            let mut chunk = vec![0u8; chunk_size];
            if reader.read_exact(&mut chunk).is_err() {
                break;
            }
            reader.read_line(&mut String::new()).ok();
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                let trimmed = line.trim();
                if let Some(data) = trimmed.strip_prefix("data: ") {
                    chunks.push(data.to_string());
                } else if trimmed.starts_with("data:") {
                    let data = trimmed[5..].trim();
                    chunks.push(data.to_string());
                }
            }
        }
    } else {
        let mut body = String::new();
        reader
            .read_to_string(&mut body)
            .map_err(|e| format!("read stream body: {}", e))?;
        for line in body.lines() {
            let trimmed = line.trim();
            if let Some(data) = trimmed.strip_prefix("data: ") {
                chunks.push(data.to_string());
            } else if trimmed.starts_with("data:") {
                chunks.push(trimmed[5..].trim().to_string());
            }
        }
    }

    if chunks.is_empty() {
        return Err("Stream returned no data".into());
    }
    Ok(chunks)
}

fn read_chunked_body(reader: &mut BufReader<&mut dyn Read>) -> Result<String, String> {
    let mut body = String::new();
    loop {
        let mut size_line = String::new();
        reader
            .read_line(&mut size_line)
            .map_err(|e| format!("chunk size: {}", e))?;
        let size_str = size_line.trim();
        let chunk_size = usize::from_str_radix(size_str, 16).unwrap_or(0);
        if chunk_size == 0 {
            reader.read_line(&mut String::new()).ok();
            break;
        }
        let mut chunk = vec![0u8; chunk_size];
        reader
            .read_exact(&mut chunk)
            .map_err(|e| format!("chunk data: {}", e))?;
        body.push_str(&String::from_utf8_lossy(&chunk));
        reader.read_line(&mut String::new()).ok();
    }
    Ok(body)
}
