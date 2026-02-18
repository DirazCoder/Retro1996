use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use std::str;
use url::Url;
use super::tcp_client::TcpClient;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub struct HttpClient {
    tcp_client: TcpClient,
    user_agent: String,
    timeout: Duration,
    keep_alive: bool,
}

impl HttpClient {
    pub fn new() -> Self {
        HttpClient {
            tcp_client: TcpClient::new(Duration::from_secs(30)),
            user_agent: "Mozilla/3.0 (compatible; Retro1996/3.0; Win95; I)".to_string(),
            timeout: Duration::from_secs(30),
            keep_alive: false,
        }
    }

    pub fn get(&mut self, url: &str) -> Result<HttpResponse, String> {
        let parsed_url = Url::parse(url)
            .map_err(|e| format!("Invalid URL: {}", e))?;
        
        let host = parsed_url.host_str()
            .ok_or("URL has no host".to_string())?
            .to_string();
        
        let port = parsed_url.port().unwrap_or(if parsed_url.scheme() == "https" { 443 } else { 80 });
        
        let path = if parsed_url.path().is_empty() {
            "/"
        } else {
            parsed_url.path()
        };
        
        let query = parsed_url.query().unwrap_or("");
        let request_path = if query.is_empty() {
            path.to_string()
        } else {
            format!("{}?{}", path, query)
        };

        let mut request = format!(
            "GET {} HTTP/1.0\r\n\
             Host: {}\r\n\
             User-Agent: {}\r\n\
             Accept: */*\r\n\
             Connection: close\r\n\
             \r\n",
            request_path, host, self.user_agent
        );

        if !self.tcp_client.is_connected() {
            let addr = format!("{}:{}", host, port);
            self.tcp_client.connect(&addr)
                .map_err(|e| format!("Connection failed: {}", e))?;
        }

        self.tcp_client.send(request.as_bytes())
            .map_err(|e| format!("Send failed: {}", e))?;

        // Read the full response - headers first, then body
        let mut response_buffer = Vec::new();
        let mut buffer = [0; 4096];
        
        // Read until we get the header terminator
        loop {
            let bytes_read = self.tcp_client.receive(&mut buffer)
                .map_err(|e| format!("Receive failed: {}", e))?;
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
            .ok_or("No header terminator found".to_string())?;
        
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
            let bytes_read = self.tcp_client.receive(&mut buffer)
                .map_err(|e| format!("Receive failed: {}", e))?;
            if bytes_read == 0 {
                break; // Connection closed by server
            }
            body.extend_from_slice(&buffer[..bytes_read]);
        }

        let header_lines: Vec<&str> = header_section.lines().collect();
        let status_line = header_lines[0];
        
        let status_parts: Vec<&str> = status_line.split_whitespace().collect();
        let status_code = status_parts.get(1)
            .ok_or("No status code in response".to_string())?
            .parse::<u16>()
            .map_err(|_| "Invalid status code".to_string())?;
        
        let status_text = status_parts.get(2..).unwrap_or(&[]).join(" ");

        let mut headers = HashMap::new();
        for line in header_lines.iter().skip(1) {
            if let Some(pos) = line.find(':') {
                let key = line[..pos].trim().to_lowercase();
                let value = line[pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        Ok(HttpResponse {
            status_code,
            status_text,
            headers,
            body,
        })
    }

    pub fn post(&mut self, url: &str, body: &[u8], content_type: &str) -> Result<HttpResponse, String> {
        let parsed_url = Url::parse(url)
            .map_err(|e| format!("Invalid URL: {}", e))?;
        
        let host = parsed_url.host_str()
            .ok_or("URL has no host".to_string())?
            .to_string();
        
        let port = parsed_url.port().unwrap_or(if parsed_url.scheme() == "https" { 443 } else { 80 });
        
        let path = if parsed_url.path().is_empty() {
            "/"
        } else {
            parsed_url.path()
        };
        
        let query = parsed_url.query().unwrap_or("");
        let request_path = if query.is_empty() {
            path.to_string()
        } else {
            format!("{}?{}", path, query)
        };

        let mut request = format!(
            "POST {} HTTP/1.0\r\n\
             Host: {}\r\n\
             User-Agent: {}\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n",
            request_path, host, self.user_agent, content_type, body.len()
        );
        request.push_str(str::from_utf8(body).unwrap_or(""));

        if !self.tcp_client.is_connected() {
            let addr = format!("{}:{}", host, port);
            self.tcp_client.connect(&addr)
                .map_err(|e| format!("Connection failed: {}", e))?;
        }

        self.tcp_client.send(request.as_bytes())
            .map_err(|e| format!("Send failed: {}", e))?;

        // Read the full response - headers first, then body
        let mut response_buffer = Vec::new();
        let mut buffer = [0; 4096];
        
        // Read until we get the header terminator
        loop {
            let bytes_read = self.tcp_client.receive(&mut buffer)
                .map_err(|e| format!("Receive failed: {}", e))?;
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
            .ok_or("No header terminator found".to_string())?;
        
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
            let bytes_read = self.tcp_client.receive(&mut buffer)
                .map_err(|e| format!("Receive failed: {}", e))?;
            if bytes_read == 0 {
                break; // Connection closed by server
            }
            body.extend_from_slice(&buffer[..bytes_read]);
        }

        let header_lines: Vec<&str> = header_section.lines().collect();
        let status_line = header_lines[0];
        
        let status_parts: Vec<&str> = status_line.split_whitespace().collect();
        let status_code = status_parts.get(1)
            .ok_or("No status code in response".to_string())?
            .parse::<u16>()
            .map_err(|_| "Invalid status code".to_string())?;
        
        let status_text = status_parts.get(2..).unwrap_or(&[]).join(" ");

        let mut headers = HashMap::new();
        for line in header_lines.iter().skip(1) {
            if let Some(pos) = line.find(':') {
                let key = line[..pos].trim().to_lowercase();
                let value = line[pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        Ok(HttpResponse {
            status_code,
            status_text,
            headers,
            body,
        })
    }

    pub fn set_user_agent(&mut self, user_agent: String) {
        self.user_agent = user_agent;
    }

    pub fn follow_redirects(&mut self, url: &str, max_redirects: usize) -> Result<HttpResponse, String> {
        let mut current_url = url.to_string();
        let mut redirects = 0;
        loop {
            let response = self.get(&current_url)?;
            if response.status_code >= 300 && response.status_code < 400 {
                if redirects >= max_redirects {
                    return Err(format!("Too many redirects (> {})", max_redirects));
                }
                if let Some(location) = response.headers.get("location") {
                    let base_url = Url::parse(&current_url)
                        .map_err(|e| format!("Invalid base URL: {}", e))?;
                    let new_url = base_url.join(location)
                        .map_err(|e| format!("Invalid redirect URL: {}", e))?;
                    current_url = new_url.to_string();
                    redirects += 1;
                } else {
                    return Ok(response);
                }
            } else {
                return Ok(response);
            }
        }
    }

    pub fn set_keep_alive(&mut self, keep_alive: bool) {
        self.keep_alive = keep_alive;
    }

    pub fn disconnect(&mut self) -> Result<(), String> {
        self.tcp_client.disconnect()
            .map_err(|e| format!("Disconnect failed: {}", e))
    }

    pub fn is_connected(&self) -> bool {
        self.tcp_client.is_connected()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_creation() {
        let client = HttpClient::new();
        assert_eq!(client.user_agent, "Mozilla/3.0 (compatible; Retro1996/3.0; Win95; I)");
        assert!(!client.keep_alive);
    }
}