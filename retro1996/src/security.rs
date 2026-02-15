use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoredPassword {
    pub username: String,
    pub encrypted_password: String,  // In 1996, passwords were often stored in plain text or simple encoding
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub times_used: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<DateTime<Utc>>,  // None means session cookie
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSitePolicy,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SameSitePolicy {
    Strict,
    Lax,
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub fingerprint: String,
    pub serial_number: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecuritySettings {
    pub accept_all_cookies: bool,
    pub accept_secure_cookies: bool,
    pub warn_on_cookie_accept: bool,
    pub store_passwords: bool,
    pub warn_on_password_save: bool,
    pub enable_referrer_header: bool,
    pub warn_on_form_submit: bool,
    pub certificate_warnings: bool,
}

pub struct SecurityManager {
    pub stored_passwords: Vec<StoredPassword>,
    pub cookies: HashMap<String, Cookie>,  // Key is domain+name
    pub certificates: Vec<CertificateInfo>,
    pub settings: SecuritySettings,
    pub password_file_path: String,
    pub cookie_file_path: String,
}

impl SecurityManager {
    pub fn new(password_file_path: String, cookie_file_path: String) -> Self {
        let mut security_manager = SecurityManager {
            stored_passwords: Vec::new(),
            cookies: HashMap::new(),
            certificates: Vec::new(),
            settings: SecuritySettings {
                accept_all_cookies: true,
                accept_secure_cookies: true,
                warn_on_cookie_accept: false,
                store_passwords: true,
                warn_on_password_save: true,
                enable_referrer_header: true,
                warn_on_form_submit: false,
                certificate_warnings: true,
            },
            password_file_path,
            cookie_file_path,
        };
        
        // Load existing data
        security_manager.load_passwords();
        security_manager.load_cookies();
        
        security_manager
    }

    pub fn save_password(&mut self, username: String, password: String, url: String) -> Result<(), SecurityError> {
        if !self.settings.store_passwords {
            return Err(SecurityError::PasswordStorageDisabled);
        }

        // In 1996, passwords were often stored with minimal encryption
        // For this implementation, we'll use a simple XOR cipher as period-appropriate
        let encrypted_password = self.encrypt_password(&password);

        let stored_password = StoredPassword {
            username,
            encrypted_password,
            url,
            created_at: Utc::now(),
            last_used: Utc::now(),
            times_used: 0,
        };

        self.stored_passwords.push(stored_password);
        self.save_passwords_to_file()
    }

    pub fn get_password(&mut self, username: &str, url: &str) -> Option<String> {
        if let Some(index) = self.stored_passwords.iter().position(|pwd| pwd.username == username && pwd.url == url) {
            let pwd = &mut self.stored_passwords[index];
            pwd.times_used += 1;
            pwd.last_used = Utc::now();
            
            let encrypted_password = pwd.encrypted_password.clone();
            // Create a separate reference to avoid borrowing issues
            let password_manager = &*self;
            Some(password_manager.decrypt_password(&encrypted_password))
        } else {
            None
        }
    }

    pub fn remove_password(&mut self, username: &str, url: &str) -> bool {
        let initial_len = self.stored_passwords.len();
        self.stored_passwords.retain(|pwd| !(pwd.username == username && pwd.url == url));
        let removed = self.stored_passwords.len() != initial_len;
        
        if removed {
            let _ = self.save_passwords_to_file();
        }
        
        removed
    }

    pub fn clear_passwords(&mut self) {
        self.stored_passwords.clear();
        let _ = self.save_passwords_to_file();
    }

    pub fn add_cookie(&mut self, cookie: Cookie) -> Result<(), SecurityError> {
        if !self.should_accept_cookie(&cookie) {
            return Err(SecurityError::CookieRejected);
        }

        let key = format!("{}:{}", cookie.domain, cookie.name);
        self.cookies.insert(key, cookie);
        self.save_cookies_to_file()
    }

    pub fn get_cookie(&self, domain: &str, name: &str) -> Option<&Cookie> {
        let key = format!("{}:{}", domain, name);
        self.cookies.get(&key)
    }

    pub fn remove_cookie(&mut self, domain: &str, name: &str) -> bool {
        let key = format!("{}:{}", domain, name);
        let removed = self.cookies.remove(&key).is_some();
        
        if removed {
            let _ = self.save_cookies_to_file();
        }
        
        removed
    }

    pub fn clear_cookies(&mut self) {
        self.cookies.clear();
        let _ = self.save_cookies_to_file();
    }

    pub fn should_accept_cookie(&self, cookie: &Cookie) -> bool {
        if !self.settings.accept_all_cookies && !cookie.secure {
            return false;
        }
        
        if cookie.secure && !self.settings.accept_secure_cookies {
            return false;
        }
        
        // Check if cookie has expired
        if let Some(expires) = cookie.expires {
            if Utc::now() > expires {
                return false;
            }
        }
        
        true
    }

    pub fn get_cookies_for_domain(&self, domain: &str) -> Vec<&Cookie> {
        self.cookies.values()
            .filter(|cookie| cookie.domain == domain)
            .collect()
    }

    pub fn get_cookies_for_url(&self, url: &str) -> Vec<&Cookie> {
        // Extract domain from URL
        let domain = self.extract_domain_from_url(url);
        self.get_cookies_for_domain(&domain)
    }

    fn extract_domain_from_url(&self, url: &str) -> String {
        // Simple domain extraction
        if let Some(start) = url.find("://") {
            let after_protocol = &url[start + 3..];
            if let Some(end) = after_protocol.find('/') {
                after_protocol[..end].to_string()
            } else {
                after_protocol.to_string()
            }
        } else {
            url.to_string()
        }
    }

    fn encrypt_password(&self, password: &str) -> String {
        // Simple XOR encryption as period-appropriate (not secure by modern standards)
        let key = b"Retro1996";
        let mut result = Vec::new();
        
        for (i, byte) in password.bytes().enumerate() {
            result.push(byte ^ key[i % key.len()]);
        }
        
        // Convert to hex string for storage
        result.iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn decrypt_password(&self, encrypted: &str) -> String {
        // Decrypt the hex string
        let key = b"Retro1996";
        let mut result = Vec::new();
        
        // Convert hex string back to bytes
        for chunk in encrypted.as_bytes().chunks(2) {
            if chunk.len() == 2 {
                let hex_str = std::str::from_utf8(chunk).unwrap();
                if let Ok(byte_val) = u8::from_str_radix(hex_str, 16) {
                    let idx = result.len();
                    result.push(byte_val ^ key[idx % key.len()]);
                }
            }
        }
        
        String::from_utf8(result).unwrap_or_default()
    }

    fn save_passwords_to_file(&self) -> Result<(), SecurityError> {
        // Create directory if it doesn't exist
        if let Some(parent) = Path::new(&self.password_file_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| SecurityError::IoError(e.to_string()))?;
        }

        let json = serde_json::to_string_pretty(&self.stored_passwords)
            .map_err(|e| SecurityError::SerializationError(e.to_string()))?;
        
        fs::write(&self.password_file_path, json)
            .map_err(|e| SecurityError::IoError(e.to_string()))?;
        
        Ok(())
    }

    fn load_passwords(&mut self) {
        if !Path::new(&self.password_file_path).exists() {
            return;
        }

        if let Ok(content) = fs::read_to_string(&self.password_file_path) {
            if let Ok(passwords) = serde_json::from_str::<Vec<StoredPassword>>(&content) {
                self.stored_passwords = passwords;
            }
        }
    }

    fn save_cookies_to_file(&self) -> Result<(), SecurityError> {
        // Create directory if it doesn't exist
        if let Some(parent) = Path::new(&self.cookie_file_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| SecurityError::IoError(e.to_string()))?;
        }

        let json = serde_json::to_string_pretty(&self.cookies)
            .map_err(|e| SecurityError::SerializationError(e.to_string()))?;
        
        fs::write(&self.cookie_file_path, json)
            .map_err(|e| SecurityError::IoError(e.to_string()))?;
        
        Ok(())
    }

    fn load_cookies(&mut self) {
        if !Path::new(&self.cookie_file_path).exists() {
            return;
        }

        if let Ok(content) = fs::read_to_string(&self.cookie_file_path) {
            if let Ok(cookies) = serde_json::from_str::<HashMap<String, Cookie>>(&content) {
                self.cookies = cookies;
            }
        }
    }

    pub fn validate_certificate(&self, cert_info: &CertificateInfo) -> CertificateValidationResult {
        let now = Utc::now();
        
        if now < cert_info.valid_from {
            return CertificateValidationResult::NotYetValid;
        }
        
        if now > cert_info.valid_until {
            return CertificateValidationResult::Expired;
        }
        
        CertificateValidationResult::Valid
    }

    pub fn add_certificate(&mut self, cert_info: CertificateInfo) {
        self.certificates.push(cert_info);
    }

    pub fn get_certificate(&self, fingerprint: &str) -> Option<&CertificateInfo> {
        self.certificates.iter().find(|cert| cert.fingerprint == fingerprint)
    }

    pub fn check_form_security(&self, form_action: &str, current_url: &str) -> FormSecurityCheck {
        // Check if form is submitting to a different origin (potential CSRF)
        let current_domain = self.extract_domain_from_url(current_url);
        let action_domain = self.extract_domain_from_url(form_action);
        
        if current_domain != action_domain {
            FormSecurityCheck {
                is_secure: false,
                warning: Some(format!("Form is submitting to a different domain: {}", action_domain)),
                allow_submit: self.settings.warn_on_form_submit,
            }
        } else {
            FormSecurityCheck {
                is_secure: true,
                warning: None,
                allow_submit: true,
            }
        }
    }

    pub fn get_password_stats(&self) -> PasswordStats {
        PasswordStats {
            total_passwords: self.stored_passwords.len(),
            most_used: self.stored_passwords.iter()
                .max_by_key(|pwd| pwd.times_used)
                .map(|pwd| pwd.username.clone()),
            least_used: self.stored_passwords.iter()
                .min_by_key(|pwd| pwd.times_used)
                .map(|pwd| pwd.username.clone()),
        }
    }
}

#[derive(Debug)]
pub struct FormSecurityCheck {
    pub is_secure: bool,
    pub warning: Option<String>,
    pub allow_submit: bool,
}

#[derive(Debug)]
pub enum CertificateValidationResult {
    Valid,
    Expired,
    NotYetValid,
    Revoked,  // Would require online check in real implementation
}

#[derive(Debug)]
pub enum SecurityError {
    IoError(String),
    SerializationError(String),
    PasswordStorageDisabled,
    CookieRejected,
    InvalidInput(String),
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::IoError(msg) => write!(f, "IO Error: {}", msg),
            SecurityError::SerializationError(msg) => write!(f, "Serialization Error: {}", msg),
            SecurityError::PasswordStorageDisabled => write!(f, "Password storage is disabled"),
            SecurityError::CookieRejected => write!(f, "Cookie was rejected based on security settings"),
            SecurityError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl std::error::Error for SecurityError {}

#[derive(Debug)]
pub struct PasswordStats {
    pub total_passwords: usize,
    pub most_used: Option<String>,
    pub least_used: Option<String>,
}

// HTTP Basic Authentication dialog simulation
pub struct BasicAuthCredentials {
    pub username: String,
    pub password: String,
}

pub fn show_basic_auth_dialog(realm: &str) -> Option<BasicAuthCredentials> {
    // In a real implementation, this would show a GUI dialog
    // For now, we'll return None to indicate the feature exists conceptually
    println!("Basic Auth Dialog would appear for realm: {}", realm);
    None
}

// Security zone management (conceptual for 1996 browsers)
#[derive(Debug, Clone)]
pub enum SecurityZone {
    LocalMachine,
    LocalIntranet,
    TrustedSites,
    Internet,
    RestrictedSites,
}

pub fn get_security_zone_for_url(url: &str) -> SecurityZone {
    if url.starts_with("file://") {
        SecurityZone::LocalMachine
    } else if url.contains("localhost") || url.starts_with("http://192.168.") || url.starts_with("http://10.") || url.starts_with("http://172.") {
        SecurityZone::LocalIntranet
    } else {
        SecurityZone::Internet
    }
}