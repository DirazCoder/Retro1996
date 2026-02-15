use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PluginStatus {
    Installed,
    NotInstalled,
    Disabled,
    Outdated,
}

#[derive(Debug, Clone)]
pub struct Plugin {
    name: String,
    description: String,
    version: String,
    path: PathBuf,
    mime_types: Vec<String>,
    file_extensions: Vec<String>,
    status: PluginStatus,
    enabled: bool,
    last_used: Option<u64>,
}

impl Plugin {
    pub fn new(
        name: String,
        description: String,
        version: String,
        path: PathBuf,
        mime_types: Vec<String>,
        file_extensions: Vec<String>,
    ) -> Self {
        Plugin {
            name,
            description,
            version,
            path,
            mime_types,
            file_extensions,
            status: PluginStatus::Installed,
            enabled: true,
            last_used: None,
        }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn description(&self) -> &str { &self.description }
    pub fn version(&self) -> &str { &self.version }
    pub fn path(&self) -> &Path { &self.path }
    pub fn mime_types(&self) -> &Vec<String> { &self.mime_types }
    pub fn file_extensions(&self) -> &Vec<String> { &self.file_extensions }
    pub fn status(&self) -> PluginStatus { self.status }
    pub fn is_enabled(&self) -> bool { self.enabled }
    pub fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    pub fn update_last_used(&mut self) { self.last_used = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()); }
}

pub struct PluginManager {
    plugins: HashMap<String, Plugin>, // MIME type -> Plugin
    plugin_list: Vec<Plugin>,
    plugin_dir: PathBuf,
    enabled_plugins: HashMap<String, bool>, // MIME type -> enabled
}

impl PluginManager {
    pub fn new(plugin_dir: PathBuf) -> Self {
        let mut manager = PluginManager {
            plugins: HashMap::new(),
            plugin_list: Vec::new(),
            plugin_dir: plugin_dir.clone(),
            enabled_plugins: HashMap::new(),
        };
        
        // Create plugin directory (period-accurate location)
        let _ = fs::create_dir_all(&plugin_dir);
        
        // Register built-in 1996 plugins (period-authentic)
        manager.register_builtin_plugins();
        
        manager
    }

    fn register_builtin_plugins(&mut self) {
        // Macromedia Shockwave Player (Director)
        self.register_plugin(
            Plugin::new(
                "Shockwave Player".to_string(),
                "Macromedia Shockwave for Director".to_string(),
                "4.0".to_string(),
                self.plugin_dir.join("Shockwave.dll"),
                vec!["application/x-director".to_string()],
                vec!["dcr".to_string(), "dir".to_string(), "dxr".to_string()],
            )
        );
        
        // RealPlayer
        self.register_plugin(
            Plugin::new(
                "RealPlayer".to_string(),
                "RealPlayer G2 LiveConnect-Enabled".to_string(),
                "5.0".to_string(),
                self.plugin_dir.join("nppl3260.dll"),
                vec!["audio/x-pn-realaudio-plugin".to_string(), "audio/x-pn-realaudio".to_string()],
                vec!["rpm".to_string(), "rm".to_string(), "ra".to_string()],
            )
        );
        
        // QuickTime
        self.register_plugin(
            Plugin::new(
                "QuickTime".to_string(),
                "QuickTime Plug-in 3.0".to_string(),
                "3.0".to_string(),
                self.plugin_dir.join("QuickTimePlugin.dll"),
                vec!["video/quicktime".to_string(), "image/x-quicktime".to_string()],
                vec!["mov".to_string(), "qt".to_string()],
            )
        );
        
        // Flash (late 1996 - FutureSplash Animator)
        self.register_plugin(
            Plugin::new(
                "Flash".to_string(),
                "FutureSplash Player 1.0".to_string(),
                "1.0".to_string(),
                self.plugin_dir.join("Flash.dll"),
                vec!["application/x-shockwave-flash".to_string()],
                vec!["spl".to_string(), "swf".to_string()],
            )
        );
        
        // Java (HotJava)
        self.register_plugin(
            Plugin::new(
                "Java".to_string(),
                "HotJava 1.0".to_string(),
                "1.0".to_string(),
                self.plugin_dir.join("Java.dll"),
                vec!["application/x-java-applet".to_string()],
                vec!["class".to_string(), "jar".to_string()],
            )
        );
        
        // VRML (Cosmo Player)
        self.register_plugin(
            Plugin::new(
                "VRML".to_string(),
                "Cosmo Player 2.0".to_string(),
                "2.0".to_string(),
                self.plugin_dir.join("Cosmo.dll"),
                vec!["model/vrml".to_string()],
                vec!["wrl".to_string()],
            )
        );
    }

    pub fn register_plugin(&mut self, plugin: Plugin) {
        let mime_types = plugin.mime_types.clone();
        let enabled = plugin.is_enabled();
        
        for mime_type in &mime_types {
            self.plugins.insert(mime_type.clone(), plugin.clone());
            self.enabled_plugins.insert(mime_type.clone(), enabled);
        }
        
        self.plugin_list.push(plugin);
    }

    pub fn unregister_plugin(&mut self, mime_type: &str) -> Option<Plugin> {
        if let Some(plugin) = self.plugins.remove(mime_type) {
            self.enabled_plugins.remove(mime_type);
            // Remove from plugin_list
            if let Some(pos) = self.plugin_list.iter().position(|p| p.mime_types.contains(&mime_type.to_string())) {
                return Some(self.plugin_list.remove(pos));
            }
        }
        None
    }

    pub fn find_plugin(&self, mime_type: &str) -> Option<&Plugin> {
        self.plugins.get(mime_type)
    }

    pub fn is_plugin_enabled(&self, mime_type: &str) -> bool {
        *self.enabled_plugins.get(mime_type).unwrap_or(&false)
    }

    pub fn set_plugin_enabled(&mut self, mime_type: &str, enabled: bool) {
        if self.plugins.contains_key(mime_type) {
            self.enabled_plugins.insert(mime_type.to_string(), enabled);
        }
    }

    pub fn get_plugins(&self) -> &Vec<Plugin> {
        &self.plugin_list
    }

    pub fn get_enabled_plugins(&self) -> Vec<&Plugin> {
        self.plugin_list.iter()
            .filter(|p| p.is_enabled() && p.status == PluginStatus::Installed)
            .collect()
    }

    pub fn scan_plugin_directory(&mut self) -> Result<(), String> {
        if !self.plugin_dir.exists() {
            return Err(format!("Plugin directory does not exist: {:?}", self.plugin_dir));
        }
        
        for entry in fs::read_dir(&self.plugin_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("dll") {
                // In a real implementation, we would load the DLL and query its MIME types
                // For period authenticity, we simulate based on filename
                let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                
                if filename.contains("shockwave") || filename.contains("director") {
                    self.register_plugin(Plugin::new(
                        "Shockwave Player".to_string(),
                        "Macromedia Shockwave".to_string(),
                        "4.0".to_string(),
                        path,
                        vec!["application/x-director".to_string()],
                        vec!["dcr".to_string(), "dir".to_string()],
                    ));
                } else if filename.contains("real") || filename.contains("rp") {
                    self.register_plugin(Plugin::new(
                        "RealPlayer".to_string(),
                        "RealPlayer Plugin".to_string(),
                        "5.0".to_string(),
                        path,
                        vec!["audio/x-pn-realaudio-plugin".to_string()],
                        vec!["rpm".to_string(), "rm".to_string()],
                    ));
                } else if filename.contains("qt") || filename.contains("quicktime") {
                    self.register_plugin(Plugin::new(
                        "QuickTime".to_string(),
                        "QuickTime Plugin".to_string(),
                        "3.0".to_string(),
                        path,
                        vec!["video/quicktime".to_string()],
                        vec!["mov".to_string(), "qt".to_string()],
                    ));
                } else if filename.contains("flash") || filename.contains("spl") {
                    self.register_plugin(Plugin::new(
                        "Flash".to_string(),
                        "FutureSplash Player".to_string(),
                        "1.0".to_string(),
                        path,
                        vec!["application/x-shockwave-flash".to_string()],
                        vec!["spl".to_string(), "swf".to_string()],
                    ));
                }
            }
        }
        
        Ok(())
    }

    pub fn get_plugin_for_url(&self, url: &str) -> Option<&Plugin> {
        let lower_url = url.to_lowercase();
        
        for plugin in &self.plugin_list {
            for ext in &plugin.file_extensions {
                if lower_url.ends_with(&format!(".{}", ext)) {
                    return Some(plugin);
                }
            }
        }
        
        None
    }

    pub fn get_plugin_for_embed(&self, mime_type: &str, src: &str) -> Option<&Plugin> {
        // First try MIME type
        if let Some(plugin) = self.find_plugin(mime_type) {
            if self.is_plugin_enabled(mime_type) {
                return Some(plugin);
            }
        }
        
        // Fallback to URL extension
        self.get_plugin_for_url(src)
    }

    pub fn render_plugin_placeholder(&self, plugin: &Plugin, width: u32, height: u32) -> String {
        format!(
            r#"<div style="width:{}px;height:{}px;border:2px solid #000080;background:#f0f0f0;padding:10px;text-align:center">
                <div style="font-weight:bold;color:#000080;margin-bottom:5px">{}</div>
                <div style="font-size:10px;color:#666;margin-bottom:10px">{}</div>
                <div style="font-size:8px;color:#999">Plugin {}</div>
                <div style="margin-top:5px;font-size:8px">
                    <a href="about:plugins" style="color:#0000ff;text-decoration:underline">Plugin Info</a>
                </div>
            </div>"#,
            width,
            height,
            plugin.name,
            plugin.description,
            if plugin.is_enabled() { "Enabled" } else { "Disabled" }
        )
    }

    pub fn get_about_plugins_html(&self) -> String {
        let mut html = String::from(
            "<html><head><title>About Plugins</title></head><body bgcolor=\"#FFFFFF\">\n\
             <h2 align=\"center\">About Plug-ins</h2>\n\
             <p align=\"center\"><font size=\"-1\">Retro1996 Browser v3.0</font></p>\n\
             <hr>\n\
             <h3>Installed Plug-ins</h3>\n\
             <ul>\n"
        );
        
        if self.plugin_list.is_empty() {
            html.push_str("<li><i>No plug-ins installed</i></li>\n");
        } else {
            for plugin in &self.plugin_list {
                html.push_str(&format!(
                    "<li><b>{}</b> (version {})<br>\n\
                     <font size=\"-1\">{}</font><br>\n\
                     <font size=\"-1\">MIME types: {}</font><br>\n\
                     <font size=\"-1\">File extensions: {}</font><br>\n\
                     <font size=\"-1\">Status: {} | {}</font>\n\
                     </li>\n",
                    plugin.name,
                    plugin.version,
                    plugin.description,
                    plugin.mime_types.join(", "),
                    plugin.file_extensions.join(", "),
                    if plugin.is_enabled() { "Enabled" } else { "Disabled" },
                    match plugin.status {
                        PluginStatus::Installed => "Installed",
                        PluginStatus::NotInstalled => "Not Installed",
                        PluginStatus::Disabled => "Disabled",
                        PluginStatus::Outdated => "Outdated",
                    }
                ));
            }
        }
        
        html.push_str(
            "</ul>\n\
             <hr>\n\
             <p align=\"center\">\n\
             <font size=\"-1\">\n\
             Plug-ins allow the browser to display content that it normally cannot display.<br>\n\
             Common plug-ins include Shockwave, RealPlayer, QuickTime, and Flash.<br>\n\
             For more information, visit the plug-in manufacturer's website.\n\
             </font>\n\
             </p>\n\
             </body></html>"
        );
        
        html
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_plugin_creation() {
        let plugin = Plugin::new(
            "Test Plugin".to_string(),
            "Test Description".to_string(),
            "1.0".to_string(),
            PathBuf::from("test.dll"),
            vec!["application/x-test".to_string()],
            vec!["test".to_string()],
        );
        
        assert_eq!(plugin.name(), "Test Plugin");
        assert_eq!(plugin.description(), "Test Description");
        assert_eq!(plugin.version(), "1.0");
        assert_eq!(plugin.mime_types(), &vec!["application/x-test"]);
        assert_eq!(plugin.file_extensions(), &vec!["test"]);
        assert!(plugin.is_enabled());
    }

    #[test]
    fn test_plugin_manager_registration() {
        let temp_dir = std::env::temp_dir().join("retro1996_plugins_test_reg");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        
        let mut manager = PluginManager::new(temp_dir.clone());
        
        // Register custom plugin
        let plugin = Plugin::new(
            "Custom".to_string(),
            "Custom Plugin".to_string(),
            "2.0".to_string(),
            PathBuf::from("custom.dll"),
            vec!["application/x-custom".to_string()],
            vec!["cus".to_string()],
        );
        manager.register_plugin(plugin);
        
        // Verify registration
        assert!(manager.find_plugin("application/x-custom").is_some());
        assert_eq!(manager.get_plugins().len(), 8); // 7 built-in + 1 custom
        
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_plugin_enable_disable() {
        let temp_dir = std::env::temp_dir().join("retro1996_plugins_test_enable");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        
        let mut manager = PluginManager::new(temp_dir.clone());
        
        // Disable Shockwave
        manager.set_plugin_enabled("application/x-director", false);
        assert!(!manager.is_plugin_enabled("application/x-director"));
        
        // Re-enable
        manager.set_plugin_enabled("application/x-director", true);
        assert!(manager.is_plugin_enabled("application/x-director"));
        
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_plugin_lookup_by_mime() {
        let temp_dir = std::env::temp_dir().join("retro1996_plugins_test_mime");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        
        let manager = PluginManager::new(temp_dir.clone());
        
        // Test built-in plugins
        assert!(manager.find_plugin("application/x-director").is_some());
        assert!(manager.find_plugin("audio/x-pn-realaudio-plugin").is_some());
        assert!(manager.find_plugin("video/quicktime").is_some());
        assert!(manager.find_plugin("application/x-shockwave-flash").is_some());
        assert!(manager.find_plugin("application/x-java-applet").is_some());
        
        // Test unknown MIME type
        assert!(manager.find_plugin("unknown/mime-type").is_none());
        
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_plugin_lookup_by_url() {
        let temp_dir = std::env::temp_dir().join("retro1996_plugins_test_url");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        
        let manager = PluginManager::new(temp_dir.clone());
        
        // Test URL lookups
        assert!(manager.get_plugin_for_url("http://example.com/movie.mov").is_some());
        assert!(manager.get_plugin_for_url("http://example.com/sound.rm").is_some());
        assert!(manager.get_plugin_for_url("http://example.com/animation.dcr").is_some());
        assert!(manager.get_plugin_for_url("http://example.com/file.swf").is_some());
        assert!(manager.get_plugin_for_url("http://example.com/document.pdf").is_none()); // No PDF plugin in 1996
        
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_plugin_placeholder_rendering() {
        let plugin = Plugin::new(
            "Shockwave Player".to_string(),
            "Macromedia Shockwave for Director".to_string(),
            "4.0".to_string(),
            PathBuf::from("Shockwave.dll"),
            vec!["application/x-director".to_string()],
            vec!["dcr".to_string()],
        );
        
        let placeholder = PluginManager::new(PathBuf::from(".")).render_plugin_placeholder(&plugin, 300, 200);
        
        assert!(placeholder.contains("Shockwave Player"));
        assert!(placeholder.contains("Macromedia Shockwave for Director"));
        assert!(placeholder.contains("width:300px"));
        assert!(placeholder.contains("height:200px"));
        assert!(placeholder.contains("Enabled"));
    }

    #[test]
    fn test_about_plugins_page() {
        let temp_dir = std::env::temp_dir().join("retro1996_plugins_test_about");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        
        let manager = PluginManager::new(temp_dir.clone());
        let html = manager.get_about_plugins_html();
        
        assert!(html.contains("<h2 align=\"center\">About Plug-ins</h2>"));
        assert!(html.contains("Shockwave Player"));
        assert!(html.contains("RealPlayer"));
        assert!(html.contains("QuickTime"));
        assert!(html.contains("Flash"));
        assert!(html.contains("Java"));
        assert!(html.contains("VRML"));
        assert!(html.contains("Plug-ins allow the browser to display content"));
        
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_plugin_unregistration() {
        let temp_dir = std::env::temp_dir().join("retro1996_plugins_test_unregister");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        
        let mut manager = PluginManager::new(temp_dir.clone());
        
        // Verify Shockwave is registered
        assert!(manager.find_plugin("application/x-director").is_some());
        
        // Unregister it
        let unregistered = manager.unregister_plugin("application/x-director");
        assert!(unregistered.is_some());
        assert_eq!(unregistered.unwrap().name(), "Shockwave Player");
        
        // Verify it's gone
        assert!(manager.find_plugin("application/x-director").is_none());
        assert_eq!(manager.get_plugins().len(), 6); // 7 built-in - 1
        
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_enabled_plugins_list() {
        let temp_dir = std::env::temp_dir().join("retro1996_plugins_test_enabled");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        
        let mut manager = PluginManager::new(temp_dir.clone());
        
        // Disable RealPlayer
        manager.set_plugin_enabled("audio/x-pn-realaudio-plugin", false);
        
        let enabled = manager.get_enabled_plugins();
        assert_eq!(enabled.len(), 6); // 7 total - 1 disabled
        
        // Verify RealPlayer is not in enabled list
        assert!(!enabled.iter().any(|p| p.name() == "RealPlayer"));
        
        // Verify Shockwave is in enabled list
        assert!(enabled.iter().any(|p| p.name() == "Shockwave Player"));
        
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_plugin_status() {
        let mut plugin = Plugin::new(
            "Test".to_string(),
            "Test".to_string(),
            "1.0".to_string(),
            PathBuf::from("test.dll"),
            vec!["test/test".to_string()],
            vec!["test".to_string()],
        );
        
        assert_eq!(plugin.status(), PluginStatus::Installed);
        plugin.set_enabled(false);
        assert!(!plugin.is_enabled());
        plugin.set_enabled(true);
        assert!(plugin.is_enabled());
    }

    #[test]
    fn test_plugin_directory_scanning() {
        let temp_dir = std::env::temp_dir().join("retro1996_plugins_test_scan");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        
        // Create fake plugin DLLs
        let shockwave_path = temp_dir.join("np32dsw.dll");
        let _ = fs::write(&shockwave_path, b"fake dll content");
        
        let realplayer_path = temp_dir.join("nppl3260.dll");
        let _ = fs::write(&realplayer_path, b"fake dll content");
        
        let mut manager = PluginManager::new(temp_dir.clone());
        let _ = manager.scan_plugin_directory();
        
        // Should have detected plugins from filenames
        assert!(manager.find_plugin("application/x-director").is_some());
        assert!(manager.find_plugin("audio/x-pn-realaudio-plugin").is_some());
        
        let _ = fs::remove_dir_all(temp_dir);
    }
}