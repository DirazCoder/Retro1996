use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, SocketAddr};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use std::str;
use std::sync::mpsc;
use std::fs;
use std::path::Path;
use std::error::Error;
use std::fmt;
use std::convert::TryInto;
use std::cell::RefCell;

use url::Url;
use chrono::{Utc, TimeZone, NaiveDateTime};
use serde::{Serialize, Deserialize};
use serde_json;

#[cfg(windows)]
use winapi::um::winsock2;
#[cfg(windows)]
use winapi::um::wininet;
#[cfg(windows)]
use winapi::um::wininetapi;
#[cfg(windows)]
use winapi::um::winbase;
#[cfg(windows)]
use winapi::um::winnt;
#[cfg(windows)]
use winapi::um::winerror;
#[cfg(windows)]
use winapi::shared::minwindef::{BOOL, DWORD, LPVOID, TRUE, FALSE};
#[cfg(windows)]
use winapi::shared::winerror::{ERROR_SUCCESS, ERROR_INTERNET_TIMEOUT, ERROR_INTERNET_CONNECTION_RESET};
#[cfg(windows)]
use winapi::shared::ws2def::{AF_INET, SOCK_STREAM, IPPROTO_TCP};
#[cfg(windows)]
use winapi::shared::ws2ipdef::SOCKADDR_IN;

/// Network error types for comprehensive error handling
#[derive(Debug, Clone)]
pub enum NetworkError {
    ConnectionFailed(String),
    Timeout,
    InvalidUrl(String),
    HttpError(u16, String),
    DnsResolutionFailed(String),
    SslError(String),
    IoError(String),
    ParseError(String),
    CacheError(String),
    Win32Error(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            NetworkError::Timeout => write!(f, "Network timeout"),
            NetworkError::InvalidUrl(msg) => write!(f, "Invalid URL: {}", msg),
            NetworkError::HttpError(code, msg) => write!(f, "HTTP error {}: {}", code, msg),
            NetworkError::DnsResolutionFailed(msg) => write!(f, "DNS resolution failed: {}", msg),
            NetworkError::SslError(msg) => write!(f, "SSL error: {}", msg),
            NetworkError::IoError(msg) => write!(f, "I/O error: {}", msg),
            NetworkError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            NetworkError::CacheError(msg) => write!(f, "Cache error: {}", msg),
            NetworkError::Win32Error(msg) => write!(f, "Win32 error: {}", msg),
        }
    }
}

impl Error for NetworkError {}

/// HTTP response structure with comprehensive headers and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub server: Option<String>,
    pub date: Option<String>,
    pub cache_control: Option<String>,
    pub expires: Option<String>,
    pub transfer_encoding: Option<String>,
    pub connection: Option<String>,
}

impl HttpResponse {
    pub fn new() -> Self {
        Self {
            status_code: 0,
            status_text: String::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            content_type: None,
            content_length: None,
            last_modified: None,
            etag: None,
            server: None,
            date: None,
            cache_control: None,
            expires: None,
            transfer_encoding: None,
            connection: None,
        }
    }
    
    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }
    
    pub fn is_redirect(&self) -> bool {
        self.status_code >= 300 && self.status_code < 400
    }
    
    pub fn is_client_error(&self) -> bool {
        self.status_code >= 400 && self.status_code < 500
    }
    
    pub fn is_server_error(&self) -> bool {
        self.status_code >= 500
    }
    
    pub fn get_content_type(&self) -> &str {
        self.content_type.as_deref().unwrap_or("text/html")
    }
    
    pub fn get_charset(&self) -> &str {
        self.content_type
            .as_ref()
            .and_then(|ct| ct.split(';').find_map(|param| {
                if param.trim().starts_with("charset=") {
                    Some(param.trim().split('=').nth(1)?.trim_matches('"'))
                } else {
                    None
                }
            }))
            .unwrap_or("utf-8")
    }
}

/// Cache entry for HTTP caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub url: String,
    pub response: HttpResponse,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// HTTP cache manager with RFC 2616 compliance
pub struct HttpCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    cache_dir: String,
    max_age: Duration,
    max_size: usize,
}

impl HttpCache {
    pub fn new(cache_dir: &str, max_age: Duration, max_size: usize) -> Result<Self, NetworkError> {
        let cache = Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            cache_dir: cache_dir.to_string(),
            max_age,
            max_size,
        };
        
        cache.load_from_disk()?;
        Ok(cache)
    }
    
    pub fn get(&self, url: &str) -> Option<HttpResponse> {
        let entries = self.entries.read().unwrap();
        if let Some(entry) = entries.get(url) {
            if self.is_fresh(entry) {
                return Some(entry.response.clone());
            }
        }
        None
    }
    
    pub fn set(&self, url: String, response: HttpResponse) {
        let mut entries = self.entries.write().unwrap();
        
        // Check cache size limit
        if entries.len() >= self.max_size {
            self.evict_lru(&mut entries);
        }
        
        let created_at = Utc::now().to_rfc3339();
        let expires_at = self.calculate_expires(&response);
        
        let entry = CacheEntry {
            url: url.clone(),
            response: response.clone(),
            created_at,
            expires_at,
            etag: response.etag.clone(),
            last_modified: response.last_modified.clone(),
        };
        
        entries.insert(url, entry);
        self.save_to_disk();
    }
    
    pub fn is_fresh(&self, entry: &CacheEntry) -> bool {
        if let Some(expires) = &entry.expires_at {
            if let Ok(exp_time) = NaiveDateTime::parse_from_str(expires, "%Y-%m-%dT%H:%M:%S%.fZ") {
                let now = Utc::now().naive_utc();
                return now < exp_time;
            }
        }
        
        // Check max-age directive
        if let Some(cache_control) = &entry.response.cache_control {
            if let Some(max_age) = self.parse_max_age(cache_control) {
                let created = NaiveDateTime::parse_from_str(&entry.created_at, "%Y-%m-%dT%H:%M:%S%.fZ").ok()?;
                let now = Utc::now().naive_utc();
                return now.signed_duration_since(created).num_seconds() < max_age as i64;
            }
        }
        
        // Default freshness lifetime
        let created = NaiveDateTime::parse_from_str(&entry.created_at, "%Y-%m-%dT%H:%M:%S%.fZ").ok()?;
        let now = Utc::now().naive_utc();
        now.signed_duration_since(created) < self.max_age
    }
    
    fn calculate_expires(&self, response: &HttpResponse) -> Option<String> {
        if let Some(expires) = &response.expires {
            return Some(expires.clone());
        }
        
        if let Some(cache_control) = &response.cache_control {
            if let Some(max_age) = self.parse_max_age(cache_control) {
                let now = Utc::now();
                return Some((now + chrono::Duration::seconds(max_age as i64)).to_rfc3339());
            }
        }
        
        None
    }
    
    fn parse_max_age(&self, cache_control: &str) -> Option<u32> {
        for directive in cache_control.split(',') {
            let directive = directive.trim();
            if directive.starts_with("max-age=") {
                return directive.split('=').nth(1)?.parse().ok();
            }
        }
        None
    }
    
    fn evict_lru(&self, entries: &mut HashMap<String, CacheEntry>) {
        let mut oldest_url = None;
        let mut oldest_time = chrono::Utc::now();
        
        for (url, entry) in entries.iter() {
            if let Ok(created) = NaiveDateTime::parse_from_str(&entry.created_at, "%Y-%m-%dT%H:%M:%S%.fZ") {
                if created < oldest_time {
                    oldest_time = created;
                    oldest_url = Some(url.clone());
                }
            }
        }
        
        if let Some(url) = oldest_url {
            entries.remove(&url);
        }
    }
    
    fn load_from_disk(&self) -> Result<(), NetworkError> {
        if !Path::new(&self.cache_dir).exists() {
            fs::create_dir_all(&self.cache_dir).map_err(|e| NetworkError::CacheError(format!("Failed to create cache directory: {}", e)))?;
            return Ok(());
        }
        
        let cache_file = format!("{}/cache.json", self.cache_dir);
        if Path::new(&cache_file).exists() {
            let content = fs::read_to_string(&cache_file).map_err(|e| NetworkError::CacheError(format!("Failed to read cache file: {}", e)))?;
            let entries: HashMap<String, CacheEntry> = serde_json::from_str(&content).map_err(|e| NetworkError::CacheError(format!("Failed to parse cache file: {}", e)))?;
            
            let mut cache_entries = self.entries.write().unwrap();
            *cache_entries = entries;
        }
        
        Ok(())
    }
    
    fn save_to_disk(&self) {
        let entries = self.entries.read().unwrap();
        let content = serde_json::to_string_pretty(&*entries).unwrap_or_default();
        let cache_file = format!("{}/cache.json", self.cache_dir);
        
        if let Err(e) = fs::write(&cache_file, content) {
            eprintln!("Failed to save cache to disk: {}", e);
        }
    }
}

/// Connection pool for reusing TCP connections
pub struct ConnectionPool {
    connections: Arc<Mutex<HashMap<String, TcpStream>>>,
    timeouts: Arc<Mutex<HashMap<String, Instant>>>,
    max_idle_time: Duration,
}

impl ConnectionPool {
    pub fn new(max_idle_time: Duration) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            timeouts: Arc::new(Mutex::new(HashMap::new())),
            max_idle_time,
        }
    }
    
    pub fn get_connection(&self, host: &str, port: u16) -> Option<TcpStream> {
        let key = format!("{}:{}", host, port);
        let mut connections = self.connections.lock().unwrap();
        let mut timeouts = self.timeouts.lock().unwrap();
        
        if let Some(timeout) = timeouts.get(&key) {
            if timeout.elapsed() > self.max_idle_time {
                connections.remove(&key);
                timeouts.remove(&key);
            }
        }
        
        connections.remove(&key)
    }
    
    pub fn return_connection(&self, host: &str, port: u16, stream: TcpStream) {
        let key = format!("{}:{}", host, port);
        let mut connections = self.connections.lock().unwrap();
        let mut timeouts = self.timeouts.lock().unwrap();
        
        connections.insert(key.clone(), stream);
        timeouts.insert(key, Instant::now());
    }
    
    pub fn close_idle_connections(&self) {
        let mut connections = self.connections.lock().unwrap();
        let mut timeouts = self.timeouts.lock().unwrap();
        
        let now = Instant::now();
        let keys_to_remove: Vec<String> = timeouts
            .iter()
            .filter(|(_, timeout)| timeout.elapsed() > self.max_idle_time)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in keys_to_remove {
            connections.remove(&key);
            timeouts.remove(&key);
        }
    }
}

/// Windows-optimized socket operations
#[cfg(windows)]
pub struct Win32Socket {
    socket: winapi::shared::winsock2::SOCKET,
    connected: bool,
}

#[cfg(windows)]
impl Win32Socket {
    pub fn new() -> Result<Self, NetworkError> {
        unsafe {
            let wsadata = winapi::um::winsock2::WSADATA {
                wVersion: 2,
                wHighVersion: 2,
                szDescription: [0; 256],
                szSystemStatus: [0; 128],
                iMaxSockets: 0,
                iMaxUdpDg: 0,
                lpVendorInfo: std::ptr::null_mut(),
            };
            
            let result = winapi::um::winsock2::WSAStartup(0x0202, &wsadata as *const _ as *mut _);
            if result != 0 {
                return Err(NetworkError::Win32Error(format!("WSAStartup failed with error: {}", result)));
            }
            
            let socket = winapi::um::winsock2::socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
            if socket == winapi::um::winsock2::INVALID_SOCKET {
                return Err(NetworkError::Win32Error("Failed to create socket".to_string()));
            }
            
            Ok(Self {
                socket,
                connected: false,
            })
        }
    }
    
    pub fn connect(&mut self, addr: &SocketAddr) -> Result<(), NetworkError> {
        unsafe {
            let sockaddr_in = SOCKADDR_IN {
                sin_family: AF_INET as u16,
                sin_port: addr.port().to_be(),
                sin_addr: winapi::shared::inaddr::IN_ADDR {
                    S_un: winapi::shared::inaddr::IN_ADDR_0 {
                        S_addr: u32::from_be_bytes(addr.ip().octets()),
                    },
                },
                sin_zero: [0; 8],
            };
            
            let result = winapi::um::winsock2::connect(
                self.socket,
                &sockaddr_in as *const _ as *const winapi::shared::ws2def::sockaddr,
                std::mem::size_of::<SOCKADDR_IN>() as i32,
            );
            
            if result == 0 {
                self.connected = true;
                Ok(())
            } else {
                Err(NetworkError::Win32Error("Failed to connect".to_string()))
            }
        }
    }
    
    pub fn send(&self, data: &[u8]) -> Result<usize, NetworkError> {
        if !self.connected {
            return Err(NetworkError::Win32Error("Socket not connected".to_string()));
        }
        
        unsafe {
            let result = winapi::um::winsock2::send(
                self.socket,
                data.as_ptr() as *const i8,
                data.len() as i32,
                0,
            );
            
            if result >= 0 {
                Ok(result as usize)
            } else {
                Err(NetworkError::Win32Error("Send failed".to_string()))
            }
        }
    }
    
    pub fn receive(&self, buffer: &mut [u8]) -> Result<usize, NetworkError> {
        if !self.connected {
            return Err(NetworkError::Win32Error("Socket not connected".to_string()));
        }
        
        unsafe {
            let result = winapi::um::winsock2::recv(
                self.socket,
                buffer.as_mut_ptr() as *mut i8,
                buffer.len() as i32,
                0,
            );
            
            if result >= 0 {
                Ok(result as usize)
            } else {
                Err(NetworkError::Win32Error("Receive failed".to_string()))
            }
        }
    }
    
    pub fn close(&mut self) {
        if self.connected {
            unsafe {
                winapi::um::winsock2::closesocket(self.socket);
                winapi::um::winsock2::WSACleanup();
            }
            self.connected = false;
        }
    }
}

#[cfg(not(windows))]
pub struct Win32Socket {
    _phantom: std::marker::PhantomData<()>,
}

#[cfg(not(windows))]
impl Win32Socket {
    pub fn new() -> Result<Self, NetworkError> {
        Ok(Self { _phantom: std::marker::PhantomData })
    }
    
    pub fn connect(&mut self, _addr: &SocketAddr) -> Result<(), NetworkError> {
        Err(NetworkError::Win32Error("Win32Socket only available on Windows".to_string()))
    }
    
    pub fn send(&self, _data: &[u8]) -> Result<usize, NetworkError> {
        Err(NetworkError::Win32Error("Win32Socket only available on Windows".to_string()))
    }
    
    pub fn receive(&self, _buffer: &mut [u8]) -> Result<usize, NetworkError> {
        Err(NetworkError::Win32Error("Win32Socket only available on Windows".to_string()))
    }
    
    pub fn close(&mut self) {}
}

/// HTTP client with connection pooling and caching
pub struct HttpClient {
    user_agent: String,
    timeout: Duration,
    connection_pool: ConnectionPool,
    cache: Option<HttpCache>,
    keep_alive: bool,
    max_redirects: usize,
    dns_cache: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl HttpClient {
    pub fn new() -> Result<Self, NetworkError> {
        let cache = HttpCache::new("./cache", Duration::from_secs(3600), 100).ok();
        
        Ok(Self {
            user_agent: "Retro1996/3.0 (Win95; I)".to_string(),
            timeout: Duration::from_secs(30),
            connection_pool: ConnectionPool::new(Duration::from_secs(60)),
            cache,
            keep_alive: true,
            max_redirects: 5,
            dns_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    pub fn set_user_agent(&mut self, user_agent: String) {
        self.user_agent = user_agent;
    }
    
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
    
    pub fn set_keep_alive(&mut self, keep_alive: bool) {
        self.keep_alive = keep_alive;
    }
    
    pub fn set_max_redirects(&mut self, max_redirects: usize) {
        self.max_redirects = max_redirects;
    }
    
    pub fn get(&self, url: &str) -> Result<HttpResponse, NetworkError> {
        self.request("GET", url, None)
    }
    
    pub fn post(&self, url: &str, body: &[u8], content_type: &str) -> Result<HttpResponse, NetworkError> {
        self.request_with_body("POST", url, body, content_type)
    }
    
    fn request(&self, method: &str, url: &str, body: Option<&[u8]>) -> Result<HttpResponse, NetworkError> {
        let parsed_url = Url::parse(url).map_err(|e| NetworkError::InvalidUrl(format!("Invalid URL: {}", e)))?;
        
        let host = parsed_url.host_str().ok_or_else(|| NetworkError::InvalidUrl("URL has no host".to_string()))?;
        let port = parsed_url.port().unwrap_or(if parsed_url.scheme() == "https" { 443 } else { 80 });
        let path = if parsed_url.path().is_empty() { "/" } else { parsed_url.path() };
        let query = parsed_url.query().unwrap_or("");
        let request_path = if query.is_empty() { path.to_string() } else { format!("{}?{}", path, query) };
        
        // Check cache for GET requests
        if method == "GET" {
            if let Some(cache) = &self.cache {
                if let Some(cached_response) = cache.get(url) {
                    return Ok(cached_response);
                }
            }
        }
        
        let mut response = self.send_request(method, host, port, &request_path, body)?;
        
        // Handle redirects
        let mut redirects = 0;
        while response.is_redirect() && redirects < self.max_redirects {
            if let Some(location) = response.headers.get("location") {
                let base_url = Url::parse(url).map_err(|e| NetworkError::InvalidUrl(format!("Invalid base URL: {}", e)))?;
                let new_url = base_url.join(location).map_err(|e| NetworkError::InvalidUrl(format!("Invalid redirect URL: {}", e)))?;
                response = self.send_request(method, host, port, &request_path, body)?;
                redirects += 1;
            } else {
                break;
            }
        }
        
        // Cache successful responses
        if method == "GET" && response.is_success() {
            if let Some(cache) = &self.cache {
                cache.set(url.to_string(), response.clone());
            }
        }
        
        Ok(response)
    }
    
    fn request_with_body(&self, method: &str, url: &str, body: &[u8], content_type: &str) -> Result<HttpResponse, NetworkError> {
        let parsed_url = Url::parse(url).map_err(|e| NetworkError::InvalidUrl(format!("Invalid URL: {}", e)))?;
        
        let host = parsed_url.host_str().ok_or_else(|| NetworkError::InvalidUrl("URL has no host".to_string()))?;
        let port = parsed_url.port().unwrap_or(if parsed_url.scheme() == "https" { 443 } else { 80 });
        let path = if parsed_url.path().is_empty() { "/" } else { parsed_url.path() };
        let query = parsed_url.query().unwrap_or("");
        let request_path = if query.is_empty() { path.to_string() } else { format!("{}?{}", path, query) };
        
        self.send_request_with_body(method, host, port, &request_path, body, content_type)
    }
    
    fn send_request(&self, method: &str, host: &str, port: u16, path: &str, body: Option<&[u8]>) -> Result<HttpResponse, NetworkError> {
        let mut stream = self.get_or_create_connection(host, port)?;
        
        let request = self.build_request(method, host, port, path, body);
        
        stream.write_all(request.as_bytes()).map_err(|e| NetworkError::IoError(format!("Failed to send request: {}", e)))?;
        
        let mut response_buffer = Vec::new();
        let mut buffer = [0; 1024];
        
        loop {
            let bytes_read = stream.read(&mut buffer).map_err(|e| NetworkError::IoError(format!("Failed to read response: {}", e)))?;
            if bytes_read == 0 {
                break;
            }
            response_buffer.extend_from_slice(&buffer[..bytes_read]);
            
            if response_buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        
        let response = self.parse_response(&response_buffer)?;
        
        if self.keep_alive && response.connection.as_deref() != Some("close") {
            self.connection_pool.return_connection(host, port, stream);
        }
        
        Ok(response)
    }
    
    fn send_request_with_body(&self, method: &str, host: &str, port: u16, path: &str, body: &[u8], content_type: &str) -> Result<HttpResponse, NetworkError> {
        let mut stream = self.get_or_create_connection(host, port)?;
        
        let request = self.build_request_with_body(method, host, port, path, body, content_type);
        
        stream.write_all(request.as_bytes()).map_err(|e| NetworkError::IoError(format!("Failed to send request: {}", e)))?;
        
        let mut response_buffer = Vec::new();
        let mut buffer = [0; 1024];
        
        loop {
            let bytes_read = stream.read(&mut buffer).map_err(|e| NetworkError::IoError(format!("Failed to read response: {}", e)))?;
            if bytes_read == 0 {
                break;
            }
            response_buffer.extend_from_slice(&buffer[..bytes_read]);
            
            if response_buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        
        let response = self.parse_response(&response_buffer)?;
        
        if self.keep_alive && response.connection.as_deref() != Some("close") {
            self.connection_pool.return_connection(host, port, stream);
        }
        
        Ok(response)
    }
    
    fn get_or_create_connection(&self, host: &str, port: u16) -> Result<TcpStream, NetworkError> {
        if let Some(stream) = self.connection_pool.get_connection(host, port) {
            return Ok(stream);
        }
        
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(&addr).map_err(|e| NetworkError::ConnectionFailed(format!("Failed to connect to {}: {}", addr, e)))?;
        stream.set_read_timeout(Some(self.timeout)).map_err(|e| NetworkError::IoError(format!("Failed to set read timeout: {}", e)))?;
        stream.set_write_timeout(Some(self.timeout)).map_err(|e| NetworkError::IoError(format!("Failed to set write timeout: {}", e)))?;
        
        Ok(stream)
    }
    
    fn build_request(&self, method: &str, host: &str, port: u16, path: &str, body: Option<&[u8]>) -> String {
        let mut request = format!(
            "{} {} HTTP/1.1\r\n\
             Host: {}\r\n\
             User-Agent: {}\r\n\
             Accept: */*\r\n\
             Connection: {}\r\n",
            method,
            path,
            host,
            self.user_agent,
            if self.keep_alive { "keep-alive" } else { "close" }
        );
        
        if let Some(body) = body {
            request.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        
        request.push_str("\r\n");
        
        if let Some(body) = body {
            request.push_str(&String::from_utf8_lossy(body));
        }
        
        request
    }
    
    fn build_request_with_body(&self, method: &str, host: &str, port: u16, path: &str, body: &[u8], content_type: &str) -> String {
        format!(
            "{} {} HTTP/1.1\r\n\
             Host: {}\r\n\
             User-Agent: {}\r\n\
             Accept: */*\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Connection: {}\r\n\
             \r\n\
             {}",
            method,
            path,
            host,
            self.user_agent,
            content_type,
            body.len(),
            if self.keep_alive { "keep-alive" } else { "close" },
            String::from_utf8_lossy(body)
        )
    }
    
    fn parse_response(&self, response_data: &[u8]) -> Result<HttpResponse, NetworkError> {
        let response_str = String::from_utf8_lossy(response_data);
        let parts: Vec<&str> = response_str.split("\r\n\r\n").collect();
        
        if parts.is_empty() {
            return Err(NetworkError::ParseError("Empty response".to_string()));
        }
        
        let header_section = parts[0];
        let body = if parts.len() > 1 { parts[1].as_bytes().to_vec() } else { Vec::new() };
        
        let header_lines: Vec<&str> = header_section.lines().collect();
        if header_lines.is_empty() {
            return Err(NetworkError::ParseError("No status line".to_string()));
        }
        
        let status_line = header_lines[0];
        let status_parts: Vec<&str> = status_line.split_whitespace().collect();
        
        if status_parts.len() < 2 {
            return Err(NetworkError::ParseError("Invalid status line".to_string()));
        }
        
        let status_code = status_parts[1].parse::<u16>().map_err(|_| NetworkError::ParseError("Invalid status code".to_string()))?;
        let status_text = status_parts.get(2..).unwrap_or(&[]).join(" ");
        
        let mut headers = HashMap::new();
        for line in header_lines.iter().skip(1) {
            if let Some(pos) = line.find(':') {
                let key = line[..pos].trim().to_lowercase();
                let value = line[pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }
        
        let mut response = HttpResponse::new();
        response.status_code = status_code;
        response.status_text = status_text;
        response.headers = headers;
        response.body = body;
        
        // Extract specific headers
        response.content_type = response.headers.get("content-type").cloned();
        response.content_length = response.headers.get("content-length").and_then(|s| s.parse().ok());
        response.last_modified = response.headers.get("last-modified").cloned();
        response.etag = response.headers.get("etag").cloned();
        response.server = response.headers.get("server").cloned();
        response.date = response.headers.get("date").cloned();
        response.cache_control = response.headers.get("cache-control").cloned();
        response.expires = response.headers.get("expires").cloned();
        response.transfer_encoding = response.headers.get("transfer-encoding").cloned();
        response.connection = response.headers.get("connection").cloned();
        
        if status_code >= 400 {
            return Err(NetworkError::HttpError(status_code, response.status_text));
        }
        
        Ok(response)
    }
}

/// FTP client implementation
pub struct FtpClient {
    user_agent: String,
    timeout: Duration,
}

impl FtpClient {
    pub fn new() -> Self {
        Self {
            user_agent: "Retro1996/3.0 (Win95; I)".to_string(),
            timeout: Duration::from_secs(30),
        }
    }
    
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
    
    pub fn get(&self, url: &str) -> Result<HttpResponse, NetworkError> {
        let parsed_url = Url::parse(url).map_err(|e| NetworkError::InvalidUrl(format!("Invalid URL: {}", e)))?;
        
        let host = parsed_url.host_str().ok_or_else(|| NetworkError::InvalidUrl("URL has no host".to_string()))?;
        let port = parsed_url.port().unwrap_or(21);
        let path = parsed_url.path();
        
        let mut stream = TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| NetworkError::ConnectionFailed(format!("Failed to connect to {}: {}", host, e)))?;
        
        stream.set_read_timeout(Some(self.timeout))
            .map_err(|e| NetworkError::IoError(format!("Failed to set read timeout: {}", e)))?;
        stream.set_write_timeout(Some(self.timeout))
            .map_err(|e| NetworkError::IoError(format!("Failed to set write timeout: {}", e)))?;
        
        // Read welcome message
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).map_err(|e| NetworkError::IoError(format!("Failed to read welcome message: {}", e)))?;
        
        // Send USER command
        let user_cmd = format!("USER anonymous\r\n");
        stream.write_all(user_cmd.as_bytes())
            .map_err(|e| NetworkError::IoError(format!("Failed to send USER command: {}", e)))?;
        
        // Read response
        stream.read(&mut buffer).map_err(|e| NetworkError::IoError(format!("Failed to read USER response: {}", e)))?;
        
        // Send PASS command
        let pass_cmd = format!("PASS anonymous@retro1996.com\r\n");
        stream.write_all(pass_cmd.as_bytes())
            .map_err(|e| NetworkError::IoError(format!("Failed to send PASS command: {}", e)))?;
        
        // Read response
        stream.read(&mut buffer).map_err(|e| NetworkError::IoError(format!("Failed to read PASS response: {}", e)))?;
        
        // Send TYPE command (ASCII)
        let type_cmd = format!("TYPE A\r\n");
        stream.write_all(type_cmd.as_bytes())
            .map_err(|e| NetworkError::IoError(format!("Failed to send TYPE command: {}", e)))?;
        
        // Read response
        stream.read(&mut buffer).map_err(|e| NetworkError::IoError(format!("Failed to read TYPE response: {}", e)))?;
        
        // Send PASV command
        let pasv_cmd = format!("PASV\r\n");
        stream.write_all(pasv_cmd.as_bytes())
            .map_err(|e| NetworkError::IoError(format!("Failed to send PASV command: {}", e)))?;
        
        // Read PASV response to get data port
        let mut pasv_response = [0; 1024];
        let bytes_read = stream.read(&mut pasv_response)
            .map_err(|e| NetworkError::IoError(format!("Failed to read PASV response: {}", e)))?;
        
        let pasv_str = String::from_utf8_lossy(&pasv_response[..bytes_read]);
        let data_port = self.parse_pasv_response(&pasv_str)?;
        
        // Connect to data port
        let data_stream = TcpStream::connect(format!("{}:{}", host, data_port))
            .map_err(|e| NetworkError::ConnectionFailed(format!("Failed to connect to data port: {}", e)))?;
        
        // Send RETR command
        let retr_cmd = format!("RETR {}\r\n", path);
        stream.write_all(retr_cmd.as_bytes())
            .map_err(|e| NetworkError::IoError(format!("Failed to send RETR command: {}", e)))?;
        
        // Read response
        stream.read(&mut buffer).map_err(|e| NetworkError::IoError(format!("Failed to read RETR response: {}", e)))?;
        
        // Read data
        let mut data = Vec::new();
        let mut data_buffer = [0; 1024];
        loop {
            match data_stream.read(&mut data_buffer) {
                Ok(0) => break,
                Ok(n) => data.extend_from_slice(&data_buffer[..n]),
                Err(e) => return Err(NetworkError::IoError(format!("Failed to read data: {}", e))),
            }
        }
        
        // Send QUIT command
        let quit_cmd = format!("QUIT\r\n");
        stream.write_all(quit_cmd.as_bytes())
            .map_err(|e| NetworkError::IoError(format!("Failed to send QUIT command: {}", e)))?;
        
        let mut response = HttpResponse::new();
        response.status_code = 200;
        response.status_text = "OK".to_string();
        response.body = data;
        response.content_type = Some("text/plain".to_string());
        
        Ok(response)
    }
    
    fn parse_pasv_response(&self, response: &str) -> Result<u16, NetworkError> {
        // Extract port numbers from PASV response
        // Format: 227 Entering Passive Mode (h1,h2,h3,h4,p1,p2)
        if let Some(start) = response.find('(') {
            if let Some(end) = response.find(')') {
                let params = &response[start + 1..end];
                let parts: Vec<&str> = params.split(',').collect();
                if parts.len() == 6 {
                    let p1 = parts[4].parse::<u8>().map_err(|_| NetworkError::ParseError("Invalid PASV response".to_string()))?;
                    let p2 = parts[5].parse::<u8>().map_err(|_| NetworkError::ParseError("Invalid PASV response".to_string()))?;
                    return Ok((p1 as u16) * 256 + p2 as u16);
                }
            }
        }
        Err(NetworkError::ParseError("Invalid PASV response".to_string()))
    }
}

/// Gopher client implementation
pub struct GopherClient {
    user_agent: String,
    timeout: Duration,
}

impl GopherClient {
    pub fn new() -> Self {
        Self {
            user_agent: "Retro1996/3.0 (Win95; I)".to_string(),
            timeout: Duration::from_secs(30),
        }
    }
    
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
    
    pub fn get(&self, url: &str) -> Result<HttpResponse, NetworkError> {
        let parsed_url = Url::parse(url).map_err(|e| NetworkError::InvalidUrl(format!("Invalid URL: {}", e)))?;
        
        let host = parsed_url.host_str().ok_or_else(|| NetworkError::InvalidUrl("URL has no host".to_string()))?;
        let port = parsed_url.port().unwrap_or(70);
        let path = parsed_url.path();
        
        let mut stream = TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| NetworkError::ConnectionFailed(format!("Failed to connect to {}: {}", host, e)))?;
        
        stream.set_read_timeout(Some(self.timeout))
            .map_err(|e| NetworkError::IoError(format!("Failed to set read timeout: {}", e)))?;
        stream.set_write_timeout(Some(self.timeout))
            .map_err(|e| NetworkError::IoError(format!("Failed to set write timeout: {}", e)))?;
        
        // Send Gopher request
        let request = format!("{}\r\n", path.trim_start_matches('/'));
        stream.write_all(request.as_bytes())
            .map_err(|e| NetworkError::IoError(format!("Failed to send request: {}", e)))?;
        
        // Read response
        let mut response_data = Vec::new();
        let mut buffer = [0; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => response_data.extend_from_slice(&buffer[..n]),
                Err(e) => return Err(NetworkError::IoError(format!("Failed to read response: {}", e))),
            }
        }
        
        let mut response = HttpResponse::new();
        response.status_code = 200;
        response.status_text = "OK".to_string();
        response.body = response_data;
        response.content_type = Some("text/plain".to_string());
        
        Ok(response)
    }
}

/// Network manager that coordinates all protocol clients
pub struct NetworkManager {
    http_client: HttpClient,
    ftp_client: FtpClient,
    gopher_client: GopherClient,
    dns_cache: Arc<RwLock<HashMap<String, Vec<String>>>>,
    connection_pool: ConnectionPool,
    cache: Option<HttpCache>,
}

impl NetworkManager {
    pub fn new() -> Result<Self, NetworkError> {
        let http_client = HttpClient::new()?;
        let ftp_client = FtpClient::new();
        let gopher_client = GopherClient::new();
        let dns_cache = Arc::new(RwLock::new(HashMap::new()));
        let connection_pool = ConnectionPool::new(Duration::from_secs(60));
        let cache = HttpCache::new("./cache", Duration::from_secs(3600), 100).ok();
        
        Ok(Self {
            http_client,
            ftp_client,
            gopher_client,
            dns_cache,
            connection_pool,
            cache,
        })
    }
    
    pub fn fetch(&mut self, url: &str) -> Result<HttpResponse, NetworkError> {
        let parsed_url = Url::parse(url).map_err(|e| NetworkError::InvalidUrl(format!("Invalid URL: {}", e)))?;
        
        match parsed_url.scheme() {
            "http" | "https" => self.http_client.get(url),
            "ftp" => self.ftp_client.get(url),
            "gopher" => self.gopher_client.get(url),
            "file" => self.handle_file_url(&parsed_url),
            _ => Err(NetworkError::InvalidUrl(format!("Unsupported protocol: {}", parsed_url.scheme()))),
        }
    }
    
    pub fn fetch_async(&self, url: String, callback: Box<dyn FnOnce(Result<HttpResponse, NetworkError>) + Send>) {
        let http_client = self.http_client.clone();
        thread::spawn(move || {
            let result = http_client.get(&url);
            callback(result);
        });
    }
    
    pub fn fetch_image(&self, url: String, callback: Box<dyn FnOnce(Result<HttpResponse, NetworkError>) + Send>) {
        self.fetch_async(url, callback);
    }
    
    fn handle_file_url(&self, url: &Url) -> Result<HttpResponse, NetworkError> {
        let path = url.to_file_path().map_err(|_| NetworkError::InvalidUrl("Invalid file path".to_string()))?;
        
        let content = fs::read(&path).map_err(|e| NetworkError::IoError(format!("Failed to read file: {}", e)))?;
        
        let mut response = HttpResponse::new();
        response.status_code = 200;
        response.status_text = "OK".to_string();
        response.body = content;
        
        // Set content type based on file extension
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            let content_type = match extension.to_lowercase().as_str() {
                "html" | "htm" => "text/html",
                "txt" => "text/plain",
                "css" => "text/css",
                "js" => "application/javascript",
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "bmp" => "image/bmp",
                _ => "application/octet-stream",
            };
            response.content_type = Some(content_type.to_string());
        }
        
        Ok(response)
    }
    
    pub fn set_user_agent(&mut self, user_agent: String) {
        self.http_client.set_user_agent(user_agent);
    }
    
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.http_client.set_timeout(timeout);
        self.ftp_client.set_timeout(timeout);
        self.gopher_client.set_timeout(timeout);
    }
    
    pub fn set_max_redirects(&mut self, max_redirects: usize) {
        self.http_client.set_max_redirects(max_redirects);
    }
    
    pub fn clear_cache(&mut self) {
        if let Some(cache) = &self.cache {
            let mut entries = cache.entries.write().unwrap();
            entries.clear();
            cache.save_to_disk();
        }
    }
    
    pub fn get_cache_stats(&self) -> Option<(usize, usize)> {
        self.cache.as_ref().map(|cache| {
            let entries = cache.entries.read().unwrap();
            (entries.len(), entries.values().map(|e| e.response.body.len()).sum())
        })
    }
    
    pub fn close_idle_connections(&self) {
        self.connection_pool.close_idle_connections();
    }
}

impl Clone for NetworkManager {
    fn clone(&self) -> Self {
        Self {
            http_client: self.http_client.clone(),
            ftp_client: self.ftp_client.clone(),
            gopher_client: self.gopher_client.clone(),
            dns_cache: self.dns_cache.clone(),
            connection_pool: self.connection_pool.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl Clone for HttpClient {
    fn clone(&self) -> Self {
        Self {
            user_agent: self.user_agent.clone(),
            timeout: self.timeout,
            connection_pool: self.connection_pool.clone(),
            cache: self.cache.clone(),
            keep_alive: self.keep_alive,
            max_redirects: self.max_redirects,
            dns_cache: self.dns_cache.clone(),
        }
    }
}

impl Clone for HttpCache {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            cache_dir: self.cache_dir.clone(),
            max_age: self.max_age,
            max_size: self.max_size,
        }
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            timeouts: self.timeouts.clone(),
            max_idle_time: self.max_idle_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_error_display() {
        let error = NetworkError::ConnectionFailed("Test error".to_string());
        assert_eq!(error.to_string(), "Connection failed: Test error");
    }
    
    #[test]
    fn test_http_response_status() {
        let mut response = HttpResponse::new();
        response.status_code = 200;
        assert!(response.is_success());
        assert!(!response.is_redirect());
        assert!(!response.is_client_error());
        assert!(!response.is_server_error());
    }
    
    #[test]
    fn test_cache_entry_creation() {
        let response = HttpResponse::new();
        let entry = CacheEntry {
            url: "http://example.com".to_string(),
            response,
            created_at: "2023-01-01T00:00:00Z".to_string(),
            expires_at: None,
            etag: None,
            last_modified: None,
        };
        
        assert_eq!(entry.url, "http://example.com");
    }
}