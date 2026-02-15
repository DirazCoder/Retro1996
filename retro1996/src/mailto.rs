use std::collections::HashMap;
use url::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailtoMessage {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
}

#[derive(Debug)]
pub enum MailtoError {
    InvalidUrl(String),
    ParseError(String),
    MissingRecipient,
    IoError(String),
}

impl std::fmt::Display for MailtoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailtoError::InvalidUrl(msg) => write!(f, "Invalid mailto URL: {}", msg),
            MailtoError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            MailtoError::MissingRecipient => write!(f, "Missing recipient"),
            MailtoError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for MailtoError {}

pub struct MailtoHandler;

impl MailtoHandler {
    pub fn parse_mailto_url(mailto_url: &str) -> Result<MailtoMessage, MailtoError> {
        // Check if it's a valid mailto URL
        if !mailto_url.to_lowercase().starts_with("mailto:") {
            return Err(MailtoError::InvalidUrl("URL does not start with 'mailto:'".to_string()));
        }

        // Parse the URL
        let parsed = Url::parse(mailto_url).map_err(|e| MailtoError::InvalidUrl(e.to_string()))?;

        // Extract recipients (the part after "mailto:")
        let path = parsed.path();
        let recipients_str = &path[1..]; // Remove leading slash if present

        // Parse recipients (comma-separated)
        let to: Vec<String> = recipients_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if to.is_empty() {
            return Err(MailtoError::MissingRecipient);
        }

        // Parse query parameters
        let mut cc = Vec::new();
        let mut bcc = Vec::new();
        let mut subject = None;
        let mut body = None;
        let mut headers = HashMap::new();

        for (key, value) in parsed.query_pairs() {
            match key.as_ref().to_lowercase().as_str() {
                "subject" => subject = Some(decode_url_component(&value)),
                "body" => body = Some(decode_url_component(&value)),
                "cc" => {
                    cc = value
                        .split(',')
                        .map(|s| decode_url_component(s.trim()))
                        .filter(|s| !s.is_empty())
                        .collect();
                },
                "bcc" => {
                    bcc = value
                        .split(',')
                        .map(|s| decode_url_component(s.trim()))
                        .filter(|s| !s.is_empty())
                        .collect();
                },
                _ => {
                    // Treat other parameters as custom headers
                    headers.insert(key.to_string(), decode_url_component(&value));
                }
            }
        }

        Ok(MailtoMessage {
            to,
            cc,
            bcc,
            subject,
            body,
            headers,
        })
    }

    pub fn create_mailto_url(message: &MailtoMessage) -> String {
        let mut url = String::from("mailto:");

        // Add recipients
        url.push_str(&message.to.join(","));

        // Add query parameters
        let mut params = Vec::new();

        if let Some(ref subject) = message.subject {
            params.push(format!("subject={}", encode_url_component(subject)));
        }

        if let Some(ref body) = message.body {
            params.push(format!("body={}", encode_url_component(body)));
        }

        if !message.cc.is_empty() {
            params.push(format!("cc={}", encode_url_component(&message.cc.join(","))));
        }

        if !message.bcc.is_empty() {
            params.push(format!("bcc={}", encode_url_component(&message.bcc.join(","))));
        }

        // Add custom headers
        for (key, value) in &message.headers {
            params.push(format!("{}={}", encode_url_component(key), encode_url_component(value)));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        url
    }

    pub fn launch_email_client(message: &MailtoMessage) -> Result<(), MailtoError> {
        let mailto_url = Self::create_mailto_url(message);
        
        // Launch the default email client
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(&["/C", "start", "", &mailto_url])
                .spawn()
                .map_err(|e| MailtoError::IoError(e.to_string()))
                .map(|_| ())
        }
        
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&mailto_url)
                .spawn()
                .map_err(|e| MailtoError::IoError(e.to_string()))
                .map(|_| ())
        }
        
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-email")
                .arg(&mailto_url)
                .spawn()
                .map_err(|e| MailtoError::IoError(e.to_string()))
                .map(|_| ())
        }
    }

    pub fn validate_email_address(email: &str) -> bool {
        // Simple email validation for 1996 compatibility
        // In 1996, email validation was much simpler
        let email = email.trim();
        
        if email.is_empty() {
            return false;
        }

        // Check for @ symbol
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return false;
        }

        let (local_part, domain_part) = (parts[0], parts[1]);

        // Basic checks for local part
        if local_part.is_empty() || local_part.len() > 64 {
            return false;
        }

        // Basic checks for domain part
        if domain_part.is_empty() || domain_part.len() > 255 {
            return false;
        }

        // Domain should contain at least one dot
        if !domain_part.contains('.') {
            return false;
        }

        true
    }

    pub fn validate_message(message: &MailtoMessage) -> Result<(), MailtoError> {
        // Validate recipients
        if message.to.is_empty() {
            return Err(MailtoError::MissingRecipient);
        }

        for recipient in &message.to {
            if !Self::validate_email_address(recipient) {
                return Err(MailtoError::InvalidUrl(format!("Invalid recipient: {}", recipient)));
            }
        }

        for recipient in &message.cc {
            if !Self::validate_email_address(recipient) {
                return Err(MailtoError::InvalidUrl(format!("Invalid CC recipient: {}", recipient)));
            }
        }

        for recipient in &message.bcc {
            if !Self::validate_email_address(recipient) {
                return Err(MailtoError::InvalidUrl(format!("Invalid BCC recipient: {}", recipient)));
            }
        }

        Ok(())
    }

    pub fn sanitize_message(message: &MailtoMessage) -> MailtoMessage {
        // Sanitize the message to prevent header injection and other issues
        MailtoMessage {
            to: message.to.iter().map(|addr| addr.trim().to_string()).filter(|addr| !addr.is_empty()).collect(),
            cc: message.cc.iter().map(|addr| addr.trim().to_string()).filter(|addr| !addr.is_empty()).collect(),
            bcc: message.bcc.iter().map(|addr| addr.trim().to_string()).filter(|addr| !addr.is_empty()).collect(),
            subject: message.subject.as_ref().map(|s| s.replace("\r", "").replace("\n", "")),
            body: message.body.as_ref().map(|s| s.replace("\r", "").replace("\n", "")),
            headers: message.headers.clone(), // In a real implementation, you'd sanitize headers too
        }
    }
}

// URL encoding/decoding utilities for mailto parameters
fn encode_url_component(component: &str) -> String {
    let mut result = String::new();
    for byte in component.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

fn decode_url_component(component: &str) -> String {
    let mut result = String::new();
    let mut chars = component.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let next1 = chars.next();
            let next2 = chars.next();

            if let (Some(c1), Some(c2)) = (next1, next2) {
                let hex_str = format!("{}{}", c1, c2);
                if let Ok(byte_val) = u8::from_str_radix(&hex_str, 16) {
                    if let Some(decoded_char) = char::from_u32(byte_val as u32) {
                        result.push(decoded_char);
                    } else {
                        result.push('%');
                        result.push(c1);
                        result.push(c2);
                    }
                } else {
                    result.push('%');
                    result.push(c1);
                    result.push(c2);
                }
            } else {
                result.push('%');
                if let Some(c1) = next1 { result.push(c1); }
                if let Some(c2) = next2 { result.push(c2); }
            }
        } else {
            result.push(c);
        }
    }

    result
}

// Email composer utility for creating mailto messages
pub struct EmailComposer {
    pub default_sender: Option<String>,
    pub default_signature: Option<String>,
}

impl EmailComposer {
    pub fn new() -> Self {
        EmailComposer {
            default_sender: None,
            default_signature: None,
        }
    }

    pub fn compose_message(&self, to: Vec<String>, subject: Option<String>, body: Option<String>) -> MailtoMessage {
        let mut full_body = String::new();
        
        if let Some(msg_body) = body {
            full_body.push_str(&msg_body);
        }
        
        if let Some(ref signature) = self.default_signature {
            if !full_body.is_empty() {
                full_body.push_str("\n\n");
            }
            full_body.push_str(signature);
        }

        MailtoMessage {
            to,
            cc: Vec::new(),
            bcc: Vec::new(),
            subject,
            body: Some(full_body),
            headers: HashMap::new(),
        }
    }

    pub fn add_cc(&self, mut message: MailtoMessage, cc: Vec<String>) -> MailtoMessage {
        message.cc = cc;
        message
    }

    pub fn add_bcc(&self, mut message: MailtoMessage, bcc: Vec<String>) -> MailtoMessage {
        message.bcc = bcc;
        message
    }

    pub fn add_header(&self, mut message: MailtoMessage, key: String, value: String) -> MailtoMessage {
        message.headers.insert(key, value);
        message
    }
}

// Mailto protocol handler for integration with the browser
pub struct BrowserMailtoHandler {
    pub email_composer: EmailComposer,
    pub client_detection_enabled: bool,
}

impl BrowserMailtoHandler {
    pub fn new() -> Self {
        BrowserMailtoHandler {
            email_composer: EmailComposer::new(),
            client_detection_enabled: true,
        }
    }

    pub fn handle_mailto_click(&self, mailto_url: &str) -> Result<(), MailtoError> {
        let message = MailtoHandler::parse_mailto_url(mailto_url)?;
        let sanitized_message = MailtoHandler::sanitize_message(&message);
        MailtoHandler::validate_message(&sanitized_message)?;
        MailtoHandler::launch_email_client(&sanitized_message)
    }

    pub fn register_protocol_handler(&self) -> Result<(), MailtoError> {
        // In a real browser implementation, this would register the mailto handler
        // For now, we'll just return Ok to indicate the function exists
        println!("Registered mailto protocol handler");
        Ok(())
    }

    pub fn get_client_info(&self) -> Option<String> {
        // Detect the default email client (simplified)
        #[cfg(target_os = "windows")]
        {
            // In a real implementation, you'd check registry entries
            Some("Default Windows Email Client".to_string())
        }
        
        #[cfg(target_os = "macos")]
        {
            Some("Apple Mail".to_string())
        }
        
        #[cfg(target_os = "linux")]
        {
            // Check for common Linux email clients
            Some("Default Linux Email Client".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_mailto() {
        let result = MailtoHandler::parse_mailto_url("mailto:test@example.com").unwrap();
        assert_eq!(result.to, vec!["test@example.com"]);
    }

    #[test]
    fn test_parse_mailto_with_subject() {
        let result = MailtoHandler::parse_mailto_url("mailto:test@example.com?subject=Hello").unwrap();
        assert_eq!(result.to, vec!["test@example.com"]);
        assert_eq!(result.subject, Some("Hello".to_string()));
    }

    #[test]
    fn test_validate_email() {
        assert!(MailtoHandler::validate_email_address("test@example.com"));
        assert!(!MailtoHandler::validate_email_address("invalid-email"));
    }
}