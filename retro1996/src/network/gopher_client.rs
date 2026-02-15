use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::time::Duration;
use url::Url;
use thiserror::Error;
use crate::network::tcp_client::TcpClient;

#[derive(Error, Debug)]
pub enum GopherError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TCP client error: {0}")]
    TcpClient(String),
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Timeout")]
    Timeout,
    #[error("Not connected")]
    NotConnected,
}

#[derive(Debug, Clone)]
pub struct GopherItem {
    pub item_type: GopherItemType,
    pub name: String,
    pub selector: String,
    pub host: String,
    pub port: u16,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum GopherItemType {
    File,
    Directory,
    PhoneServer,
    Error,
    BinHex,
    Dosexe,
    UUEncoded,
    Search,
    Telnet,
    Binary,
    Mirror,
    GIF,
    HTML,
    Info,
    WaisDoc,
    WaisSrc,
}

#[derive(Debug, Clone)]
pub struct GopherResponse {
    pub items: Vec<GopherItem>,
    pub raw_data: Vec<u8>,
}

pub struct GopherClient {
    tcp_client: TcpClient,
    host: String,
    port: u16,
    timeout: Duration,
    connected: bool,
}

impl GopherClient {
    pub fn new() -> Self {
        GopherClient {
            tcp_client: TcpClient::new(Duration::from_secs(30)),
            host: String::new(),
            port: 70,
            timeout: Duration::from_secs(30),
            connected: false,
        }
    }

    pub fn connect(&mut self, host: &str, port: Option<u16>) -> Result<(), GopherError> {
        let port = port.unwrap_or(70);
        let addr = format!("{}:{}", host, port);
        self.tcp_client.connect(&addr).map_err(GopherError::TcpClient)?;

        self.host = host.to_string();
        self.port = port;
        self.connected = true;
        Ok(())
    }

    pub fn connect_url(&mut self, url: &str) -> Result<(), GopherError> {
        let parsed = Url::parse(url).map_err(|e| GopherError::Connection(format!("Invalid URL: {}", e)))?;
        if parsed.scheme() != "gopher" {
            return Err(GopherError::Connection("Scheme must be 'gopher'".to_string()));
        }
        let host = parsed.host_str().ok_or_else(|| GopherError::Connection("No host in URL".to_string()))?.to_string();
        let port = parsed.port().unwrap_or(70);
        self.connect(&host, Some(port))
    }

    pub fn get_menu(&mut self, selector: &str) -> Result<GopherResponse, GopherError> {
        if !self.connected {
            return Err(GopherError::NotConnected);
        }

        let request = format!("{}\r\n", selector);
        self.tcp_client.send(request.as_bytes()).map_err(GopherError::TcpClient)?;

        let mut items = Vec::new();
        let mut raw_data = Vec::new();

        loop {
            let mut buffer = [0; 1024];
            let bytes_read = self.tcp_client.receive(&mut buffer).map_err(GopherError::TcpClient)?;
            if bytes_read == 0 {
                break;
            }
            raw_data.extend_from_slice(&buffer[..bytes_read]);

            let response_str = String::from_utf8_lossy(&raw_data);
            let lines: Vec<&str> = response_str.lines().collect();
            raw_data.clear(); 

            for line in lines.iter().take(lines.len().saturating_sub(1)) {
                if *line == "." {
                    return Ok(GopherResponse { items, raw_data: Vec::new() }); 
                }
                if let Some(item) = self.parse_gopher_line(line) {
                    items.push(item);
                }
            }

            if let Some(last_line) = lines.last() {
                raw_data.extend_from_slice(last_line.as_bytes());
            }
        }

        Ok(GopherResponse { items, raw_data })
    }

    pub fn retrieve_document(&mut self, selector: &str) -> Result<Vec<u8>, GopherError> {
        if !self.connected {
            return Err(GopherError::NotConnected);
        }

        let request = format!("{}\r\n", selector);
        self.tcp_client.send(request.as_bytes()).map_err(GopherError::TcpClient)?;

        let mut document = Vec::new();
        let mut buffer = [0; 1024];
        loop {
            let bytes_read = self.tcp_client.receive(&mut buffer).map_err(GopherError::TcpClient)?;
            if bytes_read == 0 {
                break;
            }
            document.extend_from_slice(&buffer[..bytes_read]);

            let len = document.len();
            if len >= 3 && &document[len-3..] == b".\r\n" {
                document.truncate(len - 3);
                break;
            }
        }

        Ok(document)
    }

    pub fn search(&mut self, query: &str) -> Result<Vec<GopherItem>, GopherError> {
        let search_selector = format!("7{}\r\n", query);
        let response = self.get_menu(&search_selector)?;
        Ok(response.items)
    }

    pub fn get_item_details(&mut self, item: &GopherItem) -> Result<Vec<u8>, GopherError> {
        match item.item_type {
            GopherItemType::Directory => {
                let response = self.get_menu(&item.selector)?;
                let mut details = Vec::new();
                for sub_item in response.items {
                    details.extend_from_slice(sub_item.name.as_bytes());
                    details.extend_from_slice(b"\n");
                }
                Ok(details)
            }
            _ => {
                self.retrieve_document(&item.selector)
            }
        }
    }

    fn parse_gopher_line(&self, line: &str) -> Option<GopherItem> {
        if line.is_empty() {
            return None;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 { 
            if parts.len() == 3 && parts[0].chars().next() == Some('i') {
                return Some(GopherItem {
                    item_type: GopherItemType::Info,
                    name: parts[1].to_string(),
                    selector: "".to_string(), 
                    host: self.host.clone(), 
                    port: self.port,
                    description: parts[1].to_string(),
                });
            }
            return None;
        }

        let item_type_char = parts[0].chars().next()?;
        let item_type = match item_type_char {
            '0' => GopherItemType::File,
            '1' => GopherItemType::Directory,
            '2' => GopherItemType::PhoneServer,
            '3' => GopherItemType::Error,
            '4' => GopherItemType::BinHex,
            '5' => GopherItemType::Dosexe,
            '6' => GopherItemType::UUEncoded,
            '7' => GopherItemType::Search,
            '8' => GopherItemType::Telnet,
            '9' => GopherItemType::Binary,
            '+' => GopherItemType::Mirror,
            'g' => GopherItemType::GIF,
            'h' => GopherItemType::HTML,
            'i' => GopherItemType::Info,
            'w' => GopherItemType::WaisDoc,
            's' => GopherItemType::WaisSrc,
            _ => GopherItemType::File, 
        };

        let name = parts[1].to_string();
        let selector = parts[2].to_string();
        let host = parts[3].to_string();
        let port = parts.get(4).and_then(|p| p.parse::<u16>().ok()).unwrap_or(self.port);

        Some(GopherItem {
            item_type,
            name: name.clone(),
            selector,
            host,
            port,
            description: name,
        })
    }

    pub fn disconnect(&mut self) -> Result<(), GopherError> {
        self.tcp_client.disconnect().map_err(GopherError::TcpClient)?;
        self.connected = false;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gopher_client_creation() {
        let client = GopherClient::new();
        assert_eq!(client.port, 70);
        assert!(!client.connected);
    }

    #[test]
    fn test_parse_gopher_line() {
        let client = GopherClient::new();
        let line = "1Welcome to Gopher\t/\tgopher.floodgap.com\t70";
        let item = client.parse_gopher_line(line);
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.name, "Welcome to Gopher");
        assert_eq!(item.selector, "/");
        assert_eq!(item.host, "gopher.floodgap.com");
        assert_eq!(item.port, 70);
        assert!(matches!(item.item_type, GopherItemType::Directory));
    }

    #[test]
    fn test_parse_gopher_info_line() {
        let client = GopherClient::new();
        let line = "iThis is an info line\t\t";
        let item = client.parse_gopher_line(line);
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.name, "This is an info line");
        assert!(matches!(item.item_type, GopherItemType::Info));
    }
}