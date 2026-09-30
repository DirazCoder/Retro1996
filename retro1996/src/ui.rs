use std::collections::{VecDeque, HashMap};
use std::time::{Instant, Duration};
use std::path::PathBuf;
use std::error::Error;
use std::fmt;
use std::fs;
use std::thread;
use std::sync::mpsc;
use std::ptr;
use std::slice;

use egui::{self, Color32, FontFamily, FontId, RichText, Stroke, Vec2, Align2, TextureHandle, TextureOptions, Rect, Pos2};
use image::{self, GenericImageView};
use gif::{self, ColorOutput};
use chrono::Utc;
use native_dialog::FileDialog;
use html_escape;

use crate::engine::{TrussCore as Engine, RenderNode, DomNode, SpecialElement, BoxModel};
use crate::javascript_engine::ChronoScript as ChronoScriptEngine;
use crate::network::{NetworkManager as Network, HttpResponse};

const WIN95_BG: Color32 = Color32::from_rgb(192, 192, 192);
const WIN95_DARK_SHADOW: Color32 = Color32::from_rgb(128, 128, 124);
const WIN95_LIGHT_HIGHLIGHT: Color32 = Color32::from_rgb(255, 255, 255);
const WIN95_MEDIUM: Color32 = Color32::from_rgb(212, 208, 200);
const WIN95_TEXT: Color32 = Color32::from_rgb(0, 0, 0);
const WIN95_ACTIVE_CAPTION: Color32 = Color32::from_rgb(0, 0, 128);
const WIN95_INACTIVE_CAPTION: Color32 = Color32::from_rgb(128, 128, 128);

pub const WEB_SAFE_COLORS: [Color32; 216] = {
    const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
        Color32::from_rgb(
            if r == 0 { 0 } else if r == 1 { 85 } else if r == 2 { 170 } else { 255 },
            if g == 0 { 0 } else if g == 1 { 85 } else if g == 2 { 170 } else { 255 },
            if b == 0 { 0 } else if b == 1 { 85 } else if b == 2 { 170 } else { 255 },
        )
    }
    
    let mut colors = [Color32::BLACK; 216];
    let mut i = 0;
    while i < 6 {
        let mut j = 0;
        while j < 6 {
            let mut k = 0;
            while k < 6 {
                let idx = (i * 36) + (j * 6) + k;
                colors[idx] = rgb(i as u8, j as u8, k as u8);
                k += 1;
            }
            j += 1;
        }
        i += 1;
    }
    colors
};

fn to_web_safe_color(r: u8, g: u8, b: u8) -> Color32 {
    fn quantize(v: u8) -> usize {
        if v < 43 { 0 }
        else if v < 128 { 1 }
        else if v < 213 { 2 }
        else { 3 }
    }
    let idx = (quantize(r) * 36) + (quantize(g) * 6) + quantize(b);
    WEB_SAFE_COLORS[idx]
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub html: String,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub name: String,
    pub url: String,
    pub added: Instant,
}

#[derive(Debug, Clone, Default)]
pub struct FindState {
    pub search_text: String,
    pub case_sensitive: bool,
    pub current_match: usize,
    pub total_matches: usize,
    pub visible: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PrintSettings {
    pub orientation: String,
    pub paper_size: String,
    pub margins: (f32, f32, f32, f32),
    pub print_background: bool,
    pub print_images: bool,
}

#[derive(Debug, Clone)]
pub enum BrowserError {
    NetworkError(String),
    ParseError(String),
    AssetError(String),
    JavaScriptError(String),
    FileError(String),
    SystemError(String),
    ImageError(String),
}

impl fmt::Display for BrowserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrowserError::NetworkError(msg) => write!(f, "Network Error: {}", msg),
            BrowserError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
            BrowserError::AssetError(msg) => write!(f, "Asset Error: {}", msg),
            BrowserError::JavaScriptError(msg) => write!(f, "JavaScript Error: {}", msg),
            BrowserError::FileError(msg) => write!(f, "File Error: {}", msg),
            BrowserError::SystemError(msg) => write!(f, "System Error: {}", msg),
            BrowserError::ImageError(msg) => write!(f, "Image Error: {}", msg),
        }
    }
}

impl Error for BrowserError {}

pub struct AssetManager {
    pub browser_icon: Option<TextureHandle>,
    pub throbber_frames: Vec<TextureHandle>,
    pub welcome_page: String,
    pub homepage: String,
    pub blank_page: String,
    pub base_path: PathBuf,
}

impl AssetManager {
    pub fn load(ctx: &egui::Context) -> Result<Self, BrowserError> {
        let base_path = std::env::current_dir()
            .map_err(|e| BrowserError::AssetError(format!("Failed to get current directory: {}", e)))?;
        
        let mut assets = Self {
            browser_icon: None,
            throbber_frames: Vec::new(),
            welcome_page: String::new(),
            homepage: String::new(),
            blank_page: String::new(),
            base_path: base_path.clone(),
        };
        
        assets.load_browser_icon(ctx)?;
        if let Err(e) = assets.load_throbber_gif(ctx) {
            eprintln!("Warning: throbber animation unavailable: {}", e);
        }
        assets.load_html_pages()?;
        
        Ok(assets)
    }
    
    fn load_browser_icon(&mut self, ctx: &egui::Context) -> Result<(), BrowserError> {
        let icon_path = self.base_path.join("assets").join("browsericon.ico");
        if icon_path.exists() {
            match self.load_image_to_texture(&icon_path, ctx) {
                Ok(texture) => self.browser_icon = Some(texture),
                Err(e) => eprintln!("Failed to load browser icon: {}", e),
            }
        }
        Ok(())
    }
    
    
    fn load_throbber_gif(&mut self, ctx: &egui::Context) -> Result<(), BrowserError> {
        let gif_path = self.base_path.join("assets").join("throbber.gif");
        if !gif_path.exists() {
            return Err(BrowserError::AssetError("throbber.gif not found".to_string()));
        }
        
        let file = fs::File::open(&gif_path)
            .map_err(|e| BrowserError::AssetError(format!("Failed to open throbber.gif: {}", e)))?;
        
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(ColorOutput::RGBA);
        
        let mut reader = decoder.read_info(file)
            .map_err(|e| BrowserError::AssetError(format!("Failed to read GIF info: {}", e)))?;
        
        let width = reader.width() as usize;
        let height = reader.height() as usize;
        
        let mut frame_index = 0;
        
        loop {
            let frame = reader.read_next_frame()
                .map_err(|e| BrowserError::AssetError(format!("Failed to read GIF frame: {}", e)))?;
            
            match frame {
                Some(f) => {
                    let texture = ctx.load_texture(
                        format!("throbber_frame_{}", frame_index),
                        egui::ColorImage::from_rgba_unmultiplied([width, height], &f.buffer),
                        TextureOptions::default(),
                    );
                    
                    self.throbber_frames.push(texture);
                    frame_index += 1;
                }
                None => break,
            }
        }
        
        if self.throbber_frames.is_empty() {
            return Err(BrowserError::AssetError("No frames found in throbber.gif".to_string()));
        }
        
        
        Ok(())
    }
    
   
    fn load_html_pages(&mut self) -> Result<(), BrowserError> {
        let welcome_path = self.base_path.join("assets").join("welcome.html");
        let homepage_path = self.base_path.join("assets").join("homepage.html");
        let blank_path = self.base_path.join("assets").join("blank.html");

        self.welcome_page = fs::read_to_string(welcome_path)
            .unwrap_or_else(|_| "<html><body><h1>Retro1996</h1><p>Welcome page not found.</p></body></html>".to_string());

        self.homepage = fs::read_to_string(homepage_path)
            .unwrap_or_else(|_| "<html><body><h1>Retro1996</h1><p>Homepage not found.</p></body></html>".to_string());

        self.blank_page = fs::read_to_string(blank_path)
            .unwrap_or_else(|_| "<html><body></body></html>".to_string());

        Ok(())
    }
    
    
    fn load_image_to_texture(&self, path: &PathBuf, ctx: &egui::Context) -> Result<TextureHandle, BrowserError> {
        let image = image::open(path)
            .map_err(|e| BrowserError::ImageError(format!("Failed to load image {}: {}", path.display(), e)))?;
        
        let (width, height) = image.dimensions();
        let rgba_image = image.to_rgba8();
        
        let texture = ctx.load_texture(
            format!("texture_{}", path.file_name().unwrap_or_default().to_string_lossy()),
            egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba_image),
            TextureOptions::default(),
        );
        
        Ok(texture)
    }
    
    
    pub fn get_page_content(&self, page_type: PageType) -> &str {
        match page_type {
            PageType::Welcome => &self.welcome_page,
            PageType::Homepage => &self.homepage,
            PageType::Blank => &self.blank_page,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PageType {
    Welcome,
    Homepage,
    Blank,
}

pub struct Throbber {
    pub frames: Vec<TextureHandle>,
    pub current_frame: usize,
    pub animation_timer: Instant,
    pub frame_duration: Duration,
    pub is_playing: bool,
    pub position: Option<Pos2>,
}

impl Throbber {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            current_frame: 0,
            animation_timer: Instant::now(),
            frame_duration: Duration::from_millis(100),
            is_playing: false,
            position: None,
        }
    }
    
    pub fn update(&mut self) {
        if !self.is_playing || self.frames.is_empty() {
            return;
        }
        
        let now = Instant::now();
        if now.duration_since(self.animation_timer) >= self.frame_duration {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.animation_timer = now;
        }
    }
    
    pub fn draw(&self, ui: &mut egui::Ui, position: Pos2) {
        if self.is_playing && !self.frames.is_empty() {
            let frame = &self.frames[self.current_frame];
            let size = Vec2::new(32.0, 32.0);
            let rect = Rect::from_center_size(position, size);
            
            ui.painter().image(
                frame.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }
    
    pub fn start_animation(&mut self) {
        self.is_playing = true;
        self.animation_timer = Instant::now();
    }
    
    pub fn stop_animation(&mut self) {
        self.is_playing = false;
        self.current_frame = 0;
    }
    
    pub fn set_frames(&mut self, frames: Vec<TextureHandle>) {
        self.frames = frames;
        self.current_frame = 0;
    }
}

/// Image loading state for tracking asynchronous image downloads
pub struct ImageLoadState {
    pub url: String,
    pub texture: Option<TextureHandle>,
    pub loading: bool,
    pub error: Option<String>,
}

/// Clipboard manager for cross-platform clipboard operations
pub struct ClipboardManager {
    #[cfg(target_os = "windows")]
    hwnd: Option<isize>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            hwnd: None,
        }
    }
    
    /// Copy text to system clipboard
    pub fn copy_text(&self, text: &str) -> Result<(), BrowserError> {
        #[cfg(target_os = "windows")]
        {
            use std::ffi::{CString, OsStr};
            use std::os::windows::ffi::OsStrExt;
            use std::ptr;
            use std::slice;
            
            unsafe {
                // Open clipboard
                if winapi::um::winuser::OpenClipboard(ptr::null_mut()) == 0 {
                    return Err(BrowserError::SystemError("Failed to open clipboard".to_string()));
                }
                
                // Clear clipboard
                if winapi::um::winuser::EmptyClipboard() == 0 {
                    winapi::um::winuser::CloseClipboard();
                    return Err(BrowserError::SystemError("Failed to clear clipboard".to_string()));
                }
                
                // Convert text to UTF-16
                let text_wide: Vec<u16> = OsStr::new(text).encode_wide().chain(std::iter::once(0)).collect();
                let text_size = text_wide.len() * 2;
                
                // Allocate global memory
                let hglobal = winapi::um::winbase::GlobalAlloc(winapi::um::winbase::GMEM_MOVEABLE, text_size);
                if hglobal.is_null() {
                    winapi::um::winuser::CloseClipboard();
                    return Err(BrowserError::SystemError("Failed to allocate clipboard memory".to_string()));
                }
                
                // Lock memory and copy text
                let mem_ptr = winapi::um::winbase::GlobalLock(hglobal);
                if mem_ptr.is_null() {
                    winapi::um::winbase::GlobalFree(hglobal);
                    winapi::um::winuser::CloseClipboard();
                    return Err(BrowserError::SystemError("Failed to lock clipboard memory".to_string()));
                }
                
                ptr::copy_nonoverlapping(text_wide.as_ptr(), mem_ptr as *mut u16, text_wide.len());
                winapi::um::winbase::GlobalUnlock(hglobal);
                
                // Set clipboard data
                if winapi::um::winuser::SetClipboardData(winapi::um::winuser::CF_UNICODETEXT, hglobal).is_null() {
                    winapi::um::winbase::GlobalFree(hglobal);
                    winapi::um::winuser::CloseClipboard();
                    return Err(BrowserError::SystemError("Failed to set clipboard data".to_string()));
                }
                
                winapi::um::winuser::CloseClipboard();
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // For non-Windows platforms, just update status text for now
            // In a full implementation, you would use platform-specific clipboard APIs
        }
        
        Ok(())
    }
    
    /// Paste text from system clipboard
    pub fn paste_text(&self) -> Result<String, BrowserError> {
        #[cfg(target_os = "windows")]
        {
            use std::ffi::OsString;
            use std::os::windows::ffi::OsStringExt;
            
            unsafe {
                if winapi::um::winuser::OpenClipboard(ptr::null_mut()) == 0 {
                    return Err(BrowserError::SystemError("Failed to open clipboard".to_string()));
                }
                
                let hglobal = winapi::um::winuser::GetClipboardData(winapi::um::winuser::CF_UNICODETEXT);
                if hglobal.is_null() {
                    winapi::um::winuser::CloseClipboard();
                    return Err(BrowserError::SystemError("No text data in clipboard".to_string()));
                }
                
                let mem_ptr = winapi::um::winbase::GlobalLock(hglobal);
                if mem_ptr.is_null() {
                    winapi::um::winuser::CloseClipboard();
                    return Err(BrowserError::SystemError("Failed to lock clipboard memory".to_string()));
                }
                
                // Calculate string length (find null terminator)
                let mut len = 0;
                let mut current_ptr = mem_ptr as *const u16;
                while *current_ptr != 0 {
                    len += 1;
                    current_ptr = current_ptr.add(1);
                }
                
                // Create string from clipboard data
                let text_slice = slice::from_raw_parts(mem_ptr as *const u16, len);
                let text = OsString::from_wide(text_slice).to_string_lossy().to_string();
                
                winapi::um::winbase::GlobalUnlock(hglobal);
                winapi::um::winuser::CloseClipboard();
                
                Ok(text)
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // For non-Windows platforms, return empty string for now
            Ok(String::new())
        }
    }
}

pub struct BrowserState {
    pub engine: Engine,
    pub js_engine: ChronoScriptEngine,
    pub network: Network,
    pub assets: AssetManager,
    pub throbber: Throbber,
    pub history: VecDeque<HistoryEntry>,
    pub current_index: usize,
    pub current_url: String,
    pub current_title: String,
    pub is_loading: bool,
    pub load_start_time: Option<Instant>,
    pub loading_progress: f32,
    pub address_input: String,
    pub status_text: String,
    pub hover_url: String,
    pub bookmarks: Vec<Bookmark>,
    pub show_bookmarks: bool,
    pub find_state: FindState,
    pub page_text_content: String,
    pub render_tree: Option<RenderNode>,
    pub last_error: Option<String>,
    pub retry_count: u32,
    pub save_path: String,
    pub print_settings: PrintSettings,
    pub image_cache: HashMap<String, ImageLoadState>,
    pub clipboard_manager: ClipboardManager,
    pub window_state: WindowState,
}

#[derive(Debug, Clone)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
}

impl BrowserState {
    pub fn new(engine: Engine, js_engine: ChronoScriptEngine, network: Network, ctx: &egui::Context) -> Result<Self, BrowserError> {
        let assets = AssetManager::load(ctx)?;
        let mut throbber = Throbber::new();
        throbber.set_frames(assets.throbber_frames.clone());
        
        let mut state = Self {
            engine,
            js_engine,
            network,
            assets,
            throbber,
            history: VecDeque::new(),
            current_index: 0,
            current_url: "about:welcome".to_string(),
            current_title: "New Tab".to_string(),
            is_loading: false,
            load_start_time: None,
            loading_progress: 0.0,
            address_input: String::new(),
            status_text: "Ready".to_string(),
            hover_url: String::new(),
            bookmarks: Vec::new(),
            show_bookmarks: false,
            find_state: FindState::default(),
            page_text_content: String::new(),
            render_tree: None,
            last_error: None,
            retry_count: 0,
            save_path: String::new(),
            print_settings: PrintSettings::default(),
            image_cache: HashMap::new(),
            clipboard_manager: ClipboardManager::new(),
            window_state: WindowState::Normal,
        };
        
        state.load_page(PageType::Welcome)?;
        Ok(state)
    }
    
    pub fn load_page(&mut self, page_type: PageType) -> Result<(), BrowserError> {
        let html = self.assets.get_page_content(page_type).to_string();
        let title = match page_type {
            PageType::Welcome => "Welcome to Retro1996".to_string(),
            PageType::Homepage => "Retro1996 Browser - Home".to_string(),
            PageType::Blank => "Blank Page".to_string(),
        };
        let url = match page_type {
            PageType::Welcome => "about:welcome".to_string(),
            PageType::Homepage => "about:homepage".to_string(),
            PageType::Blank => "about:blank".to_string(),
        };
        
        self.load_page_content(&html, title, url)
    }
    
    
    pub fn load_url(&mut self, url: &str) -> Result<(), BrowserError> {
        let url = url.trim();
        if url.is_empty() {
            return Err(BrowserError::NetworkError("Empty URL provided".to_string()));
        }
        
        self.is_loading = true;
        self.load_start_time = Some(Instant::now());
        self.loading_progress = 0.0;
        self.retry_count = 0;
        self.last_error = None;
        
        self.throbber.start_animation();
        
        self.status_text = format!("Connecting to {}...", url);
        self.current_url = url.to_string();
        self.address_input = url.to_string();
        
        if url.starts_with("about:") {
            return self.handle_about_url(url);
        }
        
        match self.network.fetch(url) {
            Ok(response) => {
                self.handle_response(response, url)?;
                Ok(())
            }
            Err(e) => {
                self.retry_count += 1;
                if self.retry_count < 3 {
                    std::thread::sleep(Duration::from_millis(500));
                    self.load_url(url)
                } else {
                    let error_msg = format!("Failed to load {}: {}", url, e);
                    self.handle_error(BrowserError::NetworkError(error_msg.clone()));
                    Err(BrowserError::NetworkError(error_msg))
                }
            }
        }
    }
    
    
    fn handle_about_url(&mut self, url: &str) -> Result<(), BrowserError> {
        match url {
            "about:blank" => self.load_page(PageType::Blank),
            "about:welcome" => self.load_page(PageType::Welcome),
            "about:homepage" => self.load_page(PageType::Homepage),
            _ => {
                let error_msg = format!("Unknown about: URL: {}", url);
                self.handle_error(BrowserError::NetworkError(error_msg.clone()));
                Err(BrowserError::NetworkError(error_msg))
            }
        }
    }
    
    
    fn handle_response(&mut self, response: HttpResponse, url: &str) -> Result<(), BrowserError> {
        self.is_loading = false;
        self.throbber.stop_animation();
        self.status_text = "Document: Done".to_string();
        
        let html = String::from_utf8_lossy(&response.body).to_string();
        self.engine.load_html(&html);
        self.engine.render(800.0);
        self.current_title = self.extract_title(&html);
        self.current_url = url.to_string();
        
        self.execute_javascript(&html)?;
        
        self.add_to_history(self.current_url.clone(), self.current_title.clone(), html.clone());
        
        self.extract_page_text();
        
        self.render_tree = self.engine.get_render_tree();
        
        Ok(())
    }
    
    
    fn load_page_content(&mut self, html: &str, title: String, url: String) -> Result<(), BrowserError> {
        self.is_loading = false;
        self.throbber.stop_animation();
        self.status_text = "Document: Done".to_string();
        
        self.engine.load_html(html);
        self.engine.render(800.0);
        self.current_title = title;
        self.current_url = url;
        
        self.execute_javascript(html)?;
        
        self.add_to_history(self.current_url.clone(), self.current_title.clone(), html.to_string());
        
        self.extract_page_text();
        
        self.render_tree = self.engine.get_render_tree();
        
        Ok(())
    }
    
    
    fn extract_title(&self, html: &str) -> String {
        match (html.find("<title>"), html.find("</title>")) {
            (Some(start), Some(end)) if end > start + 7 => {
                html[start + 7..end].trim().to_string()
            }
            _ => "Untitled Document".to_string(),
        }
    }
    
    
    fn execute_javascript(&mut self, html: &str) -> Result<(), BrowserError> {
        let mut pos = 0;
        while let Some(script_start) = html[pos..].find(r"<script") {
            pos += script_start + 7;
            if let Some(tag_end) = html[pos..].find('>') {
                pos += tag_end + 1;
                if let Some(script_end) = html[pos..].find(r"</script>") {
                    let script_content = &html[pos..pos+script_end];
                    pos += script_end + 9;
                    if !script_content.trim().is_empty() {
                        self.js_engine.clear_output();
                        match self.js_engine.execute(script_content.as_bytes()) {
                            Ok(_) => {
                                let js_output = self.js_engine.get_output();
                                if !js_output.is_empty() {
                                    if let Some(body_end) = html.rfind(r"</body>") {
                                        let mut new_html = html[..body_end].to_string();
                                        new_html.push_str(&js_output);
                                        new_html.push_str(r"</body></html>");
                                        self.engine.load_html(&new_html);
                                        self.engine.render(800.0);
                                    }
                                }
                            }
                            Err(e) => {
                                self.handle_error(BrowserError::JavaScriptError(format!("JavaScript execution failed: {}", e)));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    fn add_to_history(&mut self, url: String, title: String, html: String) {
        while self.history.len() > self.current_index + 1 {
            self.history.pop_back();
        }
        let entry = HistoryEntry { url, title, html, timestamp: Instant::now() };
        if self.current_index < self.history.len() {
            self.history[self.current_index] = entry;
        } else {
            self.history.push_back(entry);
        }
        self.current_index = self.history.len().saturating_sub(1);
    }
    
    pub fn navigate_back(&mut self) -> Result<(), BrowserError> {
        if self.current_index > 0 && !self.history.is_empty() {
            self.current_index -= 1;
            if let Some(entry) = self.history.get(self.current_index) {
                self.engine.load_html(&entry.html);
                self.engine.render(800.0);
                self.current_url = entry.url.clone();
                self.current_title = entry.title.clone();
                self.address_input = entry.url.clone();
                self.status_text = "Document: Done (from cache)".to_string();
                self.extract_page_text();
                self.render_tree = self.engine.get_render_tree();
                Ok(())
            } else {
                Err(BrowserError::NetworkError("No previous page in history".to_string()))
            }
        } else {
            Err(BrowserError::NetworkError("No previous page in history".to_string()))
        }
    }
    
    pub fn navigate_forward(&mut self) -> Result<(), BrowserError> {
        if self.current_index < self.history.len().saturating_sub(1) {
            self.current_index += 1;
            if let Some(entry) = self.history.get(self.current_index) {
                self.engine.load_html(&entry.html);
                self.engine.render(800.0);
                self.current_url = entry.url.clone();
                self.current_title = entry.title.clone();
                self.address_input = entry.url.clone();
                self.status_text = "Document: Done (from cache)".to_string();
                self.extract_page_text();
                self.render_tree = self.engine.get_render_tree();
                Ok(())
            } else {
                Err(BrowserError::NetworkError("No forward page in history".to_string()))
            }
        } else {
            Err(BrowserError::NetworkError("No forward page in history".to_string()))
        }
    }
    
    pub fn stop_loading(&mut self) {
        if self.is_loading {
            self.is_loading = false;
            self.throbber.stop_animation();
            self.status_text = "Loading stopped by user".to_string();
        } else {
            self.status_text = "No active load to stop".to_string();
        }
    }
    
    pub fn add_bookmark(&mut self, name: String, url: String) {
        self.bookmarks.push(Bookmark { name, url, added: Instant::now() });
        self.status_text = "Bookmark added".to_string();
    }
    
    
    pub fn remove_bookmark(&mut self, index: usize) {
        if index < self.bookmarks.len() {
            self.bookmarks.remove(index);
            self.status_text = "Bookmark removed".to_string();
        }
    }
    
    pub fn trigger_find(&mut self) {
        self.find_state.visible = true;
        self.find_state.search_text.clear();
        self.find_state.current_match = 0;
        self.find_state.total_matches = 0;
    }
    
    pub fn perform_find(&mut self) {
        if self.find_state.search_text.is_empty() {
            self.find_state.total_matches = 0;
            return;
        }
        let search = if self.find_state.case_sensitive {
            self.find_state.search_text.clone()
        } else {
            self.find_state.search_text.to_lowercase()
        };
        let content = if self.find_state.case_sensitive {
            self.page_text_content.clone()
        } else {
            self.page_text_content.to_lowercase()
        };
        self.find_state.total_matches = content.matches(&search).count();
        self.find_state.current_match = self.find_state.current_match.min(self.find_state.total_matches);
    }
    
    
    pub fn find_in_page(&mut self, search_text: &str, case_sensitive: bool) -> Result<(), BrowserError> {
        if search_text.is_empty() {
            return Err(BrowserError::NetworkError("Find: Please enter search text".to_string()));
        }
        
        self.find_state.search_text = search_text.to_string();
        self.find_state.case_sensitive = case_sensitive;
        self.find_state.visible = true;
        
        self.perform_find();
        
        if self.find_state.total_matches > 0 {
            self.status_text = format!("Find: Found {} match{}", 
                self.find_state.total_matches, 
                if self.find_state.total_matches == 1 { "" } else { "es" });
        } else {
            self.status_text = "Find: No matches found".to_string();
        }
        
        Ok(())
    }
    
    
    pub fn find_next(&mut self) -> Result<(), BrowserError> {
        if self.find_state.search_text.is_empty() {
            return Err(BrowserError::NetworkError("Find: No search text entered".to_string()));
        }
        
        if self.find_state.total_matches == 0 {
            return Err(BrowserError::NetworkError("Find: No matches found".to_string()));
        }
        
        self.find_state.current_match = (self.find_state.current_match + 1) % self.find_state.total_matches;
        
        self.status_text = format!("Find: Match {} of {}", 
            self.find_state.current_match + 1, 
            self.find_state.total_matches);
        
        Ok(())
    }
    
    
    pub fn find_previous(&mut self) -> Result<(), BrowserError> {
        if self.find_state.search_text.is_empty() {
            return Err(BrowserError::NetworkError("Find: No search text entered".to_string()));
        }
        
        if self.find_state.total_matches == 0 {
            return Err(BrowserError::NetworkError("Find: No matches found".to_string()));
        }
        
        if self.find_state.current_match > 0 {
            self.find_state.current_match -= 1;
        } else {
            self.find_state.current_match = self.find_state.total_matches.saturating_sub(1);
        }
        
        self.status_text = format!("Find: Match {} of {}", 
            self.find_state.current_match + 1, 
            self.find_state.total_matches);
        
        Ok(())
    }
    
    fn extract_page_text(&mut self) {
        if let Some(ref root) = self.engine.get_render_tree() {
            self.page_text_content = Self::collect_text(root);
        } else {
            self.page_text_content.clear();
        }
    }
    
    
    fn collect_text(node: &RenderNode) -> String {
        let mut text = String::new();
        match &node.dom_node {
            DomNode::Text(t) => {
                text.push_str(t.trim());
                text.push(' ');
            }
            DomNode::Element { children, .. } => {
                for child in children {
                    text.push_str(&Self::collect_text(&RenderNode { 
                        dom_node: child.clone(), 
                        box_model: node.box_model.clone(),
                        special: node.special.clone(),
                        children: Vec::new(),
                        text_content: String::new(),
                        computed_styles: HashMap::new(),
                        table_data: None,
                        z_index: 0,
                        absolute_x: 0.0,
                        absolute_y: 0.0,
                        dirty: false,
                        style_version: 0,
                        layout_version: 0,
                    }));
                }
            }
            DomNode::Comment(_) => {}
        }
        text
    }
    
    pub fn update_animation_state(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.throbber.animation_timer).as_millis();
        if elapsed > 100 {
            self.throbber.update();
        }
        if let Some(_render_tree) = self.engine.get_render_tree() {
            self.engine.update_blink_state();
        }
    }
    
    
    pub fn get_status_text(&self) -> &str {
        if !self.hover_url.is_empty() {
            &self.hover_url
        } else {
            &self.status_text
        }
    }
    
    
    pub fn set_hover_url(&mut self, url: String) {
        self.hover_url = url;
    }
    
    pub fn clear_hover_url(&mut self) {
        self.hover_url.clear();
    }
    
    
    pub fn is_loading(&self) -> bool {
        self.is_loading
    }
    
    
    pub fn get_current_url(&self) -> &str {
        &self.current_url
    }
    
    
    pub fn get_render_tree(&self) -> Option<&RenderNode> {
        // Return the cached render tree
        self.render_tree.as_ref()
    }
    
    
    pub fn handle_error(&mut self, error: BrowserError) {
        self.last_error = Some(error.to_string());
        self.is_loading = false;
        self.throbber.stop_animation();
        self.status_text = format!("Error: {}", error);
        
        eprintln!("Browser Error: {}", error);
        
        let error_html = format!(
            r#"<html><head><title>Error</title></head><body>
                <h1>Browser Error</h1>
                <p><strong>{}</strong></p>
                <p><a href="about:welcome">Return to Welcome Page</a></p>
                <p><a href="javascript:history.back()">Go Back</a></p>
            </body></html>"#,
            html_escape::encode_text(&error.to_string())
        );
        self.engine.load_html(&error_html);
        self.engine.render(800.0);
        self.render_tree = self.engine.get_render_tree();
    }
    
    pub fn print_page(&mut self) {
        let printable_content = self.generate_printable_content();
        let temp_file = std::env::temp_dir().join("retro1996_print.html");
        
        if let Err(e) = fs::write(&temp_file, &printable_content) {
            self.status_text = format!("Print failed: {}", e);
            return;
        }
        
        #[cfg(windows)]
        unsafe {
            use std::ffi::{CString, OsStr};
            use std::os::windows::ffi::OsStrExt;
            use std::ptr;
            
            let file_path = temp_file.to_string_lossy().to_string();
            let file_cstr = CString::new(file_path).unwrap();
            let operation = CString::new("print").unwrap();
            
            let result = winapi::um::shellapi::ShellExecuteA(
                ptr::null_mut(),
                operation.as_ptr() as *const i8,
                file_cstr.as_ptr() as *const i8,
                ptr::null(),
                ptr::null(),
                winapi::um::winuser::SW_HIDE
            );
            
            if result.is_null() || !result.is_null() {
                // ShellExecute returns a value > 32 on success
                let result_value = result as isize;
                if result_value > 32 {
                    self.status_text = "Print job sent to printer".to_string();
                } else {
                    self.status_text = format!("Print failed with error code: {}", result_value);
                }
            } else {
                self.status_text = "Print failed".to_string();
            }
        }
        
        let _ = fs::remove_file(temp_file);
    }
    
    
    fn generate_printable_content(&self) -> String {
        let page_title = &self.current_title;
        let page_url = &self.current_url;
        let current_time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        
        format!(
            r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
<head>
    <title>Printed Page: {}</title>
    <meta http-equiv="Content-Type" content="text/html; charset=iso-8859-1">
    <style>
        body {{ font-family: Arial, sans-serif; margin: 1in; }}
        .print-header {{ border-bottom: 1px solid #000; margin-bottom: 20px; padding-bottom: 10px; }}
        .print-footer {{ border-top: 1px solid #000; margin-top: 20px; padding-top: 10px; font-size: 10px; color: #666; }}
        .page-content {{ line-height: 1.5; }}
    </style>
</head>
<body>
    <div class="print-header">
        <h1>{}</h1>
        <p><strong>Source:</strong> {}</p>
        <p><strong>Printed:</strong> {}</p>
        <p><strong>Browser:</strong> Retro1996/3.0 (Win95; I)</p>
    </div>
    
    <div class="page-content">
        {}
    </div>
    
    <div class="print-footer">
        <p>Printed by Retro1996 Browser v3.0</p>
        <p>Page generated on {}</p>
    </div>
</body>
</html>"#,
            html_escape::encode_text(page_title),
            html_escape::encode_text(page_title),
            html_escape::encode_text(page_url),
            current_time,
            html_escape::encode_text(&self.page_text_content),
            current_time
        )
    }
    
    pub fn save_page(&mut self) {
        let filter = "HTML Files (*.html)\0*.html\0Text Files (*.txt)\0*.txt\0All Files (*.*)\0*.*\0\0";
        
        #[cfg(windows)]
        unsafe {
            use std::ffi::{CString, OsStr};
            use std::os::windows::ffi::OsStrExt;
            use std::ptr;
            use std::slice;
            
            let mut ofn: winapi::um::commdlg::OPENFILENAMEW = std::mem::zeroed();
            let mut file_path = [0u16; 260];
            let mut filter_wide: Vec<u16> = OsStr::new(filter).encode_wide().chain(std::iter::once(0)).collect();
            
            ofn.lStructSize = std::mem::size_of::<winapi::um::commdlg::OPENFILENAMEW>() as u32;
            ofn.hwndOwner = ptr::null_mut();
            ofn.lpstrFilter = filter_wide.as_ptr();
            ofn.lpstrFile = file_path.as_mut_ptr();
            ofn.nMaxFile = file_path.len() as u32;
            ofn.lpstrTitle = OsStr::new("Save Page As").encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>().as_ptr();
            ofn.Flags = winapi::um::commdlg::OFN_OVERWRITEPROMPT | winapi::um::commdlg::OFN_PATHMUSTEXIST;
            
            let result = winapi::um::commdlg::GetSaveFileNameW(&mut ofn);
            
            if result != 0 {
                let file_path_str = String::from_utf16_lossy(&file_path[..file_path.iter().position(|&x| x == 0).unwrap_or(file_path.len())]);
                
                let content_to_save = self.get_page_content_for_save();
                
                if let Err(e) = fs::write(&file_path_str, content_to_save) {
                    self.status_text = format!("Save failed: {}", e);
                } else {
                    self.status_text = format!("Page saved to: {}", file_path_str);
                }
            } else {
                self.status_text = "Save cancelled".to_string();
            }
        }
    }
    
    
    fn get_page_content_for_save(&self) -> String {
        let page_title = &self.current_title;
        let page_url = &self.current_url;
        let current_time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        
        format!(
            r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
<head>
    <title>{}</title>
    <meta http-equiv="Content-Type" content="text/html; charset=iso-8859-1">
    <meta name="generator" content="Retro1996 Browser v3.0">
    <meta name="saved-url" content="{}">
    <meta name="saved-date" content="{}">
</head>
<body>
    <div style="font-family: Arial, sans-serif; margin: 20px;">
        <h1 style="color: #000080;">{}</h1>
        <p><strong>Original URL:</strong> {}</p>
        <p><strong>Saved:</strong> {}</p>
        <hr style="border: 1px solid #000;">
        <div style="line-height: 1.5;">
            {}
        </div>
        <hr style="border: 1px solid #000;">
        <p style="font-size: 10px; color: #666;">Saved by Retro1996 Browser v3.0</p>
    </div>
</body>
</html>"#,
            html_escape::encode_text(page_title),
            html_escape::encode_text(page_url),
            current_time,
            html_escape::encode_text(page_title),
            html_escape::encode_text(page_url),
            current_time,
            html_escape::encode_text(&self.page_text_content)
        )
    }
}

fn apply_win95_style(ctx: &egui::Context) {
    use egui::style::*;
    let mut style = (*ctx.style()).clone();
    
    style.visuals.window_rounding = egui::Rounding::same(0.0);
    style.visuals.window_shadow.extrusion = 0.0;
    style.visuals.window_fill = WIN95_BG;
    style.visuals.window_stroke = Stroke::new(1.0, WIN95_DARK_SHADOW);
    
    style.visuals.widgets.noninteractive.bg_fill = WIN95_BG;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, WIN95_DARK_SHADOW);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, WIN95_TEXT);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(0.0);
    
    style.visuals.widgets.inactive.bg_fill = WIN95_MEDIUM;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, WIN95_DARK_SHADOW);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, WIN95_TEXT);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(0.0);
    
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(220, 220, 220);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, WIN95_DARK_SHADOW);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, WIN95_TEXT);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(0.0);
    
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(180, 180, 180);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, WIN95_DARK_SHADOW);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, WIN95_TEXT);
    style.visuals.widgets.active.rounding = egui::Rounding::same(0.0);
    
    style.visuals.override_text_color = Some(WIN95_TEXT);
    style.visuals.hyperlink_color = Color32::from_rgb(0, 0, 255);
    style.visuals.selection.bg_fill = Color32::from_rgb(0, 0, 128);
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(255, 255, 255));
    
    style.visuals.menu_rounding = egui::Rounding::same(0.0);
    style.visuals.popup_shadow = Default::default();
    
    style.text_styles.insert(egui::TextStyle::Heading, FontId::new(16.0, FontFamily::Proportional));
    style.text_styles.insert(egui::TextStyle::Body, FontId::new(12.0, FontFamily::Proportional));
    style.text_styles.insert(egui::TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace));
    style.text_styles.insert(egui::TextStyle::Button, FontId::new(12.0, FontFamily::Proportional));
    style.text_styles.insert(egui::TextStyle::Small, FontId::new(11.0, FontFamily::Proportional));
    
    // Use default fonts - the custom font file is not available
    let _ = ctx;
    ctx.set_style(style);
}

fn draw_bevel(ui: &mut egui::Ui, size: Vec2, pressed: bool) -> egui::Response {
    let (outer_stroke, inner_stroke) = if pressed {
        (Stroke::new(1.0, WIN95_DARK_SHADOW), Stroke::new(1.0, WIN95_LIGHT_HIGHLIGHT))
    } else {
        (Stroke::new(1.0, WIN95_LIGHT_HIGHLIGHT), Stroke::new(1.0, WIN95_DARK_SHADOW))
    };
    let rect = ui.allocate_rect(egui::Rect::from_min_size(ui.cursor().min, size), egui::Sense::click());
    let painter = ui.painter();
    painter.rect_stroke(rect.rect, 0.0, outer_stroke);
    let inner_rect = rect.rect.shrink(1.0);
    painter.rect_stroke(inner_rect, 0.0, inner_stroke);
    rect
}

fn draw_throbber(ui: &mut egui::Ui, frame: usize) -> egui::Response {
    const GLOBE_FRAMES: [&str; 12] = ["-", "\\", "|", "/", "-", "\\", "|", "/", "-", "\\", "|", "/"];
    ui.colored_label(WIN95_ACTIVE_CAPTION, RichText::new(GLOBE_FRAMES[frame % 12]).size(16.0))
}

/// Load an image from a URL asynchronously
fn load_image_from_url(url: String, tx: mpsc::Sender<Result<(String, TextureHandle), (String, String)>>, ctx: egui::Context, mut network: Network) {
    thread::spawn(move || {
        match network.fetch(&url) {
            Ok(response) => {
                match image::load_from_memory(&response.body) {
                    Ok(img) => {
                        let (width, height) = img.dimensions();
                        let rgba_img = img.to_rgba8();
                        
                        let texture = ctx.load_texture(
                            format!("image_{}", url.replace(|c: char| !c.is_alphanumeric(), "_")),
                            egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba_img),
                            TextureOptions::default(),
                        );
                        
                        if let Err(_) = tx.send(Ok((url, texture))) {
                            eprintln!("Failed to send image texture");
                        }
                    }
                    Err(e) => {
                        if let Err(_) = tx.send(Err((url, format!("Failed to decode image: {}", e)))) {
                            eprintln!("Failed to send image error");
                        }
                    }
                }
            }
            Err(e) => {
                if let Err(_) = tx.send(Err((url, format!("Failed to fetch image: {}", e)))) {
                    eprintln!("Failed to send image error");
                }
            }
        }
    });
}

fn render_to_egui(ui: &mut egui::Ui, node: &RenderNode, state: &mut BrowserState) {
    match &node.dom_node {
        DomNode::Element { tag, attributes, children, .. } => {
            if let SpecialElement::Blink { visible, .. } = &node.special {
                if !visible { return; }
            }
            if tag == "a" {
                if let Some(href) = attributes.get("href") {
                    let label = node.children.iter()
                        .filter_map(|c| if let DomNode::Text(t) = &c.dom_node { Some(t.trim()) } else { None })
                        .collect::<Vec<_>>().join(" ");
                    let response = ui.link(RichText::new(label).color(Color32::from_rgb(0, 0, 255)));
                    if response.hovered() { state.set_hover_url(href.clone()); }
                    if response.clicked() {
                        if href.starts_with("javascript:") {
                            let _ = state.js_engine.execute(href[11..].as_bytes());
                            state.status_text = "JavaScript executed".to_string();
                        } else { 
                            if let Err(e) = state.load_url(href) {
                                state.handle_error(e);
                            }
                        }
                    }
                    return;
                }
            }
            if tag == "img" {
                if let SpecialElement::Image { src, width, height, alt, .. } = &node.special {
                    let w = width.map(|v| v as f32).unwrap_or(100.0);
                    let h = height.map(|v| v as f32).unwrap_or(80.0);
                    
                    // Check if image is already loaded or loading
                    let image_state = state.image_cache.entry(src.clone()).or_insert_with(|| ImageLoadState {
                        url: src.clone(),
                        texture: None,
                        loading: false,
                        error: None,
                    });
                    
                    if !image_state.loading && image_state.texture.is_none() && image_state.error.is_none() {
                        // Start loading the image
                        image_state.loading = true;
                        let (tx, _rx) = mpsc::channel::<Result<(String, TextureHandle), (String, String)>>();
                        let ctx_clone = ui.ctx().clone();
                        let src_clone = src.clone();
                        
                        // Use a thread pool or spawn a thread for image loading
                        thread::spawn(move || {
                            // For now, we'll just create a placeholder texture
                            // In a full implementation, we would need access to the network manager
                            // This is a simplified version that doesn't actually load images from URLs
                            eprintln!("Image loading not fully implemented - would need network access");
                            
                            // Create a placeholder error for now
                            if let Err(_) = tx.send(Err((src_clone, "Image loading not implemented".to_string()))) {
                                eprintln!("Failed to send image error");
                            }
                        });
                    }
                    
                    // Draw the image or placeholder
                    ui.allocate_ui(egui::vec2(w, h), |ui| {
                        let painter = ui.painter();
                        let rect = ui.max_rect();
                        
                        if let Some(texture) = &image_state.texture {
                            // Draw the loaded image
                            ui.painter().image(
                                texture.id(),
                                rect,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        } else if image_state.loading {
                            // Draw loading placeholder
                            painter.rect_filled(rect, 0.0, to_web_safe_color(200, 200, 200));
                            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, WIN95_DARK_SHADOW));
                            ui.centered_and_justified(|ui| {
                                ui.label(RichText::new("Loading image...").size(10.0).color(WIN95_TEXT));
                            });
                        } else if let Some(error) = &image_state.error {
                            // Draw error placeholder
                            painter.rect_filled(rect, 0.0, to_web_safe_color(255, 182, 193)); // Light pink for error
                            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, WIN95_DARK_SHADOW));
                            ui.centered_and_justified(|ui| {
                                ui.label(RichText::new("Image Error").size(10.0).color(WIN95_TEXT));
                                ui.label(RichText::new(format!("{}: {}", src, error)).size(8.0).color(WIN95_INACTIVE_CAPTION));
                            });
                        } else {
                            // Draw default placeholder
                            painter.rect_filled(rect, 0.0, to_web_safe_color(200, 200, 200));
                            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, WIN95_DARK_SHADOW));
                            ui.centered_and_justified(|ui| {
                                ui.label(RichText::new(alt).size(10.0).color(WIN95_TEXT));
                                ui.label(RichText::new(format!("[{}]", src)).size(8.0).color(WIN95_INACTIVE_CAPTION));
                            });
                        }
                    });
                    
                    let response = ui.interact(ui.max_rect(), egui::Id::new(src), egui::Sense::click());
                    if response.clicked() { state.status_text = format!("Image click on {}", src); }
                    return;
                }
            }
            if tag == "hr" {
                ui.add(egui::Separator::default().spacing(10.0));
                return;
            }
            if tag == "br" {
                ui.end_row();
                return;
            }
            if tag.starts_with('h') && tag.len() == 2 && tag.chars().nth(1).unwrap().is_ascii_digit() {
                let level = tag[1..].parse::<u32>().unwrap_or(1);
                let font_size = 24.0 - (level as f32 * 3.0);
                ui.heading(RichText::new(
                    children.iter()
                        .filter_map(|c| if let DomNode::Text(t) = c { Some(t.trim()) } else { None })
                        .collect::<Vec<_>>().join(" ")
                ).size(font_size).color(WIN95_ACTIVE_CAPTION));
                ui.end_row();
                return;
            }
            if tag == "center" {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    for child in children { 
                        let render_child = RenderNode {
                            dom_node: child.clone(),
                            box_model: BoxModel::default(),
                            special: SpecialElement::Unknown,
                            children: Vec::new(),
                            text_content: String::new(),
                            computed_styles: HashMap::new(),
                            table_data: None,
                            z_index: 0,
                            absolute_x: 0.0,
                            absolute_y: 0.0,
                            dirty: false,
                            style_version: 0,
                            layout_version: 0,
                        };
                        render_to_egui(ui, &render_child, state); 
                    }
                });
                return;
            }
            if let Some((r, g, b)) = node.box_model.background_color {
                let bg_color = to_web_safe_color(r, g, b);
                ui.painter().rect_filled(ui.max_rect(), 0.0, bg_color);
            }
            for child in children {
                let render_child = RenderNode {
                    dom_node: child.clone(),
                    box_model: BoxModel::default(),
                    special: SpecialElement::Unknown,
                    children: Vec::new(),
                    text_content: String::new(),
                    computed_styles: HashMap::new(),
                    table_data: None,
                    z_index: 0,
                    absolute_x: 0.0,
                    absolute_y: 0.0,
                    dirty: false,
                    style_version: 0,
                    layout_version: 0,
                };
                render_to_egui(ui, &render_child, state);
            }
        }
        DomNode::Text(text) => {
            let color = node.box_model.color;
            let font_size = 12.0;
            let font_family = FontFamily::Proportional;
            ui.label(RichText::new(text.trim())
                .color(to_web_safe_color(color.0, color.1, color.2))
                .size(font_size)
                .font(FontId::new(font_size, font_family)));
        }
        DomNode::Comment(_) => {}
    }
}

pub struct Retro1996Browser {
    state: BrowserState,
}

impl Retro1996Browser {
    pub fn new(cc: &eframe::CreationContext, engine: Engine, js_engine: ChronoScriptEngine, network: Network) -> Result<Self, BrowserError> {
        apply_win95_style(&cc.egui_ctx);
        let state = BrowserState::new(engine, js_engine, network, &cc.egui_ctx)?;
        Ok(Self { state })
    }
    
    fn draw_title_bar(&self, ctx: &egui::Context) {
        let title_bar_height = 24.0;
        let title_bar_rect = egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(ctx.screen_rect().width(), title_bar_height)
        );
        
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("title_bar")));
        
        painter.rect_filled(title_bar_rect, 0.0, WIN95_ACTIVE_CAPTION);
        
        let title_text = format!("Retro1996 Browser v3.0 - {}", self.state.current_title);
        painter.text(
            egui::pos2(8.0, 4.0),
            egui::Align2::LEFT_TOP,
            title_text,
            egui::FontId::new(12.0, FontFamily::Proportional),
            WIN95_LIGHT_HIGHLIGHT
        );
        
        let control_x = ctx.screen_rect().width() - 90.0;
        let control_y = 2.0;
        
        let min_rect = egui::Rect::from_min_size(egui::pos2(control_x, control_y), egui::vec2(20.0, 20.0));
        self.draw_window_button(&painter, min_rect, "_", false);
        
        let max_rect = egui::Rect::from_min_size(egui::pos2(control_x + 22.0, control_y), egui::vec2(20.0, 20.0));
        self.draw_window_button(&painter, max_rect, "□", false);
        
        let close_rect = egui::Rect::from_min_size(egui::pos2(control_x + 44.0, control_y), egui::vec2(20.0, 20.0));
        self.draw_window_button(&painter, close_rect, "×", true);
    }
    
    fn draw_window_button(&self, painter: &egui::Painter, rect: egui::Rect, symbol: &str, is_close: bool) {
        let outer_stroke = Stroke::new(1.0, WIN95_LIGHT_HIGHLIGHT);
        let inner_stroke = Stroke::new(1.0, WIN95_DARK_SHADOW);
        
        painter.rect_stroke(rect, 0.0, outer_stroke);
        let inner_rect = rect.shrink(1.0);
        painter.rect_stroke(inner_rect, 0.0, inner_stroke);
        
        let bg_color = if is_close { Color32::from_rgb(255, 100, 100) } else { WIN95_BG };
        painter.rect_filled(inner_rect, 0.0, bg_color);
        
        let text_color = if is_close { WIN95_LIGHT_HIGHLIGHT } else { WIN95_TEXT };
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            symbol,
            egui::FontId::new(12.0, FontFamily::Proportional),
            text_color
        );
    }
}

fn draw_bevel_with_feedback(ui: &mut egui::Ui, size: egui::Vec2, disabled: bool) -> egui::Response {
    let (outer_stroke, inner_stroke) = if disabled {
        (Stroke::new(1.0, WIN95_INACTIVE_CAPTION), Stroke::new(1.0, WIN95_DARK_SHADOW))
    } else {
        (Stroke::new(1.0, WIN95_LIGHT_HIGHLIGHT), Stroke::new(1.0, WIN95_DARK_SHADOW))
    };

    let rect = ui.allocate_rect(egui::Rect::from_min_size(ui.cursor().min, size), egui::Sense::click());
    let painter = ui.painter();

    painter.rect_stroke(rect.rect, 0.0, outer_stroke);
    let inner_rect = rect.rect.shrink(1.0);
    painter.rect_stroke(inner_rect, 0.0, inner_stroke);

    if rect.hovered() && !disabled {
        let highlight_rect = rect.rect.expand(1.0);
        painter.rect_stroke(highlight_rect, 0.0, Stroke::new(1.0, Color32::from_gray(200)));
    }

    rect
}

impl Retro1996Browser {
    fn handle_file_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("New Window").clicked() {
            self.state.status_text = "New Window".to_string();
        }
        if ui.button("New Tab").clicked() {
            self.state.status_text = "New Tab".to_string();
        }
        ui.separator();
        if ui.button("Open File...").clicked() {
            if let Ok(Some(path)) = FileDialog::new()
                .add_filter("HTML files", &["html", "htm"])
                .add_filter("Text files", &["txt"])
                .add_filter("All files", &["*"])
                .show_open_single_file() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Err(e) = self.state.load_url(&format!("file://{}", path.display())) {
                        self.state.handle_error(e);
                    }
                }
            }
        }
        if ui.button("Open Location...").clicked() {
            ui.ctx().request_repaint();
        }
        ui.separator();
        if ui.button("Save As...").clicked() {
            self.state.save_page();
        }
        ui.separator();
        if ui.button("Send Page...").clicked() {
            self.state.status_text = "Send Page".to_string();
        }
        ui.separator();
        if ui.button("Page Setup...").clicked() {
            self.state.status_text = "Page Setup".to_string();
        }
        if ui.button("Print Preview...").clicked() {
            self.state.status_text = "Print Preview".to_string();
        }
        if ui.button("Print...").clicked() {
            self.state.print_page();
        }
        ui.separator();
        if ui.button("Close Window").clicked() {
            std::process::exit(0);
        }
        if ui.button("Exit").clicked() {
            std::process::exit(0);
        }
    }

    fn handle_edit_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Undo").clicked() {
            self.state.status_text = "Undo".to_string();
        }
        ui.separator();
        if ui.button("Cut").clicked() {
            self.state.status_text = "Cut".to_string();
        }
        if ui.button("Copy").clicked() {
            self.state.status_text = "Copy".to_string();
        }
        if ui.button("Paste").clicked() {
            self.state.status_text = "Paste".to_string();
        }
        if ui.button("Delete").clicked() {
            self.state.status_text = "Delete".to_string();
        }
        ui.separator();
        if ui.button("Select All").clicked() {
            self.state.status_text = "Select All".to_string();
        }
        ui.separator();
        if ui.button("Find in Page...").clicked() {
            self.state.trigger_find();
        }
        if ui.button("Find Next").clicked() {
            if let Err(e) = self.state.find_next() {
                self.state.handle_error(e);
            }
        }
        if ui.button("Find Previous").clicked() {
            if let Err(e) = self.state.find_previous() {
                self.state.handle_error(e);
            }
        }
    }

    fn handle_view_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Reload").clicked() {
            let current_url = self.state.current_url.clone();
            if !current_url.is_empty() {
                if let Err(e) = self.state.load_url(&current_url) {
                    self.state.handle_error(e);
                }
            }
        }
        if ui.button("Stop").clicked() {
            self.state.stop_loading();
        }
        ui.separator();
        if ui.button("View Page Source").clicked() {
            let source = format!("<html><head><title>Source of {}</title></head><body><pre>{}</pre></body></html>",
                                self.state.current_url,
                                html_escape::encode_text(&self.state.page_text_content));
            self.state.engine.load_html(&source);
            self.state.engine.render(800.0);
            self.state.current_url = String::from("view-source:");
        }
        if ui.button("View Page Info").clicked() {
            let info = format!("<html><body><h1>Page Information</h1><p>URL: {}</p><p>Loading: {}</p><p>History: {}/{}</p></body></html>",
                              self.state.current_url,
                              self.state.is_loading,
                              self.state.current_index + 1,
                              self.state.history.len());
            self.state.engine.load_html(&info);
            self.state.engine.render(800.0);
            self.state.current_url = String::from("page-info:");
        }
        ui.separator();
        ui.menu_button("Character Set", |ui| {
            if ui.button("Western (ISO-8859-1)").clicked() {
                self.state.status_text = "Character Set: Western (ISO-8859-1)".to_string();
            }
            if ui.button("Unicode (UTF-8)").clicked() {
                self.state.status_text = "Character Set: Unicode (UTF-8)".to_string();
            }
        });
        ui.menu_button("Text Size", |ui| {
            if ui.button("Smallest").clicked() {
                self.state.status_text = "Text Size: Smallest".to_string();
            }
            if ui.button("Smaller").clicked() {
                self.state.status_text = "Text Size: Smaller".to_string();
            }
            if ui.button("Medium").clicked() {
                self.state.status_text = "Text Size: Medium".to_string();
            }
            if ui.button("Larger").clicked() {
                self.state.status_text = "Text Size: Larger".to_string();
            }
            if ui.button("Largest").clicked() {
                self.state.status_text = "Text Size: Largest".to_string();
            }
        });
        ui.checkbox(&mut self.state.show_bookmarks, "Text Only Mode");
    }

    fn handle_go_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Back").clicked() {
            if let Err(e) = self.state.navigate_back() {
                self.state.handle_error(e);
            }
        }
        if ui.button("Forward").clicked() {
            if let Err(e) = self.state.navigate_forward() {
                self.state.handle_error(e);
            }
        }
        if ui.button("Home").clicked() {
            if let Err(e) = self.state.load_url("about:welcome") {
                self.state.handle_error(e);
            }
        }
        ui.separator();
        ui.label("Recent Pages:");
        let max_recent = std::cmp::min(10, self.state.history.len());
        let history_clone = self.state.history.clone();
        for i in 0..max_recent {
            if let Some(entry) = history_clone.get(history_clone.len() - 1 - i) {
                if ui.button(&entry.url).clicked() {
                    if let Err(e) = self.state.load_url(&entry.url) {
                        self.state.handle_error(e);
                    }
                }
            }
        }
    }

    fn handle_bookmarks_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Add Bookmark").clicked() {
            let name = if self.state.current_title == "Untitled Document" {
                self.state.current_url.clone()
            } else {
                self.state.current_title.clone()
            };
            self.state.add_bookmark(name, self.state.current_url.clone());
        }
        if ui.button("Manage Bookmarks").clicked() {
            self.state.show_bookmarks = true;
        }
        ui.separator();
        let bookmarks_clone = self.state.bookmarks.clone();
        for bookmark in &bookmarks_clone {
            if ui.button(&bookmark.name).clicked() {
                if let Err(e) = self.state.load_url(&bookmark.url) {
                    self.state.handle_error(e);
                }
            }
        }
    }

    fn handle_tools_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Preferences...").clicked() {
            self.state.status_text = "Preferences".to_string();
        }
        ui.separator();
        if ui.button("Clear Private Data...").clicked() {
            self.state.status_text = "Clear Private Data".to_string();
        }
    }

    fn handle_window_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Minimize").clicked() {
        }
        if ui.button("Maximize").clicked() {
        }
        if ui.button("Restore").clicked() {
        }
    }

    fn handle_help_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Help Contents").clicked() {
            self.state.engine.load_html("<html><body><h1>Retro1996 Browser Help</h1><p>Welcome to Retro1996 Browser!</p><p>This browser simulates the web browsing experience from 1996.</p></body></html>");
            self.state.current_url = String::from("help:");
        }
        ui.separator();
        if ui.button("About Retro1996").clicked() {
            self.state.engine.load_html("<html><body><h1>Retro1996 Browser v3.0</h1><p>TrussCore Rendering Engine</p><p>ChronoScript JavaScript Engine</p><p>User-Agent: Retro1996/3.0 (Win95; I)</p></body></html>");
            self.state.current_url = String::from("about:");
        }
    }
}

impl eframe::App for Retro1996Browser {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.update_animation_state();
        
        self.draw_title_bar(ctx);
        
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.set_height(24.0);
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    self.handle_file_menu(ui);
                });

                ui.menu_button("Edit", |ui| {
                    self.handle_edit_menu(ui);
                });

                ui.menu_button("View", |ui| {
                    self.handle_view_menu(ui);
                });

                ui.menu_button("Go", |ui| {
                    self.handle_go_menu(ui);
                });

                ui.menu_button("Bookmarks", |ui| {
                    self.handle_bookmarks_menu(ui);
                });

                ui.menu_button("Tools", |ui| {
                    self.handle_tools_menu(ui);
                });

                ui.menu_button("Window", |ui| {
                    self.handle_window_menu(ui);
                });

                ui.menu_button("Help", |ui| {
                    self.handle_help_menu(ui);
                });
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: WIN95_BG,
                inner_margin: egui::Margin::same(0.0),
                outer_margin: egui::Margin::same(0.0),
                rounding: egui::Rounding::same(0.0),
                shadow: Default::default(),
                stroke: Stroke::new(1.0, WIN95_DARK_SHADOW),
            })
            .show(ctx, |ui| {
                egui::Frame {
                    fill: WIN95_MEDIUM,
                    inner_margin: egui::Margin::symmetric(4.0, 2.0),
                    outer_margin: egui::Margin::same(0.0),
                    rounding: egui::Rounding::same(0.0),
                    shadow: Default::default(),
                    stroke: Stroke::new(1.0, WIN95_DARK_SHADOW),
                }
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let back_enabled = self.state.current_index > 0 && !self.state.history.is_empty();
                        if draw_bevel_with_feedback(ui, egui::vec2(24.0, 22.0), !back_enabled).clicked() && back_enabled {
                            if let Err(e) = self.state.navigate_back() {
                                self.state.handle_error(e);
                            }
                        }
                        ui.label(RichText::new("Back").color(if back_enabled { WIN95_TEXT } else { WIN95_INACTIVE_CAPTION }));

                        let forward_enabled = self.state.current_index < self.state.history.len().saturating_sub(1);
                        if draw_bevel_with_feedback(ui, egui::vec2(24.0, 22.0), !forward_enabled).clicked() && forward_enabled {
                            if let Err(e) = self.state.navigate_forward() {
                                self.state.handle_error(e);
                            }
                        }
                        ui.label(RichText::new("Forward").color(if forward_enabled { WIN95_TEXT } else { WIN95_INACTIVE_CAPTION }));

                        if draw_bevel_with_feedback(ui, egui::vec2(24.0, 22.0), !self.state.is_loading()).clicked() {
                            self.state.stop_loading();
                        }
                        ui.label(RichText::new("Stop").color(if self.state.is_loading() { WIN95_TEXT } else { WIN95_INACTIVE_CAPTION }));

                        if draw_bevel_with_feedback(ui, egui::vec2(24.0, 22.0), false).clicked() {
                            if let Err(e) = self.state.load_url("about:welcome") {
                                self.state.handle_error(e);
                            }
                        }
                        ui.label(RichText::new("Home").color(WIN95_TEXT));

                        if draw_bevel_with_feedback(ui, egui::vec2(24.0, 22.0), false).clicked() {
                            let current_url = self.state.current_url.clone();
                            if !current_url.is_empty() {
                                if let Err(e) = self.state.load_url(&current_url) {
                                    self.state.handle_error(e);
                                }
                            }
                        }
                        ui.label(RichText::new("Refresh").color(WIN95_TEXT));

                        if draw_bevel_with_feedback(ui, egui::vec2(24.0, 22.0), false).clicked() {
                            if let Err(e) = self.state.load_url("about:blank") {
                                self.state.handle_error(e);
                            }
                        }
                        ui.label(RichText::new("Search").color(WIN95_TEXT));

                        ui.add_space(8.0);

                        let addr_response = ui.text_edit_singleline(&mut self.state.address_input);
                        if addr_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let url = self.state.address_input.trim();
                            if !url.is_empty() {
                                let full_url = if !url.contains("://") && !url.starts_with("about:") {
                                    if url.contains('.') { format!("http://{}", url) } else { url.to_string() }
                                } else { url.to_string() };
                                if let Err(e) = self.state.load_url(&full_url) {
                                    self.state.handle_error(e);
                                }
                            }
                        }

                        ui.add_space(8.0);

                        if draw_bevel_with_feedback(ui, egui::vec2(24.0, 22.0), false).clicked() {
                            self.state.show_bookmarks = !self.state.show_bookmarks;
                        }
                        ui.label(RichText::new("Favorites").color(WIN95_ACTIVE_CAPTION));

                        if draw_bevel_with_feedback(ui, egui::vec2(24.0, 22.0), false).clicked() {
                            self.state.print_page();
                        }
                        ui.label(RichText::new("Print").color(WIN95_TEXT));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.state.is_loading() {
                                self.state.throbber.draw(ui, ui.cursor().center_bottom());
                                ui.label("Loading...");
                            } else {
                                ui.label("Done");
                            }
                        });
                    });
                });
                
                egui::ScrollArea::both()
                    .id_source("page_content")
                    .show(ui, |ui| {
                        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::WHITE);
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                        
                        if self.state.is_loading() {
                            ui.centered_and_justified(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(100.0);
                                    
                                    ui.horizontal(|ui| {
                                        ui.add_space(20.0);
                                        
                                        let loading_rect = ui.available_rect_before_wrap();
                                        let center_x = loading_rect.center().x;
                                        let center_y = loading_rect.center().y;
                                        
                                        let painter = ui.painter();
                                        painter.text(
                                            egui::pos2(center_x - 20.0, center_y - 10.0),
                                            egui::Align2::LEFT_TOP,
                                            "Loading...",
                                            egui::FontId::new(16.0, FontFamily::Proportional),
                                            WIN95_ACTIVE_CAPTION
                                        );
                                        
                                        if !self.state.throbber.frames.is_empty() {
                                            self.state.throbber.draw(ui, egui::pos2(center_x + 80.0, center_y - 10.0));
                                        } else {
                                            const GLOBE_FRAMES: [&str; 12] = ["-", "\\", "|", "/", "-", "\\", "|", "/", "-", "\\", "|", "/"];
                                            painter.text(
                                                egui::pos2(center_x + 80.0, center_y - 10.0),
                                                egui::Align2::LEFT_TOP,
                                                GLOBE_FRAMES[self.state.throbber.current_frame % 12],
                                                egui::FontId::new(20.0, FontFamily::Monospace),
                                                WIN95_ACTIVE_CAPTION
                                            );
                                        }
                                    });
                                });
                            });
                        } else {
                            // Always try to render the page content
                            // Clone the render tree to avoid borrow conflict
                            let render_tree_clone = self.state.render_tree.clone();
                            if let Some(tree) = render_tree_clone {
                                render_to_egui(ui, &tree, &mut self.state);
                            } else {
                                // Fallback: render text content if no render tree
                                if !self.state.page_text_content.is_empty() {
                                    ui.label(RichText::new(&self.state.page_text_content).color(WIN95_TEXT));
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.heading(RichText::new("Welcome to Retro1996").size(24.0).color(WIN95_ACTIVE_CAPTION));
                                        ui.label("Enter a URL in the address bar above");
                                        draw_throbber(ui, self.state.throbber.current_frame);
                                    });
                                }
                            }
                        }
                    });
                
                egui::Frame {
                    fill: WIN95_MEDIUM,
                    inner_margin: egui::Margin::symmetric(4.0, 2.0),
                    outer_margin: egui::Margin::same(0.0),
                    rounding: egui::Rounding::same(0.0),
                    shadow: Default::default(),
                    stroke: Stroke::new(1.0, WIN95_DARK_SHADOW),
                }
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(self.state.get_status_text()).size(12.0).color(WIN95_TEXT));
                        if self.state.find_state.visible && self.state.find_state.total_matches > 0 {
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.label(RichText::new(format!("Match {} of {}", self.state.find_state.current_match + 1, self.state.find_state.total_matches)).size(12.0).color(WIN95_ACTIVE_CAPTION));
                            });
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let protocol = if self.state.current_url.starts_with("https") { "Secure" }
                            else if self.state.current_url.starts_with("http") { "HTTP" }
                            else if self.state.current_url.starts_with("ftp") { "FTP" }
                            else if self.state.current_url.starts_with("gopher") { "Gopher" }
                            else { "Local" };
                            ui.label(RichText::new(protocol).size(12.0).color(WIN95_INACTIVE_CAPTION));
                        });
                    });
                });
            });
            
        if self.state.show_bookmarks {
            egui::Window::new("Hotlinks")
                .resizable(true)
                .default_size(egui::vec2(300.0, 400.0))
                .pivot(Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(RichText::new("Bookmark Manager").size(14.0).color(WIN95_ACTIVE_CAPTION));
                    ui.separator();
                    if ui.button("Add Current Page").clicked() {
                        let name = if self.state.current_title == "Untitled Document" {
                            self.state.current_url.clone()
                        } else {
                            self.state.current_title.clone()
                        };
                        self.state.add_bookmark(name, self.state.current_url.clone());
                    }
                    ui.separator();
                    if self.state.bookmarks.is_empty() {
                        ui.label("No bookmarks saved");
                    } else {
                        let mut to_remove = None;
                        let mut to_navigate = None;
                        let bookmarks: Vec<_> = self.state.bookmarks.iter().enumerate().collect();
                        for (i, bookmark) in bookmarks {
                            let url = bookmark.url.clone();
                            let name = bookmark.name.clone();
                            ui.horizontal(|ui| {
                                if ui.button("Go").clicked() {
                                    to_navigate = Some(url.clone());
                                }
                                if ui.button("Remove").clicked() {
                                    to_remove = Some(i);
                                }
                                ui.label(format!("{}: {}", name, url));
                            });
                        }
                        if let Some(url) = to_navigate {
                            if let Err(e) = self.state.load_url(&url) {
                                self.state.handle_error(e);
                            }
                            self.state.show_bookmarks = false;
                        }
                        if let Some(index) = to_remove {
                            self.state.remove_bookmark(index);
                        }
                    }
                    ui.separator();
                    if ui.button("Close").clicked() { self.state.show_bookmarks = false; }
                });
        }
        
        if self.state.find_state.visible {
            egui::Window::new("Find")
                .resizable(false)
                .title_bar(false)
                .pivot(Align2::CENTER_CENTER)
                .fixed_size(egui::vec2(300.0, 120.0))
                .show(ctx, |ui| {
                    ui.set_min_size(egui::vec2(300.0, 120.0));
                    ui.label("Find in page:");
                    ui.text_edit_singleline(&mut self.state.find_state.search_text);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.state.find_state.case_sensitive, "Match case");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Next").clicked() {
                                if let Err(e) = self.state.find_next() {
                                    self.state.handle_error(e);
                                }
                            }
                            if ui.button("Close").clicked() { self.state.find_state.visible = false; }
                        });
                    });
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) || self.state.find_state.search_text.len() > 1 {
                        self.state.perform_find();
                    }
                    if self.state.find_state.total_matches > 0 {
                        ui.label(format!("{} match{} found", self.state.find_state.total_matches, if self.state.find_state.total_matches == 1 { "" } else { "es" }));
                    } else if !self.state.find_state.search_text.is_empty() {
                        ui.label("No matches found");
                    }
                });
        }
        
        if ctx.input(|i| i.key_pressed(egui::Key::F3)) { self.state.trigger_find(); }
        if ctx.input(|i| i.key_pressed(egui::Key::B) && i.modifiers.ctrl) { self.state.show_bookmarks = !self.state.show_bookmarks; }
    }
}