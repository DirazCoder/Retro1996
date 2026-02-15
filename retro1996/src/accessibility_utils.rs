use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ScreenReader {
    pub enabled: bool,
    pub volume: u8,
    pub speed: u8,
    pub pitch: u8,
    pub voice: String,
    pub text_buffer: Vec<String>,
    pub position: usize,
    pub reading_mode: ReadingMode,
    pub announce_links: bool,
    pub announce_images: bool,
    pub announce_headings: bool,
    pub announce_form_controls: bool,
}

#[derive(Debug, Clone)]
pub enum ReadingMode {
    Normal,
    Reading,
    Skimming,
    DocumentStructure,
}

impl Default for ScreenReader {
    fn default() -> Self {
        ScreenReader {
            enabled: false,
            volume: 80,
            speed: 180,
            pitch: 50,
            voice: "default".to_string(),
            text_buffer: Vec::new(),
            position: 0,
            reading_mode: ReadingMode::Normal,
            announce_links: true,
            announce_images: true,
            announce_headings: true,
            announce_form_controls: true,
        }
    }
}

impl ScreenReader {
    pub fn new() -> Self {
        ScreenReader::default()
    }

    pub fn add_text(&mut self, text: &str) {
        self.text_buffer.push(text.to_string());
    }

    pub fn add_html_content(&mut self, html: &str) {
        let processed = self.extract_text_from_html(html);
        for item in processed {
            self.add_text(&item);
        }
    }

    pub fn extract_text_from_html(&self, html: &str) -> Vec<String> {
        let mut output = Vec::new();
        let mut pos = 0;
        let html_lower = html.to_lowercase();

        while pos < html.len() {
            if html[pos..].starts_with('<') {
                let tag_end_offset = html[pos..].find('>').unwrap_or(0);
                if tag_end_offset == 0 {
                    pos += 1;
                    continue;
                }
                
                let tag_content = &html_lower[pos+1..pos+tag_end_offset];
                let tag_name = tag_content.split_whitespace().next().unwrap_or("");

                match tag_name {
                    "h1" => output.push("[Heading Level 1]".to_string()),
                    "h2" => output.push("[Heading Level 2]".to_string()),
                    "h3" => output.push("[Heading Level 3]".to_string()),
                    "h4" => output.push("[Heading Level 4]".to_string()),
                    "h5" => output.push("[Heading Level 5]".to_string()),
                    "h6" => output.push("[Heading Level 6]".to_string()),
                    "p" => output.push("[Paragraph]".to_string()),
                    "ul" => output.push("[Unordered List]".to_string()),
                    "ol" => output.push("[Ordered List]".to_string()),
                    "li" => output.push("[List Item]".to_string()),
                    "table" => output.push("[Table]".to_string()),
                    "tr" => output.push("[Table Row]".to_string()),
                    "td" | "th" => output.push("[Table Cell]".to_string()),
                    "form" => output.push("[Form]".to_string()),
                    "input" => {
                        if self.announce_form_controls {
                            let input_type = self.get_attribute_value(tag_content, "type").unwrap_or("text".to_string());
                            let input_label = self.get_attribute_value(tag_content, "name").unwrap_or("unnamed".to_string());
                            output.push(format!("[Input: {} ({})]", input_label, input_type));
                        }
                    },
                    "a" => {
                        if self.announce_links {
                            let href = self.get_attribute_value(tag_content, "href").unwrap_or("#".to_string());
                            output.push(format!("[Link: {}]", href));
                        }
                    },
                    "img" => {
                        if self.announce_images {
                            let alt = self.get_attribute_value(tag_content, "alt");
                            if let Some(alt_text) = alt {
                                if !alt_text.is_empty() {
                                    output.push(format!("[Image: {}]", alt_text));
                                } else {
                                    output.push("[Decorative Image]".to_string());
                                }
                            } else {
                                output.push("[Image without description]".to_string());
                            }
                        }
                    },
                    "hr" => output.push("[Horizontal Rule]".to_string()),
                    _ => {}
                }
                pos += tag_end_offset + 1;
            } else {
                let text_end = html[pos..].find('<').unwrap_or(html.len() - pos);
                let text = &html[pos..pos + text_end].trim();
                if !text.is_empty() {
                    output.push(text.to_string());
                }
                pos += text_end;
            }
        }
        output
    }

    fn get_attribute_value(&self, tag_content: &str, attr_name: &str) -> Option<String> {
        let attr_pattern = format!("{}=\"", attr_name);
        if let Some(start) = tag_content.to_lowercase().find(&attr_pattern) {
            let value_start = start + attr_pattern.len();
            if let Some(value_end) = tag_content[value_start..].find('"') {
                return Some(tag_content[value_start..value_start + value_end].to_string());
            }
        }
        
        let attr_pattern_single = format!("{}='", attr_name);
        if let Some(start) = tag_content.to_lowercase().find(&attr_pattern_single) {
            let value_start = start + attr_pattern_single.len();
            if let Some(value_end) = tag_content[value_start..].find('\'') {
                return Some(tag_content[value_start..value_start + value_end].to_string());
            }
        }
        None
    }

    pub fn next_item(&mut self) -> Option<String> {
        if self.position < self.text_buffer.len() {
            let item = self.text_buffer[self.position].clone();
            self.position += 1;
            Some(item)
        } else {
            None
        }
    }

    pub fn previous_item(&mut self) -> Option<String> {
        if self.position > 0 {
            self.position -= 1;
            Some(self.text_buffer[self.position].clone())
        } else {
            None
        }
    }

    pub fn reset_position(&mut self) {
        self.position = 0;
    }

    pub fn jump_to(&mut self, index: usize) {
        if index < self.text_buffer.len() {
            self.position = index;
        }
    }

    pub fn current_item(&self) -> Option<String> {
        if self.position < self.text_buffer.len() {
            Some(self.text_buffer[self.position].clone())
        } else {
            None
        }
    }

    pub fn speak(&self, text: &str) {
        if self.enabled {
            println!("[SCREEN READER]: {}", text);
        }
    }

    pub fn read_all(&mut self) {
        if !self.enabled {
            return;
        }
        for item in &self.text_buffer {
            self.speak(item);
        }
    }

    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }
}

#[derive(Debug, Clone)]
pub struct HighContrastFilter {
    pub enabled: bool,
    pub scheme: HighContrastScheme,
    pub custom_foreground: (u8, u8, u8),
    pub custom_background: (u8, u8, u8),
    pub custom_link_color: (u8, u8, u8),
}

#[derive(Debug, Clone)]
pub enum HighContrastScheme {
    BlackOnWhite,
    WhiteOnBlack,
    YellowOnBlack,
    Custom,
}

impl Default for HighContrastFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl HighContrastFilter {
    pub fn new() -> Self {
        HighContrastFilter {
            enabled: false,
            scheme: HighContrastScheme::BlackOnWhite,
            custom_foreground: (0, 0, 0),
            custom_background: (255, 255, 255),
            custom_link_color: (0, 0, 255),
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_scheme(&mut self, scheme: HighContrastScheme) {
        self.scheme = scheme;
    }

    pub fn get_foreground_color(&self) -> (u8, u8, u8) {
        if !self.enabled {
            return (0, 0, 0); // Default black
        }
        match self.scheme {
            HighContrastScheme::WhiteOnBlack => (255, 255, 255),
            HighContrastScheme::BlackOnWhite => (0, 0, 0),
            HighContrastScheme::YellowOnBlack => (255, 255, 0),
            HighContrastScheme::Custom => self.custom_foreground,
        }
    }

    pub fn get_background_color(&self) -> (u8, u8, u8) {
        if !self.enabled {
            return (255, 255, 255); // Default white background
        }
        match self.scheme {
            HighContrastScheme::WhiteOnBlack => (0, 0, 0),
            HighContrastScheme::BlackOnWhite => (255, 255, 255),
            HighContrastScheme::YellowOnBlack => (0, 0, 0),
            HighContrastScheme::Custom => self.custom_background,
        }
    }

    pub fn get_link_color(&self) -> (u8, u8, u8) {
        if !self.enabled {
            return (0, 0, 255); // Default blue link
        }
        match self.scheme {
            HighContrastScheme::Custom => self.custom_link_color,
            _ => (0, 255, 255), // Bright cyan for links in standard schemes
        }
    }

    pub fn set_custom_colors(&mut self, fg: (u8, u8, u8), bg: (u8, u8, u8), link: (u8, u8, u8)) {
        self.custom_foreground = fg;
        self.custom_background = bg;
        self.custom_link_color = link;
        self.scheme = HighContrastScheme::Custom;
    }
}

#[derive(Debug, Clone)]
pub struct FontMagnifier {
    pub magnification: f32,
    pub min_size: f32,
    pub max_size: f32,
}

impl Default for FontMagnifier {
    fn default() -> Self {
        Self::new()
    }
}

impl FontMagnifier {
    pub fn new() -> Self {
        FontMagnifier {
            magnification: 1.0,
            min_size: 8.0,
            max_size: 72.0,
        }
    }

    pub fn zoom_in(&mut self) {
        self.magnification *= 1.25;
    }

    pub fn zoom_out(&mut self) {
        self.magnification /= 1.25;
    }

    pub fn reset(&mut self) {
        self.magnification = 1.0;
    }

    pub fn apply(&self, original_size: f32) -> f32 {
        let magnified = original_size * self.magnification;
        magnified.clamp(self.min_size, self.max_size)
    }

    pub fn set_magnification(&mut self, factor: f32) {
        self.magnification = factor.clamp(0.5, 5.0);
    }
}

#[derive(Debug, Clone)]
pub struct KeyboardNavigator {
    pub focusables: Vec<String>,
    pub current_focus_index: usize,
    pub cycling: bool,
}

impl Default for KeyboardNavigator {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardNavigator {
    pub fn new() -> Self {
        KeyboardNavigator {
            focusables: Vec::new(),
            current_focus_index: 0,
            cycling: true,
        }
    }

    pub fn add_focusable(&mut self, element_id: String) {
        self.focusables.push(element_id);
    }

    pub fn remove_focusable(&mut self, element_id: &str) {
        self.focusables.retain(|id| id != element_id);
        if self.current_focus_index >= self.focusables.len() {
            self.current_focus_index = self.focusables.len().saturating_sub(1);
        }
    }

    pub fn focus_next(&mut self) -> Option<String> {
        if self.focusables.is_empty() {
            return None;
        }
        
        if self.cycling {
            self.current_focus_index = (self.current_focus_index + 1) % self.focusables.len();
        } else if self.current_focus_index < self.focusables.len() - 1 {
            self.current_focus_index += 1;
        }
        
        self.focusables.get(self.current_focus_index).cloned()
    }

    pub fn focus_previous(&mut self) -> Option<String> {
        if self.focusables.is_empty() {
            return None;
        }
        
        if self.cycling {
            self.current_focus_index = if self.current_focus_index == 0 {
                self.focusables.len() - 1
            } else {
                self.current_focus_index - 1
            };
        } else if self.current_focus_index > 0 {
            self.current_focus_index -= 1;
        }
        
        self.focusables.get(self.current_focus_index).cloned()
    }

    pub fn focus_first(&mut self) -> Option<String> {
        if self.focusables.is_empty() {
            return None;
        }
        self.current_focus_index = 0;
        self.focusables.first().cloned()
    }

    pub fn focus_last(&mut self) -> Option<String> {
        if self.focusables.is_empty() {
            return None;
        }
        self.current_focus_index = self.focusables.len() - 1;
        self.focusables.last().cloned()
    }

    pub fn get_current_focus(&self) -> Option<String> {
        self.focusables.get(self.current_focus_index).cloned()
    }

    pub fn set_focus(&mut self, element_id: &str) -> bool {
        if let Some(index) = self.focusables.iter().position(|id| id == element_id) {
            self.current_focus_index = index;
            true
        } else {
            false
        }
    }

    pub fn get_focusables(&self) -> &Vec<String> {
        &self.focusables
    }

    pub fn clear_focusables(&mut self) {
        self.focusables.clear();
        self.current_focus_index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_reader() {
        let mut reader = ScreenReader::new();
        reader.add_text("Hello, world!");
        reader.add_text("This is a test.");
        
        assert_eq!(reader.next_item(), Some("Hello, world!".to_string()));
        assert_eq!(reader.next_item(), Some("This is a test.".to_string()));
        assert_eq!(reader.next_item(), None);
    }

    #[test]
    fn test_high_contrast_filter() {
        let mut filter = HighContrastFilter::new();
        filter.enable();
        filter.set_scheme(HighContrastScheme::WhiteOnBlack);
        assert_eq!(filter.get_foreground_color(), (255, 255, 255));
        assert_eq!(filter.get_background_color(), (0, 0, 0));
    }

    #[test]
    fn test_font_magnifier() {
        let mut magnifier = FontMagnifier::new();
        assert_eq!(magnifier.apply(12.0), 12.0);
        magnifier.zoom_in();
        assert_eq!(magnifier.apply(12.0), 15.0); // 12 * 1.25 = 15
        magnifier.reset();
        assert_eq!(magnifier.apply(12.0), 12.0);
    }

    #[test]
    fn test_keyboard_navigator() {
        let mut navigator = KeyboardNavigator::new();
        navigator.add_focusable("link1".to_string());
        navigator.add_focusable("button1".to_string());
        navigator.add_focusable("input1".to_string());
        
        assert_eq!(navigator.focus_next(), Some("link1".to_string()));
        assert_eq!(navigator.focus_next(), Some("button1".to_string()));
        assert_eq!(navigator.focus_next(), Some("input1".to_string()));
        assert_eq!(navigator.focus_next(), Some("link1".to_string())); // Wraps around
    }
}