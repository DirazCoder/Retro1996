use std::collections::HashMap;
use std::time::Duration;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub text: String,
    pub position: usize,
    pub line_number: usize,
    pub context_before: String,
    pub context_after: String,
    pub match_length: usize,
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub wrap_around: bool,
    pub highlight_all: bool,
    pub search_from_top: bool,
    pub match_diacritics: bool,  // Whether to match accented characters exactly
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions {
            case_sensitive: false,
            whole_word: false,
            wrap_around: true,
            highlight_all: false,
            search_from_top: true,
            match_diacritics: true,
        }
    }
}

pub struct SearchSession {
    pub query: String,
    pub options: SearchOptions,
    pub results: Vec<SearchResult>,
    pub current_result_index: Option<usize>,
    pub search_time: Duration,
    pub total_results: usize,
    pub is_searching: bool,
}

impl SearchSession {
    pub fn new(query: String, options: SearchOptions) -> Self {
        SearchSession {
            query,
            options,
            results: Vec::new(),
            current_result_index: None,
            search_time: Duration::new(0, 0),
            total_results: 0,
            is_searching: false,
        }
    }
}

pub struct PageSearcher {
    pub current_session: Option<SearchSession>,
    pub search_history: Vec<String>,
    pub max_history_items: usize,
    pub last_search_time: Option<std::time::Instant>,
}

impl PageSearcher {
    pub fn new() -> Self {
        PageSearcher {
            current_session: None,
            search_history: Vec::new(),
            max_history_items: 10,
            last_search_time: None,
        }
    }

    pub fn search_in_text(&mut self, text: &str, query: &str, options: SearchOptions) -> Vec<SearchResult> {
        self.last_search_time = Some(std::time::Instant::now());
        
        let start_time = std::time::Instant::now();
        
        // Normalize text and query based on options
        let search_text = if options.case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };
        
        let search_query = if options.case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        
        let mut results = Vec::new();
        let mut position = 0;
        
        while let Some(found_pos) = search_text[position..].find(&search_query) {
            let actual_pos = position + found_pos;
            
            // If whole word option is enabled, check boundaries
            if options.whole_word {
                let start_boundary = if actual_pos == 0 { true } else { 
                    !search_text.chars().nth(actual_pos - 1).unwrap_or(' ').is_alphanumeric()
                };
                let end_boundary = if actual_pos + search_query.len() >= search_text.len() { true } else {
                    !search_text.chars().nth(actual_pos + search_query.len()).unwrap_or(' ').is_alphanumeric()
                };
                
                if !start_boundary || !end_boundary {
                    position = actual_pos + 1;
                    continue;
                }
            }
            
            // Calculate line number
            let line_number = text[..actual_pos].chars().filter(|c| *c == '\n').count() + 1;
            
            // Get context
            let context_before = self.get_context(&text, actual_pos.saturating_sub(30), actual_pos);
            let context_after = self.get_context(&text, actual_pos + search_query.len(), actual_pos + search_query.len() + 30);
            
            let result = SearchResult {
                text: text[actual_pos..actual_pos + search_query.len()].to_string(),
                position: actual_pos,
                line_number,
                context_before,
                context_after,
                match_length: search_query.len(),
            };
            
            results.push(result);
            position = actual_pos + 1;
            
            // Prevent infinite loops
            if position >= search_text.len() {
                break;
            }
        }
        
        let search_time = start_time.elapsed();
        
        // Update search history
        if !query.is_empty() {
            self.update_search_history(query.to_string());
        }
        
        // Create a new session
        self.current_session = Some(SearchSession {
            query: query.to_string(),
            options,
            results: results.clone(),
            current_result_index: if results.is_empty() { None } else { Some(0) },
            search_time,
            total_results: results.len(),
            is_searching: false,
        });
        
        results
    }

    fn get_context(&self, text: &str, start: usize, end: usize) -> String {
        let start = start.min(text.len());
        let end = end.min(text.len());
        
        if start >= end {
            return String::new();
        }
        
        text[start..end].to_string()
    }

    fn update_search_history(&mut self, query: String) {
        // Remove duplicates
        self.search_history.retain(|q| q != &query);
        
        // Add to front
        self.search_history.insert(0, query);
        
        // Limit history size
        if self.search_history.len() > self.max_history_items {
            self.search_history.truncate(self.max_history_items);
        }
    }

    pub fn find_next(&mut self, text: &str) -> Option<SearchResult> {
        if let Some(session) = &mut self.current_session {
            if session.results.is_empty() {
                return None;
            }
            
            // Move to next result
            let current_idx = session.current_result_index.unwrap_or(0);
            let next_idx = (current_idx + 1) % session.results.len();
            session.current_result_index = Some(next_idx);
            
            Some(session.results[next_idx].clone())
        } else {
            None
        }
    }

    pub fn find_previous(&mut self, text: &str) -> Option<SearchResult> {
        if let Some(session) = &mut self.current_session {
            if session.results.is_empty() {
                return None;
            }
            
            // Move to previous result
            let current_idx = session.current_result_index.unwrap_or(0);
            let prev_idx = if current_idx == 0 { session.results.len() - 1 } else { current_idx - 1 };
            session.current_result_index = Some(prev_idx);
            
            Some(session.results[prev_idx].clone())
        } else {
            None
        }
    }

    pub fn get_current_match_position(&self) -> Option<(usize, usize)> {
        if let Some(session) = &self.current_session {
            if let Some(idx) = session.current_result_index {
                if idx < session.results.len() {
                    let result = &session.results[idx];
                    return Some((result.position, result.match_length));
                }
            }
        }
        None
    }

    pub fn get_search_stats(&self) -> Option<SearchStats> {
        if let Some(session) = &self.current_session {
            Some(SearchStats {
                total_results: session.total_results,
                current_result: session.current_result_index.map(|idx| idx + 1),
                search_time_ms: session.search_time.as_millis() as u64,
                query: session.query.clone(),
            })
        } else {
            None
        }
    }

    pub fn clear_search(&mut self) {
        self.current_session = None;
    }

    pub fn get_search_history(&self) -> Vec<String> {
        self.search_history.clone()
    }

    pub fn clear_search_history(&mut self) {
        self.search_history.clear();
    }

    pub fn highlight_matches(&self, text: &str) -> Vec<(usize, usize)> {
        // Return vector of (start, length) tuples for highlighting
        if let Some(session) = &self.current_session {
            session.results.iter()
                .map(|result| (result.position, result.match_length))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn search_in_html(&mut self, html: &str, query: &str, options: SearchOptions) -> Vec<SearchResult> {
        // Strip HTML tags for searching but preserve positions
        let (stripped_text, position_map) = self.strip_html_preserve_positions(html);
        
        // Perform search on stripped text
        let mut raw_results = self.search_in_text(&stripped_text, query, options.clone());
        
        // Map positions back to original HTML
        for result in &mut raw_results {
            // Convert position from stripped text to original HTML
            if let Some(original_pos) = position_map.get(&result.position) {
                result.position = *original_pos;
            }
        }
        
        raw_results
    }

    fn strip_html_preserve_positions(&self, html: &str) -> (String, HashMap<usize, usize>) {
        let mut stripped = String::new();
        let mut position_map = HashMap::new(); // Maps stripped position to original position
        let mut original_pos = 0;
        let mut inside_tag = false;
        
        for c in html.chars() {
            if c == '<' {
                inside_tag = true;
            } else if c == '>' {
                inside_tag = false;
                original_pos += 1;
                continue;
            }
            
            if !inside_tag {
                position_map.insert(stripped.len(), original_pos);
                stripped.push(c);
            }
            
            original_pos += c.len_utf8();
        }
        
        (stripped, position_map)
    }

    pub fn get_next_match_context(&self, text: &str, current_pos: usize) -> Option<SearchResult> {
        if let Some(session) = &self.current_session {
            // Find the next match after current position
            for result in &session.results {
                if result.position > current_pos {
                    return Some(result.clone());
                }
            }
            
            // If wrap around is enabled, return the first match
            if session.options.wrap_around {
                if let Some(first_result) = session.results.first() {
                    return Some(first_result.clone());
                }
            }
        }
        None
    }

    pub fn get_prev_match_context(&self, text: &str, current_pos: usize) -> Option<SearchResult> {
        if let Some(session) = &self.current_session {
            // Find the previous match before current position
            for result in session.results.iter().rev() {
                if result.position < current_pos {
                    return Some(result.clone());
                }
            }
            
            // If wrap around is enabled, return the last match
            if session.options.wrap_around {
                if let Some(last_result) = session.results.last() {
                    return Some(last_result.clone());
                }
            }
        }
        None
    }

    pub fn replace_text(&self, text: &str, old_text: &str, new_text: &str, options: &SearchOptions) -> String {
        if options.case_sensitive {
            text.replace(old_text, new_text)
        } else {
            // For case insensitive, we need to find and replace preserving original case
            let mut result = String::new();
            let mut last_end = 0;
            
            let search_text = text.to_lowercase();
            let search_old = old_text.to_lowercase();
            
            let mut start = 0;
            while let Some(pos) = search_text[start..].find(&search_old) {
                let actual_pos = start + pos;
                
                // Append text before match
                result.push_str(&text[last_end..actual_pos]);
                
                // Append replacement
                result.push_str(new_text);
                
                last_end = actual_pos + old_text.len();
                start = last_end;
            }
            
            // Append remaining text
            result.push_str(&text[last_end..]);
            
            result
        }
    }

    pub fn replace_all(&self, text: &str, old_text: &str, new_text: &str, options: &SearchOptions) -> String {
        if options.case_sensitive {
            text.replace(old_text, new_text)
        } else {
            self.replace_text(text, old_text, new_text, options)
        }
    }
}

#[derive(Debug)]
pub struct SearchStats {
    pub total_results: usize,
    pub current_result: Option<usize>,
    pub search_time_ms: u64,
    pub query: String,
}

// Search dialog state for UI integration
#[derive(Debug)]
pub struct SearchDialogState {
    pub is_visible: bool,
    pub search_input: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub highlight_all: bool,
    pub results_count: usize,
    pub current_result: Option<usize>,
    pub search_error: Option<String>,
}

impl SearchDialogState {
    pub fn new() -> Self {
        SearchDialogState {
            is_visible: false,
            search_input: String::new(),
            case_sensitive: false,
            whole_word: false,
            highlight_all: false,
            results_count: 0,
            current_result: None,
            search_error: None,
        }
    }

    pub fn reset(&mut self) {
        self.search_input.clear();
        self.results_count = 0;
        self.current_result = None;
        self.search_error = None;
    }

    pub fn show(&mut self) {
        self.is_visible = true;
        self.reset();
    }

    pub fn hide(&mut self) {
        self.is_visible = false;
    }
}

// Global search manager for the browser
pub struct GlobalSearchManager {
    pub page_searcher: PageSearcher,
    pub search_dialog: SearchDialogState,
    pub search_shortcut_enabled: bool,
}

impl GlobalSearchManager {
    pub fn new() -> Self {
        GlobalSearchManager {
            page_searcher: PageSearcher::new(),
            search_dialog: SearchDialogState::new(),
            search_shortcut_enabled: true,
        }
    }

    pub fn handle_search_shortcut(&mut self) {
        self.search_dialog.show();
    }

    pub fn handle_find_next(&mut self, text: &str) {
        if !self.search_dialog.search_input.is_empty() {
            let options = SearchOptions {
                case_sensitive: self.search_dialog.case_sensitive,
                whole_word: self.search_dialog.whole_word,
                wrap_around: true,
                highlight_all: self.search_dialog.highlight_all,
                search_from_top: false,
                match_diacritics: true,
            };
            
            // Perform search if not already searched
            if self.page_searcher.current_session.is_none() || 
               self.page_searcher.current_session.as_ref().unwrap().query != self.search_dialog.search_input {
                let _results = self.page_searcher.search_in_text(text, &self.search_dialog.search_input, options);
            }
            
            if let Some(result) = self.page_searcher.find_next(text) {
                self.search_dialog.current_result = Some(result.position);
                self.search_dialog.results_count = self.page_searcher.current_session.as_ref().map(|s| s.total_results).unwrap_or(0);
            }
        }
    }

    pub fn handle_find_previous(&mut self, text: &str) {
        if !self.search_dialog.search_input.is_empty() {
            if let Some(result) = self.page_searcher.find_previous(text) {
                self.search_dialog.current_result = Some(result.position);
            }
        }
    }

    pub fn perform_search(&mut self, text: &str) {
        if self.search_dialog.search_input.is_empty() {
            self.search_dialog.results_count = 0;
            self.search_dialog.current_result = None;
            return;
        }
        
        let options = SearchOptions {
            case_sensitive: self.search_dialog.case_sensitive,
            whole_word: self.search_dialog.whole_word,
            wrap_around: true,
            highlight_all: self.search_dialog.highlight_all,
            search_from_top: true,
            match_diacritics: true,
        };
        
        let results = self.page_searcher.search_in_text(text, &self.search_dialog.search_input, options);
        self.search_dialog.results_count = results.len();
        self.search_dialog.current_result = results.first().map(|r| r.position);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_search() {
        let mut searcher = PageSearcher::new();
        let results = searcher.search_in_text("Hello world, hello universe", "hello", SearchOptions::default());
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_case_sensitive_search() {
        let mut searcher = PageSearcher::new();
        let mut options = SearchOptions::default();
        options.case_sensitive = true;
        let results = searcher.search_in_text("Hello world, hello universe", "Hello", options);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_whole_word_search() {
        let mut searcher = PageSearcher::new();
        let mut options = SearchOptions::default();
        options.whole_word = true;
        let results = searcher.search_in_text("The helper helped hello", "hello", options);
        assert_eq!(results.len(), 1); // Should match "hello" but not "hello" in "helper"
    }
}