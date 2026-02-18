use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::str::Chars;

use crate::engine::{DomNode, Token, TokenType, HtmlTokenizer, DomBuilder, ParsingMode, QuirkProfile, EngineError};

#[derive(Debug, Clone)]
pub struct HtmlParser {
    mode: ParsingMode,
    quirks_mode: bool,
    open_tags: Vec<String>,
    tag_stack: Vec<String>,
    current_position: usize,
    input: String,
    tokenizer: HtmlTokenizer,
    builder: DomBuilder,
    error_recovery: bool,
    tag_counts: HashMap<String, usize>,
    nesting_depth: usize,
    max_nesting_depth: usize,
    parse_start_time: Option<Instant>,
}

impl HtmlParser {
    pub fn new(mode: ParsingMode, quirks_mode: bool) -> Self {
        HtmlParser {
            mode,
            quirks_mode,
            open_tags: Vec::new(),
            tag_stack: Vec::new(),
            current_position: 0,
            input: String::new(),
            tokenizer: HtmlTokenizer::new(String::new(), crate::engine::RenderingMode::Netscape3),
            builder: DomBuilder::new(crate::engine::RenderingMode::Netscape3),
            error_recovery: true,
            tag_counts: HashMap::new(),
            nesting_depth: 0,
            max_nesting_depth: 100,
            parse_start_time: None,
        }
    }

    pub fn parse(&mut self, html: &str) -> Result<DomNode, EngineError> {
        self.parse_start_time = Some(Instant::now());
        
        // Performance optimization: reuse tokenizer and builder
        self.tokenizer = HtmlTokenizer::new(html.to_string(), crate::engine::RenderingMode::Netscape3);
        self.builder = DomBuilder::new(crate::engine::RenderingMode::Netscape3);
        
        // Clear and reuse collections instead of creating new ones
        self.open_tags.clear();
        self.tag_stack.clear();
        self.tag_counts.clear();
        self.nesting_depth = 0;
        
        let tokens = self.tokenize()?;
        let dom = self.build_dom(&tokens)?;
        
        Ok(dom)
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, EngineError> {
        let tokens = self.tokenizer.tokenize();
        
        if self.error_recovery {
            self.validate_tokens(&tokens)?;
        }
        
        Ok(tokens)
    }

    fn validate_tokens(&mut self, tokens: &[Token]) -> Result<(), EngineError> {
        for token in tokens {
            match &token.kind {
                TokenType::StartTag => {
                    self.tag_counts.entry(token.value.clone()).and_modify(|c| *c += 1).or_insert(1);
                    
                    if self.nesting_depth > self.max_nesting_depth {
                        return Err(EngineError::ParseError("Maximum nesting depth exceeded".to_string()));
                    }
                },
                TokenType::EndTag => {
                    if let Some(count) = self.tag_counts.get_mut(&token.value) {
                        if *count == 0 {
                            return Err(EngineError::ParseError(format!("Unmatched end tag: {}", token.value)));
                        }
                        *count -= 1;
                    }
                },
                _ => {}
            }
        }
        
        Ok(())
    }

    fn build_dom(&mut self, tokens: &[Token]) -> Result<DomNode, EngineError> {
        let dom = self.builder.build(tokens);
        
        if self.quirks_mode {
            self.apply_quirks_mode_fixes(&dom)?;
        }
        
        Ok(dom)
    }

    fn apply_quirks_mode_fixes(&self, dom: &DomNode) -> Result<(), EngineError> {
        match dom {
            DomNode::Element { tag, children, .. } => {
                if tag == "table" {
                    self.fix_table_structure(children)?;
                }
                
                for child in children {
                    self.apply_quirks_mode_fixes(child)?;
                }
            },
            _ => {}
        }
        
        Ok(())
    }

    fn fix_table_structure(&self, children: &[DomNode]) -> Result<(), EngineError> {
        for child in children {
            if let DomNode::Element { tag, .. } = child {
                if tag == "tr" {
                    self.validate_table_row(child)?;
                }
            }
        }
        Ok(())
    }

    fn validate_table_row(&self, row: &DomNode) -> Result<(), EngineError> {
        if let DomNode::Element { children, .. } = row {
            for cell in children {
                if let DomNode::Element { tag, .. } = cell {
                    if tag != "td" && tag != "th" {
                        return Err(EngineError::ParseError(format!("Invalid table cell tag: {}", tag)));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn parse_fragment(&mut self, html: &str) -> Result<DomNode, EngineError> {
        let tokens = {
            let mut tokenizer = HtmlTokenizer::new(html.to_string(), crate::engine::RenderingMode::Netscape3);
            tokenizer.tokenize()
        };
        
        Ok(self.builder.build(&tokens))
    }

    pub fn parse_with_recovery(&mut self, html: &str) -> Result<DomNode, EngineError> {
        match self.parse(html) {
            Ok(dom) => Ok(dom),
            Err(e) => {
                if self.error_recovery {
                    self.recover_from_error(html, e)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn recover_from_error(&mut self, html: &str, error: EngineError) -> Result<DomNode, EngineError> {
        let mut recovered_html = html.to_string();
        
        recovered_html = self.fix_unclosed_tags(&recovered_html);
        recovered_html = self.fix_unquoted_attributes(&recovered_html);
        recovered_html = self.fix_mismatched_tags(&recovered_html);
        
        self.parse(&recovered_html)
    }

    fn fix_unclosed_tags(&self, html: &str) -> String {
        let mut fixed = html.to_string();
        
        let unclosed_tags = ["p", "li", "td", "th", "tr"];
        
        for tag in unclosed_tags {
            let open_tag = format!("<{}", tag);
            let close_tag = format!("</{}>", tag);
            
            let mut open_count = 0;
            let mut pos = 0;
            
            while let Some(start) = fixed[pos..].find(&open_tag) {
                let start = pos + start;
                if !fixed[start..].starts_with(&format!("<{} ", tag)) && !fixed[start..].starts_with(&format!("<{}/", tag)) {
                    open_count += 1;
                }
                pos = start + 1;
            }
            
            let close_count = fixed.matches(&close_tag).count();
            
            if open_count > close_count {
                for _ in 0..(open_count - close_count) {
                    fixed.push_str(&close_tag);
                }
            }
        }
        
        fixed
    }

    fn fix_unquoted_attributes(&self, html: &str) -> String {
        let mut fixed = html.to_string();
        
        let re = regex::Regex::new(r#"(\w+)=([^"'\s>]+)(?=\s|>|$)"#).unwrap();
        fixed = re.replace_all(&fixed, r#"$1="$2""#).to_string();
        
        fixed
    }

    fn fix_mismatched_tags(&self, html: &str) -> String {
        let mut fixed = html.to_string();
        
        let mut stack = Vec::new();
        let mut pos = 0;
        
        while let Some(start) = fixed[pos..].find('<') {
            let start = pos + start;
            let end = if let Some(end_pos) = fixed[start..].find('>') {
                start + end_pos + 1
            } else {
                break;
            };
            
            let tag = &fixed[start..end];
            
            if tag.starts_with("</") {
                if let Some(expected) = stack.pop() {
                    let expected_close = format!("</{}>", expected);
                    if !tag.starts_with(&expected_close) {
                        fixed.insert_str(start, &expected_close);
                    }
                }
            } else if !tag.ends_with("/>") {
                if let Some(tag_name) = self.extract_tag_name(tag) {
                    stack.push(tag_name);
                }
            }
            
            pos = end;
        }
        
        while let Some(tag) = stack.pop() {
            fixed.push_str(&format!("</{}>", tag));
        }
        
        fixed
    }

    fn extract_tag_name(&self, tag: &str) -> Option<String> {
        let tag = tag.trim_start_matches('<');
        let tag = tag.split_whitespace().next()?;
        Some(tag.to_string())
    }

    pub fn get_parse_stats(&self) -> ParseStats {
        ParseStats {
            total_tokens: 0,
            tag_count: self.tag_counts.len(),
            nesting_depth: self.nesting_depth,
            parse_time: self.parse_start_time.map_or(0, |start| start.elapsed().as_millis() as u64),
        }
    }

    pub fn set_quirks_mode(&mut self, quirks_mode: bool) {
        self.quirks_mode = quirks_mode;
    }

    pub fn set_error_recovery(&mut self, recovery: bool) {
        self.error_recovery = recovery;
    }

    pub fn set_max_nesting_depth(&mut self, depth: usize) {
        self.max_nesting_depth = depth;
    }

    pub fn get_tag_counts(&self) -> HashMap<String, usize> {
        self.tag_counts.clone()
    }

    pub fn validate_html(&mut self, html: &str) -> Result<(), EngineError> {
        let tokens = {
            let mut tokenizer = HtmlTokenizer::new(html.to_string(), crate::engine::RenderingMode::Netscape3);
            tokenizer.tokenize()
        };
        
        self.validate_tokens(&tokens)
    }

    pub fn optimize_for_performance(&mut self) {
        self.error_recovery = false;
        self.max_nesting_depth = 200;
    }

    pub fn optimize_for_compatibility(&mut self) {
        self.error_recovery = true;
        self.max_nesting_depth = 100;
        self.quirks_mode = true;
    }
}

#[derive(Debug, Clone)]
pub struct ParseStats {
    pub total_tokens: usize,
    pub tag_count: usize,
    pub nesting_depth: usize,
    pub parse_time: u64,
}

impl Default for HtmlParser {
    fn default() -> Self {
        Self::new(ParsingMode::Quirks1996, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_html() {
        let mut parser = HtmlParser::new(ParsingMode::Quirks1996, true);
        let html = "<html><body><p>Hello World</p></body></html>";
        
        let result = parser.parse(html);
        assert!(result.is_ok());
        
        let dom = result.unwrap();
        assert!(matches!(dom, DomNode::Element { .. }));
    }

    #[test]
    fn test_parse_unclosed_tags() {
        let mut parser = HtmlParser::new(ParsingMode::Quirks1996, true);
        let html = "<p>Paragraph 1<p>Paragraph 2";
        
        let result = parser.parse_with_recovery(html);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_unquoted_attributes() {
        let mut parser = HtmlParser::new(ParsingMode::Quirks1996, true);
        let html = r#"<img src=image.gif width=100>"#;
        
        let result = parser.parse_with_recovery(html);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_table_structure() {
        let mut parser = HtmlParser::new(ParsingMode::Quirks1996, true);
        let html = "<table><tr><td>Cell 1</td><td>Cell 2</td></tr></table>";
        
        let result = parser.parse(html);
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_recovery() {
        let mut parser = HtmlParser::new(ParsingMode::Quirks1996, true);
        let html = "<html><body><p>Unclosed paragraph";
        
        let result = parser.parse_with_recovery(html);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nesting_depth_limit() {
        let mut parser = HtmlParser::new(ParsingMode::Quirks1996, true);
        parser.set_max_nesting_depth(5);
        
        let html = "<div>".repeat(10) + "Content" + "</div>".repeat(10);
        
        let result = parser.parse(html);
        assert!(result.is_err());
    }
}