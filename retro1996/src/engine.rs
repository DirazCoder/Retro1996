use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use image::RgbaImage;
use url::Url;
use lazy_static::lazy_static;

use crate::javascript_engine::ChronoScript;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderingMode {
    Netscape3,
    IE3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParsingMode {
    Strict,
    Quirks1996,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayType {
    Block,
    Inline,
    InlineBlock,
    None,
}

#[derive(Debug, Clone)]
pub struct QuirkProfile {
    pub collapse_margin_bug: bool,
    pub percent_width_bug: bool,
    pub nested_table_bug: bool,
    pub font_line_height_bug: bool,
    pub inline_block_spacing_bug: bool,
    pub table_cell_padding_bug: bool,
    pub form_field_rendering_bug: bool,
    pub image_alignment_bug: bool,
}

impl QuirkProfile {
    pub fn netscape_3() -> Self {
        QuirkProfile {
            collapse_margin_bug: true,
            percent_width_bug: true,
            nested_table_bug: true,
            font_line_height_bug: true,
            inline_block_spacing_bug: true,
            table_cell_padding_bug: true,
            form_field_rendering_bug: true,
            image_alignment_bug: true,
        }
    }

    pub fn ie_3() -> Self {
        QuirkProfile {
            collapse_margin_bug: false,
            percent_width_bug: true,
            nested_table_bug: false,
            font_line_height_bug: true,
            inline_block_spacing_bug: false,
            table_cell_padding_bug: false,
            form_field_rendering_bug: false,
            image_alignment_bug: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CssValue {
    Length(f32),
    Percentage(f32),
    Color(u8, u8, u8),
    Keyword(String),
    Auto,
}

#[derive(Debug, Clone)]
pub struct CssRule {
    pub selector: String,
    pub properties: HashMap<String, CssValue>,
}

pub struct WebSafePalette;

impl WebSafePalette {
    pub fn snap_to_web_safe(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        (
            Self::snap_component(r),
            Self::snap_component(g),
            Self::snap_component(b),
        )
    }

    fn snap_component(component: u8) -> u8 {
        let index = (component as f32 / 255.0 * 5.0).round() as u8;
        (index * 51).min(255)
    }

    pub fn parse_color(color_str: &str) -> Option<(u8, u8, u8)> {
        let color = color_str.trim_start_matches('#');
        if color.len() == 6 {
            let r = u8::from_str_radix(&color[0..2], 16).ok()?;
            let g = u8::from_str_radix(&color[2..4], 16).ok()?;
            let b = u8::from_str_radix(&color[4..6], 16).ok()?;
            Some(Self::snap_to_web_safe(r, g, b))
        } else if color.len() == 3 {
            let r = u8::from_str_radix(&color[0..1], 16).ok()?;
            let g = u8::from_str_radix(&color[1..2], 16).ok()?;
            let b = u8::from_str_radix(&color[2..3], 16).ok()?;
            Some(Self::snap_to_web_safe(r * 17, g * 17, b * 17))
        } else {
            Self::parse_named_color(&color.to_lowercase())
        }
    }

    fn parse_named_color(name: &str) -> Option<(u8, u8, u8)> {
        match name {
            "black" => Some((0, 0, 0)),
            "white" => Some((255, 255, 255)),
            "red" => Some((255, 0, 0)),
            "green" => Some((0, 128, 0)),
            "blue" => Some((0, 0, 255)),
            "yellow" => Some((255, 255, 0)),
            "cyan" => Some((0, 255, 255)),
            "magenta" => Some((255, 0, 255)),
            "orange" => Some((255, 165, 0)),
            "purple" => Some((128, 0, 128)),
            "brown" => Some((165, 42, 42)),
            "pink" => Some((255, 192, 203)),
            "gray" | "grey" => Some((128, 128, 128)),
            "silver" => Some((192, 192, 192)),
            "lime" => Some((0, 255, 0)),
            "navy" => Some((0, 0, 128)),
            "olive" => Some((128, 128, 0)),
            "teal" => Some((0, 128, 128)),
            "maroon" => Some((128, 0, 0)),
            "aqua" => Some((0, 255, 255)),
            "fuchsia" => Some((255, 0, 255)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DomNode {
    Element {
        tag: String,
        attributes: HashMap<String, String>,
        children: Vec<DomNode>,
        id: Option<String>,
        class: Option<String>,
        node_id: usize,
    },
    Text(String),
    Comment(String),
}

impl DomNode {
    pub fn tag(&self) -> &str {
        match self {
            DomNode::Element { tag, .. } => tag,
            _ => "",
        }
    }

    pub fn attributes(&self) -> &HashMap<String, String> {
        match self {
            DomNode::Element { attributes, .. } => attributes,
            _ => &EMPTY_ATTRS,
        }
    }

    pub fn children(&self) -> &Vec<DomNode> {
        match self {
            DomNode::Element { children, .. } => children,
            _ => &EMPTY_CHILDREN,
        }
    }

    pub fn node_id(&self) -> Option<usize> {
        match self {
            DomNode::Element { node_id, .. } => Some(*node_id),
            _ => None,
        }
    }

    pub fn find_by_id(&self, id: &str) -> Option<&DomNode> {
        match self {
            DomNode::Element { id: ref node_id, .. } if node_id.as_deref() == Some(id) => Some(self),
            DomNode::Element { ref children, .. } => {
                for child in children {
                    if let Some(found) = child.find_by_id(id) {
                        return Some(found);
                    }
                }
                None
            },
            _ => None,
        }
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<&DomNode> {
        let mut results = Vec::new();
        match self {
            DomNode::Element { tag: ref node_tag, ref children, .. } if node_tag == tag => {
                results.push(self);
                for child in children {
                    results.extend(child.find_by_tag(tag));
                }
            },
            DomNode::Element { ref children, .. } => {
                for child in children {
                    results.extend(child.find_by_tag(tag));
                }
            },
            _ => {},
        }
        results
    }

    pub fn find_by_node_id(&self, node_id: usize) -> Option<&DomNode> {
        match self {
            DomNode::Element { node_id: current_node_id, .. } if *current_node_id == node_id => Some(self),
            DomNode::Element { ref children, .. } => {
                for child in children {
                    if let Some(found) = child.find_by_node_id(node_id) {
                        return Some(found);
                    }
                }
                None
            },
            _ => None,
        }
    }

    pub fn text_content(&self) -> String {
        match self {
            DomNode::Text(text) => text.clone(),
            DomNode::Element { children, .. } => {
                children.iter().map(|c| c.text_content()).collect()
            },
            DomNode::Comment(_) => String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StyleContext {
    pub font_face: String,
    pub font_size: f32,
    pub color: (u8, u8, u8),
    pub background_color: Option<(u8, u8, u8)>,
    pub text_decoration: String,
    pub text_align: String,
    pub vertical_align: String,
    pub font_weight: String,
    pub font_style: String,
}

impl Default for StyleContext {
    fn default() -> Self {
        StyleContext {
            font_face: "Times New Roman".to_string(),
            font_size: 16.0,
            color: (0, 0, 0),
            background_color: None,
            text_decoration: String::new(),
            text_align: "left".to_string(),
            vertical_align: "baseline".to_string(),
            font_weight: "normal".to_string(),
            font_style: "normal".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct ExclusionZone {
    pub rect: Rect,
    pub alignment: String,
}

#[derive(Debug, Clone)]
pub struct BoxModel {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub border_top: f32,
    pub border_right: f32,
    pub border_bottom: f32,
    pub border_left: f32,
    pub background_color: Option<(u8, u8, u8)>,
    pub color: (u8, u8, u8),
}

impl Default for BoxModel {
    fn default() -> Self {
        BoxModel {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            border_top: 0.0,
            border_right: 0.0,
            border_bottom: 0.0,
            border_left: 0.0,
            background_color: None,
            color: (0, 0, 0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FloatContext {
    pub left_floats: Vec<BoxModel>,
    pub right_floats: Vec<BoxModel>,
    pub exclusion_zones: Vec<ExclusionZone>,
    pub current_y: f32,
}

impl FloatContext {
    pub fn new() -> Self {
        FloatContext {
            left_floats: Vec::new(),
            right_floats: Vec::new(),
            exclusion_zones: Vec::new(),
            current_y: 0.0,
        }
    }

    pub fn get_available_space(&self, y: f32, _height: f32) -> (f32, f32) {
        let mut left_edge: f32 = 0.0;
        let mut right_edge: f32 = 1000.0;
        
        for zone in &self.exclusion_zones {
            if y >= zone.rect.y && y < zone.rect.y + zone.rect.height {
                match zone.alignment.as_str() {
                    "left" => left_edge = left_edge.max(zone.rect.x + zone.rect.width),
                    "right" => right_edge = right_edge.min(zone.rect.x),
                    _ => {}
                }
            }
        }
        
        (left_edge, (right_edge - left_edge).max(0.0))
    }

    pub fn add_exclusion_zone(&mut self, rect: Rect, alignment: &str) {
        self.exclusion_zones.push(ExclusionZone {
            rect,
            alignment: alignment.to_string(),
        });
    }

    pub fn reset(&mut self) {
        self.left_floats.clear();
        self.right_floats.clear();
        self.exclusion_zones.clear();
        self.current_y = 0.0;
    }
}

impl Default for FloatContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum SpecialElement {
    Image {
        src: String,
        alt: String,
        width: Option<u32>,
        height: Option<u32>,
        natural_width: u32,
        natural_height: u32,
        loaded: bool,
        animated_gif: Option<()>,
        visible: bool,
        usemap: Option<String>,
        align: Option<String>,
    },
    Map {
        name: String,
        areas: Vec<ImageArea>,
    },
    Form {
        action: String,
        method: String,
        name: String,
    },
    Input {
        input_type: String,
        name: String,
        value: String,
        checked: bool,
        disabled: bool,
    },
    Link {
        href: String,
        target: String,
    },
    Script {
        src: Option<String>,
        content: String,
        language: String,
    },
    Frame {
        src: String,
        name: String,
        width: String,
        height: String,
    },
    Frameset {
        rows: String,
        cols: String,
        border: String,
        frameborder: String,
        framespacing: String,
    },
    Marquee {
        direction: String,
        speed: u32,
        behavior: String,
        position: f32,
        content_width: f32,
    },
    Blink {
        start_time: Instant,
        visible: bool,
    },
    Font {
        face: Option<String>,
        size: Option<i32>,
        color: Option<(u8, u8, u8)>,
    },
    Body {
        bgcolor: Option<(u8, u8, u8)>,
        background: Option<String>,
    },
    Table {
        border: u32,
        cellpadding: u32,
        cellspacing: u32,
    },
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ImageArea {
    pub shape: String,
    pub coords: Vec<i32>,
    pub href: String,
    pub alt: String,
}

#[derive(Debug, Clone)]
pub struct TableCellData {
    pub content: RenderNode,
    pub colspan: usize,
    pub rowspan: usize,
    pub width: f32,
    pub height: f32,
    pub nowrap: bool,
}

#[derive(Debug, Clone)]
pub struct TableBox {
    pub rows: Vec<Vec<Option<TableCellData>>>,
    pub column_widths: Vec<f32>,
    pub row_heights: Vec<f32>,
    pub border: f32,
    pub cellpadding: f32,
    pub cellspacing: f32,
}

#[derive(Debug, Clone)]
pub struct RenderNode {
    pub dom_node: DomNode,
    pub box_model: BoxModel,
    pub special: SpecialElement,
    pub children: Vec<RenderNode>,
    pub text_content: String,
    pub computed_styles: HashMap<String, CssValue>,
    pub table_data: Option<TableBox>,
    pub z_index: i32,
    pub absolute_x: f32,
    pub absolute_y: f32,
    pub dirty: bool,
    pub style_version: u64,
    pub layout_version: u64,
}

#[derive(Debug, Clone)]
pub enum RenderCommand {
    DrawRect { x: f32, y: f32, w: f32, h: f32, color: (u8, u8, u8) },
    DrawText { x: f32, y: f32, text: String, font: String, size: f32, color: (u8, u8, u8) },
    DrawImage { x: f32, y: f32, w: f32, h: f32, image_handle: usize },
    DrawTableBorder { x: f32, y: f32, w: f32, h: f32, border_width: f32, border_color: (u8, u8, u8) },
}

#[derive(Debug, Clone)]
pub struct DisplayList {
    pub commands: Vec<RenderCommand>,
}

impl DisplayList {
    pub fn new() -> Self {
        DisplayList { commands: Vec::new() }
    }
}

impl Default for DisplayList {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub rendering_mode: RenderingMode,
    pub authentic_mode: bool,
    pub modern_scaling: bool,
    pub smoothing: bool,
    pub emulate_800x600_viewport: bool,
    pub dpi_scale: f32,
    pub enable_javascript: bool,
    pub enable_images: bool,
    pub enable_plugins: bool,
    pub progressive_rendering: bool,
    pub web_safe_palette: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            rendering_mode: RenderingMode::Netscape3,
            authentic_mode: true,
            modern_scaling: true,
            smoothing: false,
            emulate_800x600_viewport: true,
            dpi_scale: 1.0,
            enable_javascript: true,
            enable_images: true,
            enable_plugins: false,
            progressive_rendering: true,
            web_safe_palette: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DisplayContext {
    pub dpi_scale: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub device_pixel_ratio: f32,
}

impl Default for DisplayContext {
    fn default() -> Self {
        DisplayContext {
            dpi_scale: 1.0,
            viewport_width: 800.0,
            viewport_height: 600.0,
            device_pixel_ratio: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EngineError {
    ParseError(String),
    ImageDecodeError(String),
    NetworkError(String),
    ScriptExecutionError(String),
    InvalidOperation(String),
    NotFound(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            EngineError::ImageDecodeError(msg) => write!(f, "Image decode error: {}", msg),
            EngineError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            EngineError::ScriptExecutionError(msg) => write!(f, "Script execution error: {}", msg),
            EngineError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            EngineError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for EngineError {}

#[derive(Debug, Clone)]
pub enum DomMutation {
    SetAttribute { node_id: usize, key: String, value: String },
    SetStyle { node_id: usize, property: String, value: String },
    InsertChild { parent_id: usize, child: DomNode },
    RemoveChild { parent_id: usize, child_id: usize },
    SetTextContent { node_id: usize, text: String },
}

#[derive(Debug, Clone)]
pub struct HtmlTokenizer {
    input: String,
    pos: usize,
    mode: RenderingMode,
    open_tags: std::collections::VecDeque<String>,
    quirks_mode: bool,
    parsing_mode: ParsingMode,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenType,
    pub value: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum TokenType {
    StartTag,
    EndTag,
    SelfClosingTag,
    Text,
    Comment,
    Doctype,
}

impl HtmlTokenizer {
    pub fn new(input: String, mode: RenderingMode) -> Self {
        HtmlTokenizer {
            input,
            pos: 0,
            mode,
            open_tags: std::collections::VecDeque::new(),
            quirks_mode: true,
            parsing_mode: ParsingMode::Quirks1996,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        
        while self.pos < self.input.len() {
            if self.consume_whitespace() {
                continue;
            }
            
            if self.pos < self.input.len() && self.input.chars().nth(self.pos) == Some('<') {
                if let Some(token) = self.parse_tag() {
                    tokens.push(token);
                }
            } else {
                if let Some(token) = self.parse_text() {
                    tokens.push(token);
                }
            }
        }
        
        tokens
    }

    fn consume_whitespace(&mut self) -> bool {
        let mut consumed = false;
        while self.pos < self.input.len() {
            let ch = self.input.chars().nth(self.pos);
            if let Some(c) = ch {
                if c.is_whitespace() {
                    self.pos += 1;
                    consumed = true;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        consumed
    }

    fn parse_tag(&mut self) -> Option<Token> {
        if self.pos >= self.input.len() || self.input.chars().nth(self.pos) != Some('<') {
            return None;
        }
        
        self.pos += 1;
        
        if self.pos < self.input.len() && self.input.chars().nth(self.pos) == Some('!') {
            return self.parse_comment_or_doctype();
        }
        
        let is_end_tag = self.pos < self.input.len() && self.input.chars().nth(self.pos) == Some('/');
        if is_end_tag {
            self.pos += 1;
        }
        
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.chars().nth(self.pos);
            if ch == Some('>') {
                break;
            }
            self.pos += 1;
        }
        
        if self.pos >= self.input.len() {
            return None;
        }
        
        let tag_content = self.input.get(start..self.pos).unwrap_or("");
        let mut parts = tag_content.split_whitespace();
        let tag_name = parts.next().unwrap_or("").to_lowercase();
        
        let mut attributes = HashMap::new();
        for part in parts {
            if let Some(eq_pos) = part.find('=') {
                let key = part[..eq_pos].to_lowercase();
                let value = part[eq_pos+1..].trim_matches(|c| c == '"' || c == '\'');
                attributes.insert(key, value.to_string());
            } else {
                attributes.insert(part.to_lowercase(), String::new());
            }
        }
        
        self.pos += 1;
        
        if is_end_tag {
            if let Some(last_tag) = self.open_tags.back() {
                if last_tag == &tag_name {
                    self.open_tags.pop_back();
                }
            }
            Some(Token {
                kind: TokenType::EndTag,
                value: tag_name,
                attributes: HashMap::new(),
            })
        } else {
            if tag_content.ends_with('/') {
                Some(Token {
                    kind: TokenType::SelfClosingTag,
                    value: tag_name,
                    attributes,
                })
            } else {
                if self.quirks_mode {
                    self.handle_quirks_mode_closing(&tag_name);
                }
                
                self.open_tags.push_back(tag_name.clone());
                Some(Token {
                    kind: TokenType::StartTag,
                    value: tag_name,
                    attributes,
                })
            }
        }
    }

    fn handle_quirks_mode_closing(&mut self, current_tag: &str) {
        if current_tag == "p" && self.open_tags.back() == Some(&"p".to_string()) {
            self.open_tags.pop_back();
        }
        
        if current_tag == "li" && self.open_tags.contains(&"li".to_string()) {
            self.open_tags.retain(|tag| tag != "li");
        }
        
        if (current_tag == "td" || current_tag == "th") && 
           (self.open_tags.contains(&"td".to_string()) || self.open_tags.contains(&"th".to_string())) {
            self.open_tags.retain(|tag| tag != "td" && tag != "th");
        }
        
        if current_tag == "tr" && self.open_tags.contains(&"tr".to_string()) {
            self.open_tags.retain(|tag| tag != "tr");
        }
    }

    fn parse_comment_or_doctype(&mut self) -> Option<Token> {
        if self.pos >= self.input.len() {
            return None;
        }
        
        let remaining = self.input.get(self.pos..).unwrap_or("");
        if remaining.starts_with("!--") {
            self.pos += 3;
            let start = self.pos;
            while self.pos + 2 < self.input.len() {
                let slice = self.input.get(self.pos..).unwrap_or("");
                if slice.starts_with("-->") {
                    break;
                }
                self.pos += 1;
            }
            let comment = self.input.get(start..self.pos).unwrap_or("").to_string();
            self.pos += 3;
            Some(Token {
                kind: TokenType::Comment,
                value: comment,
                attributes: HashMap::new(),
            })
        } else if remaining.starts_with("!DOCTYPE") || remaining.starts_with("!doctype") {
            self.pos += 9;
            let start = self.pos;
            while self.pos < self.input.len() && self.input.chars().nth(self.pos) != Some('>') {
                self.pos += 1;
            }
            self.pos += 1;
            let doctype = self.input.get(start..self.pos.saturating_sub(1)).unwrap_or("").to_string();
            Some(Token {
                kind: TokenType::Doctype,
                value: doctype,
                attributes: HashMap::new(),
            })
        } else {
            None
        }
    }

    fn parse_text(&mut self) -> Option<Token> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.chars().nth(self.pos);
            if ch == Some('<') {
                break;
            }
            self.pos += 1;
        }
        
        if start == self.pos {
            return None;
        }
        
        let text = self.input.get(start..self.pos).unwrap_or("").to_string();
        Some(Token {
            kind: TokenType::Text,
            value: text,
            attributes: HashMap::new(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct DomBuilder {
    mode: RenderingMode,
    next_node_id: usize,
}

impl DomBuilder {
    pub fn new(mode: RenderingMode) -> Self {
        DomBuilder { mode, next_node_id: 0 }
    }

    pub fn build(&mut self, tokens: &[Token]) -> DomNode {
        let mut stack = vec![DomNode::Element {
            tag: "root".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            id: None,
            class: None,
            node_id: self.next_node_id(),
        }];
        
        for token in tokens {
            match &token.kind {
                TokenType::StartTag | TokenType::SelfClosingTag => {
                    let element = DomNode::Element {
                        tag: token.value.clone(),
                        attributes: token.attributes.clone(),
                        children: Vec::new(),
                        id: token.attributes.get("id").cloned(),
                        class: token.attributes.get("class").cloned(),
                        node_id: self.next_node_id(),
                    };
                    
                    if matches!(token.kind, TokenType::SelfClosingTag) {
                        if let Some(parent) = stack.last_mut() {
                            if let DomNode::Element { children, .. } = parent {
                                children.push(element);
                            }
                        }
                    } else {
                        stack.push(element);
                    }
                },
                TokenType::EndTag => {
                    if stack.len() > 1 {
                        let element = stack.pop().unwrap();
                        if let Some(parent) = stack.last_mut() {
                            if let DomNode::Element { children, .. } = parent {
                                children.push(element);
                            }
                        }
                    }
                },
                TokenType::Text => {
                    let text_node = DomNode::Text(token.value.clone());
                    if let Some(parent) = stack.last_mut() {
                        if let DomNode::Element { children, .. } = parent {
                            children.push(text_node);
                        }
                    }
                },
                TokenType::Comment => {
                    let comment_node = DomNode::Comment(token.value.clone());
                    if let Some(parent) = stack.last_mut() {
                        if let DomNode::Element { children, .. } = parent {
                            children.push(comment_node);
                        }
                    }
                },
                TokenType::Doctype => {},
            }
        }
        
        stack.pop().unwrap_or_else(|| DomNode::Element {
            tag: "empty".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            id: None,
            class: None,
            node_id: 0,
        })
    }

    fn next_node_id(&mut self) -> usize {
        let id = self.next_node_id;
        self.next_node_id += 1;
        id
    }
}

pub struct CssToken {
    pub kind: CssTokenKind,
    pub value: String,
}

#[derive(Debug, Clone)]
pub enum CssTokenKind {
    Ident,
    Number,
    Percentage,
    Color,
    String,
    Delim,
    Whitespace,
    CurlyBracketOpen,
    CurlyBracketClose,
    Semicolon,
    Colon,
    Comma,
}

pub struct CssTokenizer {
    input: String,
    pos: usize,
}

impl CssTokenizer {
    pub fn new(input: String) -> Self {
        CssTokenizer { input, pos: 0 }
    }

    pub fn tokenize(&mut self) -> Vec<CssToken> {
        let mut tokens = Vec::new();
        
        while self.pos < self.input.len() {
            let ch = self.input.chars().nth(self.pos);
            
            if let Some(c) = ch {
                if c.is_whitespace() {
                    self.pos += 1;
                    continue;
                }
                
                match c {
                    '{' => {
                        tokens.push(CssToken { kind: CssTokenKind::CurlyBracketOpen, value: "{".to_string() });
                        self.pos += 1;
                        continue;
                    },
                    '}' => {
                        tokens.push(CssToken { kind: CssTokenKind::CurlyBracketClose, value: "}".to_string() });
                        self.pos += 1;
                        continue;
                    },
                    ';' => {
                        tokens.push(CssToken { kind: CssTokenKind::Semicolon, value: ";".to_string() });
                        self.pos += 1;
                        continue;
                    },
                    ':' => {
                        tokens.push(CssToken { kind: CssTokenKind::Colon, value: ":".to_string() });
                        self.pos += 1;
                        continue;
                    },
                    ',' => {
                        tokens.push(CssToken { kind: CssTokenKind::Comma, value: ",".to_string() });
                        self.pos += 1;
                        continue;
                    },
                    '#' => {
                        self.pos += 1;
                        let start = self.pos;
                        while self.pos < self.input.len() {
                            let ch = self.input.chars().nth(self.pos);
                            if let Some(c) = ch {
                                if !c.is_alphanumeric() {
                                    break;
                                }
                            }
                            self.pos += 1;
                        }
                        let color = self.input.get(start..self.pos).unwrap_or("").to_string();
                        tokens.push(CssToken { kind: CssTokenKind::Color, value: color });
                        continue;
                    },
                    '"' | '\'' => {
                        let quote = c;
                        self.pos += 1;
                        let start = self.pos;
                        while self.pos < self.input.len() {
                            let ch = self.input.chars().nth(self.pos);
                            if ch == Some(quote) {
                                break;
                            }
                            self.pos += 1;
                        }
                        let string = self.input.get(start..self.pos).unwrap_or("").to_string();
                        self.pos += 1;
                        tokens.push(CssToken { kind: CssTokenKind::String, value: string });
                        continue;
                    },
                    _ if c.is_alphabetic() => {
                        let start = self.pos;
                        while self.pos < self.input.len() {
                            let ch = self.input.chars().nth(self.pos);
                            if let Some(c) = ch {
                                if !c.is_alphanumeric() && c != '-' {
                                    break;
                                }
                            }
                            self.pos += 1;
                        }
                        let ident = self.input.get(start..self.pos).unwrap_or("").to_string();
                        tokens.push(CssToken { kind: CssTokenKind::Ident, value: ident });
                        continue;
                    },
                    _ if c.is_numeric() => {
                        let start = self.pos;
                        while self.pos < self.input.len() {
                            let ch = self.input.chars().nth(self.pos);
                            if let Some(c) = ch {
                                if !c.is_numeric() && c != '.' && c != '%' {
                                    break;
                                }
                            }
                            self.pos += 1;
                        }
                        let number = self.input.get(start..self.pos).unwrap_or("").to_string();
                        if number.ends_with('%') {
                            tokens.push(CssToken { kind: CssTokenKind::Percentage, value: number });
                        } else {
                            tokens.push(CssToken { kind: CssTokenKind::Number, value: number });
                        }
                        continue;
                    },
                    _ => {
                        tokens.push(CssToken { kind: CssTokenKind::Delim, value: c.to_string() });
                        self.pos += 1;
                        continue;
                    }
                }
            }
        }
        
        tokens
    }
}

pub struct CssParser {
    input: String,
    pos: usize,
}

impl CssParser {
    pub fn new(input: String) -> Self {
        CssParser { input, pos: 0 }
    }

    pub fn parse_rules(&mut self) -> Vec<CssRule> {
        let mut tokenizer = CssTokenizer::new(self.input.clone());
        let tokens = tokenizer.tokenize();
        let mut rules = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            if let CssToken { kind: CssTokenKind::Ident, value: selector } = &tokens[i] {
                i += 1;
                if i < tokens.len() && matches!(tokens[i].kind, CssTokenKind::CurlyBracketOpen) {
                    i += 1;
                    let mut properties = HashMap::new();
                    
                    while i < tokens.len() && !matches!(tokens[i].kind, CssTokenKind::CurlyBracketClose) {
                        if let CssToken { kind: CssTokenKind::Ident, value: property } = &tokens[i] {
                            i += 1;
                            if i < tokens.len() && matches!(tokens[i].kind, CssTokenKind::Colon) {
                                i += 1;
                                if i < tokens.len() {
                                    let value = Self::parse_value(&tokens, &mut i);
                                    properties.insert(property.clone(), value);
                                }
                            }
                        }
                        i += 1;
                    }
                    
                    rules.push(CssRule {
                        selector: selector.clone(),
                        properties,
                    });
                }
            }
            i += 1;
        }
        
        rules
    }

    fn parse_value(tokens: &[CssToken], pos: &mut usize) -> CssValue {
        if *pos >= tokens.len() {
            return CssValue::Auto;
        }
        
        let token = &tokens[*pos];
        
        match token.kind {
            CssTokenKind::Number => {
                token.value.parse::<f32>().map(CssValue::Length).unwrap_or(CssValue::Auto)
            },
            CssTokenKind::Percentage => {
                token.value.strip_suffix('%')
                    .and_then(|s| s.parse::<f32>().ok())
                    .map(CssValue::Percentage)
                    .unwrap_or(CssValue::Auto)
            },
            CssTokenKind::Color => {
                WebSafePalette::parse_color(&token.value)
                    .map(|(r, g, b)| CssValue::Color(r, g, b))
                    .unwrap_or(CssValue::Auto)
            },
            CssTokenKind::Ident => CssValue::Keyword(token.value.clone()),
            CssTokenKind::String => CssValue::Keyword(token.value.clone()),
            _ => CssValue::Auto,
        }
    }
}

pub struct LayoutEngine {
    config: EngineConfig,
    quirk_profile: QuirkProfile,
    style_cache: HashMap<String, HashMap<String, CssValue>>,
    style_version: u64,
    layout_cache: HashMap<String, RenderNode>,
    layout_version: u64,
}

impl LayoutEngine {
    pub fn new(config: EngineConfig) -> Self {
        let quirk_profile = match config.rendering_mode {
            RenderingMode::Netscape3 => QuirkProfile::netscape_3(),
            RenderingMode::IE3 => QuirkProfile::ie_3(),
        };

        LayoutEngine {
            config,
            quirk_profile,
            style_cache: HashMap::new(),
            style_version: 0,
            layout_cache: HashMap::new(),
            layout_version: 0,
        }
    }

    pub fn layout(&mut self, node: &DomNode, available_width: f32) -> RenderNode {
        self.layout_node(node, available_width, 0.0, 0.0, &StyleContext::default(), FloatContext::new())
    }

    fn layout_node(
        &mut self,
        node: &DomNode,
        width: f32,
        x: f32,
        y: f32,
        parent_style: &StyleContext,
        float_context: FloatContext,
    ) -> RenderNode {
        let mut style = parent_style.clone();
        
        if let DomNode::Element { ref tag, ref attributes, .. } = node {
            if tag == "font" {
                if let Some(face) = attributes.get("face") {
                    style.font_face = face.clone();
                }
                if let Some(size_str) = attributes.get("size") {
                    style.font_size = Self::get_legacy_font_size(size_str, style.font_size);
                }
                if let Some(color_str) = attributes.get("color") {
                    if let Some(color) = WebSafePalette::parse_color(color_str) {
                        style.color = color;
                    }
                }
            }
        }
        
        let (box_model, special, computed_styles) = self.compute_styles(node, width, x, y, &style);
        
        let text_content = match node {
            DomNode::Text(text) => text.clone(),
            _ => String::new(),
        };
        
        let mut children = Vec::new();
        let mut current_y = box_model.y + box_model.padding_top + box_model.border_top;
        
        if let DomNode::Element { ref tag, children: ref dom_children, .. } = node {
            if tag == "table" {
                let table_box = self.layout_table(node, width, x, y, &style);
                return RenderNode {
                    dom_node: node.clone(),
                    box_model,
                    special,
                    children: vec![],
                    text_content,
                    computed_styles,
                    table_data: Some(table_box),
                    z_index: 0,
                    absolute_x: x,
                    absolute_y: y,
                    dirty: false,
                    style_version: self.style_version,
                    layout_version: self.layout_version,
                };
            } else if tag == "frameset" {
                return RenderNode {
                    dom_node: node.clone(),
                    box_model,
                    special,
                    children: vec![],
                    text_content,
                    computed_styles,
                    table_data: None,
                    z_index: 0,
                    absolute_x: x,
                    absolute_y: y,
                    dirty: false,
                    style_version: self.style_version,
                    layout_version: self.layout_version,
                };
            } else {
                for child in dom_children {
                    let child_width = width - box_model.padding_left - box_model.padding_right - box_model.border_left - box_model.border_right;
                    let child_render_node = self.layout_node(
                        child,
                        child_width,
                        x + box_model.padding_left + box_model.border_left,
                        current_y,
                        &style,
                        float_context.clone(),
                    );
                    
                    current_y += child_render_node.box_model.height + 10.0;
                    children.push(child_render_node);
                }
            }
        }

        RenderNode {
            dom_node: node.clone(),
            box_model,
            special,
            children,
            text_content,
            computed_styles,
            table_data: None,
            z_index: 0,
            absolute_x: x,
            absolute_y: y,
            dirty: false,
            style_version: self.style_version,
            layout_version: self.layout_version,
        }
    }

    fn layout_table(
        &mut self,
        node: &DomNode,
        available_width: f32,
        _x: f32,
        _y: f32,
        parent_style: &StyleContext,
    ) -> TableBox {
        if let DomNode::Element { ref children, ref attributes, .. } = node {
            let border = attributes.get("border").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
            let cellpadding = attributes.get("cellpadding").and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);
            let cellspacing = attributes.get("cellspacing").and_then(|s| s.parse::<f32>().ok()).unwrap_or(2.0);
            
            let mut rows = Vec::new();
            let mut max_cols = 0;
            
            for child in children {
                if let DomNode::Element { ref tag, children: ref row_children, .. } = child {
                    if tag == "tr" {
                        let mut row_cells = Vec::new();
                        
                        for cell in row_children {
                            if let DomNode::Element { tag: ref cell_tag, attributes: ref cell_attrs, children: ref cell_children, .. } = cell {
                                if cell_tag == "td" || cell_tag == "th" {
                                    let colspan = cell_attrs.get("colspan").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                                    let rowspan = cell_attrs.get("rowspan").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                                    let nowrap = cell_attrs.contains_key("nowrap");
                                    
                                    let mut cell_content = RenderNode {
                                        dom_node: cell.clone(),
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
                                    
                                    for cell_child in cell_children {
                                        let cell_child_node = self.layout_node(cell_child, 100.0, 0.0, 0.0, parent_style, FloatContext::new());
                                        cell_content.children.push(cell_child_node);
                                    }
                                    
                                    row_cells.push(Some(TableCellData {
                                        content: cell_content,
                                        colspan,
                                        rowspan,
                                        width: 0.0,
                                        height: 0.0,
                                        nowrap,
                                    }));
                                    
                                    max_cols = max_cols.max(row_cells.len());
                                }
                            }
                        }
                        
                        rows.push(row_cells);
                    }
                }
            }
            
            let column_widths = vec![available_width / max_cols as f32; max_cols];
            let row_heights = vec![20.0; rows.len()];
            
            TableBox {
                rows,
                column_widths,
                row_heights,
                border,
                cellpadding,
                cellspacing,
            }
        } else {
            TableBox {
                rows: vec![],
                column_widths: vec![],
                row_heights: vec![],
                border: 0.0,
                cellpadding: 1.0,
                cellspacing: 2.0,
            }
        }
    }

    fn compute_styles(
        &self,
        node: &DomNode,
        width: f32,
        x: f32,
        y: f32,
        style_context: &StyleContext,
    ) -> (BoxModel, SpecialElement, HashMap<String, CssValue>) {
        let mut computed_styles = HashMap::new();
        let mut box_model = BoxModel {
            x,
            y,
            width,
            height: 0.0,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            border_top: 0.0,
            border_right: 0.0,
            border_bottom: 0.0,
            border_left: 0.0,
            background_color: style_context.background_color,
            color: style_context.color,
        };

        let special = match node {
            DomNode::Element { ref tag, ref attributes, .. } => {
                match tag.as_str() {
                    "img" => {
                        let src = attributes.get("src").cloned().unwrap_or_default();
                        let alt = attributes.get("alt").cloned().unwrap_or_default();
                        let width_attr = attributes.get("width").and_then(|w| w.parse::<u32>().ok());
                        let height_attr = attributes.get("height").and_then(|h| h.parse::<u32>().ok());
                        let usemap = attributes.get("usemap").cloned();
                        let align = attributes.get("align").cloned();
                        
                        SpecialElement::Image {
                            src,
                            alt,
                            width: width_attr,
                            height: height_attr,
                            natural_width: 0,
                            natural_height: 0,
                            loaded: false,
                            animated_gif: None,
                            visible: true,
                            usemap,
                            align,
                        }
                    },
                    "form" => {
                        let action = attributes.get("action").cloned().unwrap_or_default();
                        let method = attributes.get("method").cloned().unwrap_or_else(|| "get".to_string());
                        let name = attributes.get("name").cloned().unwrap_or_default();
                        
                        SpecialElement::Form {
                            action,
                            method,
                            name,
                        }
                    },
                    "input" => {
                        let input_type = attributes.get("type").cloned().unwrap_or_else(|| "text".to_string());
                        let name = attributes.get("name").cloned().unwrap_or_default();
                        let value = attributes.get("value").cloned().unwrap_or_default();
                        let checked = attributes.get("checked").is_some();
                        let disabled = attributes.get("disabled").is_some();
                        
                        SpecialElement::Input {
                            input_type,
                            name,
                            value,
                            checked,
                            disabled,
                        }
                    },
                    "a" => {
                        let href = attributes.get("href").cloned().unwrap_or_default();
                        let target = attributes.get("target").cloned().unwrap_or_else(|| "_self".to_string());
                        
                        SpecialElement::Link {
                            href,
                            target,
                        }
                    },
                    "script" => {
                        let src = attributes.get("src").cloned();
                        let language = attributes.get("language").cloned().unwrap_or_else(|| "javascript".to_string());
                        
                        SpecialElement::Script {
                            src,
                            content: String::new(),
                            language,
                        }
                    },
                    "frame" => {
                        let src = attributes.get("src").cloned().unwrap_or_default();
                        let name = attributes.get("name").cloned().unwrap_or_default();
                        let width = attributes.get("width").cloned().unwrap_or_default();
                        let height = attributes.get("height").cloned().unwrap_or_default();
                        
                        SpecialElement::Frame {
                            src,
                            name,
                            width,
                            height,
                        }
                    },
                    "frameset" => {
                        let rows = attributes.get("rows").cloned().unwrap_or_default();
                        let cols = attributes.get("cols").cloned().unwrap_or_default();
                        let border = attributes.get("border").cloned().unwrap_or_default();
                        let frameborder = attributes.get("frameborder").cloned().unwrap_or_default();
                        let framespacing = attributes.get("framespacing").cloned().unwrap_or_default();
                        
                        SpecialElement::Frameset {
                            rows,
                            cols,
                            border,
                            frameborder,
                            framespacing,
                        }
                    },
                    "marquee" => {
                        let direction = attributes.get("direction").cloned().unwrap_or_else(|| "left".to_string());
                        let speed = attributes.get("scrollamount").and_then(|s| s.parse::<u32>().ok()).unwrap_or(6);
                        let behavior = attributes.get("behavior").cloned().unwrap_or_else(|| "scroll".to_string());
                        
                        SpecialElement::Marquee {
                            direction,
                            speed,
                            behavior,
                            position: 0.0,
                            content_width: 0.0,
                        }
                    },
                    "blink" => {
                        SpecialElement::Blink {
                            start_time: Instant::now(),
                            visible: true,
                        }
                    },
                    "font" => {
                        let face = attributes.get("face").cloned();
                        let size = attributes.get("size").and_then(|s| s.parse::<i32>().ok());
                        let color = attributes.get("color").and_then(|c| WebSafePalette::parse_color(c));
                        
                        SpecialElement::Font {
                            face,
                            size,
                            color,
                        }
                    },
                    "body" => {
                        let bgcolor = attributes.get("bgcolor").and_then(|c| WebSafePalette::parse_color(c));
                        let background = attributes.get("background").cloned();
                        
                        SpecialElement::Body {
                            bgcolor,
                            background,
                        }
                    },
                    "table" => {
                        let border = attributes.get("border").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                        let cellpadding = attributes.get("cellpadding").and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                        let cellspacing = attributes.get("cellspacing").and_then(|s| s.parse::<u32>().ok()).unwrap_or(2);
                        
                        SpecialElement::Table {
                            border,
                            cellpadding,
                            cellspacing,
                        }
                    },
                    _ => SpecialElement::Unknown,
                }
            },
            DomNode::Text(_) => SpecialElement::Unknown,
            DomNode::Comment(_) => SpecialElement::Unknown,
        };

        match node {
            DomNode::Element { ref attributes, .. } => {
                if let Some(bgcolor) = attributes.get("bgcolor") {
                    if let Some(color) = WebSafePalette::parse_color(bgcolor) {
                        box_model.background_color = Some(color);
                    }
                }
                
                if let Some(color) = attributes.get("color") {
                    if let Some(rgb) = WebSafePalette::parse_color(color) {
                        box_model.color = rgb;
                    }
                }
                
                if let Some(width_attr) = attributes.get("width") {
                    if width_attr.ends_with('%') {
                        if let Ok(pct) = width_attr.trim_end_matches('%').parse::<f32>() {
                            box_model.width = width * pct / 100.0;
                        }
                    } else if let Ok(px) = width_attr.parse::<f32>() {
                        box_model.width = px;
                    }
                }
                
                if let Some(border_attr) = attributes.get("border") {
                    if let Ok(border_width) = border_attr.parse::<f32>() {
                        box_model.border_top = border_width;
                        box_model.border_right = border_width;
                        box_model.border_bottom = border_width;
                        box_model.border_left = border_width;
                    }
                }
                
                if let Some(align) = attributes.get("align") {
                    computed_styles.insert("text-align".to_string(), CssValue::Keyword(align.clone()));
                }
            },
            _ => {},
        }

        (box_model, special, computed_styles)
    }

    fn get_legacy_font_size(size_attr: &str, _parent_size: f32) -> f32 {
        let base_size = 16.0;
        
        let step = if size_attr.starts_with('+') || size_attr.starts_with('-') {
            let current_step = 3;
            current_step + size_attr.parse::<i32>().unwrap_or(0)
        } else {
            size_attr.parse::<i32>().unwrap_or(3)
        };

        match step {
            1 => base_size * 0.60,
            2 => base_size * 0.80,
            3 => base_size * 1.00,
            4 => base_size * 1.15,
            5 => base_size * 1.50,
            6 => base_size * 2.00,
            7 => base_size * 3.00,
            _ => base_size * 1.00,
        }
    }
}

lazy_static! {
    static ref EMPTY_ATTRS: HashMap<String, String> = HashMap::new();
    static ref EMPTY_CHILDREN: Vec<DomNode> = Vec::new();
}

pub struct TrussCore {
    pub dom: Arc<Mutex<Option<DomNode>>>,
    pub render_tree: Arc<Mutex<Option<RenderNode>>>,
    pub css_rules: Arc<Mutex<Vec<CssRule>>>,
    pub config: EngineConfig,
    pub quirk_profile: QuirkProfile,
    pub image_cache: Arc<Mutex<HashMap<String, RgbaImage>>>,
    pub mutation_buffer: Arc<Mutex<Vec<DomMutation>>>,
    pub display_context: Arc<Mutex<DisplayContext>>,
    pub document_base_url: Arc<Mutex<Option<String>>>,
    pub progressive_render_callback: Option<Box<dyn Fn() + Send + Sync>>,
    pub user_agent: String,
    pub js_engine: Arc<Mutex<ChronoScript>>,
}

impl TrussCore {
    pub fn new() -> Self {
        let config = EngineConfig::default();
        let quirk_profile = match config.rendering_mode {
            RenderingMode::Netscape3 => QuirkProfile::netscape_3(),
            RenderingMode::IE3 => QuirkProfile::ie_3(),
        };

        TrussCore {
            dom: Arc::new(Mutex::new(None)),
            render_tree: Arc::new(Mutex::new(None)),
            css_rules: Arc::new(Mutex::new(Vec::new())),
            config,
            quirk_profile,
            image_cache: Arc::new(Mutex::new(HashMap::new())),
            mutation_buffer: Arc::new(Mutex::new(Vec::new())),
            display_context: Arc::new(Mutex::new(DisplayContext::default())),
            document_base_url: Arc::new(Mutex::new(None)),
            progressive_render_callback: None,
            user_agent: "Mozilla/3.0 (Windows 95; TrussCore/1.0; Rust; en)".to_string(),
            js_engine: Arc::new(Mutex::new(ChronoScript::new())),
        }
    }

    pub fn load_html(&self, html: &str) {
        let tokens = {
            let mut tokenizer = HtmlTokenizer::new(html.to_string(), self.config.rendering_mode);
            tokenizer.tokenize()
        };
        
        let dom = {
            let mut builder = DomBuilder::new(self.config.rendering_mode);
            builder.build(&tokens)
        };
        
        {
            let mut dom_guard = self.dom.lock().unwrap();
            *dom_guard = Some(dom);
        }
        
        self.process_scripts();
    }

    pub fn load_html_from_url(&self, url: &str, html: &str) {
        {
            let mut base_url_guard = self.document_base_url.lock().unwrap();
            *base_url_guard = Some(url.to_string());
        }
        
        self.load_html(html);
    }

    pub fn get_dom_tree(&self) -> DomNode {
        let dom_guard = self.dom.lock().unwrap();
        dom_guard.clone().unwrap_or_else(|| DomNode::Element {
            tag: "empty".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            id: None,
            class: None,
            node_id: 0,
        })
    }

    pub fn render(&mut self, width: f32) -> DisplayList {
        self.rebuild_render_tree(width);
        self.render_to_display_list()
    }

    fn rebuild_render_tree(&self, width: f32) {
        let dom_guard = self.dom.lock().unwrap();
        
        if let Some(ref dom) = *dom_guard {
            let mut layout_engine = LayoutEngine::new(self.config.clone());
            let render_tree = layout_engine.layout(dom, width);
            
            drop(dom_guard);
            
            let mut render_guard = self.render_tree.lock().unwrap();
            *render_guard = Some(render_tree);
        }
    }

    fn render_to_display_list(&self) -> DisplayList {
        let render_guard = self.render_tree.lock().unwrap();
        if let Some(ref node) = *render_guard {
            Self::render_node_to_display_list_recursive(node)
        } else {
            DisplayList::new()
        }
    }

    fn render_node_to_display_list_recursive(node: &RenderNode) -> DisplayList {
        let mut commands = Vec::new();
        
        if let Some(bg_color) = node.box_model.background_color {
            commands.push(RenderCommand::DrawRect {
                x: node.absolute_x,
                y: node.absolute_y,
                w: node.box_model.width,
                h: node.box_model.height,
                color: bg_color,
            });
        }
        
        if let Some(ref table_data) = node.table_data {
            for (row_idx, row) in table_data.rows.iter().enumerate() {
                for (col_idx, cell_opt) in row.iter().enumerate() {
                    if let Some(_cell) = cell_opt {
                        let x = node.absolute_x + table_data.column_widths.get(0..col_idx).map(|w| w.iter().sum::<f32>()).unwrap_or(0.0);
                        let y = node.absolute_y + table_data.row_heights.get(0..row_idx).map(|h| h.iter().sum::<f32>()).unwrap_or(0.0);
                        let w = *table_data.column_widths.get(col_idx).unwrap_or(&0.0);
                        let h = *table_data.row_heights.get(row_idx).unwrap_or(&0.0);
                        
                        commands.push(RenderCommand::DrawTableBorder {
                            x,
                            y,
                            w,
                            h,
                            border_width: table_data.border,
                            border_color: (128, 128, 128),
                        });
                    }
                }
            }
        }
        
        if !node.text_content.is_empty() {
            commands.push(RenderCommand::DrawText {
                x: node.absolute_x,
                y: node.absolute_y,
                text: node.text_content.clone(),
                font: "Times New Roman".to_string(),
                size: 12.0,
                color: node.box_model.color,
            });
        }
        
        for child in &node.children {
            let child_commands = Self::render_node_to_display_list_recursive(child);
            commands.extend(child_commands.commands);
        }
        
        DisplayList { commands }
    }

    pub fn set_image_data(&self, src: &str, data: Vec<u8>) -> Result<(), EngineError> {
        let rgba = image::load_from_memory(&data)
            .map_err(|e| EngineError::ImageDecodeError(format!("Failed to decode image: {}", e)))?
            .to_rgba8();

        {
            let mut cache_guard = self.image_cache.lock().unwrap();
            cache_guard.insert(src.to_string(), rgba);
        }

        Ok(())
    }

    pub fn update_blink_state(&self) {
        let mut render_guard = self.render_tree.lock().unwrap();
        if let Some(ref mut render_tree) = *render_guard {
            Self::update_blink_recursive(render_tree, Instant::now());
        }
    }

    fn update_blink_recursive(node: &mut RenderNode, now: Instant) {
        if let SpecialElement::Blink { start_time, visible } = &mut node.special {
            let elapsed = now.duration_since(*start_time).as_millis() / 500;
            *visible = elapsed % 2 == 0;
        }
        
        for child in &mut node.children {
            Self::update_blink_recursive(child, now);
        }
    }

    pub fn update_marquee_positions(&self, delta_time: f32) {
        let mut render_guard = self.render_tree.lock().unwrap();
        if let Some(ref mut render_tree) = *render_guard {
            Self::update_marquee_recursive(render_tree, delta_time);
        }
    }

    fn update_marquee_recursive(node: &mut RenderNode, delta_time: f32) {
        if let SpecialElement::Marquee { speed, position, content_width, direction, .. } = &mut node.special {
            let movement = (*speed as f32) * delta_time;
            match direction.as_str() {
                "right" => *position += movement,
                "up" => *position -= movement,
                "down" => *position += movement,
                _ => *position -= movement,
            }
            
            if *position > *content_width {
                *position = -*content_width;
            } else if *position < -*content_width {
                *position = *content_width;
            }
        }
        
        for child in &mut node.children {
            Self::update_marquee_recursive(child, delta_time);
        }
    }

    pub fn has_blink_elements(&self) -> bool {
        let render_guard = self.render_tree.lock().unwrap();
        if let Some(ref render_tree) = *render_guard {
            Self::has_blink_recursive(render_tree)
        } else {
            false
        }
    }

    fn has_blink_recursive(node: &RenderNode) -> bool {
        if matches!(node.special, SpecialElement::Blink { .. }) {
            return true;
        }
        
        for child in &node.children {
            if Self::has_blink_recursive(child) {
                return true;
            }
        }
        
        false
    }

    pub fn has_marquee_elements(&self) -> bool {
        let render_guard = self.render_tree.lock().unwrap();
        if let Some(ref render_tree) = *render_guard {
            Self::has_marquee_recursive(render_tree)
        } else {
            false
        }
    }

    fn has_marquee_recursive(node: &RenderNode) -> bool {
        if matches!(node.special, SpecialElement::Marquee { .. }) {
            return true;
        }
        
        for child in &node.children {
            if Self::has_marquee_recursive(child) {
                return true;
            }
        }
        
        false
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<DomNode> {
        let dom_guard = self.dom.lock().unwrap();
        if let Some(ref dom) = *dom_guard {
            dom.find_by_id(id).cloned()
        } else {
            None
        }
    }

    pub fn get_elements_by_tag(&self, tag: &str) -> Vec<DomNode> {
        let dom_guard = self.dom.lock().unwrap();
        if let Some(ref dom) = *dom_guard {
            dom.find_by_tag(tag).into_iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_forms(&self) -> Vec<DomNode> {
        self.get_elements_by_tag("form")
    }

    pub fn get_images(&self) -> Vec<DomNode> {
        self.get_elements_by_tag("img")
    }

    pub fn get_links(&self) -> Vec<DomNode> {
        self.get_elements_by_tag("a")
    }

    pub fn set_css_rules(&self, css_rules: Vec<CssRule>) {
        let mut css_guard = self.css_rules.lock().unwrap();
        *css_guard = css_rules;
    }

    pub fn process_mutations(&self) -> Result<(), String> {
        let mutations = {
            let mut buffer_guard = self.mutation_buffer.lock().unwrap();
            std::mem::take(&mut *buffer_guard)
        };

        for mutation in mutations {
            match mutation {
                DomMutation::SetAttribute { node_id, key, value } => {
                    self.update_dom_attribute(node_id, &key, &value);
                },
                DomMutation::SetStyle { node_id, property, value } => {
                    self.update_dom_style(node_id, &property, &value);
                },
                DomMutation::InsertChild { parent_id, child } => {
                    self.insert_dom_child(parent_id, child);
                },
                DomMutation::RemoveChild { parent_id, child_id } => {
                    self.remove_dom_child(parent_id, child_id);
                },
                DomMutation::SetTextContent { node_id, text } => {
                    self.set_dom_text_content(node_id, &text);
                },
            }
        }

        Ok(())
    }

    fn update_dom_attribute(&self, node_id: usize, key: &str, value: &str) {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            Self::update_dom_node_attribute(dom, node_id, key, value);
        }
    }

    fn update_dom_node_attribute(node: &mut DomNode, target_node_id: usize, key: &str, value: &str) {
        match node {
            DomNode::Element { node_id, attributes, .. } if *node_id == target_node_id => {
                attributes.insert(key.to_string(), value.to_string());
            },
            DomNode::Element { ref mut children, .. } => {
                for child in children {
                    Self::update_dom_node_attribute(child, target_node_id, key, value);
                }
            },
            _ => {}
        }
    }

    fn update_dom_style(&self, node_id: usize, property: &str, value: &str) {
        self.update_dom_attribute(node_id, "style", &format!("{}: {}", property, value));
    }

    fn insert_dom_child(&self, parent_id: usize, child: DomNode) {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            Self::insert_dom_node_child(dom, parent_id, child);
        }
    }

    fn insert_dom_node_child(node: &mut DomNode, target_node_id: usize, child: DomNode) {
        match node {
            DomNode::Element { node_id, ref mut children, .. } if *node_id == target_node_id => {
                children.push(child);
            },
            DomNode::Element { ref mut children, .. } => {
                for child_node in children {
                    Self::insert_dom_node_child(child_node, target_node_id, child.clone());
                }
            },
            _ => {}
        }
    }

    fn remove_dom_child(&self, parent_id: usize, child_id: usize) {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            Self::remove_dom_node_child(dom, parent_id, child_id);
        }
    }

    fn remove_dom_node_child(node: &mut DomNode, target_node_id: usize, child_id: usize) {
        match node {
            DomNode::Element { node_id, ref mut children, .. } if *node_id == target_node_id => {
                children.retain(|child| {
                    if let DomNode::Element { node_id: child_node_id, .. } = child {
                        *child_node_id != child_id
                    } else {
                        true
                    }
                });
            },
            DomNode::Element { ref mut children, .. } => {
                for child_node in children {
                    Self::remove_dom_node_child(child_node, target_node_id, child_id);
                }
            },
            _ => {}
        }
    }

    fn set_dom_text_content(&self, node_id: usize, text: &str) {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            Self::set_dom_node_text_content(dom, node_id, text);
        }
    }

    fn set_dom_node_text_content(node: &mut DomNode, target_node_id: usize, text: &str) {
        match node {
            DomNode::Element { node_id, ref mut children, .. } if *node_id == target_node_id => {
                *children = vec![DomNode::Text(text.to_string())];
            },
            DomNode::Element { ref mut children, .. } => {
                for child in children {
                    Self::set_dom_node_text_content(child, target_node_id, text);
                }
            },
            _ => {}
        }
    }

    pub fn document_write(&self, html_fragment: &str) -> Result<(), String> {
        let tokens = {
            let mut tokenizer = HtmlTokenizer::new(html_fragment.to_string(), self.config.rendering_mode);
            tokenizer.tokenize()
        };
        
        let new_nodes = {
            let mut builder = DomBuilder::new(self.config.rendering_mode);
            builder.build(&tokens)
        };

        {
            let mut buffer_guard = self.mutation_buffer.lock().unwrap();
            buffer_guard.push(DomMutation::InsertChild {
                parent_id: 0,
                child: new_nodes,
            });
        }

        self.process_mutations()?;

        Ok(())
    }

    fn process_scripts(&self) {
        if !self.config.enable_javascript {
            return;
        }
        
        let dom_guard = self.dom.lock().unwrap();
        if let Some(ref dom) = *dom_guard {
            self.extract_and_execute_scripts(dom);
        }
    }

    fn extract_and_execute_scripts(&self, node: &DomNode) {
        match node {
            DomNode::Element { tag, attributes, children, .. } if tag == "script" => {
                let content = match attributes.get("src") {
                    Some(_script_src) => String::new(),
                    None => {
                        let mut content = String::new();
                        for child in children {
                            if let DomNode::Text(text) = child {
                                content.push_str(text);
                            }
                        }
                        content
                    }
                };
                
                if !content.is_empty() {
                    if let Err(e) = self.execute_script(&content) {
                        eprintln!("Script execution error: {}", e);
                    }
                }
            },
            DomNode::Element { children, .. } => {
                for child in children {
                    self.extract_and_execute_scripts(child);
                }
            },
            _ => {}
        }
    }

    pub fn execute_script(&self, script: &str) -> Result<(), String> {
        let mut js_guard = self.js_engine.lock().unwrap();
        js_guard.execute(script.as_bytes()).map(|_| ()).map_err(|e| format!("{:?}", e))
    }

    pub fn get_display_context(&self) -> DisplayContext {
        self.display_context.lock().unwrap().clone()
    }

    pub fn set_display_context(&self, context: DisplayContext) {
        let mut ctx_guard = self.display_context.lock().unwrap();
        *ctx_guard = context;
    }

    pub fn get_base_url(&self) -> Option<String> {
        self.document_base_url.lock().unwrap().clone()
    }

    pub fn resolve_url(&self, url: &str) -> String {
        if let Ok(parsed) = Url::parse(url) {
            return parsed.to_string();
        }
        
        if let Some(base) = self.get_base_url() {
            if let Ok(base_url) = Url::parse(&base) {
                if let Ok(resolved) = base_url.join(url) {
                    return resolved.to_string();
                }
            }
        }
        
        url.to_string()
    }

    pub fn get_render_tree(&self) -> Option<RenderNode> {
        self.render_tree.lock().unwrap().clone()
    }
}

impl Default for TrussCore {
    fn default() -> Self {
        Self::new()
    }
}