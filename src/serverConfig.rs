use mio::net::TcpStream;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, PartialEq, Clone, serde::Deserialize)]
pub struct ServerConfig {
    pub server_name: String,
    pub server_address: Vec<ServerAddress>, //ip and Port
    pub max_body_size: usize,               // in bytes
    pub router: Vec<RouterConfig>,
    pub error_msg: HashMap<u16, String>, // status code and page path
}
#[derive(Debug, PartialEq, Clone, serde::Deserialize)]
pub struct RouterConfig {
    pub path: String,
    pub methods: Vec<String>, // GET, POST, etc.
    pub root: String,
    pub index: Option<String>, // default file to serve
    pub cgi: Option<(String, String)>,
    pub directory_listing: Option<bool>, // enable/disable directory listing for this route
    pub redirection: Option<RedirectionConfig>, // optional redirection
}

#[derive(Debug, PartialEq, Clone, serde::Deserialize)]
pub struct RedirectionConfig {
    pub target: String,
    pub status: Option<u16>, // 301 or 302, default to 302
}

#[derive(Debug, PartialEq, Clone, serde::Deserialize)]
pub struct ServerAddress {
    pub ip: String,
    pub port: u16,
}
pub struct Connection {
    pub stream: TcpStream,
    pub read_buffer: Vec<u8>,
    pub write_buffer: Vec<u8>,
    pub is_writing: bool,
    pub last_active: Instant,
}
