use std::io::{BufRead, BufReader, Write};
use std::time::Duration;
use url::Url;
use thiserror::Error;
use crate::network::tcp_client::TcpClient;

#[derive(Error, Debug)]
pub enum FtpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TCP client error: {0}")]
    TcpClient(String),
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("Authentication failed: {0}")]
    Authentication(String),
    #[error("Command failed: {0}")]
    Command(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Timeout")]
    Timeout,
    #[error("Not logged in")]
    NotLoggedIn,
    #[error("No data connection established")]
    NoDataConnection,
}

#[derive(Debug, Clone)]
pub struct FtpResponse {
    pub code: u16,
    pub message: String,
    pub data: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct FtpFile {
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub permissions: String,
    pub modified: String,
}

pub struct FtpClient {
    control_stream: Option<TcpClient>,
    data_stream: Option<TcpClient>,
    host: String,
    port: u16,
    logged_in: bool,
    timeout: Duration,
    username: String,
    password: String,
    passive_mode: bool,
    current_directory: String,
}

impl FtpClient {
    pub fn new() -> Self {
        FtpClient {
            control_stream: None,
            data_stream: None,
            host: String::new(),
            port: 21,
            logged_in: false,
            timeout: Duration::from_secs(30),
            username: "anonymous".to_string(),
            password: "anonymous@".to_string(),
            passive_mode: true,
            current_directory: "/".to_string(),
        }
    }

    pub fn connect(&mut self, host: &str, port: Option<u16>) -> Result<(), FtpError> {
        let port = port.unwrap_or(21);
        let addr = format!("{}:{}", host, port);
        let mut tcp_client = TcpClient::new(self.timeout);
        tcp_client.connect(&addr).map_err(FtpError::TcpClient)?;

        self.control_stream = Some(tcp_client);
        self.host = host.to_string();
        self.port = port;

        let response = self.read_response()?;
        if response.code != 220 {
            return Err(FtpError::Connection(format!("Unexpected welcome code: {}", response.code)));
        }

        self.login()?;
        self.logged_in = true;
        Ok(())
    }

    pub fn connect_url(&mut self, url: &str) -> Result<(), FtpError> {
        let parsed = Url::parse(url).map_err(|e| FtpError::Connection(format!("Invalid URL: {}", e)))?;
        if parsed.scheme() != "ftp" {
            return Err(FtpError::Connection("Scheme must be 'ftp'".to_string()));
        }
        let host = parsed.host_str().ok_or_else(|| FtpError::Connection("No host in URL".to_string()))?.to_string();
        let port = parsed.port().unwrap_or(21);
        let username = parsed.username();
        let password = parsed.password().unwrap_or(" ");
        if !username.is_empty() {
            self.username = username.to_string();
        }
        if !password.is_empty() {
            self.password = password.to_string();
        }
        self.connect(&host, Some(port))
    }

    fn login(&mut self) -> Result<(), FtpError> {
        let username = self.username.clone();
        self.send_command("USER", &username)?;
        let response = self.read_response()?;
        match response.code {
            230 => {
            }
            331 => {
                let password = self.password.clone();
                self.send_command("PASS", &password)?;
                let pass_response = self.read_response()?;
                if pass_response.code != 230 {
                    return Err(FtpError::Authentication(format!("Login failed: {}", pass_response.message)));
                }
            }
            _ => {
                return Err(FtpError::Authentication(format!("Login failed: {}", response.message)));
            }
        }
        Ok(())
    }

    fn send_command(&mut self, command: &str, args: &str) -> Result<(), FtpError> {
        let cmd_line = if !args.is_empty() {
            format!("{} {}\r\n", command, args)
        } else {
            format!("{}\r\n", command)
        };

        if let Some(ref mut stream) = self.control_stream {
            stream.send(cmd_line.as_bytes()).map_err(FtpError::TcpClient)?;
        } else {
            return Err(FtpError::Connection("Control stream not connected".to_string()));
        }
        Ok(())
    }

    fn read_response(&mut self) -> Result<FtpResponse, FtpError> {
        let mut buffer = Vec::new();
        if let Some(ref mut stream) = self.control_stream {
            let mut byte = [0u8; 1];
            loop {
                stream.receive(&mut byte).map_err(FtpError::TcpClient)?;
                buffer.push(byte[0]);
                if buffer.len() >= 2 && &buffer[buffer.len()-2..] == b"\r\n" {
                    break;
                }
            }
        } else {
            return Err(FtpError::Connection("Control stream not connected".to_string()));
        }

        let response_str = String::from_utf8_lossy(&buffer);
        let response_str = response_str.trim_end_matches(|c| c == '\r' || c == '\n');

        if response_str.len() < 3 {
            return Err(FtpError::InvalidResponse("Response too short".to_string()));
        }

        let code_str = &response_str[..3];
        let code = code_str.parse::<u16>().map_err(|_| FtpError::Parse("Invalid response code".to_string()))?;
        let message = response_str[4..].trim_start().to_string();

        Ok(FtpResponse { code, message, data: None })
    }

    fn enter_passive_mode(&mut self) -> Result<(String, u16), FtpError> {
        self.send_command("PASV", "")?;
        let response = self.read_response()?;
        if response.code != 227 {
            return Err(FtpError::Command(format!("PASV failed: {}", response.message)));
        }

        let start = response.message.find('(').ok_or_else(|| FtpError::Parse("PASV response missing '('".to_string()))?;
        let end = response.message.find(')').ok_or_else(|| FtpError::Parse("PASV response missing ')'".to_string()))?;
        let addr_part = &response.message[start+1..end];

        let parts: Vec<&str> = addr_part.split(',').collect();
        if parts.len() != 6 {
            return Err(FtpError::Parse("PASV response has invalid format".to_string()));
        }

        let ip_parts: Result<Vec<u8>, _> = parts[0..4].iter().map(|s| s.parse::<u8>().map_err(|e| FtpError::Parse(e.to_string()))).collect();
        let ip_parts = ip_parts?;
        let port_high: u16 = parts[4].parse().map_err(|e| FtpError::Parse(e.to_string()))?;
        let port_low: u16 = parts[5].parse().map_err(|e| FtpError::Parse(e.to_string()))?;
        let data_port = (port_high << 8) | port_low;

        let data_host = format!("{}.{}.{}.{}", ip_parts[0], ip_parts[1], ip_parts[2], ip_parts[3]);

        Ok((data_host, data_port))
    }

    pub fn list_directory(&mut self, path: Option<&str>) -> Result<Vec<FtpFile>, FtpError> {
        if !self.logged_in {
            return Err(FtpError::NotLoggedIn);
        }

        let (data_host, data_port) = self.enter_passive_mode()?;
        self.send_command("LIST", path.unwrap_or(""))?;

        let response = self.read_response()?;
        if response.code != 150 {
            return Err(FtpError::Command(format!("LIST command failed: {}", response.message)));
        }

        let mut data_tcp_client = TcpClient::new(self.timeout);
        let data_addr = format!("{}:{}", data_host, data_port);
        data_tcp_client.connect(&data_addr).map_err(FtpError::TcpClient)?;
        self.data_stream = Some(data_tcp_client);

        let mut listing = String::new();
        if let Some(ref mut data_stream) = self.data_stream {
            let mut data_buffer = Vec::new();
            loop {
                let mut chunk = vec![0; 1024];
                match data_stream.receive(&mut chunk) {
                    Ok(bytes_read) => {
                        if bytes_read == 0 { break; }
                        data_buffer.extend_from_slice(&chunk[..bytes_read]);
                    }
                    Err(_) => break,
                }
            }
            listing = String::from_utf8_lossy(&data_buffer).into_owned();
        } else {
            return Err(FtpError::NoDataConnection);
        }
        self.data_stream = None;

        let response = self.read_response()?;
        if response.code != 226 {
            return Err(FtpError::Command("Transfer incomplete".to_string()));
        }

        let files = self.parse_list_output(&listing);
        Ok(files)
    }

    fn parse_list_output(&self, output: &str) -> Vec<FtpFile> {
        let mut files = Vec::new();
        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 9 {
                continue;
            }

            let permissions = parts[0];
            let month = parts[5];
            let day = parts[6];
            let year_or_time = parts[7];
            let name = parts[8..].join(" ");

            let is_directory = permissions.starts_with('d');
            let size = if is_directory {
                0
            } else {
                parts[4].parse::<u64>().unwrap_or(0)
            };

            let modified = format!("{} {} {}", month, day, year_or_time);

            files.push(FtpFile {
                name,
                size,
                is_directory,
                permissions: permissions.to_string(),
                modified,
            });
        }
        files
    }

    pub fn retrieve_file(&mut self, remote_path: &str, local_path: &str) -> Result<(), FtpError> {
        if !self.logged_in {
            return Err(FtpError::NotLoggedIn);
        }

        let (data_host, data_port) = self.enter_passive_mode()?;
        self.send_command("RETR", remote_path)?;

        let response = self.read_response()?;
        if response.code != 150 {
            return Err(FtpError::Command(format!("RETR command failed: {}", response.message)));
        }

        let mut data_tcp_client = TcpClient::new(self.timeout);
        let data_addr = format!("{}:{}", data_host, data_port);
        data_tcp_client.connect(&data_addr).map_err(FtpError::TcpClient)?;
        self.data_stream = Some(data_tcp_client);

        use std::fs::File;
        let mut local_file = File::create(local_path).map_err(FtpError::Io)?;

        if let Some(ref mut data_stream) = self.data_stream {
            loop {
                let mut chunk = vec![0; 1024];
                match data_stream.receive(&mut chunk) {
                    Ok(bytes_read) => {
                        if bytes_read == 0 { break; }
                        local_file.write_all(&chunk[..bytes_read]).map_err(FtpError::Io)?;
                    }
                    Err(_) => break,
                }
            }
        } else {
            return Err(FtpError::NoDataConnection);
        }
        self.data_stream = None;

        let response = self.read_response()?;
        if response.code != 226 {
            return Err(FtpError::Command("Transfer incomplete".to_string()));
        }

        Ok(())
    }

    pub fn store_file(&mut self, local_path: &str, remote_path: &str) -> Result<(), FtpError> {
        if !self.logged_in {
            return Err(FtpError::NotLoggedIn);
        }

        let (data_host, data_port) = self.enter_passive_mode()?;
        self.send_command("STOR", remote_path)?;

        let response = self.read_response()?;
        if response.code != 150 {
            return Err(FtpError::Command(format!("STOR command failed: {}", response.message)));
        }

        let mut data_tcp_client = TcpClient::new(self.timeout);
        let data_addr = format!("{}:{}", data_host, data_port);
        data_tcp_client.connect(&data_addr).map_err(FtpError::TcpClient)?;
        self.data_stream = Some(data_tcp_client);

        use std::fs::File;
        let mut local_file = File::open(local_path).map_err(FtpError::Io)?;

        if let Some(ref mut data_stream) = self.data_stream {
            let mut buffer = [0; 1024];
            loop {
                let bytes_read = std::io::Read::read(&mut local_file, &mut buffer).map_err(FtpError::Io)?;
                if bytes_read == 0 { break; }
                data_stream.send(&buffer[..bytes_read]).map_err(FtpError::TcpClient)?;
            }
        } else {
            return Err(FtpError::NoDataConnection);
        }
        self.data_stream = None;

        let response = self.read_response()?;
        if response.code != 226 {
            return Err(FtpError::Command("Transfer incomplete".to_string()));
        }

        Ok(())
    }

    pub fn cwd(&mut self, path: &str) -> Result<(), FtpError> {
        if !self.logged_in {
            return Err(FtpError::NotLoggedIn);
        }
        self.send_command("CWD", path)?;
        let response = self.read_response()?;
        if response.code != 250 {
            return Err(FtpError::Command(format!("CWD command failed: {}", response.message)));
        }
        self.current_directory = path.to_string();
        Ok(())
    }

    pub fn pwd(&mut self) -> Result<String, FtpError> {
        if !self.logged_in {
            return Err(FtpError::NotLoggedIn);
        }
        self.send_command("PWD", "")?;
        let response = self.read_response()?;
        if response.code != 257 {
            return Err(FtpError::Command(format!("PWD command failed: {}", response.message)));
        }
        let msg = &response.message;
        if msg.len() < 2 || !msg.starts_with('"') {
            return Err(FtpError::Parse("PWD response format invalid".to_string()));
        }
        let end_quote = msg[1..].find('"').ok_or_else(|| FtpError::Parse("PWD response format invalid".to_string()))?;
        Ok(msg[1..=end_quote].to_string())
    }

    pub fn quit(&mut self) -> Result<(), FtpError> {
        if self.logged_in {
            self.send_command("QUIT", "")?;
            let response = self.read_response()?;
            if response.code != 221 {
                eprintln!("Warning: QUIT command did not return 221: {}", response.message);
            }
            self.logged_in = false;
        }
        self.control_stream = None;
        Ok(())
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
        if let Some(ref mut stream) = self.control_stream {
        }
        if let Some(ref mut stream) = self.data_stream {
        }  
    }

    pub fn get_file_size(&mut self, path: &str) -> Result<u64, FtpError> {
        if !self.logged_in {
            return Err(FtpError::NotLoggedIn);
        }
        self.send_command("SIZE", path)?;
        let response = self.read_response()?;
        if response.code != 213 {
            return Err(FtpError::Command(format!("SIZE command failed: {}", response.message)));
        }
        let size_str = response.message.trim();
        size_str.parse::<u64>().map_err(|_| FtpError::Parse("Invalid size response".to_string()))
    }

    pub fn change_transfer_type(&mut self, binary: bool) -> Result<(), FtpError> {
        if !self.logged_in {
            return Err(FtpError::NotLoggedIn);
        }
        let type_cmd = if binary { "I" } else { "A" };
        self.send_command("TYPE", type_cmd)?;
        let response = self.read_response()?;
        if response.code != 200 {
            return Err(FtpError::Command(format!("TYPE command failed: {}", response.message)));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ftp_client_creation() {
        let client = FtpClient::new();
        assert_eq!(client.port, 21);
        assert!(!client.logged_in);
    }
}