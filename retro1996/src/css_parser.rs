use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::str::Chars;

use crate::engine::{CssRule, CssValue, CssToken, CssTokenKind, EngineError, WebSafePalette};

#[derive(Debug, Clone)]
pub struct CssParser {
    input: String,
    pos: usize,
    mode: CssParsingMode,
    error_recovery: bool,
    parse_start_time: Option<Instant>,
    property_counts: HashMap<String, usize>,
    selector_counts: HashMap<String, usize>,
    invalid_rules: Vec<String>,
    nesting_level: usize,
    max_nesting_level: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CssParsingMode {
    Strict,
    Quirks1996,
    Lenient,
}

impl CssParser {
    pub fn new(mode: CssParsingMode) -> Self {
        CssParser {
            input: String::new(),
            pos: 0,
            mode,
            error_recovery: true,
            parse_start_time: None,
            property_counts: HashMap::new(),
            selector_counts: HashMap::new(),
            invalid_rules: Vec::new(),
            nesting_level: 0,
            max_nesting_level: 50,
        }
    }

    pub fn parse(&mut self, css: &str) -> Result<Vec<CssRule>, EngineError> {
        self.parse_start_time = Some(Instant::now());
        self.input = css.to_string();
        self.pos = 0;
        self.property_counts.clear();
        self.selector_counts.clear();
        self.invalid_rules.clear();
        self.nesting_level = 0;
        
        let tokens = self.tokenize()?;
        let rules = self.parse_rules(&tokens)?;
        
        Ok(rules)
    }

    fn tokenize(&mut self) -> Result<Vec<CssToken>, EngineError> {
        let mut tokens = Vec::new();
        
        while self.pos < self.input.len() {
            if self.consume_whitespace() {
                continue;
            }
            
            if let Some(token) = self.parse_token()? {
                tokens.push(token);
            } else {
                break;
            }
        }
        
        Ok(tokens)
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

    fn parse_token(&mut self) -> Result<Option<CssToken>, EngineError> {
        if self.pos >= self.input.len() {
            return Ok(None);
        }
        
        let ch = self.input.chars().nth(self.pos).unwrap();
        
        match ch {
            '{' => {
                self.pos += 1;
                Ok(Some(CssToken { kind: CssTokenKind::CurlyBracketOpen, value: "{".to_string() }))
            },
            '}' => {
                self.pos += 1;
                Ok(Some(CssToken { kind: CssTokenKind::CurlyBracketClose, value: "}".to_string() }))
            },
            ';' => {
                self.pos += 1;
                Ok(Some(CssToken { kind: CssTokenKind::Semicolon, value: ";".to_string() }))
            },
            ':' => {
                self.pos += 1;
                Ok(Some(CssToken { kind: CssTokenKind::Colon, value: ":".to_string() }))
            },
            ',' => {
                self.pos += 1;
                Ok(Some(CssToken { kind: CssTokenKind::Comma, value: ",".to_string() }))
            },
            '#' => {
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
                Ok(Some(CssToken { kind: CssTokenKind::Color, value: color }))
            },
            '"' | '\'' => {
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
                Ok(Some(CssToken { kind: CssTokenKind::String, value: string }))
            },
            _ if ch.is_alphabetic() => {
                let start = self.pos;
                while self.pos < self.input.len() {
                    let ch = self.input.chars().nth(self.pos).unwrap();
                    if !ch.is_alphanumeric() && ch != '-' && ch != '_' {
                        break;
                    }
                    self.pos += 1;
                }
                let ident = self.input[start..self.pos].to_string();
                Ok(Some(CssToken { kind: CssTokenKind::Ident, value: ident }))
            },
            _ if ch.is_numeric() => {
                let start = self.pos;
                while self.pos < self.input.len() {
                    let ch = self.input.chars().nth(self.pos).unwrap();
                    if !ch.is_numeric() && ch != '.' && ch != '%' && ch != 'e' && ch != 'E' {
                        break;
                    }
                    self.pos += 1;
                }
                let number = self.input[start..self.pos].to_string();
                if number.ends_with('%') {
                    Ok(Some(CssToken { kind: CssTokenKind::Percentage, value: number }))
                } else {
                    Ok(Some(CssToken { kind: CssTokenKind::Number, value: number }))
                }
            },
            _ => {
                self.pos += 1;
                Ok(Some(CssToken { kind: CssTokenKind::Delim, value: ch.to_string() }))
            },
        }
    }

    fn parse_rules(&mut self, tokens: &[CssToken]) -> Result<Vec<CssRule>, EngineError> {
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
                                    let value = self.parse_value(&tokens, &mut i)?;
                                    properties.insert(property.clone(), value);
                                    
                                    self.property_counts.entry(property.clone())
                                        .and_modify(|c| *c += 1)
                                        .or_insert(1);
                                }
                            }
                        }
                        i += 1;
                    }
                    
                    rules.push(CssRule {
                        selector: selector.clone(),
                        properties,
                    });
                    
                    self.selector_counts.entry(selector.clone())
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                }
            }
            i += 1;
        }
        
        Ok(rules)
    }

    fn parse_value(&mut self, tokens: &[CssToken], pos: &mut usize) -> Result<CssValue, EngineError> {
        if *pos >= tokens.len() {
            return Ok(CssValue::Auto);
        }
        
        let token = &tokens[*pos];
        
        match token.kind {
            CssTokenKind::Number => {
                if let Ok(num) = token.value.parse::<f32>() {
                    Ok(CssValue::Length(num))
                } else {
                    self.handle_invalid_value(&token.value)
                }
            },
            CssTokenKind::Percentage => {
                if let Some(percentage) = token.value.strip_suffix('%') {
                    if let Ok(num) = percentage.parse::<f32>() {
                        Ok(CssValue::Percentage(num))
                    } else {
                        self.handle_invalid_value(&token.value)
                    }
                } else {
                    self.handle_invalid_value(&token.value)
                }
            },
            CssTokenKind::Color => {
                if let Some(color) = self.parse_color(&token.value) {
                            Ok(CssValue::Color(color.0, color.1, color.2))
                } else {
                    self.handle_invalid_value(&token.value)
                }
            },
            CssTokenKind::Ident => {
                Ok(CssValue::Keyword(token.value.clone()))
            },
            CssTokenKind::String => {
                Ok(CssValue::Keyword(token.value.clone()))
            },
            _ => {
                self.handle_invalid_value(&token.value)
            },
        }
    }

    fn parse_color(&self, color_str: &str) -> Option<(u8, u8, u8)> {
        let color = color_str.trim_start_matches('#');
        if color.len() == 6 {
            let r = u8::from_str_radix(&color[0..2], 16).ok()?;
            let g = u8::from_str_radix(&color[2..4], 16).ok()?;
            let b = u8::from_str_radix(&color[4..6], 16).ok()?;
            Some(WebSafePalette::snap_to_web_safe(r, g, b))
        } else {
            self.parse_named_color(&color.to_lowercase())
        }
    }

    fn parse_named_color(&self, color_name: &str) -> Option<(u8, u8, u8)> {
        match color_name {
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

    fn handle_invalid_value(&self, value: &str) -> Result<CssValue, EngineError> {
        match self.mode {
            CssParsingMode::Strict => {
                Err(EngineError::ParseError(format!("Invalid CSS value: {}", value)))
            },
            CssParsingMode::Quirks1996 => {
                if self.error_recovery {
                    Ok(CssValue::Auto)
                } else {
                    Err(EngineError::ParseError(format!("Invalid CSS value: {}", value)))
                }
            },
            CssParsingMode::Lenient => {
                Ok(CssValue::Auto)
            },
        }
    }

    pub fn parse_with_recovery(&mut self, css: &str) -> Result<Vec<CssRule>, EngineError> {
        match self.parse(css) {
            Ok(rules) => Ok(rules),
            Err(e) => {
                if self.error_recovery {
                    self.recover_from_error(css, e)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn recover_from_error(&mut self, css: &str, error: EngineError) -> Result<Vec<CssRule>, EngineError> {
        let mut recovered_css = css.to_string();
        
        recovered_css = self.fix_invalid_selectors(&recovered_css);
        recovered_css = self.fix_invalid_properties(&recovered_css);
        recovered_css = self.fix_unclosed_blocks(&recovered_css);
        
        self.parse(&recovered_css)
    }

    fn fix_invalid_selectors(&self, css: &str) -> String {
        let mut fixed = css.to_string();
        
        let re = regex::Regex::new(r"([^{]+)\s*\{").unwrap();
        fixed = re.replace_all(&fixed, |caps: &regex::Captures| {
            let selector = &caps[1];
            if selector.trim().is_empty() {
                String::new()
            } else {
                caps[0].to_string()
            }
        }).to_string();
        
        fixed
    }

    fn fix_invalid_properties(&self, css: &str) -> String {
        let mut fixed = css.to_string();
        
        let re = regex::Regex::new(r"(\w+)\s*:\s*([^;]+);").unwrap();
        fixed = re.replace_all(&fixed, |caps: &regex::Captures| {
            let property = &caps[1];
            let value = &caps[2];
            
            if self.is_valid_property(property) && self.is_valid_value(value) {
                caps[0].to_string()
            } else {
                String::new()
            }
        }).to_string();
        
        fixed
    }

    fn fix_unclosed_blocks(&self, css: &str) -> String {
        let mut fixed = css.to_string();
        
        let open_count = fixed.matches('{').count();
        let close_count = fixed.matches('}').count();
        
        if open_count > close_count {
            for _ in 0..(open_count - close_count) {
                fixed.push('}');
            }
        }
        
        fixed
    }

    fn is_valid_property(&self, property: &str) -> bool {
        let css1_properties = [
            "color", "background-color", "background-image", "background-repeat", "background-attachment", "background-position", "background",
            "font-family", "font-style", "font-variant", "font-weight", "font-size", "font",
            "word-spacing", "letter-spacing", "text-decoration", "vertical-align", "text-transform", "text-align", "text-indent", "line-height",
            "margin", "margin-top", "margin-right", "margin-bottom", "margin-left",
            "padding", "padding-top", "padding-right", "padding-bottom", "padding-left",
            "border", "border-width", "border-color", "border-style", "border-top", "border-right", "border-bottom", "border-left",
            "width", "height", "float", "clear",
            "display", "white-space", "list-style", "list-style-type", "list-style-image", "list-style-position",
        ];
        
        css1_properties.contains(&property.to_lowercase().as_str())
    }

    fn is_valid_value(&self, value: &str) -> bool {
        value.trim().len() > 0 && !value.contains('\n') && !value.contains('\r')
    }

    pub fn get_parse_stats(&self) -> CssParseStats {
        CssParseStats {
            total_rules: 0,
            total_properties: self.property_counts.len(),
            total_selectors: self.selector_counts.len(),
            invalid_rules: self.invalid_rules.len(),
            parse_time: self.parse_start_time.map_or(0, |start| start.elapsed().as_millis() as u64),
        }
    }

    pub fn get_property_counts(&self) -> HashMap<String, usize> {
        self.property_counts.clone()
    }

    pub fn get_selector_counts(&self) -> HashMap<String, usize> {
        self.selector_counts.clone()
    }

    pub fn get_invalid_rules(&self) -> Vec<String> {
        self.invalid_rules.clone()
    }

    pub fn set_error_recovery(&mut self, recovery: bool) {
        self.error_recovery = recovery;
    }

    pub fn set_max_nesting_level(&mut self, level: usize) {
        self.max_nesting_level = level;
    }

    pub fn set_mode(&mut self, mode: CssParsingMode) {
        self.mode = mode;
    }

    pub fn validate_css(&self, css: &str) -> Result<(), EngineError> {
        let tokens = {
            let mut parser = CssParser::new(CssParsingMode::Strict);
            parser.tokenize()?
        };
        
        self.validate_tokens(&tokens)
    }

    fn validate_tokens(&self, tokens: &[CssToken]) -> Result<(), EngineError> {
        for token in tokens {
            match token.kind {
                CssTokenKind::Ident => {
                    if !self.is_valid_property(&token.value) {
                        return Err(EngineError::ParseError(format!("Invalid CSS property: {}", token.value)));
                    }
                },
                CssTokenKind::Color => {
                    if self.parse_color(&token.value).is_none() {
                        return Err(EngineError::ParseError(format!("Invalid color value: {}", token.value)));
                    }
                },
                _ => {}
            }
        }
        
        Ok(())
    }

    pub fn optimize_for_performance(&mut self) {
        self.error_recovery = false;
        self.max_nesting_level = 100;
        self.mode = CssParsingMode::Strict;
    }

    pub fn optimize_for_compatibility(&mut self) {
        self.error_recovery = true;
        self.max_nesting_level = 50;
        self.mode = CssParsingMode::Quirks1996;
    }

    pub fn parse_inline_styles(&mut self, style_attr: &str) -> Result<HashMap<String, CssValue>, EngineError> {
        let css = format!("dummy {{ {} }}", style_attr);
        let rules = self.parse(&css)?;
        
        if let Some(rule) = rules.first() {
            Ok(rule.properties.clone())
        } else {
            Ok(HashMap::new())
        }
    }

    pub fn merge_rules(&self, rules1: &[CssRule], rules2: &[CssRule]) -> Vec<CssRule> {
        let mut merged = rules1.to_vec();
        
        for rule in rules2 {
            if let Some(existing) = merged.iter_mut().find(|r| r.selector == rule.selector) {
                for (prop, value) in &rule.properties {
                    existing.properties.insert(prop.clone(), value.clone());
                }
            } else {
                merged.push(rule.clone());
            }
        }
        
        merged
    }

    pub fn filter_css1_properties(&self, rules: &[CssRule]) -> Vec<CssRule> {
        rules.iter()
            .map(|rule| {
                let filtered_properties: HashMap<String, CssValue> = rule.properties.iter()
                    .filter(|(prop, _)| self.is_valid_property(prop))
                    .map(|(prop, value)| (prop.clone(), value.clone()))
                    .collect();
                
                CssRule {
                    selector: rule.selector.clone(),
                    properties: filtered_properties,
                }
            })
            .collect()
    }

    pub fn apply_fallback_styles(&self, rules: &[CssRule]) -> Vec<CssRule> {
        rules.iter()
            .map(|rule| {
                let mut properties = rule.properties.clone();
                
                if properties.contains_key("font-family") {
                    properties.entry("font-family".to_string())
                        .or_insert(CssValue::Keyword("Times New Roman".to_string()));
                }
                
                if properties.contains_key("color") {
                    properties.entry("color".to_string())
                        .or_insert(CssValue::Color(0, 0, 0));
                }
                
                CssRule {
                    selector: rule.selector.clone(),
                    properties,
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct CssParseStats {
    pub total_rules: usize,
    pub total_properties: usize,
    pub total_selectors: usize,
    pub invalid_rules: usize,
    pub parse_time: u64,
}

impl Default for CssParser {
    fn default() -> Self {
        Self::new(CssParsingMode::Quirks1996)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_css() {
        let mut parser = CssParser::new(CssParsingMode::Quirks1996);
        let css = "body { color: red; font-family: Arial; }";
        
        let result = parser.parse(css);
        assert!(result.is_ok());
        
        let rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].properties.len(), 2);
    }

    #[test]
    fn test_parse_invalid_properties() {
        let mut parser = CssParser::new(CssParsingMode::Quirks1996);
        let css = "body { invalid-property: value; color: red; }";
        
        let result = parser.parse_with_recovery(css);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_unclosed_blocks() {
        let mut parser = CssParser::new(CssParsingMode::Quirks1996);
        let css = "body { color: red; font-family: Arial; ";
        
        let result = parser.parse_with_recovery(css);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_color_values() {
        let mut parser = CssParser::new(CssParsingMode::Quirks1996);
        let css = "body { color: #ff0000; background-color: red; }";
        
        let result = parser.parse(css);
        assert!(result.is_ok());
    }

    #[test]
    fn test_filter_css1_properties() {
        let mut parser = CssParser::new(CssParsingMode::Quirks1996);
        let css = "body { color: red; display: flex; position: absolute; }";
        
        let rules = parser.parse(css).unwrap();
        let filtered = parser.filter_css1_properties(&rules);
        
        assert_eq!(filtered[0].properties.len(), 1); // Only color should remain
        assert!(filtered[0].properties.contains_key("color"));
    }

    #[test]
    fn test_parse_inline_styles() {
        let mut parser = CssParser::new(CssParsingMode::Quirks1996);
        let style = "color: red; font-family: Arial;";
        
        let result = parser.parse_inline_styles(style);
        assert!(result.is_ok());
        
        let properties = result.unwrap();
        assert_eq!(properties.len(), 2);
    }

    #[test]
    fn test_merge_rules() {
        let mut parser = CssParser::new(CssParsingMode::Quirks1996);
        let css1 = "body { color: red; }";
        let css2 = "body { font-family: Arial; }";
        
        let rules1 = parser.parse(css1).unwrap();
        let rules2 = parser.parse(css2).unwrap();
        
        let merged = parser.merge_rules(&rules1, &rules2);
        assert_eq!(merged[0].properties.len(), 2);
    }
}