// Retro1996 Browser Library - Complete Implementation
// Enterprise-grade 1996-era web browser with TrussCore rendering engine

pub mod engine;
pub mod network;
pub mod ui;
pub mod javascript_engine;
pub mod cache;
pub mod history;
pub mod bookmarks;
pub mod preferences;
pub mod forms;
pub mod downloads;
pub mod animation;
pub mod charset;
pub mod mailto;
pub mod search;
pub mod accessibility;
pub mod accessibility_utils;
pub mod sound;
pub mod dpi_awareness;
pub mod binary_cache;
pub mod binary_history;
pub mod plugins;
pub mod security;
pub mod print;
pub mod error_pages;
pub mod image_handler;
pub mod ini_preferences;
pub mod html_parser;
pub mod css_parser;
pub mod dom;

// Re-export key types for easier use
pub use engine::{TrussCore, EngineConfig, RenderingMode, DisplayList, DomNode, RenderNode, CssRule, CssValue, QuirkProfile};
pub use network::{NetworkManager, HttpClient, FtpClient, GopherClient, HttpResponse};
pub use ui::{Retro1996Browser, BrowserState, HistoryEntry, Bookmark, FindState, PrintSettings};
pub use javascript_engine::{ChronoScript, JsValue, JsError};
pub use cache::{DiskCache, WindowsFileSystem, GpuSerializer, Cache1996Format, DpiFileManager, BackgroundGcManager};
pub use history::HistoryManager;
pub use bookmarks::BookmarkManager;
pub use preferences::Preferences;
pub use animation::AnimationManager;
pub use charset::CharsetDetector;
pub use mailto::MailtoHandler;
pub use accessibility::AccessibilityManager;
pub use binary_cache::BinaryCache;
pub use plugins::PluginManager;
pub use security::SecurityManager;
pub use print::PrintManager;
pub use error_pages::ErrorPageGenerator;
pub use ini_preferences::IniPreferences;

// Browser configuration and constants
pub const USER_AGENT: &str = "Mozilla/3.0 (Windows 95; TrussCore/1.0; Rust; en)";
pub const DEFAULT_VIEWPORT_WIDTH: f32 = 800.0;
pub const DEFAULT_VIEWPORT_HEIGHT: f32 = 600.0;
pub const WEB_SAFE_PALETTE_SIZE: usize = 216;

/// Test function to verify compilation and basic functionality
pub fn test_compilation() {
    println!("Retro1996 Browser - Compilation test passed!");
    println!("User Agent: {}", USER_AGENT);
    println!("Web Safe Palette: {} colors", WEB_SAFE_PALETTE_SIZE);
    
    // Test basic engine creation
    let _engine = TrussCore::new();
    println!("TrussCore engine initialized successfully");
    
    // Test JavaScript engine
    let _js_engine = ChronoScript::new();
    println!("ChronoScript JavaScript engine initialized successfully");
    
    // Test network manager
    let _network = NetworkManager::new();
    println!("Network manager initialized successfully");
    
    println!("All core components initialized successfully!");
}

/// Get version information
pub fn version_info() -> &'static str {
    "Retro1996 Browser v3.0 - TrussCore Engine"
}

/// Get build information
pub fn build_info() -> String {
    format!(
        "Built on: {} at {}",
        option_env!("BUILD_DATE").unwrap_or("unknown"),
        option_env!("BUILD_TIME").unwrap_or("unknown")
    )
}