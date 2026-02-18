use egui::{Color32, FontId, Style, TextStyle};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AccessibilitySettings {
    pub high_contrast_mode: bool,
    pub large_text_mode: bool,
    pub text_only_mode: bool,
    pub minimum_font_size: u8,
    pub force_colors: bool,
    pub screen_reader_support: bool,
    pub keyboard_navigation_only: bool,
    pub skip_images: bool,
    pub alt_text_priority: bool,
    pub title_tooltip_priority: bool,
    pub show_alt_text_as_tooltip: bool,
    pub invert_colors: bool,
    pub reduce_motion: bool,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        AccessibilitySettings {
            high_contrast_mode: false,
            large_text_mode: false,
            text_only_mode: false,
            minimum_font_size: 12,
            force_colors: false,
            screen_reader_support: false,
            keyboard_navigation_only: false,
            skip_images: false,
            alt_text_priority: true,
            title_tooltip_priority: true,
            show_alt_text_as_tooltip: true,
            invert_colors: false,
            reduce_motion: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessibleElement {
    pub element_type: ElementType,
    pub role: String,
    pub label: String,
    pub description: String,
    pub id: String,
    pub tab_index: i32,
}

#[derive(Debug, Clone)]
pub enum ElementType {
    Button,
    Link,
    Input,
    Image,
    Heading,
    Paragraph,
    ListItem,
    Table,
    Form,
    Navigation,
    Region,
    Main,
    Banner,
    ContentInfo,
    Complementary,
}

#[derive(Debug, Clone)]
pub struct Landmark {
    pub id: String,
    pub role: String,
    pub label: String,
}

pub struct AccessibilityManager {
    pub settings: AccessibilitySettings,
    pub current_focus_element_id: Option<String>,
    pub focus_history: Vec<String>,
    pub element_labels: HashMap<String, String>,
    pub element_descriptions: HashMap<String, String>,
    pub landmarks: HashMap<String, Landmark>,
    pub screen_reader_active: bool,
    pub navigation_stack: Vec<String>,
    pub announce_queue: Vec<String>,
}

impl AccessibilityManager {
    pub fn new() -> Self {
        AccessibilityManager {
            settings: AccessibilitySettings::default(),
            current_focus_element_id: None,
            focus_history: Vec::new(),
            element_labels: HashMap::new(),
            element_descriptions: HashMap::new(),
            landmarks: HashMap::new(),
            screen_reader_active: false,
            navigation_stack: Vec::new(),
            announce_queue: Vec::new(),
        }
    }

    pub fn update_settings(&mut self, new_settings: AccessibilitySettings) {
        self.settings = new_settings;
    }

    pub fn apply_visual_adjustments(&self, style: &mut Style) {
        if self.settings.high_contrast_mode {
            style.visuals.hyperlink_color = Color32::BLACK;
            style.visuals.extreme_bg_color = Color32::WHITE;
            style.visuals.text_cursor.color = Color32::BLACK;
        }

        if self.settings.invert_colors {
            let temp = style.visuals.extreme_bg_color;
            let text_color = style.visuals.text_color();
            style.visuals.extreme_bg_color = text_color;
            // Note: text_color is a getter, can't directly set it
        }

        if self.settings.large_text_mode {
            let new_size = (self.settings.minimum_font_size as f32).max(16.0);
            if let Some(font_id) = style.text_styles.get_mut(&TextStyle::Body) {
                font_id.size = new_size;
            }
            if let Some(font_id) = style.text_styles.get_mut(&TextStyle::Heading) {
                font_id.size = new_size * 1.2;
            }
            if let Some(font_id) = style.text_styles.get_mut(&TextStyle::Button) {
                font_id.size = new_size;
            }
        }
    }

    pub fn register_element(&mut self, element: AccessibleElement) {
        self.element_labels.insert(element.id.clone(), element.label.clone());
        self.element_descriptions.insert(element.id.clone(), element.description.clone());
    }

    pub fn focus_element(&mut self, element_id: String) {
        if let Some(current_id) = &self.current_focus_element_id {
            self.focus_history.push(current_id.clone());
        }
        self.current_focus_element_id = Some(element_id);
    }

    pub fn navigate_forward(&mut self) {
        if let Some(current_id) = &self.current_focus_element_id {
            self.navigation_stack.push(current_id.clone());
        }
    }

    pub fn navigate_backward(&mut self) {
        if let Some(prev_id) = self.navigation_stack.pop() {
            self.current_focus_element_id = Some(prev_id);
        }
    }

    pub fn process_html_for_accessibility(&self, html: &str) -> String {
        let mut processed_html = html.to_string();
        
        if self.settings.text_only_mode {
            processed_html = processed_html.replace("<img", "<!-- IMG REMOVED (TEXT ONLY MODE) --><span>IMAGE REMOVED</span>");
            processed_html = format!(
                r#"<style>
                    img, picture, video, audio, canvas {{
                        display: none !important;
                    }}
                    </style>
                    {}"#,
                processed_html
            );
        }
        
        if self.settings.skip_images {
            processed_html = processed_html.replace("<img", "<span class='image-skip'>[IMAGE SKIPPED]</span><img");
        }
        
        processed_html
    }

    pub fn announce_to_screen_reader(&mut self, text: &str) {
        if self.screen_reader_active {
            self.announce_queue.push(text.to_string());
            println!("[SCREEN READER SIMULATOR]: {}", text);
        }
    }

    pub fn get_accessibility_keyboard_shortcuts(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("Tab", "Move to next link/control"),
            ("Shift+Tab", "Move to previous link/control"),
            ("Alt+Home", "Go to homepage (if supported by OS/browser)"),
            ("Alt+Left Arrow", "Go back"),
            ("Alt+Right Arrow", "Go forward"),
            ("Ctrl+0", "Reset zoom level"),
            ("Ctrl++", "Zoom in"),
            ("Ctrl+-", "Zoom out"),
        ]
    }

    pub fn handle_key_event(&mut self, key: &str) -> bool {
        match key {
            "Tab" => {
                self.navigate_forward();
                true
            },
            "Alt+Home" => {
                self.announce_to_screen_reader("Navigating to homepage");
                true
            },
            "Alt+Left" => {
                self.navigate_backward();
                true
            },
            _ => false,
        }
    }

    pub fn register_landmark(&mut self, landmark: Landmark) {
        self.landmarks.insert(landmark.id.clone(), landmark);
    }

    pub fn get_current_focus_label(&self) -> String {
        if let Some(id) = &self.current_focus_element_id {
            self.element_labels.get(id).cloned().unwrap_or_default()
        } else {
            "No focused element".to_string()
        }
    }

    pub fn get_current_focus_description(&self) -> String {
        if let Some(id) = &self.current_focus_element_id {
            self.element_descriptions.get(id).cloned().unwrap_or_default()
        } else {
            "".to_string()
        }
    }

    pub fn clear_announce_queue(&mut self) -> Vec<String> {
        self.announce_queue.drain(..).collect()
    }

    pub fn enable_screen_reader(&mut self) {
        self.screen_reader_active = true;
    }

    pub fn disable_screen_reader(&mut self) {
        self.screen_reader_active = false;
    }
}

pub struct AccessibilityUtils;

impl AccessibilityUtils {
    pub fn get_heading_level(heading_tag: &str) -> Option<u8> {
        match heading_tag.to_lowercase().as_str() {
            "h1" => Some(1),
            "h2" => Some(2),
            "h3" => Some(3),
            "h4" => Some(4),
            "h5" => Some(5),
            "h6" => Some(6),
            _ => None,
        }
    }

    pub fn calculate_simple_contrast_ratio(luminance1: f64, luminance2: f64) -> f64 {
        let lighter = luminance1.max(luminance2);
        let darker = luminance1.min(luminance2);
        (lighter + 0.05) / (darker + 0.05)
    }

    pub fn relative_luminance(color: (u8, u8, u8)) -> f64 {
        let r = color.0 as f64 / 255.0;
        let g = color.1 as f64 / 255.0;
        let b = color.2 as f64 / 255.0;

        let r = if r <= 0.03928 {
            r / 12.92
        } else {
            ((r + 0.055) / 1.055).powf(2.4)
        };
        let g = if g <= 0.03928 {
            g / 12.92
        } else {
            ((g + 0.055) / 1.055).powf(2.4)
        };
        let b = if b <= 0.03928 {
            b / 12.92
        } else {
            ((b + 0.055) / 1.055).powf(2.4)
        };

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    pub fn is_valid_contrast_ratio(color1: (u8, u8, u8), color2: (u8, u8, u8), min_ratio: f64) -> bool {
        let lum1 = AccessibilityUtils::relative_luminance(color1);
        let lum2 = AccessibilityUtils::relative_luminance(color2);
        let contrast = AccessibilityUtils::calculate_simple_contrast_ratio(lum1, lum2);
        contrast >= min_ratio
    }

    pub fn get_accessible_color_pair(background: (u8, u8, u8)) -> (u8, u8, u8) {
        let bg_luminance = AccessibilityUtils::relative_luminance(background);
        if bg_luminance > 0.5 {
            (0, 0, 0) 
        } else {
            (255, 255, 255) 
        }
    }
}