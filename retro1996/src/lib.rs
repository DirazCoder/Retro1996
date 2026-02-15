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

// Re-export key types for easier use
pub use engine::{TrussCore, EngineConfig, RenderingMode, DisplayList, DomNode, RenderNode, CssRule, CssValue, QuirkProfile};
pub use network::{NetworkManager, HttpClient, FtpClient, GopherClient, TcpClient, HttpResponse};
pub use ui::{Retro1996Browser, BrowserState, HistoryEntry, Bookmark, FindState, PrintSettings};
pub use javascript_engine::{ChronoScript, JsValue, JsError};
pub use cache::HybridCache;
pub use history::HistoryManager;
pub use bookmarks::BookmarkManager;
pub use preferences::Preferences;
pub use forms::FormManager;
pub use downloads::DownloadManager;
pub use animation::AnimationManager;
pub use charset::CharsetDetector;
pub use mailto::MailtoHandler;
pub use search::SearchManager;
pub use accessibility::AccessibilityManager;
pub use accessibility_utils::AccessibilityUtils;
pub use sound::SoundManager;
pub use dpi_awareness::DpiAwareness;
pub use binary_cache::BinaryCache;
pub use binary_history::BinaryHistory;
pub use plugins::PluginManager;
pub use security::SecurityManager;
pub use print::PrintManager;
pub use error_pages::ErrorPageGenerator;
pub use image_handler::ImageHandler;
pub use ini_preferences::IniPreferences;

// Browser configuration and constants
pub const USER_AGENT: &str = "Mozilla/3.0 (Windows 95; TrussCore/1.0; Rust; en)";
pub const DEFAULT_VIEWPORT_WIDTH: f32 = 800.0;
pub const DEFAULT_VIEWPORT_HEIGHT: f32 = 600.0;
pub const WEB_SAFE_PALETTE_SIZE: usize = 216;

/// Initialize a complete Retro1996 browser instance
pub fn create_browser() -> Retro1996Browser {
    let engine = TrussCore::new();
    let js_engine = ChronoScript::new();
    let network = NetworkManager::new();
    Retro1996Browser::new(engine, js_engine, network)
}

/// Test function to verify compilation and basic functionality
pub fn test_compilation() {
    println!("Retro1996 Browser - Compilation test passed!");
    println!("User Agent: {}", USER_AGENT);
    println!("Web Safe Palette: {} colors", WEB_SAFE_PALETTE_SIZE);
    
    // Test basic engine creation
    let engine = TrussCore::new();
    println!("TrussCore engine initialized successfully");
    
    // Test JavaScript engine
    let js_engine = ChronoScript::new();
    println!("ChronoScript JavaScript engine initialized successfully");
    
    // Test network manager
    let network = NetworkManager::new();
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
        env!("BUILD_DATE", "unknown"),
        env!("BUILD_TIME", "unknown")
    )
}
