use std::io::{Read, Write};
use std::net::{TcpStream, SocketAddr};
use std::time::Duration;
use std::str;

#[derive(Debug)]
pub enum TcpError {
    Io(std::io::Error),
    Connection(String),
    Timeout,
}

pub struct TcpClient {
    stream: Option<TcpStream>,
    timeout: Duration,
    connected: bool,
}

impl TcpClient {
    pub fn new(timeout: Duration) -> Self {
        TcpClient {
            stream: None,
            timeout,
            connected: false,
        }
    }

    pub fn connect(&mut self, addr: &str) -> Result<(), TcpError> {
        let stream = TcpStream::connect(addr)
            .map_err(|e| TcpError::Connection(format!("Failed to connect to {}: {}", addr, e)))?;
        
        stream.set_read_timeout(Some(self.timeout))
            .map_err(TcpError::Io)?;
        stream.set_write_timeout(Some(self.timeout))
            .map_err(TcpError::Io)?;

        self.stream = Some(stream);
        self.connected = true;
        Ok(())
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TcpError> {
        if let Some(ref mut stream) = self.stream {
            stream.write_all(data).map_err(TcpError::Io)?;
            stream.flush().map_err(TcpError::Io)?;
            Ok(())
        } else {
            Err(TcpError::Connection("Not connected".to_string()))
        }
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TcpError> {
        if let Some(ref mut stream) = self.stream {
            stream.read(buffer).map_err(TcpError::Io)
        } else {
            Err(TcpError::Connection("Not connected".to_string()))
        }
    }

    pub fn receive_all(&mut self) -> Result<Vec<u8>, TcpError> {
        let mut data = Vec::new();
        if let Some(ref mut stream) = self.stream {
            let mut buffer = [0; 1024];
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => data.extend_from_slice(&buffer[..n]),
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(e) => return Err(TcpError::Io(e)),
                }
            }
        } else {
            return Err(TcpError::Connection("Not connected".to_string()));
        }
        Ok(data)
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn disconnect(&mut self) -> Result<(), TcpError> {
        self.stream = None;
        self.connected = false;
        Ok(())
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
        if let Some(ref mut stream) = self.stream {
            stream.set_read_timeout(Some(timeout)).map_err(TcpError::Io)?;
            stream.set_write_timeout(Some(timeout)).map_err(TcpError::Io)?;
        }
        Ok(())
    }

    pub fn peek(&mut self) -> Result<usize, TcpError> {
        if let Some(ref mut stream) = self.stream {
            stream.peek(&mut [0; 1]).map_err(TcpError::Io)
        } else {
            Err(TcpError::Connection("Not connected".to_string()))
        }
    }

    pub fn available(&mut self) -> Result<usize, TcpError> {
        if let Some(ref mut stream) = self.stream {
            let mut buf = [0; 1];
            match stream.peek(&mut buf) {
                Ok(n) => Ok(n),
                Err(e) => Err(TcpError::Io(e)),
            }
        } else {
            Err(TcpError::Connection("Not connected".to_string()))
        }
    }

    pub fn flush(&mut self) -> Result<(), TcpError> {
        if let Some(ref mut stream) = self.stream {
            stream.flush().map_err(TcpError::Io)
        } else {
            Err(TcpError::Connection("Not connected".to_string()))
        }
    }

    pub fn get_local_addr(&self) -> Result<SocketAddr, TcpError> {
        if let Some(ref stream) = self.stream {
            stream.local_addr().map_err(TcpError::Io)
        } else {
            Err(TcpError::Connection("Not connected".to_string()))
        }
    }

    pub fn get_peer_addr(&self) -> Result<SocketAddr, TcpError> {
        if let Some(ref stream) = self.stream {
            stream.peer_addr().map_err(TcpError::Io)
        } else {
            Err(TcpError::Connection("Not connected".to_string()))
        }
    }
}