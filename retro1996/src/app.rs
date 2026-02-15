use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Instant;
use crate::engine::{TrussCore, DomNode, RenderNode};
use crate::javascript_engine::ChronoScript;
use crate::network::NetworkManager;
use crate::ui::{Retro1996Browser, BrowserState};

pub struct BrowserContext {
    pub rendering_engine: Arc<Mutex<TrussCore>>,
    pub js_engine: Arc<Mutex<ChronoScript>>,
    pub network_manager: Arc<Mutex<NetworkManager>>,
    pub shared_state: Arc<Mutex<SharedBrowserState>>,
}

pub struct SharedBrowserState {
    pub current_url: String,
    pub current_title: String,
    pub dom_tree: Option<DomNode>,
    pub render_tree: Option<RenderNode>,
    pub js_variables: HashMap<String, String>,
    pub last_interaction_time: Instant,
    pub page_loaded: bool,
    pub loading_status: LoadingStatus,
    pub history: Vec<String>,
    pub history_index: usize,
    pub cookies: HashMap<String, String>,
    pub cache: HashMap<String, Vec<u8>>,
    pub favicon_url: Option<String>,
    pub security_level: SecurityLevel,
    pub javascript_enabled: bool,
    pub plugins_enabled: bool,
    pub images_enabled: bool,
    pub css_enabled: bool,
}

#[derive(Debug, Clone)]
pub enum LoadingStatus {
    Idle,
    LookingUpHost(String),
    Connecting(String),
    SendingRequest,
    ReceivingData(usize, usize), 
    ParsingHtml,
    LoadingImages(usize, usize), 
    Complete,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum SecurityLevel {
    Permissive, 
    Standard,
    Strict,
}

impl SharedBrowserState {
    pub fn new() -> Self {
        SharedBrowserState {
            current_url: "about:welcome".to_string(),
            current_title: "Retro1996 Browser".to_string(),
            dom_tree: None,
            render_tree: None,
            js_variables: HashMap::new(),
            last_interaction_time: Instant::now(),
            page_loaded: false,
            loading_status: LoadingStatus::Idle,
            history: vec!["about:welcome".to_string()],
            history_index: 0,
            cookies: HashMap::new(),
            cache: HashMap::new(),
            favicon_url: None,
            security_level: SecurityLevel::Permissive,
            javascript_enabled: true,
            plugins_enabled: false,
            images_enabled: true,
            css_enabled: true,
        }
    }

    pub fn update_loading_status(&mut self, status: LoadingStatus) {
        self.loading_status = status;
    }

    pub fn push_to_history(&mut self, url: String) {
        if self.history_index < self.history.len() - 1 {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(url);
        self.history_index = self.history.len() - 1;
    }

    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_index < self.history.len() - 1
    }

    pub fn go_back(&mut self) -> Option<String> {
        if self.can_go_back() {
            self.history_index -= 1;
            self.history.get(self.history_index).cloned()
        } else {
            None
        }
    }

    pub fn go_forward(&mut self) -> Option<String> {
        if self.can_go_forward() {
            self.history_index += 1;
            self.history.get(self.history_index).cloned()
        } else {
            None
        }
    }

    pub fn set_current_url(&mut self, url: String) {
        self.current_url = url;
        self.page_loaded = false;
    }

    pub fn set_page_loaded(&mut self) {
        self.page_loaded = true;
        self.loading_status = LoadingStatus::Complete;
    }

    pub fn set_title(&mut self, title: String) {
        self.current_title = title;
    }

    pub fn get_cookie(&self, name: &str) -> Option<String> {
        self.cookies.get(name).cloned()
    }

    pub fn set_cookie(&mut self, name: String, value: String) {
        self.cookies.insert(name, value);
    }

    pub fn has_cached_resource(&self, url: &str) -> bool {
        self.cache.contains_key(url)
    }

    pub fn get_cached_resource(&self, url: &str) -> Option<Vec<u8>> {
        self.cache.get(url).cloned()
    }

    pub fn cache_resource(&mut self, url: String, data: Vec<u8>) {
        self.cache.insert(url, data);
    }
}

impl BrowserContext {
    pub fn new() -> Self {
        BrowserContext {
            rendering_engine: Arc::new(Mutex::new(TrussCore::new())),
            js_engine: Arc::new(Mutex::new(ChronoScript::new())),
            network_manager: Arc::new(Mutex::new(NetworkManager::new())),
            shared_state: Arc::new(Mutex::new(SharedBrowserState::new())),
        }
    }

    pub fn execute_javascript_with_dom_access(&self, script: &str) -> Result<String, String> {
        let mut js_engine = self.js_engine.lock().unwrap();
        js_engine.execute_script(script)
    }

    pub fn update_shared_state<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut SharedBrowserState),
    {
        let mut state = self.shared_state.lock().unwrap();
        f(&mut state);
        Ok(())
    }

    pub fn get_shared_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&SharedBrowserState) -> R,
    {
        let state = self.shared_state.lock().unwrap();
        f(&state)
    }

    pub fn load_url(&self, url: &str) -> Result<(), String> {
        let mut state = self.shared_state.lock().unwrap();
        state.update_loading_status(LoadingStatus::LookingUpHost(url.to_string()));
        drop(state);

        let network_result = {
            let mut network_manager = self.network_manager.lock().unwrap();
            network_manager.fetch_url(url)?
        };

        let html_content = String::from_utf8(network_result.body)
            .map_err(|e| format!("Failed to decode HTML: {}", e))?;

        let mut state = self.shared_state.lock().unwrap();
        state.update_loading_status(LoadingStatus::ParsingHtml);
        state.set_current_url(url.to_string());

        let mut dom_tree = TrussCore::parse_html(&html_content);
        self.extract_metadata(&mut dom_tree, &mut state);

        let mut rendering_engine = self.rendering_engine.lock().unwrap();
        let render_tree = rendering_engine.build_render_tree(&dom_tree);

        state.dom_tree = Some(dom_tree);
        state.render_tree = Some(render_tree);
        state.set_page_loaded();

        Ok(())
    }

    fn extract_metadata(&self, dom_tree: &mut DomNode, state: &mut SharedBrowserState) {
        if let Some(head_node) = dom_tree.find_first_tag("head") {
            if let Some(title_node) = head_node.find_first_tag("title") {
                if let Some(text) = &title_node.children.first() {
                    if let DomNode::Text(content) = text {
                        state.set_title(content.clone());
                    }
                }
            }

            if let Some(favicon_link) = head_node.find_first_with_attr("link", "rel", "shortcut icon") {
                if let Some(href) = favicon_link.attributes.get("href") {
                    state.favicon_url = Some(href.clone());
                }
            }
        }
    }

    pub fn handle_ui_event(&self, event_type: &str, target_element_id: &str) -> Result<(), String> {
        match event_type {
            "click" => {
                if let Ok(mut state) = self.shared_state.lock() {
                    state.last_interaction_time = Instant::now();
                }
                
                if let Some(href) = self.get_element_href(target_element_id)? {
                    self.load_url(&href)?;
                }
            },
            "submit" => {
                self.submit_form(target_element_id)?;
            },
            _ => {},
        }
        Ok(())
    }

    fn get_element_href(&self, element_id: &str) -> Result<Option<String>, String> {
        let state = self.shared_state.lock().unwrap();
        if let Some(ref dom_tree) = state.dom_tree {
            if let Some(element) = dom_tree.find_by_id(element_id) {
                if element.tag_name == "a" {
                    return Ok(element.attributes.get("href").cloned());
                }
            }
        }
        Ok(None)
    }

    fn submit_form(&self, form_id: &str) -> Result<(), String> {
        let state = self.shared_state.lock().unwrap();
        if let Some(ref dom_tree) = state.dom_tree {
            if let Some(form_element) = dom_tree.find_by_id(form_id) {
                if form_element.tag_name == "form" {
                    let action = form_element.attributes.get("action")
                        .cloned()
                        .unwrap_or_default();
                    let method = form_element.attributes.get("method")
                        .cloned()
                        .unwrap_or_else(|| "get".to_string());

                    let mut form_data = Vec::new();
                    self.collect_form_inputs(&form_element, &mut form_data);

                    let url = if method.to_lowercase() == "get" {
                        let query_string = self.build_query_string(&form_data);
                        format!("{}?{}", action, query_string)
                    } else {
                        action
                    };

                    drop(state);
                    self.load_url(&url)?;
                }
            }
        }
        Ok(())
    }

    fn collect_form_inputs(&self, parent: &DomNode, inputs: &mut Vec<(String, String)>) {
        for child in &parent.children {
            match child {
                DomNode::Element { tag_name, attributes, .. } => {
                    if tag_name == "input" {
                        if let (Some(name), Some(value)) = (
                            attributes.get("name"),
                            attributes.get("value")
                        ) {
                            inputs.push((name.clone(), value.clone()));
                        }
                    } else {
                        self.collect_form_inputs(child, inputs);
                    }
                },
                _ => {}
            }
        }
    }

    fn build_query_string(&self, inputs: &[(String, String)]) -> String {
        inputs.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    }

    pub fn execute_javascript(&self, code: &str) -> Result<String, String> {
        let mut js_engine = self.js_engine.lock().unwrap();
        js_engine.execute_script(code)
    }

    pub fn inject_javascript_variable(&self, name: &str, value: &str) -> Result<(), String> {
        let script = format!("var {} = '{}';", name, value);
        self.execute_javascript(&script).map(|_| ()).map_err(|e| e)
    }

    pub fn get_javascript_variable(&self, name: &str) -> Result<String, String> {
        let script = format!("{};", name);
        self.execute_javascript(&script)
    }

    pub fn navigate_back(&self) -> Result<(), String> {
        let previous_url = {
            let mut state = self.shared_state.lock().unwrap();
            state.go_back()
        };

        if let Some(url) = previous_url {
            self.load_url(&url)?;
        }
        Ok(())
    }

    pub fn navigate_forward(&self) -> Result<(), String> {
        let next_url = {
            let mut state = self.shared_state.lock().unwrap();
            state.go_forward()
        };

        if let Some(url) = next_url {
            self.load_url(&url)?;
        }
        Ok(())
    }

    pub fn refresh(&self) -> Result<(), String> {
        let current_url = {
            let state = self.shared_state.lock().unwrap();
            state.current_url.clone()
        };
        self.load_url(&current_url)
    }

    pub fn stop_loading(&self) -> Result<(), String> {
        let mut network_manager = self.network_manager.lock().unwrap();
        network_manager.cancel_current_request();
        Ok(())
    }

    pub fn get_current_url(&self) -> String {
        let state = self.shared_state.lock().unwrap();
        state.current_url.clone()
    }

    pub fn get_current_title(&self) -> String {
        let state = self.shared_state.lock().unwrap();
        state.current_title.clone()
    }

    pub fn is_page_loaded(&self) -> bool {
        let state = self.shared_state.lock().unwrap();
        state.page_loaded
    }

    pub fn get_loading_status(&self) -> LoadingStatus {
        let state = self.shared_state.lock().unwrap();
        state.loading_status.clone()
    }

    pub fn enable_javascript(&self) {
        let mut state = self.shared_state.lock().unwrap();
        state.javascript_enabled = true;
    }

    pub fn disable_javascript(&self) {
        let mut state = self.shared_state.lock().unwrap();
        state.javascript_enabled = false;
    }

    pub fn are_images_enabled(&self) -> bool {
        let state = self.shared_state.lock().unwrap();
        state.images_enabled
    }

    pub fn toggle_images(&self) {
        let mut state = self.shared_state.lock().unwrap();
        state.images_enabled = !state.images_enabled;
    }

    pub fn get_favicon_url(&self) -> Option<String> {
        let state = self.shared_state.lock().unwrap();
        state.favicon_url.clone()
    }

    pub fn get_security_level(&self) -> SecurityLevel {
        let state = self.shared_state.lock().unwrap();
        state.security_level.clone()
    }

    pub fn set_security_level(&self, level: SecurityLevel) {
        let mut state = self.shared_state.lock().unwrap();
        state.security_level = level;
    }
}