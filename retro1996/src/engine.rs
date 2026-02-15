use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use image::RgbaImage;
use url::Url;
use lazy_static::lazy_static;

use crate::javascript_engine::ChronoScript;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderingMode {
    Netscape3,
    IE3,
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
            DomNode::Element { id: ref node_id, node_id: _, .. } if node_id.as_deref() == Some(id) => Some(self),
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
            DomNode::Element { tag: ref node_tag, ref children, node_id: _, .. } if node_tag == tag => {
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
            DomNode::Element { id: _, attributes: _, children, class: _, node_id: current_node_id } if *current_node_id == node_id => Some(self),
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
}

impl Default for StyleContext {
    fn default() -> Self {
        StyleContext {
            font_face: "Times New Roman".to_string(),
            font_size: 12.0,
            color: (0, 0, 0),
            background_color: None,
            text_decoration: String::new(),
            text_align: String::new(),
            vertical_align: String::new(),
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

    pub fn get_available_space(&self, y: f32, height: f32) -> (f32, f32) {
        let mut left_edge = 0.0;
        let mut right_edge = 1000.0;
        
        for zone in &self.exclusion_zones {
            if y >= zone.rect.y && y < zone.rect.y + zone.rect.height {
                match zone.alignment.as_str() {
                    "left" => left_edge = left_edge.max(zone.rect.x + zone.rect.width),
                    "right" => right_edge = right_edge.min(zone.rect.x),
                    _ => {}
                }
            }
        }
        
        let available_width = (right_edge - left_edge).max(0.0);
        (left_edge, available_width)
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

    pub fn clone_with_reset(&self) -> Self {
        FloatContext {
            left_floats: Vec::new(),
            right_floats: Vec::new(),
            exclusion_zones: Vec::new(),
            current_y: 0.0,
        }
    }
}

pub struct FloatContextPool {
    pool: Arc<Mutex<Vec<FloatContext>>>,
}

impl FloatContextPool {
    pub fn new() -> Self {
        FloatContextPool {
            pool: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get(&self) -> FloatContext {
        if let Ok(mut pool_guard) = self.pool.try_lock() {
            if let Some(context) = pool_guard.pop() {
                return context;
            }
        }
        FloatContext::new()
    }

    pub fn return_context(&self, mut context: FloatContext) {
        context.reset();
        if let Ok(mut pool_guard) = self.pool.try_lock() {
            if pool_guard.len() < 100 {
                pool_guard.push(context);
            }
        }
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
    Area {
        shape: String,
        coords: Vec<i32>,
        href: String,
        alt: String,
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
    Applet {
        code: String,
        archive: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
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
    Layer {
        id: String,
        z_index: i32,
    },
    ILayer {
        id: String,
        z_index: i32,
    },
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
    FillBackgroundPattern { image_handle: usize },
    DrawTableBorder { x: f32, y: f32, w: f32, h: f32, border_width: f32, border_color: (u8, u8, u8) },
}

#[derive(Debug, Clone)]
pub struct DisplayList {
    pub commands: Vec<RenderCommand>,
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

pub struct HtmlTokenizer {
    input: String,
    pos: usize,
    mode: RenderingMode,
    open_tags: std::collections::VecDeque<String>,
    quirks_mode: bool,
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
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        
        while self.pos < self.input.len() {
            if self.consume_whitespace() {
                continue;
            }
            
            if self.input[self.pos..].starts_with('<') {
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
            let ch = self.input.chars().nth(self.pos).unwrap();
            if ch.is_whitespace() {
                self.pos += 1;
                consumed = true;
            } else {
                break;
            }
        }
        consumed
    }

    fn parse_tag(&mut self) -> Option<Token> {
        if self.pos >= self.input.len() || self.input.chars().nth(self.pos).unwrap() != '<' {
            return None;
        }
        
        self.pos += 1;
        
        if self.input[self.pos..].starts_with('!') {
            return self.parse_comment_or_doctype();
        }
        
        let is_end_tag = self.input[self.pos..].starts_with('/');
        if is_end_tag {
            self.pos += 1;
        }
        
        let start = self.pos;
        while self.pos < self.input.len() && 
              self.input.chars().nth(self.pos).unwrap() != '>' {
            self.pos += 1;
        }
        
        let tag_content = &self.input[start..self.pos];
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
        
        if self.pos >= self.input.len() {
            return None;
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
        if self.input[self.pos..].starts_with("!--") {
            self.pos += 3;
            let start = self.pos;
            while self.pos + 2 < self.input.len() {
                if self.input[self.pos..].starts_with("-->") {
                    break;
                }
                self.pos += 1;
            }
            let comment = self.input[start..self.pos].to_string();
            self.pos += 3;
            Some(Token {
                kind: TokenType::Comment,
                value: comment,
                attributes: HashMap::new(),
            })
        } else if self.input[self.pos..].starts_with("!DOCTYPE") {
            self.pos += 9;
            let start = self.pos;
            while self.pos < self.input.len() && self.input.chars().nth(self.pos).unwrap() != '>' {
                self.pos += 1;
            }
            self.pos += 1;
            Some(Token {
                kind: TokenType::Doctype,
                value: self.input[start..self.pos-1].to_string(),
                attributes: HashMap::new(),
            })
        } else {
            None
        }
    }

    fn parse_text(&mut self) -> Option<Token> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.chars().nth(self.pos).unwrap();
            if ch == '<' {
                break;
            }
            self.pos += 1;
        }
        
        if start == self.pos {
            return None;
        }
        
        let text = self.input[start..self.pos].to_string();
        Some(Token {
            kind: TokenType::Text,
            value: text,
            attributes: HashMap::new(),
        })
    }
}

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
                TokenType::Doctype => {
                },
            }
        }
        
        stack.pop().unwrap_or(DomNode::Element {
            tag: "empty".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            id: None,
            class: None,
            node_id: self.next_node_id(),
        })
    }

    fn next_node_id(&mut self) -> usize {
        let id = self.next_node_id;
        self.next_node_id += 1;
        id
    }
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
}

pub struct CssParser {
    input: String,
    pos: usize,
}

#[derive(Debug, Clone)]
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

impl CssParser {
    pub fn new(input: String) -> Self {
        CssParser { input, pos: 0 }
    }

    pub fn parse_rules(&mut self) -> Vec<CssRule> {
        let mut rules = Vec::new();
        let tokens = self.tokenize();
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
                                    let value = self.parse_value(&tokens, &mut i);
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

    fn tokenize(&mut self) -> Vec<CssToken> {
        let mut tokens = Vec::new();
        
        while self.pos < self.input.len() {
            let ch = self.input.chars().nth(self.pos).unwrap();
            
            if ch.is_whitespace() {
                self.pos += 1;
                continue;
            }
            
            if ch == '{' {
                tokens.push(CssToken { kind: CssTokenKind::CurlyBracketOpen, value: "{".to_string() });
                self.pos += 1;
                continue;
            }
            
            if ch == '}' {
                tokens.push(CssToken { kind: CssTokenKind::CurlyBracketClose, value: "}".to_string() });
                self.pos += 1;
                continue;
            }
            
            if ch == ';' {
                tokens.push(CssToken { kind: CssTokenKind::Semicolon, value: ";".to_string() });
                self.pos += 1;
                continue;
            }
            
            if ch == ':' {
                tokens.push(CssToken { kind: CssTokenKind::Colon, value: ":".to_string() });
                self.pos += 1;
                continue;
            }
            
            if ch == ',' {
                tokens.push(CssToken { kind: CssTokenKind::Comma, value: ",".to_string() });
                self.pos += 1;
                continue;
            }
            
            if ch == '#' {
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.input.len() {
                    let ch = self.input.chars().nth(self.pos).unwrap();
                    if !ch.is_alphanumeric() {
                        break;
                    }
                    self.pos += 1;
                }
                let color = self.input[start..self.pos].to_string();
                tokens.push(CssToken { kind: CssTokenKind::Color, value: color });
                continue;
            }
            
            if ch == '"' || ch == '\'' {
                let quote = ch;
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.input.len() {
                    let ch = self.input.chars().nth(self.pos).unwrap();
                    if ch == quote {
                        break;
                    }
                    self.pos += 1;
                }
                let string = self.input[start..self.pos].to_string();
                self.pos += 1;
                tokens.push(CssToken { kind: CssTokenKind::String, value: string });
                continue;
            }
            
            if ch.is_alphabetic() {
                let start = self.pos;
                while self.pos < self.input.len() {
                    let ch = self.input.chars().nth(self.pos).unwrap();
                    if !ch.is_alphanumeric() && ch != '-' {
                        break;
                    }
                    self.pos += 1;
                }
                let ident = self.input[start..self.pos].to_string();
                tokens.push(CssToken { kind: CssTokenKind::Ident, value: ident });
                continue;
            }
            
            if ch.is_numeric() {
                let start = self.pos;
                while self.pos < self.input.len() {
                    let ch = self.input.chars().nth(self.pos).unwrap();
                    if !ch.is_numeric() && ch != '.' && ch != '%' {
                        break;
                    }
                    self.pos += 1;
                }
                let number = self.input[start..self.pos].to_string();
                if number.ends_with('%') {
                    tokens.push(CssToken { kind: CssTokenKind::Percentage, value: number });
                } else {
                    tokens.push(CssToken { kind: CssTokenKind::Number, value: number });
                }
                continue;
            }
            
            tokens.push(CssToken { kind: CssTokenKind::Delim, value: ch.to_string() });
            self.pos += 1;
        }
        
        tokens
    }

    fn parse_value(&mut self, tokens: &[CssToken], pos: &mut usize) -> CssValue {
        if *pos >= tokens.len() {
            return CssValue::Auto;
        }
        
        let token = &tokens[*pos];
        
        match token.kind {
            CssTokenKind::Number => {
                if let Ok(num) = token.value.parse::<f32>() {
                    CssValue::Length(num)
                } else {
                    CssValue::Auto
                }
            },
            CssTokenKind::Percentage => {
                if let Some(percentage) = token.value.strip_suffix('%') {
                    if let Ok(num) = percentage.parse::<f32>() {
                        CssValue::Percentage(num)
                    } else {
                        CssValue::Auto
                    }
                } else {
                    CssValue::Auto
                }
            },
            CssTokenKind::Color => {
                if let Some(color) = Self::parse_color(&token.value) {
                    CssValue::Color(color)
                } else {
                    CssValue::Auto
                }
            },
            CssTokenKind::Ident => {
                CssValue::Keyword(token.value.clone())
            },
            _ => CssValue::Auto,
        }
    }

    fn parse_color(color_str: &str) -> Option<(u8, u8, u8)> {
        let color = color_str.trim_start_matches('#');
        if color.len() == 6 {
            let r = u8::from_str_radix(&color[0..2], 16).ok()?;
            let g = u8::from_str_radix(&color[2..4], 16).ok()?;
            let b = u8::from_str_radix(&color[4..6], 16).ok()?;
            Some(WebSafePalette::snap_to_web_safe(r, g, b))
        } else {
            match color.to_lowercase().as_str() {
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
}

pub struct LayoutEngine {
    config: EngineConfig,
    quirk_profile: QuirkProfile,
    style_cache: Arc<Mutex<HashMap<String, HashMap<String, CssValue>>>>,
    style_version: u64,
    layout_cache: Arc<Mutex<HashMap<String, RenderNode>>>,
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
            style_cache: Arc::new(Mutex::new(HashMap::new())),
            style_version: 0,
            layout_cache: Arc::new(Mutex::new(HashMap::new())),
            layout_version: 0,
        }
    }

    pub fn layout(&self, node: &DomNode, available_width: f32) -> RenderNode {
        self.layout_node(node, available_width, 0.0, 0.0, &StyleContext::default(), FloatContext::new())
    }

    fn layout_node(
        &self,
        node: &DomNode,
        width: f32,
        x: f32,
        y: f32,
        parent_style: StyleContext,
        float_context: FloatContext,
    ) -> RenderNode {
        let mut style = parent_style.clone();
        
        if let DomNode::Element { ref tag, ref attributes, .. } = node {
            if tag == "font" {
                if let Some(face) = attributes.get("face") {
                    style.font_face = face.clone();
                }
                if let Some(size_str) = attributes.get("size") {
                    style.font_size = self.get_legacy_font_size(size_str, style.font_size);
                }
                if let Some(color_str) = attributes.get("color") {
                    if let Some(color) = Self::parse_color(color_str) {
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
        
        if let DomNode::Element { ref tag, ref attributes, children: ref dom_children, node_id: _, .. } = node {
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
                };
            } else if tag == "frameset" {
                let frameset_special = SpecialElement::Frameset {
                    rows: attributes.get("rows").cloned().unwrap_or_default(),
                    cols: attributes.get("cols").cloned().unwrap_or_default(),
                    border: attributes.get("border").cloned().unwrap_or_default(),
                    frameborder: attributes.get("frameborder").cloned().unwrap_or_default(),
                    framespacing: attributes.get("framespacing").cloned().unwrap_or_default(),
                };
                
                return RenderNode {
                    dom_node: node.clone(),
                    box_model,
                    special: frameset_special,
                    children: vec![],
                    text_content,
                    computed_styles,
                    table_data: None,
                    z_index: 0,
                    absolute_x: x,
                    absolute_y: y,
                };
            } else {
                if tag == "img" {
                    if let Some(align) = attributes.get("align") {
                        if align == "left" || align == "right" {
                            let exclusion_rect = Rect {
                                x: box_model.x,
                                y: box_model.y,
                                width: box_model.width,
                                height: box_model.height,
                            };
                            float_context.add_exclusion_zone(exclusion_rect, align);
                        }
                    }
                }
                
                let (start_x, available_width) = float_context.get_available_space(current_y, box_model.height);
                
                for child in dom_children {
                    let child_width = available_width - box_model.padding_left - box_model.padding_right - box_model.border_left - box_model.border_right;
                    let child_render_node = self.layout_node(
                        child,
                        child_width,
                        start_x + box_model.padding_left + box_model.border_left,
                        current_y,
                        style.clone(),
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
        }
    }

    fn layout_table(
        &self,
        node: &DomNode,
        available_width: f32,
        x: f32,
        y: f32,
        parent_style: &StyleContext,
    ) -> TableBox {
        
        let cache_key = self.create_table_cache_key(node, available_width);
        
        
        if let Ok(cache_guard) = self.layout_cache.try_lock() {
            if let Some(cached_table) = cache_guard.get(&cache_key) {
                return cached_table.clone();
            }
        }
        
        let table_box = self.layout_table_uncached(node, available_width, x, y, parent_style);
        
        
        if let Ok(mut cache_guard) = self.layout_cache.try_lock() {
            cache_guard.insert(cache_key, table_box.clone());
        }
        
        table_box
    }

    fn layout_table_uncached(
        &self,
        node: &DomNode,
        available_width: f32,
        x: f32,
        y: f32,
        parent_style: &StyleContext,
    ) -> TableBox {
        if let DomNode::Element { ref children, ref attributes, .. } = node {
            let border = attributes.get("border").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
            let cellpadding = attributes.get("cellpadding").and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);
            let cellspacing = attributes.get("cellspacing").and_then(|s| s.parse::<f32>().ok()).unwrap_or(2.0);
            
            let mut rows = Vec::new();
            let mut max_cols = 0;
            
            
            for child in children {
                if let DomNode::Element { ref tag, children: ref row_children, attributes: ref row_attrs, .. } = child {
                    if tag == "tr" {
                        let mut row_cells = Vec::new();
                        let mut col_idx = 0;
                        
                        for cell in row_children {
                            if let DomNode::Element { tag: ref cell_tag, attributes: ref cell_attrs, children: ref cell_children, .. } = cell {
                                if cell_tag == "td" || cell_tag == "th" {
                                    let colspan = cell_attrs.get("colspan").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                                    let rowspan = cell_attrs.get("rowspan").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                                    let nowrap = cell_attrs.contains_key("nowrap");
                                    
                                    let mut cell_content = RenderNode {
                                        dom_node: cell.clone(),
                                        box_model: BoxModel {
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
                                        },
                                        special: SpecialElement::Link { href: String::new(), target: String::new() },
                                        children: Vec::new(),
                                        text_content: String::new(),
                                        computed_styles: HashMap::new(),
                                        table_data: None,
                                        z_index: 0,
                                        absolute_x: 0.0,
                                        absolute_y: 0.0,
                                    };
                                    
                                    let mut cell_y = 0.0;
                                    for cell_child in cell_children {
                                        let cell_child_node = self.layout_node(cell_child, 100.0, 0.0, cell_y, parent_style, FloatContext::new());
                                        cell_content.children.push(cell_child_node);
                                        cell_y += cell_child_node.box_model.height;
                                    }
                                    
                                    row_cells.push(Some(TableCellData {
                                        content: cell_content,
                                        colspan,
                                        rowspan,
                                        width: 0.0,
                                        height: 0.0,
                                        nowrap,
                                    }));
                                    
                                    col_idx += colspan;
                                }
                            }
                        }
                        
                        rows.push(row_cells);
                        max_cols = max_cols.max(col_idx);
                    }
                }
            }
            
            let mut grid = vec![vec![None; max_cols]; rows.len()];
            let mut row_idx = 0;
            
            for row in &rows {
                let mut col_idx = 0;
                for cell_opt in row {
                    if let Some(cell_data) = cell_opt {
                        while col_idx < max_cols && grid[row_idx][col_idx].is_some() {
                            col_idx += 1;
                        }
                        
                        if col_idx < max_cols {
                            for c in 0..cell_data.colspan {
                                for r in 0..cell_data.rowspan {
                                    let grid_row = row_idx + r;
                                    let grid_col = col_idx + c;
                                    if grid_row < grid.len() && grid_col < grid[grid_row].len() {
                                        grid[grid_row][grid_col] = Some(TableCellData {
                                            content: cell_data.content.clone(),
                                            colspan: if c == 0 && r == 0 { cell_data.colspan } else { 1 },
                                            rowspan: if c == 0 && r == 0 { cell_data.rowspan } else { 1 },
                                            width: 0.0,
                                            height: 0.0,
                                            nowrap: cell_data.nowrap,
                                        });
                                    }
                                }
                            }
                        }
                        col_idx += cell_data.colspan;
                    }
                }
                row_idx += 1;
            }
            
            let mut column_widths = vec![0.0; max_cols];
            
            let mut cell_widths = HashMap::new();
            for (row_idx, row) in grid.iter().enumerate() {
                for (col_idx, cell_opt) in row.iter().enumerate() {
                    if let Some(cell) = cell_opt {
                        if col_idx == 0 || grid[row_idx][col_idx - 1].is_none() {
                            let min_width = self.calculate_min_cell_width(cell);
                            cell_widths.insert((row_idx, col_idx), min_width);
                        }
                    }
                }
            }
            
            for (row_idx, row) in grid.iter().enumerate() {
                for (col_idx, cell_opt) in row.iter().enumerate() {
                    if let Some(cell) = cell_opt {
                        if col_idx == 0 || grid[row_idx][col_idx - 1].is_none() {
                            if let Some(min_width) = cell_widths.get(&(row_idx, col_idx)) {
                                let col_span = cell.colspan;
                                let width_per_col = min_width / col_span as f32;
                                for i in 0..col_span {
                                    if col_idx + i < column_widths.len() {
                                        column_widths[col_idx + i] = column_widths[col_idx + i].max(width_per_col);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            let total_width = column_widths.iter().sum::<f32>() + (max_cols as f32 - 1.0) * cellspacing;
            if total_width < available_width && max_cols > 0 {
                let extra_space = available_width - total_width;
                let extra_per_col = extra_space / max_cols as f32;
                for width in &mut column_widths {
                    *width += extra_per_col;
                }
            }
            
            let mut row_heights = vec![0.0; grid.len()];
            for (row_idx, row) in grid.iter().enumerate() {
                let mut max_height = 0.0;
                for (col_idx, cell_opt) in row.iter().enumerate() {
                    if let Some(cell) = cell_opt {
                        if col_idx == 0 || grid[row_idx][col_idx - 1].is_none() {
                            let cell_height = self.calculate_cell_height(cell, column_widths[col_idx]);
                            max_height = max_height.max(cell_height);
                        }
                    }
                }
                row_heights[row_idx] = max_height;
            }
            
            TableBox {
                rows: grid,
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

    fn create_table_cache_key(&self, node: &DomNode, available_width: f32) -> String {
        let mut hasher = DefaultHasher::new();
        
        match node {
            DomNode::Element { tag, attributes, children, id, class, node_id } => {
                tag.hash(&mut hasher);
                attributes.hash(&mut hasher);
                id.hash(&mut hasher);
                class.hash(&mut hasher);
                node_id.hash(&mut hasher);
                
                for child in children {
                    if let DomNode::Element { tag: child_tag, attributes: child_attrs, .. } = child {
                        if child_tag == "tr" {
                            child_attrs.hash(&mut hasher);
                        }
                    }
                }
            },
            _ => {},
        }
        
        available_width.to_bits().hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }

    fn calculate_min_cell_width(&self, cell: &TableCellData) -> f32 {
        if cell.nowrap {
            let mut total_width = 0.0;
            for child in &cell.content.children {
                total_width += child.box_model.width;
            }
            total_width
        } else {
            let mut min_width = 0.0;
            for child in &cell.content.children {
                min_width = min_width.max(child.box_model.width);
            }
            min_width
        }
    }

    fn calculate_cell_height(&self, cell: &TableCellData, width: f32) -> f32 {
        let mut total_height = 0.0;
        for child in &cell.content.children {
            total_height += child.box_model.height;
        }
        total_height
    }

    fn compute_styles(
        &self,
        node: &DomNode,
        width: f32,
        x: f32,
        y: f32,
        style_context: &StyleContext,
    ) -> (BoxModel, SpecialElement, HashMap<String, CssValue>) {
        let cache_key = self.create_style_cache_key(node, width, x, y);
        
        if let Ok(cache_guard) = self.style_cache.try_lock() {
            if let Some(cached_styles) = cache_guard.get(&cache_key) {
                let special = self.create_special_element(node);
                let box_model = self.create_box_model(node, width, x, y, style_context);
                return (box_model, special, cached_styles.clone());
            }
        }
        
        let (box_model, special, computed_styles) = self.compute_styles_uncached(node, width, x, y, style_context);
        
        if let Ok(mut cache_guard) = self.style_cache.try_lock() {
            cache_guard.insert(cache_key, computed_styles.clone());
        }
        
        (box_model, special, computed_styles)
    }

    fn compute_styles_uncached(
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
                    "map" => {
                        let name = attributes.get("name").cloned().unwrap_or_default();
                        let areas = Vec::new();
                        
                        SpecialElement::Map {
                            name,
                            areas,
                        }
                    },
                    "area" => {
                        let shape = attributes.get("shape").cloned().unwrap_or_else(|| "rect".to_string());
                        let coords_str = attributes.get("coords").cloned().unwrap_or_default();
                        let coords = coords_str.split(',')
                            .filter_map(|c| c.trim().parse::<i32>().ok())
                            .collect();
                        let href = attributes.get("href").cloned().unwrap_or_default();
                        let alt = attributes.get("alt").cloned().unwrap_or_default();
                        
                        SpecialElement::Area {
                            shape,
                            coords,
                            href,
                            alt,
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
                        let color = attributes.get("color").and_then(|c| Self::parse_color(c));
                        
                        SpecialElement::Font {
                            face,
                            size,
                            color,
                        }
                    },
                    "body" => {
                        let bgcolor = attributes.get("bgcolor").and_then(|c| Self::parse_color(c));
                        let background = attributes.get("background").cloned();
                        
                        SpecialElement::Body {
                            bgcolor,
                            background,
                        }
                    },
                    "layer" => {
                        let id = attributes.get("id").cloned().unwrap_or_default();
                        let z_index = attributes.get("z-index").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        
                        SpecialElement::Layer {
                            id,
                            z_index,
                        }
                    },
                    "ilayer" => {
                        let id = attributes.get("id").cloned().unwrap_or_default();
                        let z_index = attributes.get("z-index").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        
                        SpecialElement::ILayer {
                            id,
                            z_index,
                        }
                    },
                    _ => SpecialElement::Link {
                        href: String::new(),
                        target: String::new(),
                    },
                }
            },
            DomNode::Text(_) => SpecialElement::Link {
                href: String::new(),
                target: String::new(),
            },
            DomNode::Comment(_) => SpecialElement::Link {
                href: String::new(),
                target: String::new(),
            },
        };

        match node {
            DomNode::Element { ref tag, ref attributes, .. } => {
                if let Some(bgcolor) = attributes.get("bgcolor") {
                    if let Some(color) = Self::parse_color(bgcolor) {
                        box_model.background_color = Some(color);
                    }
                }
                
                if let Some(color) = attributes.get("color") {
                    if let Some(rgb) = Self::parse_color(color) {
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
                
                if let Some(height_attr) = attributes.get("height") {
                    if height_attr.ends_with('%') {
                        if let Ok(pct) = height_attr.trim_end_matches('%').parse::<f32>() {
                            box_model.height = style_context.font_size * 1.2 * pct / 100.0;
                        }
                    } else if let Ok(px) = height_attr.parse::<f32>() {
                        box_model.height = px;
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

    fn parse_color(color_str: &str) -> Option<(u8, u8, u8)> {
        let color = color_str.trim_start_matches('#');
        if color.len() == 6 {
            let r = u8::from_str_radix(&color[0..2], 16).ok()?;
            let g = u8::from_str_radix(&color[2..4], 16).ok()?;
            let b = u8::from_str_radix(&color[4..6], 16).ok()?;
            Some(WebSafePalette::snap_to_web_safe(r, g, b))
        } else {
            match color.to_lowercase().as_str() {
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

    fn get_legacy_font_size(size_attr: &str, parent_size: f32) -> f32 {
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

    fn create_style_cache_key(&self, node: &DomNode, width: f32, x: f32, y: f32) -> String {
        let mut hasher = DefaultHasher::new();
        
        match node {
            DomNode::Element { tag, attributes, id, class, node_id, .. } => {
                tag.hash(&mut hasher);
                attributes.hash(&mut hasher);
                id.hash(&mut hasher);
                class.hash(&mut hasher);
                node_id.hash(&mut hasher);
            },
            DomNode::Text(text) => {
                "text".hash(&mut hasher);
                text.hash(&mut hasher);
            },
            DomNode::Comment(comment) => {
                "comment".hash(&mut hasher);
                comment.hash(&mut hasher);
            },
        }
        
        width.to_bits().hash(&mut hasher);
        x.to_bits().hash(&mut hasher);
        y.to_bits().hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }

    fn create_special_element(&self, node: &DomNode) -> SpecialElement {
        match node {
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
                    "map" => {
                        let name = attributes.get("name").cloned().unwrap_or_default();
                        let areas = Vec::new();
                        
                        SpecialElement::Map {
                            name,
                            areas,
                        }
                    },
                    "area" => {
                        let shape = attributes.get("shape").cloned().unwrap_or_else(|| "rect".to_string());
                        let coords_str = attributes.get("coords").cloned().unwrap_or_default();
                        let coords = coords_str.split(',')
                            .filter_map(|c| c.trim().parse::<i32>().ok())
                            .collect();
                        let href = attributes.get("href").cloned().unwrap_or_default();
                        let alt = attributes.get("alt").cloned().unwrap_or_default();
                        
                        SpecialElement::Area {
                            shape,
                            coords,
                            href,
                            alt,
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
                        let color = attributes.get("color").and_then(|c| Self::parse_color(c));
                        
                        SpecialElement::Font {
                            face,
                            size,
                            color,
                        }
                    },
                    "body" => {
                        let bgcolor = attributes.get("bgcolor").and_then(|c| Self::parse_color(c));
                        let background = attributes.get("background").cloned();
                        
                        SpecialElement::Body {
                            bgcolor,
                            background,
                        }
                    },
                    "layer" => {
                        let id = attributes.get("id").cloned().unwrap_or_default();
                        let z_index = attributes.get("z-index").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        
                        SpecialElement::Layer {
                            id,
                            z_index,
                        }
                    },
                    "ilayer" => {
                        let id = attributes.get("id").cloned().unwrap_or_default();
                        let z_index = attributes.get("z-index").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        
                        SpecialElement::ILayer {
                            id,
                            z_index,
                        }
                    },
                    _ => SpecialElement::Link {
                        href: String::new(),
                        target: String::new(),
                    },
                }
            },
            DomNode::Text(_) => SpecialElement::Link {
                href: String::new(),
                target: String::new(),
            },
            DomNode::Comment(_) => SpecialElement::Link {
                href: String::new(),
                target: String::new(),
            },
        }
    }

    fn create_box_model(&self, node: &DomNode, width: f32, x: f32, y: f32, style_context: &StyleContext) -> BoxModel {
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

        match node {
            DomNode::Element { ref tag, ref attributes, .. } => {
                if let Some(bgcolor) = attributes.get("bgcolor") {
                    if let Some(color) = Self::parse_color(bgcolor) {
                        box_model.background_color = Some(color);
                    }
                }
                
                if let Some(color) = attributes.get("color") {
                    if let Some(rgb) = Self::parse_color(color) {
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
                
                if let Some(height_attr) = attributes.get("height") {
                    if height_attr.ends_with('%') {
                        if let Ok(pct) = height_attr.trim_end_matches('%').parse::<f32>() {
                            box_model.height = style_context.font_size * 1.2 * pct / 100.0;
                        }
                    } else if let Ok(px) = height_attr.parse::<f32>() {
                        box_model.height = px;
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
            },
            _ => {},
        }

        box_model
    }
}

lazy_static! {
    static ref EMPTY_ATTRS: HashMap<String, String> = HashMap::new();
    static ref EMPTY_CHILDREN: Vec<DomNode> = Vec::new();
    static ref GLOBAL_CACHE: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref GLOBAL_CSS_CACHE: Mutex<HashMap<String, Vec<CssRule>>> = Mutex::new(HashMap::new());
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
    pub dom_hash_cache: Arc<Mutex<HashMap<String, u64>>>,
    pub last_dom_hash: Arc<Mutex<Option<u64>>>,
    pub render_cache: Arc<Mutex<HashMap<u64, RenderNode>>>,
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
            display_context: Arc::new(Mutex::new(DisplayContext {
                dpi_scale: 1.0,
                viewport_width: 800.0,
                viewport_height: 600.0,
                device_pixel_ratio: 1.0,
            })),
            document_base_url: Arc::new(Mutex::new(None)),
            progressive_render_callback: None,
            user_agent: "Mozilla/3.0 (Windows 95; TrussCore/1.0; Rust; en)".to_string(),
            js_engine: Arc::new(Mutex::new(ChronoScript::new())),
            dom_hash_cache: Arc::new(Mutex::new(HashMap::new())),
            last_dom_hash: Arc::new(Mutex::new(None)),
            render_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn load_html(&self, html: &str) {
        let tokens = {
            let mut tokenizer = HtmlTokenizer::new(html.to_string(), self.config.rendering_mode.clone());
            tokenizer.tokenize()
        };
        
        let dom = {
            let mut builder = DomBuilder::new(self.config.rendering_mode.clone());
            builder.build(&tokens)
        };
        
        {
            let mut dom_guard = self.dom.lock().unwrap();
            *dom_guard = Some(dom);
        }
        
        self.process_scripts();
        
        if self.config.progressive_rendering {
            if let Some(callback) = &self.progressive_render_callback {
                callback();
            }
        }
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

    fn calculate_dom_hash(&self, node: &DomNode) -> u64 {
        let mut hasher = DefaultHasher::new();
        
        match node {
            DomNode::Element { tag, attributes, children, id, class, node_id } => {
                tag.hash(&mut hasher);
                attributes.hash(&mut hasher);
                id.hash(&mut hasher);
                class.hash(&mut hasher);
                node_id.hash(&mut hasher);
                
                for child in children {
                    self.calculate_dom_hash(child).hash(&mut hasher);
                }
            },
            DomNode::Text(text) => {
                "text".hash(&mut hasher);
                text.hash(&mut hasher);
            },
            DomNode::Comment(comment) => {
                "comment".hash(&mut hasher);
                comment.hash(&mut hasher);
            },
        }
        
        hasher.finish()
    }

    fn should_rebuild_render_tree(&self) -> bool {
        let dom_guard = self.dom.lock().unwrap();
        if let Some(ref dom) = *dom_guard {
            let current_hash = self.calculate_dom_hash(dom);
            
            let mut last_hash_guard = self.last_dom_hash.lock().unwrap();
            if let Some(last_hash) = *last_hash_guard {
                if last_hash == current_hash {
                    return false;
                }
            }
            
            *last_hash_guard = Some(current_hash);
        }
        
        true
    }

    fn rebuild_render_tree(&self, width: f32) {
        if !self.should_rebuild_render_tree() {
            return;
        }
        
        let dom_guard = self.dom.lock().unwrap();
        let css_guard = self.css_rules.lock().unwrap();
        
        if let Some(ref dom) = *dom_guard {
            let layout_engine = LayoutEngine::new(self.config.clone());
            let render_tree = layout_engine.layout(dom, width);
            
            drop(dom_guard);
            drop(css_guard);
            
            let mut render_guard = self.render_tree.lock().unwrap();
            *render_guard = Some(render_tree);
            
            self.update_image_nodes();
        }
    }

    fn render_to_display_list(&self) -> DisplayList {
        let render_guard = self.render_tree.lock().unwrap();
        if let Some(ref node) = *render_guard {
            Self::render_node_to_display_list_recursive(node)
        } else {
            DisplayList { commands: vec![] }
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
                    if let Some(cell) = cell_opt {
                        if col_idx == 0 || table_data.rows[row_idx][col_idx - 1].is_none() {
                            let x = node.absolute_x + table_data.column_widths[0..col_idx].iter().sum::<f32>();
                            let y = node.absolute_y + table_data.row_heights[0..row_idx].iter().sum::<f32>();
                            let w = table_data.column_widths[col_idx];
                            let h = table_data.row_heights[row_idx];
                            
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

        {
            let mut render_guard = self.render_tree.lock().unwrap();
            if let Some(ref mut render_tree) = *render_guard {
                self.update_image_node(render_tree, src);
            }
        }

        Ok(())
    }

    fn update_image_node(&self, node: &mut RenderNode, src: &str) {
        if let SpecialElement::Image { src: ref mut node_src, ref mut natural_width, ref mut natural_height, ref mut loaded, .. } = node.special {
            if node_src == src {
                if let Ok(cache_guard) = self.image_cache.try_lock() {
                    if let Some(img) = cache_guard.get(src) {
                        let (w, h) = img.dimensions();
                        *natural_width = w;
                        *natural_height = h;
                        *loaded = true;
                    }
                }
            }
        }
        
        for child in &mut node.children {
            self.update_image_node(child, src);
        }
    }

    fn update_image_nodes(&self) {
        let mut render_guard = self.render_tree.lock().unwrap();
        if let Some(ref mut render_tree) = *render_guard {
            self.update_all_image_nodes(render_tree);
        }
    }

    fn update_all_image_nodes(&self, node: &mut RenderNode) {
        if let SpecialElement::Image { ref src, ref mut natural_width, ref mut natural_height, ref mut loaded, width, height, .. } = node.special {
            if let Ok(cache_guard) = self.image_cache.try_lock() {
                if let Some(img) = cache_guard.get(src) {
                    let (w, h) = img.dimensions();
                    *natural_width = w;
                    *natural_height = h;
                    *loaded = true;
                    
                    if width.is_none() && height.is_none() {
                        node.box_model.width = *natural_width as f32;
                        node.box_model.height = *natural_height as f32;
                    }
                } else if width.is_some() && height.is_some() {
                    *natural_width = width.unwrap();
                    *natural_height = height.unwrap();
                    *loaded = true;
                    node.box_model.width = width.unwrap() as f32;
                    node.box_model.height = height.unwrap() as f32;
                }
            }
        }
        
        for child in &mut node.children {
            self.update_all_image_nodes(child);
        }
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

    pub fn get_element_by_id_ref(&self, id: &str) -> Option<&DomNode> {
        let dom_guard = self.dom.lock().unwrap();
        if let Some(ref dom) = *dom_guard {
            dom.find_by_id(id)
        } else {
            None
        }
    }

    pub fn get_element_by_id_mut(&self, id: &str) -> Option<&mut DomNode> {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            self.find_by_id_mut(dom, id)
        } else {
            None
        }
    }

    fn find_by_id_mut<'a>(&self, node: &'a mut DomNode, id: &str) -> Option<&'a mut DomNode> {
        match node {
            DomNode::Element { id: ref node_id, node_id: _, .. } if node_id.as_deref() == Some(id) => Some(node),
            DomNode::Element { ref mut children, .. } => {
                for child in children {
                    if let Some(found) = self.find_by_id_mut(child, id) {
                        return Some(found);
                    }
                }
                None
            },
            _ => None,
        }
    }

    pub fn get_element_by_node_id(&self, node_id: usize) -> Option<DomNode> {
        let dom_guard = self.dom.lock().unwrap();
        if let Some(ref dom) = *dom_guard {
            dom.find_by_node_id(node_id).cloned()
        } else {
            None
        }
    }

    pub fn get_element_by_node_id_ref(&self, node_id: usize) -> Option<&DomNode> {
        let dom_guard = self.dom.lock().unwrap();
        if let Some(ref dom) = *dom_guard {
            dom.find_by_node_id(node_id)
        } else {
            None
        }
    }

    pub fn get_element_by_node_id_mut(&self, node_id: usize) -> Option<&mut DomNode> {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            self.find_by_node_id_mut(dom, node_id)
        } else {
            None
        }
    }

    fn find_by_node_id_mut<'a>(&self, node: &'a mut DomNode, target_node_id: usize) -> Option<&'a mut DomNode> {
        match node {
            DomNode::Element { id: _, attributes: _, children, class: _, node_id: current_node_id } if *current_node_id == target_node_id => Some(node),
            DomNode::Element { ref mut children, .. } => {
                for child in children {
                    if let Some(found) = self.find_by_node_id_mut(child, target_node_id) {
                        return Some(found);
                    }
                }
                None
            },
            _ => None,
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

    pub fn get_elements_by_tag_ref(&self, tag: &str) -> Vec<&DomNode> {
        let dom_guard = self.dom.lock().unwrap();
        if let Some(ref dom) = *dom_guard {
            dom.find_by_tag(tag)
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

    pub fn get_areas_for_image(&self, usemap: &str) -> Vec<ImageArea> {
        let dom_guard = self.dom.lock().unwrap();
        if let Some(ref dom) = *dom_guard {
            let maps = dom.find_by_tag("map");
            for map_node in maps {
                if let DomNode::Element { ref attributes, ref children, .. } = map_node {
                    if let Some(map_name) = attributes.get("name") {
                        if map_name == usemap.trim_start_matches('#') {
                            let mut areas = Vec::new();
                            for child in children {
                                if let DomNode::Element { ref tag, ref attributes, .. } = child {
                                    if tag == "area" {
                                        let shape = attributes.get("shape").cloned().unwrap_or_else(|| "rect".to_string());
                                        let coords_str = attributes.get("coords").cloned().unwrap_or_default();
                                        let coords = coords_str.split(',')
                                            .filter_map(|c| c.trim().parse::<i32>().ok())
                                            .collect();
                                        let href = attributes.get("href").cloned().unwrap_or_default();
                                        let alt = attributes.get("alt").cloned().unwrap_or_default();
                                        
                                        areas.push(ImageArea {
                                            shape,
                                            coords,
                                            href,
                                            alt,
                                        });
                                    }
                                }
                            }
                            return areas;
                        }
                    }
                }
            }
        }
        Vec::new()
    }

    pub fn hit_test_image_area(&self, img_src: &str, x: f32, y: f32) -> Option<String> {
        let dom_guard = self.dom.lock().unwrap();
        if let Some(ref dom) = *dom_guard {
            let images = dom.find_by_tag("img");
            for img_node in images {
                if let DomNode::Element { ref attributes, .. } = img_node {
                    if let Some(src) = attributes.get("src") {
                        if src == img_src {
                            if let Some(usemap) = attributes.get("usemap") {
                                let areas = self.get_areas_for_image(usemap);
                                for area in &areas {
                                    if self.point_in_area(x as i32, y as i32, area) {
                                        return Some(area.href.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn point_in_area(&self, x: i32, y: i32, area: &ImageArea) -> bool {
        match area.shape.as_str() {
            "rect" => {
                if area.coords.len() >= 4 {
                    let x1 = area.coords[0];
                    let y1 = area.coords[1];
                    let x2 = area.coords[2];
                    let y2 = area.coords[3];
                    x >= x1 && x <= x2 && y >= y1 && y <= y2
                } else {
                    false
                }
            },
            "circle" => {
                if area.coords.len() >= 3 {
                    let cx = area.coords[0];
                    let cy = area.coords[1];
                    let r = area.coords[2];
                    let dx = x - cx;
                    let dy = y - cy;
                    dx * dx + dy * dy <= r * r
                } else {
                    false
                }
            },
            "poly" => {
                if area.coords.len() >= 6 && area.coords.len() % 2 == 0 {
                    let points: Vec<(i32, i32)> = area.coords.chunks(2)
                        .map(|chunk| (chunk[0], chunk[1]))
                        .collect();
                    
                    let mut inside = false;
                    let mut j = points.len() - 1;
                    
                    for i in 0..points.len() {
                        let (xi, yi) = points[i];
                        let (xj, yj) = points[j];
                        
                        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
                            inside = !inside;
                        }
                        j = i;
                    }
                    
                    inside
                } else {
                    false
                }
            },
            _ => false,
        }
    }

    pub fn set_css_rules(&self, css_rules: Vec<CssRule>) {
        let mut css_guard = self.css_rules.lock().unwrap();
        *css_guard = css_rules;
        self.rebuild_render_tree(800.0);
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

        self.rebuild_render_tree(800.0);
        Ok(())
    }

    fn update_dom_attribute(&self, node_id: usize, key: &str, value: &str) {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            self.update_dom_node_attribute(dom, node_id, key, value);
        }
    }

    fn update_dom_node_attribute(&self, node: &mut DomNode, target_node_id: usize, key: &str, value: &str) {
        match node {
            DomNode::Element { node_id, attributes, .. } if *node_id == target_node_id => {
                attributes.insert(key.to_string(), value.to_string());
            },
            DomNode::Element { ref mut children, .. } => {
                for child in children {
                    self.update_dom_node_attribute(child, target_node_id, key, value);
                }
            },
            _ => {}
        }
    }

    fn update_dom_style(&self, node_id: usize, property: &str, value: &str) {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            self.update_dom_node_attribute(dom, node_id, "style", &format!("{}: {}", property, value));
        }
    }

    fn insert_dom_child(&self, parent_id: usize, child: DomNode) {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            self.insert_dom_node_child(dom, parent_id, child);
        }
    }

    fn insert_dom_node_child(&self, node: &mut DomNode, target_node_id: usize, child: DomNode) {
        match node {
            DomNode::Element { node_id, ref mut children, .. } if *node_id == target_node_id => {
                children.push(child);
            },
            DomNode::Element { ref mut children, .. } => {
                for child_node in children {
                    self.insert_dom_node_child(child_node, target_node_id, child.clone());
                }
            },
            _ => {}
        }
    }

    fn remove_dom_child(&self, parent_id: usize, child_id: usize) {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            self.remove_dom_node_child(dom, parent_id, child_id);
        }
    }

    fn remove_dom_node_child(&self, node: &mut DomNode, target_node_id: usize, child_id: usize) {
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
                    self.remove_dom_node_child(child_node, target_node_id, child_id);
                }
            },
            _ => {}
        }
    }

    fn set_dom_text_content(&self, node_id: usize, text: &str) {
        let mut dom_guard = self.dom.lock().unwrap();
        if let Some(ref mut dom) = *dom_guard {
            self.set_dom_node_text_content(dom, node_id, text);
        }
    }

    fn set_dom_node_text_content(&self, node: &mut DomNode, target_node_id: usize, text: &str) {
        match node {
            DomNode::Element { node_id, ref mut children, .. } if *node_id == target_node_id => {
                *children = vec![DomNode::Text(text.to_string())];
            },
            DomNode::Element { ref mut children, .. } => {
                for child in children {
                    self.set_dom_node_text_content(child, target_node_id, text);
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

    pub fn set_element_style(&self, node_id: usize, property: &str, value: &str) -> Result<(), String> {
        {
            let mut buffer_guard = self.mutation_buffer.lock().unwrap();
            buffer_guard.push(DomMutation::SetStyle {
                node_id,
                property: property.to_string(),
                value: value.to_string(),
            });
        }

        self.process_mutations()?;

        Ok(())
    }

    pub fn mark_node_dirty(&self, node_id: usize) {
        let mut render_guard = self.render_tree.lock().unwrap();
        if let Some(ref mut render_tree) = *render_guard {
            self.mark_node_dirty_recursive(render_tree, node_id);
        }
    }

    fn mark_node_dirty_recursive(&self, node: &mut RenderNode, target_node_id: usize) {
        if let Some(node_id) = node.dom_node.node_id() {
            if node_id == target_node_id {
                node.dirty = true;
                node.style_version += 1;
                node.layout_version += 1;
            }
        }
        
        for child in &mut node.children {
            self.mark_node_dirty_recursive(child, target_node_id);
        }
    }

    pub fn incremental_layout(&self, node_id: usize, new_width: f32) {
        let mut render_guard = self.render_tree.lock().unwrap();
        if let Some(ref mut render_tree) = *render_guard {
            self.perform_incremental_layout(render_tree, node_id, new_width);
        }
    }

    fn perform_incremental_layout(&self, node: &mut RenderNode, target_node_id: usize, new_width: f32) {
        if let Some(node_id) = node.dom_node.node_id() {
            if node_id == target_node_id {
                if node.dirty {
                    let layout_engine = LayoutEngine::new(self.config.clone());
                    let new_render_node = layout_engine.layout(&node.dom_node, new_width);
                    
                    let absolute_x = node.absolute_x;
                    let absolute_y = node.absolute_y;
                    
                    *node = new_render_node;
                    node.absolute_x = absolute_x;
                    node.absolute_y = absolute_y;
                    node.dirty = false;
                }
                return;
            }
        }
        
        for child in &mut node.children {
            self.perform_incremental_layout(child, target_node_id, new_width);
        }
    }

    pub fn dispatch_event(&self, node_id: usize, event_type: &str) -> Result<(), String> {
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
                let src = attributes.get("src");
                let content = match src {
                    Some(_script_src) => {
                        String::new()
                    },
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

    pub fn get_display_context(&self) -> DisplayContext {
        let ctx_guard = self.display_context.lock().unwrap();
        ctx_guard.clone()
    }

    pub fn set_display_context(&self, context: DisplayContext) {
        let mut ctx_guard = self.display_context.lock().unwrap();
        *ctx_guard = context;
    }

    pub fn get_base_url(&self) -> Option<String> {
        let base_guard = self.document_base_url.lock().unwrap();
        base_guard.clone()
    }

    pub fn resolve_url(&self, url: &str) -> String {
        if let Ok(parsed) = Url::parse(url) {
            return url.to_string();
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

    pub fn set_progressive_render_callback<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.progressive_render_callback = Some(Box::new(callback));
    }

    pub fn trigger_progressive_render(&self) {
        if let Some(ref callback) = self.progressive_render_callback {
            callback();
        }
    }

    pub fn get_render_tree(&self) -> Option<RenderNode> {
        let render_guard = self.render_tree.lock().unwrap();
        render_guard.clone()
    }

    pub fn update_animated_gifs(&self) {
        let mut render_guard = self.render_tree.lock().unwrap();
        if let Some(ref mut render_tree) = *render_guard {
            self.update_animated_gifs_recursive(render_tree);
        }
    }

    fn update_animated_gifs_recursive(&self, node: &mut RenderNode) {
        if let SpecialElement::Image { animated_gif: Some(_animation), .. } = node.special {
        }
        
        for child in &mut node.children {
            self.update_animated_gifs_recursive(child);
        }
    }

    pub fn execute_script(&self, script: &str) -> Result<(), String> {
        let mut js_guard = self.js_engine.lock().unwrap();
        js_guard.execute(script).map_err(|e| format!("{:?}", e))
    }

    pub fn clear_output(&self) {
    }

    pub fn get_output(&self) -> String {
        String::new()
    }

    pub fn fetch_url(&self, url: &str) -> Result<String, String> {
        Err("Network functionality not implemented".to_string())
    }

    pub fn parse_html(&self, html: &str) -> DomNode {
        let tokens = {
            let mut tokenizer = HtmlTokenizer::new(html.to_string(), self.config.rendering_mode.clone());
            tokenizer.tokenize()
        };
        
        let mut builder = DomBuilder::new(self.config.rendering_mode.clone());
        builder.build(&tokens)
    }

    pub fn build_render_tree(&self, dom: &DomNode) -> RenderNode {
        let layout_engine = LayoutEngine::new(self.config.rendering_mode.clone());
        layout_engine.layout(dom, 800.0)
    }

    pub fn find_first_tag<'a>(&self, dom: &'a mut DomNode, tag_name: &str) -> Option<&'a mut DomNode> {
        match dom {
            DomNode::Element { tag, children, .. } if tag == tag_name => Some(dom),
            DomNode::Element { children, .. } => {
                for child in children {
                    if let Some(found) = self.find_first_tag(child, tag_name) {
                        return Some(found);
                    }
                }
                None
            },
            _ => None,
        }
    }

    pub fn execute_document_scripts(&self) {
        self.process_scripts();
    }

    pub fn clear_caches(&self) {
        let mut style_cache_guard = self.style_cache.lock().unwrap();
        style_cache_guard.clear();
        
        let mut layout_cache_guard = self.layout_cache.lock().unwrap();
        layout_cache_guard.clear();
        
        let mut dom_hash_cache_guard = self.dom_hash_cache.lock().unwrap();
        dom_hash_cache_guard.clear();
        
        let mut render_cache_guard = self.render_cache.lock().unwrap();
        render_cache_guard.clear();
        
        let mut last_hash_guard = self.last_dom_hash.lock().unwrap();
        *last_hash_guard = None;
    }

    pub fn get_memory_usage(&self) -> MemoryUsage {
        let style_cache_guard = self.style_cache.lock().unwrap();
        let layout_cache_guard = self.layout_cache.lock().unwrap();
        let dom_hash_cache_guard = self.dom_hash_cache.lock().unwrap();
        let render_cache_guard = self.render_cache.lock().unwrap();
        let image_cache_guard = self.image_cache.lock().unwrap();
        
        MemoryUsage {
            style_cache_entries: style_cache_guard.len(),
            layout_cache_entries: layout_cache_guard.len(),
            dom_hash_cache_entries: dom_hash_cache_guard.len(),
            render_cache_entries: render_cache_guard.len(),
            image_cache_entries: image_cache_guard.len(),
        }
    }

    pub fn optimize_memory(&self) {
        self.clear_caches();
        
        let mut image_cache_guard = self.image_cache.lock().unwrap();
        image_cache_guard.clear();
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        let style_cache_guard = self.style_cache.lock().unwrap();
        let layout_cache_guard = self.layout_cache.lock().unwrap();
        let dom_hash_cache_guard = self.dom_hash_cache.lock().unwrap();
        let render_cache_guard = self.render_cache.lock().unwrap();
        
        CacheStats {
            style_cache_size: style_cache_guard.len(),
            layout_cache_size: layout_cache_guard.len(),
            dom_hash_cache_size: dom_hash_cache_guard.len(),
            render_cache_size: render_cache_guard.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryUsage {
    pub style_cache_entries: usize,
    pub layout_cache_entries: usize,
    pub dom_hash_cache_entries: usize,
    pub render_cache_entries: usize,
    pub image_cache_entries: usize,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub style_cache_size: usize,
    pub layout_cache_size: usize,
    pub dom_hash_cache_size: usize,
    pub render_cache_size: usize,
}

impl Default for TrussCore {
    fn default() -> Self {
        Self::new()
    }
}
