use cgi::run_cgi_script;
use mio::{Events, Interest, Poll, Token};
use serde::Deserialize;
use serverConfig::ServerConfig;
use static_file::{FileResponse, build_http_response, read_static_file_with_listing};
use std::collections::HashMap;
use std::env;
use std::io;
use std::io::{Read, Write};
// use std::os::unix::io::{AsRawFd, RawFd};
use mio::net::{TcpListener, TcpStream};
use requests::{Request, Response, build_response, parse_http_request};
use std::time::Instant;
use std::{fs, time::Duration};
use upload_handler::{UploadResult, build_upload_response, handle_file_upload};
mod session_manager;
use session_manager::SessionManager;

use crate::serverConfig::Connection;
mod cgi;
mod requests;
mod serverConfig;
mod static_file;
mod upload_handler;

const CLIENT_TIMEOUT: Duration = Duration::from_secs(30); // Increased from 10 to 30 seconds

fn main() {
    let servers = json_parser();
    let mut session_manager = SessionManager::new(); // create a new session manager
    for server in &servers {
        listener_socket(server, &mut session_manager, server);
    }
}
// fn test_cgi()-> Result<(), Box<dyn Error>> {
// let mut wd = env::current_dir().unwrap();
// let python_file_path = wd.join("py.py");
// println!("{}", python_file_path.display());
// let body = "name=Rust";
// let path_info = wd.to_string_lossy();
// ///mnt/c/Users/admar/Desktop/lh/py.py
// match run_cgi_script(python_file_path.to_str().unwrap(), body, &path_info) {
//     Ok(output) => println!("{}", output),
//     Err(e) => eprintln!("Error: {}", e),
// }
// Ok(())
// }

fn json_parser() -> Vec<ServerConfig> {
    let mut wd = env::current_dir().unwrap();
    let config_path = wd.join("src/config.json");
    println!("wd {}", wd.display());
    let file = fs::read_to_string(config_path).expect("file don't exist");
    println!("fu=ile {}", file);
    let servers: Vec<ServerConfig> =
        serde_json::from_str(&file).expect("JSON was not well-formatted");

    // let servers: Vec<ServerConfig> =
    //     serde_yaml::from_str(&file).expect("yaml was not well-formatted");
    //for printing the servers configs
    // for server in servers {
    //     println!("Server Name: {:#?}", server);
    // }

    servers
}
const SERVER: Token = Token(0);

fn listener_socket(
    server: &ServerConfig,
    session_manager: &mut SessionManager,
    server_config: &ServerConfig,
) -> std::io::Result<()> {
    let mut listeners: HashMap<Token, TcpListener> = HashMap::new();

    for address in &server.server_address {
        let mut next_token = listeners.len();
        let bind_address = format!("{}:{}", address.ip, address.port);
        let socket_addr: std::net::SocketAddr =
            bind_address.parse().expect("Invalid socket address");

        // opens a tcp socket and it become passive
        // binding is like : I want to listen for connections on this IP:PORT
        let mut listener =
            TcpListener::bind(socket_addr).expect(&format!("Failed to bind to {}", bind_address));
        println!("Listening on {}", bind_address);

        let token = Token(next_token);
        println!("listenr tokken {:?}", token);

        listeners.insert(token, listener);
    }

    run_mio_server(listeners, session_manager, server_config)
}

// Centralized request handler
fn handle_request(
    raw_request: &[u8],
    session_manager: &mut SessionManager,
    server_config: &ServerConfig,
) -> Vec<u8> {
    // Helper to load custom error page if configured
    fn custom_error_body(code: u16, config: &ServerConfig) -> Option<Vec<u8>> {
        if let Some(page_name) = config.error_msg.get(&code) {
            let path = format!("html/{}.html", page_name.replace(' ', "_").to_lowercase());
            if let Ok(contents) = std::fs::read(&path) {
                return Some(contents);
            }
        }
        None
    }
    // Parse the HTTP request
    let req = match parse_http_request(raw_request) {
        Some(r) => r,
        None => {
            let mut headers = HashMap::new();
            let body = custom_error_body(400, server_config)
                .unwrap_or_else(|| b"<h1>400 Bad Request</h1>".to_vec());
            headers.insert("Content-Type".to_string(), "text/html".to_string());
            return build_response(Response {
                status_code: 400,
                reason_phrase: "Bad Request".to_string(),
                headers,
                body,
            });
        }
    };
    // Session management
    let cookie_header = req.headers.get("Cookie").map(|s| s.as_str());
    let session = session_manager.get_or_create_session(cookie_header);
    let mut set_cookie_header = None;
    if cookie_header.is_none() || !cookie_header.unwrap().contains(&session.id) {
        set_cookie_header = Some(format!("session_id={}; Path=/; HttpOnly", session.id));
    }
    // Routing: find matching route
    println!("DEBUG: Request path: '{}'", req.path);
    println!("DEBUG: Request method: '{}'", req.method);
    let route = server_config.router.iter().max_by_key(|r| {
        if req.path.starts_with(&r.path) {
            r.path.len()
        } else {
            0
        }
    });
    if let Some(route) = route {
        println!("DEBUG: Matched route path: '{}'", route.path);
        println!("DEBUG: Route methods: {:?}", route.methods);

        // Redirection support
        if let Some(redir) = &route.redirection {
            let status = redir.status.unwrap_or(302);
            let reason = match status {
                301 => "Moved Permanently",
                302 => "Found",
                307 => "Temporary Redirect",
                308 => "Permanent Redirect",
                _ => "Found",
            };
            let mut headers = HashMap::new();
            headers.insert("Location".to_string(), redir.target.clone());
            if let Some(cookie) = set_cookie_header.clone() {
                headers.insert("Set-Cookie".to_string(), cookie);
            }
            return build_response(Response {
                status_code: status,
                reason_phrase: reason.to_string(),
                headers,
                body: Vec::new(),
            });
        }
        // Method allowed?
        if !route.methods.iter().any(|m| m == &req.method) {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "text/html".to_string());
            if let Some(cookie) = set_cookie_header.clone() {
                headers.insert("Set-Cookie".to_string(), cookie);
            }
            let body = custom_error_body(405, server_config)
                .unwrap_or_else(|| b"<h1>405 Method Not Allowed</h1>".to_vec());
            return build_response(Response {
                status_code: 405,
                reason_phrase: "Method Not Allowed".to_string(),
                headers,
                body,
            });
        }
        // CGI handler
        if let Some((ext, script)) = &route.cgi {
            if req.path.ends_with(ext) {
                let path_info = &req.path;
                // Construct the full path to the script
                let script_path = format!("{}/{}", route.root, script);
                match run_cgi_script(
                    &script_path,
                    std::str::from_utf8(&req.body).unwrap_or(""),
                    path_info,
                ) {
                    Ok(output) => {
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "text/plain".to_string());
                        if let Some(cookie) = set_cookie_header.clone() {
                            headers.insert("Set-Cookie".to_string(), cookie);
                        }
                        return build_response(Response {
                            status_code: 200,
                            reason_phrase: "OK".to_string(),
                            headers,
                            body: output.into_bytes(),
                        });
                    }
                    Err(_) => {
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "text/html".to_string());
                        if let Some(cookie) = set_cookie_header.clone() {
                            headers.insert("Set-Cookie".to_string(), cookie);
                        }
                        let body = custom_error_body(500, server_config)
                            .unwrap_or_else(|| b"<h1>500 Internal Server Error</h1>".to_vec());
                        return build_response(Response {
                            status_code: 500,
                            reason_phrase: "Internal Server Error".to_string(),
                            headers,
                            body,
                        });
                    }
                }
            }
        }
        // Upload handler
        if req.method == "POST" && route.path == "/upload" {
            println!("DEBUG: Upload handler condition met!");
            let content_type = req
                .headers
                .get("Content-Type")
                .map(|s| s.as_str())
                .unwrap_or("");
            let result = handle_file_upload(&req.body, content_type);
            let mut response = build_upload_response(result);
            if let Some(cookie) = set_cookie_header {
                // Insert Set-Cookie header if needed
                let mut resp_str = String::from_utf8_lossy(&response).to_string();
                let insert_pos = resp_str.find("\r\n\r\n").unwrap_or(resp_str.len());
                resp_str.insert_str(insert_pos, &format!("Set-Cookie: {}\r\n", cookie));
                return resp_str.into_bytes();
            }
            return response;
        } else {
            println!(
                "DEBUG: Upload handler condition NOT met. req.method='{}', route.path='{}'",
                req.method, route.path
            );
        }
        // DELETE handler
        if req.method == "DELETE" {
            // Only allow DELETE for files, not directories
            let rel_path = if req.path == "/" { "" } else { &req.path[1..] };
            let base = std::path::Path::new(&route.root);
            let full_path = base.join(rel_path);
            let full_path = match std::fs::canonicalize(&full_path) {
                Ok(path) => path,
                Err(_) => {
                    let body = custom_error_body(404, server_config)
                        .unwrap_or_else(|| b"<h1>404 Not Found</h1>".to_vec());
                    return build_response(Response {
                        status_code: 404,
                        reason_phrase: "Not Found".to_string(),
                        headers: {
                            let mut h = HashMap::new();
                            h.insert("Content-Type".to_string(), "text/html".to_string());
                            if let Some(cookie) = set_cookie_header.clone() {
                                h.insert("Set-Cookie".to_string(), cookie);
                            }
                            h
                        },
                        body,
                    });
                }
            };
            if !full_path.starts_with(std::fs::canonicalize(base).unwrap()) {
                let body = custom_error_body(403, server_config)
                    .unwrap_or_else(|| b"<h1>403 Forbidden</h1>".to_vec());
                return build_response(Response {
                    status_code: 403,
                    reason_phrase: "Forbidden".to_string(),
                    headers: {
                        let mut h = HashMap::new();
                        h.insert("Content-Type".to_string(), "text/html".to_string());
                        if let Some(cookie) = set_cookie_header.clone() {
                            h.insert("Set-Cookie".to_string(), cookie);
                        }
                        h
                    },
                    body,
                });
            }
            if full_path.is_dir() {
                let body = custom_error_body(403, server_config)
                    .unwrap_or_else(|| b"<h1>403 Forbidden</h1>".to_vec());
                return build_response(Response {
                    status_code: 403,
                    reason_phrase: "Forbidden".to_string(),
                    headers: {
                        let mut h = HashMap::new();
                        h.insert("Content-Type".to_string(), "text/html".to_string());
                        if let Some(cookie) = set_cookie_header.clone() {
                            h.insert("Set-Cookie".to_string(), cookie);
                        }
                        h
                    },
                    body,
                });
            }
            match std::fs::remove_file(&full_path) {
                Ok(_) => {
                    let body = b"<h1>File deleted successfully</h1>".to_vec();
                    return build_response(Response {
                        status_code: 200,
                        reason_phrase: "OK".to_string(),
                        headers: {
                            let mut h = HashMap::new();
                            h.insert("Content-Type".to_string(), "text/html".to_string());
                            if let Some(cookie) = set_cookie_header.clone() {
                                h.insert("Set-Cookie".to_string(), cookie);
                            }
                            h
                        },
                        body,
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
                    let body = custom_error_body(404, server_config)
                        .unwrap_or_else(|| b"<h1>404 Not Found</h1>".to_vec());
                    return build_response(Response {
                        status_code: 404,
                        reason_phrase: "Not Found".to_string(),
                        headers: {
                            let mut h = HashMap::new();
                            h.insert("Content-Type".to_string(), "text/html".to_string());
                            if let Some(cookie) = set_cookie_header.clone() {
                                h.insert("Set-Cookie".to_string(), cookie);
                            }
                            h
                        },
                        body,
                    });
                }
                Err(_) => {
                    let body = custom_error_body(500, server_config)
                        .unwrap_or_else(|| b"<h1>500 Internal Server Error</h1>".to_vec());
                    return build_response(Response {
                        status_code: 500,
                        reason_phrase: "Internal Server Error".to_string(),
                        headers: {
                            let mut h = HashMap::new();
                            h.insert("Content-Type".to_string(), "text/html".to_string());
                            if let Some(cookie) = set_cookie_header.clone() {
                                h.insert("Set-Cookie".to_string(), cookie);
                            }
                            h
                        },
                        body,
                    });
                }
            }
        }
        // Static file handler
        let rel_path = if req.path == "/" { "" } else { &req.path[1..] };
        let file_response = read_static_file_with_listing(
            rel_path,
            &route.root,
            route.index.as_deref(),
            route.directory_listing.unwrap_or(false),
        );
        let mut response = match &file_response {
            FileResponse::Ok(_) | FileResponse::DirectoryListing(_) => {
                build_http_response(file_response)
            }
            FileResponse::NotFound => {
                let body = custom_error_body(404, server_config)
                    .unwrap_or_else(|| b"<h1>404 Not Found</h1>".to_vec());
                build_response(Response {
                    status_code: 404,
                    reason_phrase: "Not Found".to_string(),
                    headers: {
                        let mut h = HashMap::new();
                        h.insert("Content-Type".to_string(), "text/html".to_string());
                        if let Some(cookie) = set_cookie_header.clone() {
                            h.insert("Set-Cookie".to_string(), cookie);
                        }
                        h
                    },
                    body,
                })
            }
            FileResponse::Forbidden => {
                let body = custom_error_body(403, server_config)
                    .unwrap_or_else(|| b"<h1>403 Forbidden</h1>".to_vec());
                build_response(Response {
                    status_code: 403,
                    reason_phrase: "Forbidden".to_string(),
                    headers: {
                        let mut h = HashMap::new();
                        h.insert("Content-Type".to_string(), "text/html".to_string());
                        if let Some(cookie) = set_cookie_header.clone() {
                            h.insert("Set-Cookie".to_string(), cookie);
                        }
                        h
                    },
                    body,
                })
            }
        };
        if let Some(cookie) = set_cookie_header {
            // Insert Set-Cookie header if needed
            let mut resp_str = String::from_utf8_lossy(&response).to_string();
            let insert_pos = resp_str.find("\r\n\r\n").unwrap_or(resp_str.len());
            resp_str.insert_str(insert_pos, &format!("Set-Cookie: {}\r\n", cookie));
            return resp_str.into_bytes();
        }
        return response;
    }
    // No matching route
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "text/html".to_string());
    if let Some(cookie) = set_cookie_header {
        headers.insert("Set-Cookie".to_string(), cookie);
    }
    let body =
        custom_error_body(404, server_config).unwrap_or_else(|| b"<h1>404 Not Found</h1>".to_vec());
    build_response(Response {
        status_code: 404,
        reason_phrase: "Not Found".to_string(),
        headers,
        body,
    })
}

pub fn run_mio_server(
    mut listeners: HashMap<Token, TcpListener>,
    session_manager: &mut SessionManager,
    server_config: &ServerConfig,
) -> std::io::Result<()> {
    let mut poll = Poll::new()?; //this is an event loop to watch socket's 
    let mut events = Events::with_capacity(2048);

    let mut clients: HashMap<Token, Connection> = HashMap::new();
    let mut next_token = listeners.len() + 1;

    // Register all listening sockets
    for (token, listener) in listeners.iter_mut() {
        poll.registry()
            .register(listener, *token, Interest::READABLE)?;
    }

    println!("Starting mio event loop...");
    loop {
        poll.poll(&mut events, Some(Duration::from_millis(10)))?;
        for event in events.iter() {
            let token = event.token();
            if listeners.contains_key(&token) {
                let listener = listeners.get_mut(&token).unwrap();
                loop {
                    match listener.accept() {
                        Ok((mut stream, addr)) => {
                            let client_token = Token(next_token);
                            next_token += 1;
                            poll.registry().register(
                                &mut stream,
                                client_token,
                                Interest::READABLE,
                            )?;
                            clients.insert(
                                client_token,
                                Connection {
                                    stream,
                                    read_buffer: Vec::new(),
                                    write_buffer: Vec::new(),
                                    is_writing: false,
                                    last_active: Instant::now(),
                                },
                            );
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
                continue;
            }
            if let Some(conn) = clients.get_mut(&token) {
                println!("DEBUG: Handling read event for client {:?}", token);
                let mut temp_buf = [0; 10000];
                match conn.stream.read(&mut temp_buf) {
                    Ok(0) => {
                        clients.remove(&token);
                        continue;
                    }
                    Ok(n) => {
                        conn.read_buffer.extend_from_slice(&temp_buf[..n]);
                        conn.last_active = Instant::now();
                        println!(
                            "DEBUG: Received {} bytes from client {:?}, total buffer: {} bytes",
                            n,
                            token,
                            conn.read_buffer.len()
                        );

                        // Debug: Show the first 200 bytes of what we received
                        let debug_len = conn.read_buffer.len().min(200);
                        let debug_data = &conn.read_buffer[..debug_len];
                        println!(
                            "DEBUG: First {} bytes received: {:?}",
                            debug_len,
                            String::from_utf8_lossy(debug_data)
                        );

                        // Try to process the request with whatever data we have
                        if let Some(header_end) =
                            conn.read_buffer.windows(4).position(|w| w == b"\r\n\r\n")
                        {
                            let headers = &conn.read_buffer[..header_end + 4];
                            // Try to parse Content-Length
                            let headers_str = String::from_utf8_lossy(headers);
                            println!("DEBUG: Headers received:\n{}", headers_str);

                            // Check if this is a chunked transfer encoding request
                            let is_chunked = headers_str.lines().any(|line| {
                                line.to_ascii_lowercase()
                                    .starts_with("transfer-encoding: chunked")
                            });

                            let content_length = if is_chunked {
                                0 // For chunked requests, we'll determine length differently
                            } else {
                                headers_str
                                    .lines()
                                    .find(|line| {
                                        line.to_ascii_lowercase().starts_with("content-length:")
                                    })
                                    .and_then(|line| line.split(':').nth(1))
                                    .and_then(|v| v.trim().parse::<usize>().ok())
                                    .unwrap_or(0)
                            };

                            let total_len = if is_chunked {
                                // For chunked requests, we need to find the end of the chunked body
                                // Look for the final "0\r\n\r\n" that marks the end of chunked data
                                if let Some(chunked_end) =
                                    find_chunked_body_end(&conn.read_buffer[header_end + 4..])
                                {
                                    header_end + 4 + chunked_end
                                } else {
                                    // Haven't received the complete chunked body yet
                                    conn.read_buffer.len() + 1 // Force waiting for more data
                                }
                            } else {
                                header_end + 4 + content_length
                            };

                            println!(
                                "DEBUG: Header end: {}, Content-Length: {}, Is chunked: {}, Total needed: {}, Buffer size: {}",
                                header_end,
                                content_length,
                                is_chunked,
                                total_len,
                                conn.read_buffer.len()
                            );

                            // Only process if we have the complete request
                            if conn.read_buffer.len() >= total_len {
                                println!(
                                    "DEBUG: Processing complete request with {} bytes",
                                    total_len
                                );
                                let response = handle_request(
                                    &conn.read_buffer[..total_len],
                                    session_manager,
                                    server_config,
                                );
                            conn.write_buffer = response;
                            conn.is_writing = true;
                                conn.read_buffer.drain(..total_len);
                            poll.registry().reregister(
                                &mut conn.stream,
                                token,
                                Interest::WRITABLE,
                            )?;
                            } else {
                                println!(
                                    "DEBUG: Waiting for more data... Need {} more bytes",
                                    total_len - conn.read_buffer.len()
                                );
                                // Keep listening for more readable events
                                poll.registry().reregister(
                                    &mut conn.stream,
                                    token,
                                    Interest::READABLE,
                                )?;
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => {
                        clients.remove(&token);
                        continue;
                    }
                }
                if event.is_writable() && conn.is_writing {
                    match conn.stream.write(&conn.write_buffer) {
                        Ok(n) => {
                            conn.write_buffer.drain(..n);
                            if conn.write_buffer.is_empty() {
                                conn.is_writing = false;
                                let _ = conn.stream.shutdown(std::net::Shutdown::Both);
                                poll.registry().deregister(&mut conn.stream)?;
                                clients.remove(&token);
                            }
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                        Err(_) => {
                            clients.remove(&token);
                            continue;
                        }
                    }
                }
            }
        }
        // Timeout check: remove clients that have been idle for too long
        let now = Instant::now();
        let timed_out: Vec<Token> = clients
            .iter()
            .filter(|(_, conn)| now.duration_since(conn.last_active) > CLIENT_TIMEOUT)
            .map(|(token, _)| *token)
            .collect();
        for token in timed_out {
            if let Some(mut conn) = clients.remove(&token) {
                println!(
                    "DEBUG: Client {:?} timed out after {} seconds (buffer size: {} bytes)",
                    token,
                    now.duration_since(conn.last_active).as_secs(),
                    conn.read_buffer.len()
                );
                let _ = conn.stream.shutdown(std::net::Shutdown::Both);
                poll.registry().deregister(&mut conn.stream).ok();
            }
        }
    }
}

fn handle_path(path: &str) -> io::Result<String> {
    let mut wd = env::current_dir().unwrap();
    let file_name = format!("{}.html", path);
    let html_path = wd.join("html").join(file_name);
    println!("html path {}", html_path.display());
    fs::read_to_string(html_path)
}

// Helper function to find the end of a chunked transfer encoding body
fn find_chunked_body_end(body: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < body.len() {
        // Find the next CRLF
        let crlf = match body[i..].windows(2).position(|w| w == b"\r\n") {
            Some(pos) => i + pos,
            None => return None, // No CRLF found, incomplete chunk
        };

        let len_str = match std::str::from_utf8(&body[i..crlf]) {
            Ok(s) => s,
            Err(_) => return None, // Invalid UTF-8 in chunk size
        };

        let chunk_size = match usize::from_str_radix(len_str.trim(), 16) {
            Ok(size) => size,
            Err(_) => return None, // Invalid chunk size
        };

        if chunk_size == 0 {
            // Found the final "0\r\n\r\n" chunk
            return Some(i + 5); // Include the final CRLF
        }

        i = crlf + 2; // Skip CRLF
        if i + chunk_size > body.len() {
            return None; // Chunk size exceeds remaining data
        }

        i += chunk_size + 2; // Skip chunk data and trailing CRLF
    }
    None // Haven't found the end yet
}
