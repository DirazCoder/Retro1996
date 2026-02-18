use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use std::error::Error;
use std::fmt;

use native_tls::{TlsConnector, TlsStream};

/// 1996-Era HTTPS Support
/// 
/// Uses native-tls for TLS 1.0+ compatibility while maintaining 1996 authenticity
/// in HTTP protocol (HTTP/1.0, period-authentic headers).
/// 
/// Note: While modern TLS (1.2/1.3) is used for security, the HTTP layer
/// maintains full 1996 authenticity with HTTP/1.0 protocol.
#[derive(Debug)]
pub struct SslConnection {
    stream: TlsStream<TcpStream>,
    domain: String,
    connected: bool,
}

#[derive(Debug, Clone)]
pub enum SslError {
    ConnectionFailed(String),
    HandshakeFailed(String),
    CertificateError(String),
    IoError(String),
    ProtocolError(String),
}

impl fmt::Display for SslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SslError::ConnectionFailed(msg) => write!(f, "SSL connection failed: {}", msg),
            SslError::HandshakeFailed(msg) => write!(f, "SSL handshake failed: {}", msg),
            SslError::CertificateError(msg) => write!(f, "SSL certificate error: {}", msg),
            SslError::IoError(msg) => write!(f, "SSL I/O error: {}", msg),
            SslError::ProtocolError(msg) => write!(f, "SSL protocol error: {}", msg),
        }
    }
}

impl Error for SslError {}

impl SslConnection {
    /// Create a new SSL connection to the specified domain and port
    pub fn new(domain: &str, port: u16, timeout: Duration) -> Result<Self, SslError> {
        // Connect to the server via TCP
        let addr = format!("{}:{}", domain, port);
        let tcp_stream = TcpStream::connect(&addr)
            .map_err(|e| SslError::ConnectionFailed(format!("Failed to connect to {}: {}", addr, e)))?;
        
        tcp_stream.set_read_timeout(Some(timeout))
            .map_err(|e| SslError::IoError(format!("Failed to set read timeout: {}", e)))?;
        tcp_stream.set_write_timeout(Some(timeout))
            .map_err(|e| SslError::IoError(format!("Failed to set write timeout: {}", e)))?;
        
        // Create TLS connector with appropriate settings for 1996 authenticity
        // We use native-tls which will negotiate the best available TLS version
        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(false) // Security: validate certs
            .danger_accept_invalid_hostnames(false) // Security: validate hostnames
            .build()
            .map_err(|e| SslError::HandshakeFailed(format!("Failed to create TLS connector: {}", e)))?;
        
        // Perform TLS handshake
        let tls_stream = connector.connect(domain, tcp_stream)
            .map_err(|e| SslError::HandshakeFailed(format!("TLS handshake failed: {}", e)))?;
        
        Ok(Self {
            stream: tls_stream,
            domain: domain.to_string(),
            connected: true,
        })
    }
    
    /// Send data over SSL connection
    pub fn write(&mut self, data: &[u8]) -> Result<usize, SslError> {
        if !self.connected {
            return Err(SslError::ProtocolError("SSL connection not established".to_string()));
        }
        
        self.stream.write_all(data)
            .map_err(|e| SslError::IoError(format!("Failed to write data: {}", e)))?;
        
        Ok(data.len())
    }
    
    /// Read data from SSL connection
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SslError> {
        if !self.connected {
            return Err(SslError::ProtocolError("SSL connection not established".to_string()));
        }
        
        self.stream.read(buffer)
            .map_err(|e| SslError::IoError(format!("Failed to read data: {}", e)))
    }
    
    /// Close SSL connection
    pub fn close(mut self) -> Result<(), SslError> {
        if self.connected {
            // TLS close notify is handled by Drop
            self.connected = false;
        }
        Ok(())
    }
}

/// HTTPS client for 1996-era HTTPS support
/// 
/// Provides HTTPS support with HTTP/1.0 protocol and period-authentic headers.
pub struct HttpsClient {
    user_agent: String,
    timeout: Duration,
}

impl HttpsClient {
    pub fn new() -> Self {
        Self {
            user_agent: "Mozilla/3.0 (compatible; Retro1996/3.0; Win95; I)".to_string(),
            timeout: Duration::from_secs(30),
        }
    }
    
    pub fn set_user_agent(&mut self, user_agent: String) {
        self.user_agent = user_agent;
    }
    
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
    
    /// Fetch HTTPS URL with SSL support
    pub fn get(&self, url: &str) -> Result<HttpResponse, SslError> {
        // Parse URL
        let (domain, port, path, query) = self.parse_url(url)?;
        
        // Create SSL connection
        let mut ssl_conn = SslConnection::new(&domain, port, self.timeout)?;
        
        // Build HTTP/1.0 request (period-authentic)
        let request_path = if query.is_empty() {
            path.clone()
        } else {
            format!("{}?{}", path, query)
        };
        let request = self.build_request("GET", &domain, &request_path);
        
        // Send request
        ssl_conn.write(request.as_bytes())
            .map_err(|e| SslError::IoError(format!("Failed to send request: {}", e)))?;
        
        // Read full response
        let (header_section, body) = self.read_full_response(&mut ssl_conn)?;
        
        // Parse HTTP response
        self.parse_response(&header_section, &body)
    }
    
    /// POST to HTTPS URL with SSL support
    pub fn post(&self, url: &str, post_body: &[u8], content_type: &str) -> Result<HttpResponse, SslError> {
        // Parse URL
        let (domain, port, path, query) = self.parse_url(url)?;
        
        // Create SSL connection
        let mut ssl_conn = SslConnection::new(&domain, port, self.timeout)?;
        
        // Build HTTP/1.0 request (period-authentic)
        let request_path = if query.is_empty() {
            path.clone()
        } else {
            format!("{}?{}", path, query)
        };
        let request = self.build_post_request(&domain, &request_path, post_body, content_type);
        
        // Send request
        ssl_conn.write(request.as_bytes())
            .map_err(|e| SslError::IoError(format!("Failed to send request: {}", e)))?;
        
        // Read full response
        let (header_section, body) = self.read_full_response(&mut ssl_conn)?;
        
        // Parse HTTP response
        self.parse_response(&header_section, &body)
    }
    
    /// Read full response from SSL connection
    fn read_full_response(&self, ssl_conn: &mut SslConnection) -> Result<(String, Vec<u8>), SslError> {
        let mut response_buffer = Vec::new();
        let mut buffer = [0; 4096];
        
        // Read until we get the header terminator
        loop {
            let bytes_read = ssl_conn.read(&mut buffer)
                .map_err(|e| SslError::IoError(format!("Failed to read response: {}", e)))?;
            if bytes_read == 0 {
                break;
            }
            response_buffer.extend_from_slice(&buffer[..bytes_read]);
            
            if response_buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        
        // Find header/body boundary
        let header_end = response_buffer.windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or_else(|| SslError::ProtocolError("No header terminator found".to_string()))?;
        
        // Extract header section as string
        let header_section = String::from_utf8_lossy(&response_buffer[..header_end]).to_string();
        
        // Get any body data that came with the headers
        let mut body = if response_buffer.len() > header_end + 4 {
            response_buffer[header_end + 4..].to_vec()
        } else {
            Vec::new()
        };
        
        // Continue reading the rest of the body until connection closes
        // (HTTP/1.0 with Connection: close)
        loop {
            let bytes_read = ssl_conn.read(&mut buffer)
                .map_err(|e| SslError::IoError(format!("Failed to read response body: {}", e)))?;
            if bytes_read == 0 {
                break; // Connection closed by server
            }
            body.extend_from_slice(&buffer[..bytes_read]);
        }
        
        Ok((header_section, body))
    }
    
    /// Parse HTTPS URL
    fn parse_url(&self, url: &str) -> Result<(String, u16, String, String), SslError> {
        if !url.starts_with("https://") {
            return Err(SslError::ProtocolError("URL must start with https://".to_string()));
        }
        
        let url_no_scheme = &url[8..]; // Remove "https://"
        
        let (domain_port, path_query) = if let Some(slash_pos) = url_no_scheme.find('/') {
            (&url_no_scheme[..slash_pos], &url_no_scheme[slash_pos..])
        } else {
            (url_no_scheme, "/")
        };
        
        let (path, query) = if let Some(qmark_pos) = path_query.find('?') {
            (&path_query[..qmark_pos], &path_query[qmark_pos + 1..])
        } else {
            (path_query, "")
        };
        
        let (domain, port) = if let Some(colon_pos) = domain_port.find(':') {
            let domain = &domain_port[..colon_pos];
            let port = domain_port[colon_pos + 1..].parse::<u16>()
                .map_err(|_| SslError::ProtocolError("Invalid port number".to_string()))?;
            (domain.to_string(), port)
        } else {
            (domain_port.to_string(), 443) // HTTPS default port
        };
        
        Ok((domain, port, path.to_string(), query.to_string()))
    }
    
    /// Build HTTP/1.0 GET request (period-authentic)
    fn build_request(&self, method: &str, domain: &str, path: &str) -> String {
        format!(
            "{} {} HTTP/1.0\r\n\
             Host: {}\r\n\
             User-Agent: {}\r\n\
             Accept: */*\r\n\
             Connection: close\r\n\
             \r\n",
            method,
            path,
            domain,
            self.user_agent
        )
    }
    
    /// Build HTTP/1.0 POST request (period-authentic)
    fn build_post_request(&self, domain: &str, path: &str, body: &[u8], content_type: &str) -> String {
        let mut request = format!(
            "POST {} HTTP/1.0\r\n\
             Host: {}\r\n\
             User-Agent: {}\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n",
            path,
            domain,
            self.user_agent,
            content_type,
            body.len()
        );
        
        // Append body
        if let Ok(body_str) = std::str::from_utf8(body) {
            request.push_str(body_str);
        }
        
        request
    }
    
    /// Parse HTTP response
    fn parse_response(&self, header_section: &str, body: &[u8]) -> Result<HttpResponse, SslError> {
        let header_lines: Vec<&str> = header_section.lines().collect();
        
        if header_lines.is_empty() {
            return Err(SslError::ProtocolError("No status line".to_string()));
        }
        
        let status_line = header_lines[0];
        let status_parts: Vec<&str> = status_line.split_whitespace().collect();
        
        if status_parts.len() < 2 {
            return Err(SslError::ProtocolError("Invalid status line".to_string()));
        }
        
        let status_code = status_parts[1].parse::<u16>()
            .map_err(|_| SslError::ProtocolError("Invalid status code".to_string()))?;
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
        response.body = body.to_vec();
        
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
        
        Ok(response)
    }
}

impl Default for HttpsClient {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP response structure
#[derive(Debug, Clone)]
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
}

impl Default for HttpResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssl_error_display() {
        let error = SslError::ConnectionFailed("Test error".to_string());
        assert_eq!(error.to_string(), "SSL connection failed: Test error");
    }
    
    #[test]
    fn test_https_client_url_parsing() {
        let client = HttpsClient::new();
        
        // Test basic HTTPS URL
        let (domain, port, path, query) = client.parse_url("https://example.com/path").unwrap();
        assert_eq!(domain, "example.com");
        assert_eq!(port, 443);
        assert_eq!(path, "/path");
        assert_eq!(query, "");
        
        // Test HTTPS URL with port
        let (domain, port, path, query) = client.parse_url("https://example.com:8443/path?foo=bar").unwrap();
        assert_eq!(domain, "example.com");
        assert_eq!(port, 8443);
        assert_eq!(path, "/path");
        assert_eq!(query, "foo=bar");
    }
    
    #[test]
    fn test_https_client_request_building() {
        let client = HttpsClient::new();
        let request = client.build_request("GET", "example.com", "/test");
        
        assert!(request.contains("GET /test HTTP/1.0"));
        assert!(request.contains("Host: example.com"));
        assert!(request.contains("User-Agent: Mozilla/3.0 (compatible; Retro1996/3.0; Win95; I)"));
    }
    
    #[test]
    fn test_http_response() {
        let mut response = HttpResponse::new();
        response.status_code = 200;
        
        assert!(response.is_success());
        assert!(!response.is_redirect());
        assert!(!response.is_client_error());
        assert!(!response.is_server_error());
        
        response.status_code = 404;
        assert!(response.is_client_error());
    }
}