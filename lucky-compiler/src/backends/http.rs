use std::io::{BufRead, BufReader, Read, Write};

use super::tls::TlsStream;

/// JSON-escape a string for embedding in a JSON body.
pub fn json_escape(s: &str) -> String {
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

/// Extract a JSON string value by key (top-level).
pub fn extract_json_string(json: &str, key: &str) -> Option<String> {
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

/// Extract a JSON string value by key (nested search — finds first match).
pub fn extract_json_string_nested(json: &str, key: &str) -> Option<String> {
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

/// Parse HTTP status code from response.
pub fn parse_status(response: &str) -> Result<u16, String> {
    let first_line = response.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse().map_err(|_| format!("Bad status: {}", first_line))
    } else {
        Err(format!("No status line in: {}", &first_line[..first_line.len().min(100)]))
    }
}

pub fn send_http_request(
    stream: &mut dyn Write, host: &str, path: &str, auth_header: &str, body: &str,
) -> Result<(), String> {
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\n{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, auth_header, body.len(), body
    );
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("HTTP write: {}", e))?;
    stream.flush()
        .map_err(|e| format!("HTTP flush: {}", e))?;
    Ok(())
}

pub fn read_http_response(stream: &mut dyn Read) -> Result<String, String> {
    let mut reader = BufReader::new(stream);
    let mut response = String::new();

    let mut status_line = String::new();
    reader.read_line(&mut status_line)
        .map_err(|e| format!("read status: {}", e))?;
    response.push_str(&status_line);

    let mut content_length: Option<usize> = None;
    let mut is_chunked = false;
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
        if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
            is_chunked = true;
        }
    }

    if is_chunked {
        let body = read_chunked_body(&mut reader)?;
        response.push_str(&body);
    } else if let Some(len) = content_length {
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

pub fn read_chunked_body(reader: &mut BufReader<&mut dyn Read>) -> Result<String, String> {
    let mut body = String::new();
    loop {
        let mut size_line = String::new();
        reader.read_line(&mut size_line)
            .map_err(|e| format!("chunk size: {}", e))?;
        let size_str = size_line.trim();
        let chunk_size = usize::from_str_radix(size_str, 16).unwrap_or(0);
        if chunk_size == 0 {
            reader.read_line(&mut String::new()).ok();
            break;
        }
        let mut chunk = vec![0u8; chunk_size];
        reader.read_exact(&mut chunk)
            .map_err(|e| format!("chunk data: {}", e))?;
        body.push_str(&String::from_utf8_lossy(&chunk));
        reader.read_line(&mut String::new()).ok();
    }
    Ok(body)
}

pub fn http_post_tls(
    host: &str, port: u16, path: &str, auth_header: &str, body: &str,
) -> Result<String, String> {
    let mut stream = TlsStream::connect(host, port)?;
    send_http_request(&mut stream, host, path, auth_header, body)?;
    read_http_response(&mut stream)
}

pub fn http_post_plain(
    host: &str, port: u16, path: &str, auth_header: &str, body: &str,
) -> Result<String, String> {
    let mut stream = std::net::TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| format!("TCP connect: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(120)))
        .map_err(|e| format!("set timeout: {}", e))?;
    send_http_request(&mut stream, host, path, auth_header, body)?;
    read_http_response(&mut stream)
}

pub fn http_post_stream_tls(
    host: &str, port: u16, path: &str, auth_header: &str, body: &str,
) -> Result<Vec<String>, String> {
    let mut stream = TlsStream::connect(host, port)?;
    send_http_request(&mut stream, host, path, auth_header, body)?;
    read_sse_stream(&mut stream)
}

pub fn http_post_stream_plain(
    host: &str, port: u16, path: &str, auth_header: &str, body: &str,
) -> Result<Vec<String>, String> {
    let mut stream = std::net::TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| format!("TCP connect: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(300)))
        .map_err(|e| format!("set timeout: {}", e))?;
    send_http_request(&mut stream, host, path, auth_header, body)?;
    read_sse_stream(&mut stream)
}

pub fn read_sse_stream(stream: &mut dyn Read) -> Result<Vec<String>, String> {
    let mut reader = BufReader::new(stream);
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

    let mut is_chunked = false;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)
            .map_err(|e| format!("read header: {}", e))?;
        if header.trim().is_empty() { break; }
        if header.to_lowercase().starts_with("transfer-encoding:")
            && header.to_lowercase().contains("chunked") {
            is_chunked = true;
        }
    }

    let mut chunks = Vec::new();
    if is_chunked {
        loop {
            let mut size_line = String::new();
            if reader.read_line(&mut size_line).is_err() { break; }
            let size_str = size_line.trim();
            if size_str.is_empty() { continue; }
            let chunk_size = usize::from_str_radix(size_str, 16).unwrap_or(0);
            if chunk_size == 0 {
                reader.read_line(&mut String::new()).ok();
                break;
            }
            let mut chunk = vec![0u8; chunk_size];
            if reader.read_exact(&mut chunk).is_err() { break; }
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
        reader.read_to_string(&mut body)
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
        Err("Stream returned no data".into())
    } else {
        Ok(chunks)
    }
}
