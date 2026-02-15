use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::ini_preferences::{IniPreferences, IniError};

#[derive(Debug, Clone)]
pub struct Preferences {
    // General preferences
    pub home_page: String,
    pub startup_behavior: StartupBehavior,
    pub blank_page_as_home: bool,

    // Appearance preferences
    pub default_font_family: String,
    pub default_font_size: u8,
    pub link_color: String,
    pub visited_link_color: String,
    pub active_link_color: String,

    // Cache preferences
    pub memory_cache_size: u32,  // in MB
    pub disk_cache_location: String,
    pub disk_cache_size: u32,    // in MB

    // Network preferences
    pub proxy_host: String,
    pub proxy_port: u16,
    pub proxy_username: String,
    pub proxy_password: String,
    pub connection_timeout: u32, // in seconds

    // Security preferences
    pub accept_cookies: CookiePolicy,
    pub warn_before_downloading: bool,

    // Advanced preferences
    pub enable_javascript: bool,
    pub auto_load_images: bool,
    pub enable_plugins: bool,
    pub enable_java: bool,

    // Privacy preferences
    pub delete_personal_data_on_exit: bool,
    pub remember_form_data: bool,
    pub remember_passwords: bool,

    // Accessibility preferences
    pub minimum_font_size: u8,
    pub force_colors: bool,
    pub text_only_mode: bool,
}

#[derive(Debug, Clone)]
pub enum StartupBehavior {
    ShowHomePage,
    ShowBlankPage,
    ShowLastVisitedPage,
    ShowCustomPage(String),
}

#[derive(Debug, Clone)]
pub enum CookiePolicy {
    AcceptAll,
    AcceptFromSitesVisited,
    BlockAll,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            home_page: "about:welcome".to_string(),
            startup_behavior: StartupBehavior::ShowHomePage,
            blank_page_as_home: false,

            default_font_family: "Times New Roman".to_string(),
            default_font_size: 16,
            link_color: "#0000FF".to_string(),      // Blue
            visited_link_color: "#800080".to_string(), // Purple
            active_link_color: "#FF0000".to_string(),  // Red

            memory_cache_size: 5,  // 5MB default
            disk_cache_location: "./cache".to_string(),
            disk_cache_size: 50,   // 50MB default

            proxy_host: "".to_string(),
            proxy_port: 8080,
            proxy_username: "".to_string(),
            proxy_password: "".to_string(),
            connection_timeout: 30, // 30 seconds

            accept_cookies: CookiePolicy::AcceptFromSitesVisited,
            warn_before_downloading: true,

            enable_javascript: true,
            auto_load_images: true,
            enable_plugins: true,
            enable_java: false,

            delete_personal_data_on_exit: false,
            remember_form_data: true,
            remember_passwords: true,

            minimum_font_size: 9,
            force_colors: false,
            text_only_mode: false,
        }
    }
}

impl Preferences {
    pub fn new() -> Self {
        // Check if preferences file exists and load it, otherwise return defaults
        if let Ok(prefs) = Self::load_from_file("./prefs.ini") {
            prefs
        } else {
            Self::default()
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        let mut ini_prefs = IniPreferences::new(path.to_string());

        // Set general preferences
        ini_prefs.set_value("General", "HomePage", &self.home_page);
        ini_prefs.set_value("General", "BlankPageAsHome", &self.blank_page_as_home.to_string());

        // Set appearance preferences
        ini_prefs.set_value("Appearance", "DefaultFontFamily", &self.default_font_family);
        ini_prefs.set_value("Appearance", "DefaultFontSize", &self.default_font_size.to_string());
        ini_prefs.set_value("Appearance", "LinkColor", &self.link_color);
        ini_prefs.set_value("Appearance", "VisitedLinkColor", &self.visited_link_color);
        ini_prefs.set_value("Appearance", "ActiveLinkColor", &self.active_link_color);

        // Set cache preferences
        ini_prefs.set_value("Cache", "MemoryCacheSize", &self.memory_cache_size.to_string());
        ini_prefs.set_value("Cache", "DiskCacheLocation", &self.disk_cache_location);
        ini_prefs.set_value("Cache", "DiskCacheSize", &self.disk_cache_size.to_string());

        // Set network preferences
        ini_prefs.set_value("Network", "ProxyHost", &self.proxy_host);
        ini_prefs.set_value("Network", "ProxyPort", &self.proxy_port.to_string());
        ini_prefs.set_value("Network", "ProxyUsername", &self.proxy_username);
        ini_prefs.set_value("Network", "ProxyPassword", &self.proxy_password);
        ini_prefs.set_value("Network", "ConnectionTimeout", &self.connection_timeout.to_string());

        // Set security preferences
        ini_prefs.set_value("Security", "AcceptCookies", &match self.accept_cookies {
            CookiePolicy::AcceptAll => "AcceptAll",
            CookiePolicy::AcceptFromSitesVisited => "AcceptFromSitesVisited",
            CookiePolicy::BlockAll => "BlockAll",
        });
        ini_prefs.set_value("Security", "WarnBeforeDownloading", &self.warn_before_downloading.to_string());

        // Set advanced preferences
        ini_prefs.set_value("Advanced", "EnableJavaScript", &self.enable_javascript.to_string());
        ini_prefs.set_value("Advanced", "AutoLoadImages", &self.auto_load_images.to_string());
        ini_prefs.set_value("Advanced", "EnablePlugins", &self.enable_plugins.to_string());
        ini_prefs.set_value("Advanced", "EnableJava", &self.enable_java.to_string());

        // Set privacy preferences
        ini_prefs.set_value("Privacy", "DeletePersonalDataOnExit", &self.delete_personal_data_on_exit.to_string());
        ini_prefs.set_value("Privacy", "RememberFormData", &self.remember_form_data.to_string());
        ini_prefs.set_value("Privacy", "RememberPasswords", &self.remember_passwords.to_string());

        // Set accessibility preferences
        ini_prefs.set_value("Accessibility", "MinimumFontSize", &self.minimum_font_size.to_string());
        ini_prefs.set_value("Accessibility", "ForceColors", &self.force_colors.to_string());
        ini_prefs.set_value("Accessibility", "TextOnlyMode", &self.text_only_mode.to_string());

        ini_prefs.save_to_file()
            .map_err(|e| format!("Failed to save preferences: {}", e))
    }

    pub fn load_from_file(path: &str) -> Result<Self, String> {
        if !Path::new(path).exists() {
            return Err("Preferences file does not exist".to_string());
        }

        let mut ini_prefs = IniPreferences::new(path.to_string());
        ini_prefs.load_from_file()
            .map_err(|e| format!("Failed to load preferences: {}", e))?;

        let mut prefs = Preferences::default();

        // Load general preferences
        prefs.home_page = ini_prefs.get_string("General", "HomePage");
        prefs.blank_page_as_home = ini_prefs.get_bool("General", "BlankPageAsHome");

        // Load appearance preferences
        prefs.default_font_family = ini_prefs.get_string("Appearance", "DefaultFontFamily");
        prefs.default_font_size = ini_prefs.get_int("Appearance", "DefaultFontSize") as u8;
        prefs.link_color = ini_prefs.get_string("Appearance", "LinkColor");
        prefs.visited_link_color = ini_prefs.get_string("Appearance", "VisitedLinkColor");
        prefs.active_link_color = ini_prefs.get_string("Appearance", "ActiveLinkColor");

        // Load cache preferences
        prefs.memory_cache_size = ini_prefs.get_int("Cache", "MemoryCacheSize") as u32;
        prefs.disk_cache_location = ini_prefs.get_string("Cache", "DiskCacheLocation");
        prefs.disk_cache_size = ini_prefs.get_int("Cache", "DiskCacheSize") as u32;

        // Load network preferences
        prefs.proxy_host = ini_prefs.get_string("Network", "ProxyHost");
        prefs.proxy_port = ini_prefs.get_int("Network", "ProxyPort") as u16;
        prefs.proxy_username = ini_prefs.get_string("Network", "ProxyUsername");
        prefs.proxy_password = ini_prefs.get_string("Network", "ProxyPassword");
        prefs.connection_timeout = ini_prefs.get_int("Network", "ConnectionTimeout") as u32;

        // Load security preferences
        let cookie_policy_str = ini_prefs.get_string("Security", "AcceptCookies");
        prefs.accept_cookies = match cookie_policy_str.as_str() {
            "AcceptAll" => CookiePolicy::AcceptAll,
            "AcceptFromSitesVisited" => CookiePolicy::AcceptFromSitesVisited,
            "BlockAll" => CookiePolicy::BlockAll,
            _ => CookiePolicy::AcceptFromSitesVisited,
        };
        prefs.warn_before_downloading = ini_prefs.get_bool("Security", "WarnBeforeDownloading");

        // Load advanced preferences
        prefs.enable_javascript = ini_prefs.get_bool("Advanced", "EnableJavaScript");
        prefs.auto_load_images = ini_prefs.get_bool("Advanced", "AutoLoadImages");
        prefs.enable_plugins = ini_prefs.get_bool("Advanced", "EnablePlugins");
        prefs.enable_java = ini_prefs.get_bool("Advanced", "EnableJava");

        // Load privacy preferences
        prefs.delete_personal_data_on_exit = ini_prefs.get_bool("Privacy", "DeletePersonalDataOnExit");
        prefs.remember_form_data = ini_prefs.get_bool("Privacy", "RememberFormData");
        prefs.remember_passwords = ini_prefs.get_bool("Privacy", "RememberPasswords");

        // Load accessibility preferences
        prefs.minimum_font_size = ini_prefs.get_int("Accessibility", "MinimumFontSize") as u8;
        prefs.force_colors = ini_prefs.get_bool("Accessibility", "ForceColors");
        prefs.text_only_mode = ini_prefs.get_bool("Accessibility", "TextOnlyMode");

        // Determine startup behavior from loaded settings
        prefs.startup_behavior = StartupBehavior::ShowHomePage; // Default, could be extended based on settings

        Ok(prefs)
    }

    pub fn reset_to_defaults(&mut self) {
        *self = Self::default();
    }

    pub fn set_home_page(&mut self, url: String) {
        self.home_page = url;
    }

    pub fn set_default_font(&mut self, family: String, size: u8) {
        self.default_font_family = family;
        self.default_font_size = size;
    }

    pub fn set_cache_sizes(&mut self, memory_mb: u32, disk_mb: u32) {
        self.memory_cache_size = memory_mb;
        self.disk_cache_size = disk_mb;
    }

    pub fn set_proxy(&mut self, host: String, port: u16, username: String, password: String) {
        self.proxy_host = host;
        self.proxy_port = port;
        self.proxy_username = username;
        self.proxy_password = password;
    }

    pub fn set_security_settings(&mut self, cookie_policy: CookiePolicy, enable_js: bool, auto_load_images: bool) {
        self.accept_cookies = cookie_policy;
        self.enable_javascript = enable_js;
        self.auto_load_images = auto_load_images;
    }

    pub fn get_category_preferences(&self, category: PreferenceCategory) -> HashMap<String, String> {
        let mut prefs_map = HashMap::new();

        match category {
            PreferenceCategory::General => {
                prefs_map.insert("home_page".to_string(), self.home_page.clone());
                prefs_map.insert("startup_behavior".to_string(),
                    match self.startup_behavior {
                        StartupBehavior::ShowHomePage => "home_page".to_string(),
                        StartupBehavior::ShowBlankPage => "blank_page".to_string(),
                        StartupBehavior::ShowLastVisitedPage => "last_visited".to_string(),
                        StartupBehavior::ShowCustomPage(ref url) => format!("custom:{}", url),
                    }
                );
                prefs_map.insert("blank_page_as_home".to_string(),
                    self.blank_page_as_home.to_string());
            },
            PreferenceCategory::Appearance => {
                prefs_map.insert("default_font_family".to_string(), self.default_font_family.clone());
                prefs_map.insert("default_font_size".to_string(), self.default_font_size.to_string());
                prefs_map.insert("link_color".to_string(), self.link_color.clone());
                prefs_map.insert("visited_link_color".to_string(), self.visited_link_color.clone());
                prefs_map.insert("active_link_color".to_string(), self.active_link_color.clone());
            },
            PreferenceCategory::Cache => {
                prefs_map.insert("memory_cache_size".to_string(), self.memory_cache_size.to_string());
                prefs_map.insert("disk_cache_location".to_string(), self.disk_cache_location.clone());
                prefs_map.insert("disk_cache_size".to_string(), self.disk_cache_size.to_string());
            },
            PreferenceCategory::Network => {
                prefs_map.insert("proxy_host".to_string(), self.proxy_host.clone());
                prefs_map.insert("proxy_port".to_string(), self.proxy_port.to_string());
                prefs_map.insert("connection_timeout".to_string(), self.connection_timeout.to_string());
            },
            PreferenceCategory::Security => {
                prefs_map.insert("accept_cookies".to_string(),
                    match self.accept_cookies {
                        CookiePolicy::AcceptAll => "accept_all".to_string(),
                        CookiePolicy::AcceptFromSitesVisited => "accept_visited".to_string(),
                        CookiePolicy::BlockAll => "block_all".to_string(),
                    }
                );
                prefs_map.insert("warn_before_downloading".to_string(),
                    self.warn_before_downloading.to_string());
            },
            PreferenceCategory::Advanced => {
                prefs_map.insert("enable_javascript".to_string(), self.enable_javascript.to_string());
                prefs_map.insert("auto_load_images".to_string(), self.auto_load_images.to_string());
                prefs_map.insert("enable_plugins".to_string(), self.enable_plugins.to_string());
                prefs_map.insert("enable_java".to_string(), self.enable_java.to_string());
            },
            PreferenceCategory::Privacy => {
                prefs_map.insert("delete_personal_data_on_exit".to_string(),
                    self.delete_personal_data_on_exit.to_string());
                prefs_map.insert("remember_form_data".to_string(),
                    self.remember_form_data.to_string());
                prefs_map.insert("remember_passwords".to_string(),
                    self.remember_passwords.to_string());
            },
            PreferenceCategory::Accessibility => {
                prefs_map.insert("minimum_font_size".to_string(), self.minimum_font_size.to_string());
                prefs_map.insert("force_colors".to_string(), self.force_colors.to_string());
                prefs_map.insert("text_only_mode".to_string(), self.text_only_mode.to_string());
            },
        }

        prefs_map
    }
}

pub enum PreferenceCategory {
    General,
    Appearance,
    Cache,
    Network,
    Security,
    Advanced,
    Privacy,
    Accessibility,
}

// Preferences manager that handles the UI aspects
pub struct PreferencesManager {
    pub preferences: Preferences,
    pub current_category: PreferenceCategory,
    pub is_dirty: bool,  // Indicates if preferences have been modified
}

impl PreferencesManager {
    pub fn new() -> Self {
        PreferencesManager {
            preferences: Preferences::new(),
            current_category: PreferenceCategory::General,
            is_dirty: false,
        }
    }

    pub fn switch_category(&mut self, category: PreferenceCategory) {
        self.current_category = category;
    }

    pub fn save_preferences(&mut self) -> Result<(), String> {
        self.preferences.save_to_file("./prefs.json")?;
        self.is_dirty = false;
        Ok(())
    }

    pub fn load_preferences(&mut self) -> Result<(), String> {
        self.preferences = Preferences::load_from_file("./prefs.json")?;
        self.is_dirty = false;
        Ok(())
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.is_dirty
    }

    pub fn apply_preferences(&self) {
        // This would typically notify other parts of the application
        // that preferences have changed and need to be applied
    }
}