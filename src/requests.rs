use std::collections::HashMap;
use crate::upload_handler::decode_chunked_body;
#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug)]
pub struct Response {
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub fn parse_http_request(raw: &[u8]) -> Option<Request> {
    println!("DEBUG: parse_http_request called with {} bytes", raw.len());
    
    // Find the end of headers (double CRLF)
    let header_end = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    let header_end = header_end + 4; // Include the CRLF
    
    println!("DEBUG: Header end at position: {}", header_end);
    
    // Parse headers as string
    let header_str = std::str::from_utf8(&raw[..header_end]).ok()?;
    let mut lines = header_str.split("\r\n");
    let request_line = lines.next()?; // e.g. GET /index.html HTTP/1.1

    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let version = parts.next()?.to_string();

    let mut headers = HashMap::new();
    for line in &mut lines {
        if line.is_empty() {
            break; // End of headers
        }
        if let Some((key, value)) = line.split_once(": ") {
            headers.insert(key.to_string(), value.to_string());
        }
    }

    // Get the body as raw bytes
    let mut body = raw[header_end..].to_vec();
    println!("DEBUG: Raw body length: {}", body.len());
    
    // Handle chunked transfer encoding
    if headers.get("Transfer-Encoding").map(|s| s.to_lowercase()) == Some("chunked".to_string()) {
        println!("DEBUG: Detected chunked transfer encoding, decoding body...");
        // Decode chunked body
        if let Ok(decoded) = decode_chunked_body(&body) {
            println!("DEBUG: Successfully decoded chunked body, new length: {}", decoded.len());
            body = decoded;
        } else {
            println!("DEBUG: Failed to decode chunked body");
        }
    }

    println!("DEBUG: Final body length: {}", body.len());
    Some(Request {
        method,
        path,
        version,
        headers,
        body,
    })
}

pub fn build_response(res: Response) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 {} {}\r\n", res.status_code, res.reason_phrase);

    if !res.headers.contains_key("Content-Length") {
        response += &format!("Content-Length: {}\r\n", res.body.len());
    }

    for (key, value) in res.headers {
        response += &format!("{}: {}\r\n", key, value);
    }

    response += "\r\n";

    let mut full_response = response.into_bytes();
    full_response.extend(res.body);

    full_response
}
