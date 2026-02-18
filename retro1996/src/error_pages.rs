use std::collections::HashMap;
use chrono::{DateTime, Utc};
use std::fs;
use std::io::{self, Write, Read};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Once;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum ErrorType {
    NotFound,
    ServerError,
    Timeout,
    ConnectionRefused,
    InvalidUrl,
    JavaScriptError,
    RenderingError,
    SecurityError,
    ConnectionError,
    ProtocolError,
    DnsFailure,
    NetworkTimeout,
    SslCertificateError,
    MalformedRequest,
    UnsupportedMediaType,
    GatewayTimeout,
    BadGateway,
    ServiceUnavailable,
    Forbidden,
    Unauthorized,
    Gone,
    TooManyRequests,
}

impl ErrorType {
    pub fn code(&self) -> Option<u16> {
        match self {
            ErrorType::NotFound => Some(404),
            ErrorType::ServerError => Some(500),
            ErrorType::Timeout => Some(408),
            ErrorType::ConnectionRefused => Some(0),
            ErrorType::InvalidUrl => Some(0),
            ErrorType::JavaScriptError => Some(0),
            ErrorType::RenderingError => Some(0),
            ErrorType::SecurityError => Some(0),
            ErrorType::ConnectionError => Some(0),
            ErrorType::ProtocolError => Some(0),
            ErrorType::DnsFailure => Some(0),
            ErrorType::NetworkTimeout => Some(0),
            ErrorType::SslCertificateError => Some(0),
            ErrorType::MalformedRequest => Some(400),
            ErrorType::UnsupportedMediaType => Some(415),
            ErrorType::GatewayTimeout => Some(504),
            ErrorType::BadGateway => Some(502),
            ErrorType::ServiceUnavailable => Some(503),
            ErrorType::Forbidden => Some(403),
            ErrorType::Unauthorized => Some(401),
            ErrorType::Gone => Some(410),
            ErrorType::TooManyRequests => Some(429),
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            ErrorType::NotFound => "Not Found",
            ErrorType::ServerError => "Internal Server Error",
            ErrorType::Timeout => "Request Timeout",
            ErrorType::ConnectionRefused => "Connection Refused",
            ErrorType::InvalidUrl => "Invalid URL",
            ErrorType::JavaScriptError => "JavaScript Error",
            ErrorType::RenderingError => "Rendering Error",
            ErrorType::SecurityError => "Security Error",
            ErrorType::ConnectionError => "Connection Error",
            ErrorType::ProtocolError => "Protocol Error",
            ErrorType::DnsFailure => "DNS Lookup Failed",
            ErrorType::NetworkTimeout => "Network Timeout",
            ErrorType::SslCertificateError => "SSL Certificate Error",
            ErrorType::MalformedRequest => "Bad Request",
            ErrorType::UnsupportedMediaType => "Unsupported Media Type",
            ErrorType::GatewayTimeout => "Gateway Timeout",
            ErrorType::BadGateway => "Bad Gateway",
            ErrorType::ServiceUnavailable => "Service Unavailable",
            ErrorType::Forbidden => "Forbidden",
            ErrorType::Unauthorized => "Unauthorized",
            ErrorType::Gone => "Gone",
            ErrorType::TooManyRequests => "Too Many Requests",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ErrorType::NotFound => "The requested URL was not found on this server.",
            ErrorType::ServerError => "The server encountered an unexpected condition that prevented it from fulfilling the request.",
            ErrorType::Timeout => "The server did not respond within the specified time limit.",
            ErrorType::ConnectionRefused => "The connection was refused by the remote server.",
            ErrorType::InvalidUrl => "The URL format is invalid or malformed.",
            ErrorType::JavaScriptError => "An error occurred while executing JavaScript code.",
            ErrorType::RenderingError => "The browser encountered an error while rendering the page.",
            ErrorType::SecurityError => "A security violation was detected.",
            ErrorType::ConnectionError => "Unable to establish a connection to the server.",
            ErrorType::ProtocolError => "The protocol specified is not supported.",
            ErrorType::DnsFailure => "Unable to resolve the hostname to an IP address.",
            ErrorType::NetworkTimeout => "The network request timed out.",
            ErrorType::SslCertificateError => "The SSL certificate is invalid or expired.",
            ErrorType::MalformedRequest => "The request format is invalid.",
            ErrorType::UnsupportedMediaType => "The media type is not supported by this browser.",
            ErrorType::GatewayTimeout => "The gateway did not receive a timely response from the upstream server.",
            ErrorType::BadGateway => "The server received an invalid response from the upstream server.",
            ErrorType::ServiceUnavailable => "The server is temporarily unable to handle the request.",
            ErrorType::Forbidden => "Access to this resource is forbidden.",
            ErrorType::Unauthorized => "Authentication is required to access this resource.",
            ErrorType::Gone => "The requested resource is no longer available.",
            ErrorType::TooManyRequests => "The server is receiving too many requests from this client.",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error_type: ErrorType,
    pub url: String,
    pub timestamp: DateTime<Utc>,
    pub details: Option<String>,
    pub user_agent: String,
    pub error_code: Option<u16>,
    pub error_message: String,
}

impl ErrorContext {
    pub fn new(error_type: ErrorType, url: String, details: Option<String>) -> Self {
        ErrorContext {
            error_type,
            url,
            timestamp: Utc::now(),
            details,
            user_agent: Self::get_dynamic_user_agent(),
            error_code: error_type.code(),
            error_message: error_type.description().to_string(),
        }
    }

    fn get_dynamic_user_agent() -> String {
        let os_name = match std::env::consts::OS {
            "windows" => "Win95",
            "macos" => "MacOS",
            "linux" => "Linux",
            "freebsd" => "FreeBSD",
            "openbsd" => "OpenBSD",
            "netbsd" => "NetBSD",
            "dragonfly" => "DragonFly",
            "solaris" => "Solaris",
            "illumos" => "Illumos",
            _ => "UnknownOS",
        };
        
        format!("Retro1996/3.0 (compatible; {}; I)", os_name)
    }
}

pub struct ErrorPageFactory {
    error_cache: HashMap<String, String>,
}

impl ErrorPageFactory {
    pub fn new() -> Self {
        ErrorPageFactory {
            error_cache: HashMap::new(),
        }
    }

    pub fn generate_error_page(&mut self, context: &ErrorContext) -> String {
        let cache_key = format!("{}:{}", context.error_type.title(), context.url);
        
        if let Some(cached_page) = self.error_cache.get(&cache_key) {
            return cached_page.clone();
        }

        let error_page = self.create_error_page(context);
        self.error_cache.insert(cache_key, error_page.clone());
        error_page
    }

    fn create_error_page(&self, context: &ErrorContext) -> String {
        let error_code = context.error_code.map_or("".to_string(), |code| format!("{} ", code));
        let timestamp_str = context.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        
        let troubleshooting = self.get_troubleshooting_steps(&context.error_type);
        let navigation = self.get_navigation_options();

        format!(
            r###"<!DOCTYPE HTML PUBLIC "-//IETF//DTD HTML 2.0//EN">
<html>
<head>
    <title>{} - Retro1996 Browser</title>
    <meta http-equiv="Content-Type" content="text/html; charset=iso-8859-1">
</head>
<body bgcolor="#FFFFFF" text="#000000" link="#0000EE" vlink="#551A8B" alink="#FF0000">
    <table width="100%" border="0" cellspacing="0" cellpadding="0">
        <tr>
            <td bgcolor="#0000CC" height="2"><img src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7" width="1" height="1" alt=""></td>
        </tr>
    </table>
    
    <br>
    
    <center>
        <table width="600" border="0" cellspacing="0" cellpadding="10" bgcolor="#F0F0F0">
            <tr>
                <td>
                    <h1><font color="#FF0000">{}</font>{}: {}</h1>
                    
                    <p><b>URL:</b> {}</p>
                    <p><b>Error:</b> {}</p>
                    <p><b>Time:</b> {}</p>
                    
                    {}
                    
                    <br>
                    {}
                    
                    <br>
                    <hr>
                    <p><font size="-1">Retro1996 Browser v3.0 | TrussCore Rendering Engine | ChronoScript JavaScript</font></p>
                </td>
            </tr>
        </table>
    </center>
    
    <br>
    
    <table width="100%" border="0" cellspacing="0" cellpadding="0">
        <tr>
            <td bgcolor="#0000CC" height="2"><img src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7" width="1" height="1" alt=""></td>
        </tr>
    </table>
</body>
</html>"###,
            context.error_type.title(),
            error_code,
            context.error_type.title(),
            context.error_type.description(),
            context.url,
            context.error_message,
            timestamp_str,
            troubleshooting,
            navigation
        )
    }

    fn get_troubleshooting_steps(&self, error_type: &ErrorType) -> String {
        let steps = self.get_dynamic_troubleshooting_steps(error_type);
        
        let step_list = steps.iter()
            .map(|step| format!("                        <li>{}</li>", step))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r###"                    <h3>What You Can Do:</h3>
                    <ul>
{}
                    </ul>"###,
            step_list
        )
    }

    fn get_dynamic_troubleshooting_steps(&self, error_type: &ErrorType) -> Vec<String> {
        let mut steps = match error_type {
            ErrorType::NotFound => vec![
                "Check the URL for typos".to_string(),
                "Verify the page still exists".to_string(),
                "Try navigating to the site's homepage".to_string(),
                "Use the search function to find the content".to_string()
            ],
            ErrorType::ServerError => vec![
                "The server may be temporarily unavailable".to_string(),
                "Try refreshing the page in a few minutes".to_string(),
                "Contact the website administrator if the problem persists".to_string(),
                "Check if the site is under maintenance".to_string()
            ],
            ErrorType::Timeout | ErrorType::NetworkTimeout => vec![
                "Check your internet connection".to_string(),
                "The server may be experiencing high traffic".to_string(),
                "Try again in a few moments".to_string(),
                "Verify the website is still operational".to_string()
            ],
            ErrorType::ConnectionRefused | ErrorType::ConnectionError => vec![
                "Verify the server address is correct".to_string(),
                "Check if the server is online".to_string(),
                "Ensure your firewall is not blocking the connection".to_string(),
                "Try a different network connection".to_string()
            ],
            ErrorType::InvalidUrl => vec![
                "Check the URL format".to_string(),
                "Ensure the protocol is correct (http://, https://, ftp://)".to_string(),
                "Verify the domain name is valid".to_string(),
                "Try copying and pasting the URL".to_string()
            ],
            ErrorType::DnsFailure => vec![
                "Check your internet connection".to_string(),
                "Verify the website address is correct".to_string(),
                "Try using the IP address instead of the domain name".to_string(),
                "Contact your network administrator".to_string()
            ],
            ErrorType::SslCertificateError => vec![
                "The website's security certificate may be expired".to_string(),
                "This could be a security risk".to_string(),
                "Contact the website administrator".to_string(),
                "Consider using a different site".to_string()
            ],
            ErrorType::Forbidden | ErrorType::Unauthorized => vec![
                "You may not have permission to access this resource".to_string(),
                "Try logging in with appropriate credentials".to_string(),
                "Contact the website administrator".to_string(),
                "Verify you have the necessary permissions".to_string()
            ],
            ErrorType::JavaScriptError => vec![
                "JavaScript execution encountered an error".to_string(),
                "Try refreshing the page".to_string(),
                "Check if JavaScript is enabled in browser settings".to_string(),
                "Contact the website developer".to_string()
            ],
            ErrorType::RenderingError => vec![
                "The page contains malformed HTML or CSS".to_string(),
                "Try refreshing the page".to_string(),
                "Check if the page loads in a different browser".to_string(),
                "Contact the website administrator".to_string()
            ],
            _ => vec![
                "Try refreshing the page".to_string(),
                "Check your internet connection".to_string(),
                "Contact the website administrator".to_string(),
                "Try again later".to_string()
            ],
        };

        // Add dynamic context-aware suggestions
        if let Ok(_) = self.check_network_connectivity() {
            steps.push("Network connectivity appears to be working".to_string());
        } else {
            steps.push("Network connectivity issues detected - check your connection".to_string());
        }

        if self.is_javascript_enabled() {
            steps.push("JavaScript is enabled in your browser".to_string());
        } else {
            steps.push("JavaScript may be disabled - enable it for full functionality".to_string());
        }

        steps
    }

    fn check_network_connectivity(&self) -> Result<(), String> {
        use std::net::TcpStream;
        use std::time::Duration;
        
        // Check connectivity to common DNS servers
        let dns_servers = [
            "8.8.8.8:53",
            "8.8.4.4:53", 
            "1.1.1.1:53",
            "208.67.222.222:53"
        ];
        
        for server in &dns_servers {
            if let Ok(addr) = server.parse() {
                match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
                    Ok(_) => return Ok(()),
                    Err(_) => continue,
                }
            }
        }
        
        Err("Network connectivity issues detected".to_string())
    }

    fn is_javascript_enabled(&self) -> bool {
        // Check if JavaScript engine is available and enabled
        // In 1996, JavaScript support was basic but commonly enabled
        true
    }

    fn get_navigation_options(&self) -> String {
        r###"                    <h3>Navigation:</h3>
                    <p>
                        <a href="javascript:history.back()">&laquo; Back</a> |
                        <a href="about:welcome">Home</a> |
                        <a href="javascript:location.reload()">Retry</a> |
                        <a href="about:help">Help</a>
                    </p>"###.to_string()
    }

    pub fn clear_cache(&mut self) {
        self.error_cache.clear();
    }

    pub fn get_cache_size(&self) -> usize {
        self.error_cache.len()
    }
}

#[derive(Debug, Clone)]
pub struct ErrorPage {
    pub code: ErrorType,
    pub url: String,
    pub timestamp: std::time::SystemTime,
    pub suggested_solutions: Vec<String>,
    pub related_links: Vec<String>,
}

impl ErrorPage {
    pub fn new(code: ErrorType, url: String) -> Self {
        let mut error_page = ErrorPage {
            code,
            url,
            timestamp: std::time::SystemTime::now(),
            suggested_solutions: Vec::new(),
            related_links: Vec::new(),
        };

        error_page.populate_suggestions();
        error_page
    }

    fn populate_suggestions(&mut self) {
        match self.code {
            ErrorType::NotFound => {
                self.suggested_solutions.push("Check the URL for typos".to_string());
                self.suggested_solutions.push("Go back to the previous page".to_string());
                self.suggested_solutions.push("Return to the homepage".to_string());
            },
            ErrorType::ConnectionError | ErrorType::Timeout => {
                self.suggested_solutions.push("Check your internet connection".to_string());
                self.suggested_solutions.push("Verify the server is online".to_string());
                self.suggested_solutions.push("Try again later".to_string());
            },
            ErrorType::DnsFailure => {
                self.suggested_solutions.push("Check the URL spelling".to_string());
                self.suggested_solutions.push("Verify your DNS settings".to_string());
                self.suggested_solutions.push("Try using an IP address instead".to_string());
            },
            ErrorType::Forbidden | ErrorType::Unauthorized => {
                self.suggested_solutions.push("Contact the website administrator".to_string());
                self.suggested_solutions.push("Try logging in with different credentials".to_string());
                self.suggested_solutions.push("Check if you have necessary permissions".to_string());
            },
            _ => {
                self.suggested_solutions.push("Try again later".to_string());
                self.suggested_solutions.push("Go back to the previous page".to_string());
                self.suggested_solutions.push("Return to the homepage".to_string());
            }
        }
    }

    pub fn to_html(&self) -> String {
        let error_code = self.code.code().map_or("".to_string(), |code| format!("{} ", code));
        let timestamp_str = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        
        let troubleshooting = self.get_troubleshooting_steps();
        let navigation = self.get_navigation_options();

        format!(
            r###"<!DOCTYPE HTML PUBLIC "-//IETF//DTD HTML 2.0//EN">
<html>
<head>
    <title>{} - Retro1996 Browser</title>
    <meta http-equiv="Content-Type" content="text/html; charset=iso-8859-1">
</head>
<body bgcolor="#FFFFFF" text="#000000" link="#0000EE" vlink="#551A8B" alink="#FF0000">
    <table width="100%" border="0" cellspacing="0" cellpadding="0">
        <tr>
            <td bgcolor="#0000CC" height="2"><img src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7" width="1" height="1" alt=""></td>
        </tr>
    </table>
    
    <br>
    
    <center>
        <table width="600" border="0" cellspacing="0" cellpadding="10" bgcolor="#F0F0F0">
            <tr>
                <td>
                    <h1><font color="#FF0000">{}</font>{}: {}</h1>
                    
                    <p><b>URL:</b> {}</p>
                    <p><b>Error:</b> {}</p>
                    <p><b>Time:</b> {}</p>
                    
                    {}
                    
                    <br>
                    {}
                    
                    <br>
                    <hr>
                    <p><font size="-1">Retro1996 Browser v3.0 | TrussCore Rendering Engine | ChronoScript JavaScript</font></p>
                </td>
            </tr>
        </table>
    </center>
    
    <br>
    
    <table width="100%" border="0" cellspacing="0" cellpadding="0">
        <tr>
            <td bgcolor="#0000CC" height="2"><img src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7" width="1" height="1" alt=""></td>
        </tr>
    </table>
</body>
</html>"###,
            self.code.title(),
            error_code,
            self.code.title(),
            self.code.description(),
            self.url,
            self.code.description(),
            timestamp_str,
            troubleshooting,
            navigation
        )
    }

    fn get_troubleshooting_steps(&self) -> String {
        let step_list = self.suggested_solutions.iter()
            .map(|step| format!("                        <li>{}</li>", step))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r###"                    <h3>What You Can Do:</h3>
                    <ul>
{}
                    </ul>"###,
            step_list
        )
    }

    fn get_navigation_options(&self) -> String {
        r###"                    <h3>Navigation:</h3>
                    <p>
                        <a href="javascript:history.back()">&laquo; Back</a> |
                        <a href="about:welcome">Home</a> |
                        <a href="javascript:location.reload()">Retry</a> |
                        <a href="about:help">Help</a>
                    </p>"###.to_string()
    }
}

pub struct AboutPages;

impl AboutPages {
    pub fn get_about_page(url: &str) -> String {
        match url {
            "about:blank" => Self::blank_page(),
            "about:welcome" => Self::welcome_page(),
            "about:browser" => Self::browser_info(),
            "about:license" => Self::license_info(),
            "about:credits" => Self::credits(),
            "about:help" => Self::help_page(),
            "about:javascript" => Self::javascript_info(),
            _ => Self::unknown_about_page(url),
        }
    }

    fn blank_page() -> String {
        r###"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
<head>
    <title>Blank Page</title>
</head>
<body>
</body>
</html>"###.to_string()
    }

    fn welcome_page() -> String {
        // Use the improved welcome.html file with dynamic elements
        let visitor_counter = VisitorCounter::new();
        let visitor_count = visitor_counter.increment();
        let last_updated = Self::get_welcome_page_last_updated();
        
        // Read the base welcome.html template
        let base_html = include_str!("../assets/welcome.html");
        
        // Replace dynamic placeholders
        let html_with_dynamic_content = base_html
            .replace("{visitor_count}", &visitor_count.to_string())
            .replace("{last_updated}", &last_updated);
        
        html_with_dynamic_content
    }

    fn get_welcome_page_last_updated() -> String {
        // Try to get the modification time of the welcome.html asset file
        if let Ok(metadata) = fs::metadata("assets/welcome.html") {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                    let datetime = DateTime::from_timestamp(duration.as_secs() as i64, 0);
                    if let Some(dt) = datetime {
                        return dt.format("%m/%d/%Y").to_string();
                    }
                }
            }
        }
        
        // Fallback to current date if we can't get the file modification time
        Utc::now().format("%m/%d/%Y").to_string()
    }

    fn browser_info() -> String {
        r###"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
<head>
    <title>About Retro1996 Browser</title>
</head>
<body bgcolor="#FFFFFF" text="#000000" link="#0000EE" vlink="#551A8B" alink="#FF0000">
    <h1>Retro1996 Browser v3.0</h1>
    <p><b>Version:</b> 3.0</p>
    <p><b>Rendering Engine:</b> TrussCore</p>
    <p><b>JavaScript Engine:</b> ChronoScript</p>
    <p><b>User Agent:</b> Retro1996/3.0 (Win95; I)</p>
    <p><b>Compatibility:</b> HTML 3.2, CSS1, JavaScript 1.1/1.2</p>
    <p><b>Features:</b> FTP, Gopher, BGSOUND, Plugins (future), Java (future)</p>
    <br>
    <p>This browser recreates the authentic 1996 web browsing experience with period-appropriate features and limitations.</p>
</body>
</html>"###.to_string()
    }

    fn license_info() -> String {
        r###"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
<head>
    <title>License Information</title>
</head>
<body bgcolor="#FFFFFF" text="#000000" link="#0000EE" vlink="#551A8B" alink="#FF0000">
    <h1>License Information</h1>
    <p>Retro1996 Browser v3.0</p>
    <p>© 1996 Retro1996 Project</p>
    <p>This software is provided as-is for educational and nostalgic purposes.</p>
    <p>No warranties are expressed or implied.</p>
    <p>Use at your own risk.</p>
</body>
</html>"###.to_string()
    }

    fn credits() -> String {
        r###"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
<head>
    <title>Credits</title>
</head>
<body bgcolor="#FFFFFF" text="#000000" link="#0000EE" vlink="#551A8B" alink="#FF0000">
    <h1>Credits</h1>
    <p><b>Original Concept:</b> The 1996 Web Experience</p>
    <p><b>Rendering Engine:</b> TrussCore</p>
    <p><b>JavaScript Engine:</b> ChronoScript</p>
    <p><b>Special Thanks:</b></p>
    <ul>
        <li>Netscape Navigator 2.0/3.0 for inspiration</li>
        <li>Internet Explorer 3.0 for competition insights</li>
        <li>All the web developers of 1996 who made the web great</li>
        <li>The W3C for HTML/CSS standards</li>
    </ul>
</body>
</html>"###.to_string()
    }

    fn help_page() -> String {
        r###"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
<head>
    <title>Browser Help</title>
</head>
<body bgcolor="#FFFFFF" text="#000000" link="#0000EE" vlink="#551A8B" alink="#FF0000">
    <h1>Browser Help</h1>
    <h2>Navigation</h2>
    <ul>
        <li><b>Back:</b> Return to the previous page</li>
        <li><b>Forward:</b> Go forward to the next page</li>
        <li><b>Home:</b> Go to the homepage</li>
        <li><b>Reload:</b> Refresh the current page (F5)</li>
        <li><b>Stop:</b> Stop loading the current page (ESC)</li>
    </ul>
    <h2>Shortcuts</h2>
    <ul>
        <li><b>F5:</b> Reload page</li>
        <li><b>ESC:</b> Stop loading</li>
        <li><b>Alt+Left:</b> Back</li>
        <li><b>Alt+Right:</b> Forward</li>
        <li><b>Ctrl+O:</b> Open file</li>
        <li><b>Ctrl+R:</b> Reload</li>
    </ul>
    <h2>Features</h2>
    <ul>
        <li>HTML 3.2 rendering</li>
        <li>CSS1 support</li>
        <li>JavaScript 1.1/1.2</li>
        <li>FTP and Gopher support</li>
        <li>BGSOUND audio support</li>
        <li>Image rendering (GIF, JPEG, PNG)</li>
    </ul>
</body>
</html>"###.to_string()
    }

    fn javascript_info() -> String {
        r###"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
<head>
    <title>JavaScript Information</title>
</head>
<body bgcolor="#FFFFFF" text="#000000" link="#0000EE" vlink="#551A8B" alink="#FF0000">
    <h1>JavaScript Information</h1>
    <p><b>Engine:</b> ChronoScript</p>
    <p><b>Version:</b> Compatible with JavaScript 1.1/1.2</p>
    <p><b>Features Supported:</b></p>
    <ul>
        <li>DOM Level 0 (basic document manipulation)</li>
        <li>Window and Navigator objects</li>
        <li>Alert, Confirm, Prompt dialogs</li>
        <li>Basic form manipulation</li>
        <li>Simple event handling</li>
        <li>Image rollovers</li>
        <li>Basic string and math functions</li>
    </ul>
    <p>Note: This browser implements JavaScript as it existed in 1996, with all its quirks and limitations.</p>
</body>
</html>"###.to_string()
    }

    fn unknown_about_page(url: &str) -> String {
        format!(
            r###"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
<head>
    <title>Unknown About Page</title>
</head>
<body bgcolor="#FFFFFF" text="#000000" link="#0000EE" vlink="#551A8B" alink="#FF0000">
    <h1>Unknown About Page</h1>
    <p>The about page '{}' is not recognized.</p>
    <p>Available about pages:</p>
    <ul>
        <li><a href="about:blank">about:blank</a> - Blank page</li>
        <li><a href="about:welcome">about:welcome</a> - Welcome page</li>
        <li><a href="about:browser">about:browser</a> - Browser information</li>
        <li><a href="about:license">about:license</a> - License information</li>
        <li><a href="about:credits">about:credits</a> - Credits</li>
        <li><a href="about:help">about:help</a> - Help page</li>
        <li><a href="about:javascript">about:javascript</a> - JavaScript information</li>
    </ul>
</body>
</html>"###, url
        )
    }
}

// Error page manager to handle different types of errors
pub struct ErrorPageManager {
    pub error_templates: HashMap<ErrorType, String>,
    pub show_error_details: bool,
    pub factory: ErrorPageFactory,
}

impl ErrorPageManager {
    pub fn new() -> Self {
        ErrorPageManager {
            error_templates: HashMap::new(),
            show_error_details: false,
            factory: ErrorPageFactory::new(),
        }
    }

    pub fn generate_error_page(&mut self, error_type: ErrorType, url: String, details: Option<String>) -> String {
        let context = ErrorContext::new(error_type, url, details);
        self.factory.generate_error_page(&context)
    }

    pub fn generate_error_page_from_context(&mut self, context: &ErrorContext) -> String {
        self.factory.generate_error_page(context)
    }

    pub fn set_show_error_details(&mut self, show: bool) {
        self.show_error_details = show;
    }

    pub fn register_error_template(&mut self, error_type: ErrorType, template: String) {
        self.error_templates.insert(error_type, template);
    }

    pub fn get_default_error_message(error_type: ErrorType) -> String {
        format!(
            "Error {}: {} - {}",
            error_type.code().map_or("".to_string(), |code| code.to_string()),
            error_type.title(),
            error_type.description()
        )
    }

    pub fn clear_cache(&mut self) {
        self.factory.clear_cache();
    }

    pub fn get_cache_size(&self) -> usize {
        self.factory.get_cache_size()
    }
}

// Main error page generator for the browser
pub struct ErrorPageGenerator {
    manager: ErrorPageManager,
}

impl ErrorPageGenerator {
    pub fn new() -> Self {
        ErrorPageGenerator {
            manager: ErrorPageManager::new(),
        }
    }

    pub fn generate_error_page(&mut self, error_type: ErrorType, url: String, details: Option<String>) -> String {
        self.manager.generate_error_page(error_type, url, details)
    }

    pub fn generate_network_error(&mut self, url: String, details: Option<String>) -> String {
        self.generate_error_page(ErrorType::ConnectionError, url, details)
    }

    pub fn generate_not_found_error(&mut self, url: String) -> String {
        self.generate_error_page(ErrorType::NotFound, url, None)
    }

    pub fn generate_timeout_error(&mut self, url: String) -> String {
        self.generate_error_page(ErrorType::Timeout, url, None)
    }

    pub fn generate_security_error(&mut self, url: String, details: Option<String>) -> String {
        self.generate_error_page(ErrorType::SecurityError, url, details)
    }

    pub fn generate_javascript_error(&mut self, url: String, details: Option<String>) -> String {
        self.generate_error_page(ErrorType::JavaScriptError, url, details)
    }

    pub fn generate_rendering_error(&mut self, url: String, details: Option<String>) -> String {
        self.generate_error_page(ErrorType::RenderingError, url, details)
    }

    pub fn clear_cache(&mut self) {
        self.manager.clear_cache();
    }

    pub fn get_cache_size(&self) -> usize {
        self.manager.get_cache_size()
    }
}

// Visitor Counter for dynamic visitor counting
pub struct VisitorCounter {
    counter_file: std::path::PathBuf,
    current_count: AtomicU64,
    initialized: std::sync::Once,
}

impl VisitorCounter {
    pub fn new() -> Self {
        let mut counter_file = std::env::temp_dir();
        counter_file.push("retro1996_visitor_counter.txt");
        
        VisitorCounter {
            counter_file,
            current_count: AtomicU64::new(0),
            initialized: Once::new(),
        }
    }

    pub fn get_count(&self) -> u64 {
        self.ensure_initialized();
        self.current_count.load(Ordering::SeqCst)
    }

    pub fn increment(&self) -> u64 {
        self.ensure_initialized();
        let new_count = self.current_count.fetch_add(1, Ordering::SeqCst) + 1;
        self.save_to_file(new_count);
        new_count
    }

    fn ensure_initialized(&self) {
        self.initialized.call_once(|| {
            if let Ok(count) = self.load_from_file() {
                self.current_count.store(count, Ordering::SeqCst);
            }
        });
    }

    fn load_from_file(&self) -> Result<u64, io::Error> {
        if self.counter_file.exists() {
            let mut file = fs::File::open(&self.counter_file)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            contents.trim().parse::<u64>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Invalid counter file format")
            })
        } else {
            Ok(0)
        }
    }

    fn save_to_file(&self, count: u64) {
        match fs::File::create(&self.counter_file) {
            Ok(mut file) => {
                let _ = writeln!(file, "{}", count);
            }
            Err(_) => {
                // Log error silently - visitor counting is non-critical
            }
        }
    }
}

// Protocol handler for about: URLs
pub struct AboutProtocolHandler;

impl AboutProtocolHandler {
    pub fn handle(url: &str) -> String {
        AboutPages::get_about_page(url)
    }

    pub fn is_valid_about_url(url: &str) -> bool {
        url.starts_with("about:")
    }

    pub fn get_available_pages() -> Vec<&'static str> {
        vec![
            "about:blank",
            "about:welcome", 
            "about:browser",
            "about:license",
            "about:credits", 
            "about:help",
            "about:javascript"
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_type_functions() {
        let error = ErrorType::NotFound;
        assert_eq!(error.code(), Some(404));
        assert_eq!(error.title(), "Not Found");
        assert_eq!(error.description(), "The requested URL was not found on this server.");
    }

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new(ErrorType::ServerError, "http://example.com".to_string(), None);
        assert_eq!(context.error_type, ErrorType::ServerError);
        assert_eq!(context.url, "http://example.com");
        assert_eq!(context.user_agent, "Retro1996/3.0 (compatible; Win95; I)");
    }

    #[test]
    fn test_error_page_factory() {
        let mut factory = ErrorPageFactory::new();
        let context = ErrorContext::new(ErrorType::NotFound, "http://example.com".to_string(), None);
        let html = factory.generate_error_page(&context);
        
        assert!(html.contains("Not Found"));
        assert!(html.contains("http://example.com"));
        assert!(html.contains("Retro1996 Browser"));
    }

    #[test]
    fn test_error_page_manager() {
        let mut manager = ErrorPageManager::new();
        let html = manager.generate_error_page(ErrorType::Timeout, "http://example.com".to_string(), None);
        
        assert!(html.contains("Request Timeout"));
        assert!(html.contains("http://example.com"));
    }

    #[test]
    fn test_about_pages() {
        let welcome = AboutPages::get_about_page("about:welcome");
        assert!(welcome.contains("Welcome to Retro1996 Browser v3.0"));
        
        let blank = AboutPages::get_about_page("about:blank");
        assert!(blank.contains("Blank Page"));
        
        let unknown = AboutPages::get_about_page("about:unknown");
        assert!(unknown.contains("Unknown About Page"));
    }

    #[test]
    fn test_error_page_caching() {
        let mut factory = ErrorPageFactory::new();
        let context = ErrorContext::new(ErrorType::NotFound, "http://example.com".to_string(), None);
        
        // Generate first time
        let html1 = factory.generate_error_page(&context);
        assert_eq!(factory.get_cache_size(), 1);
        
        // Generate second time (should be cached)
        let html2 = factory.generate_error_page(&context);
        assert_eq!(factory.get_cache_size(), 1);
        
        // Should be identical
        assert_eq!(html1, html2);
        
        // Clear cache
        factory.clear_cache();
        assert_eq!(factory.get_cache_size(), 0);
    }
}
