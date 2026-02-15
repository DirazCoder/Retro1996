use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::time::Instant;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visit_count: u32,
    pub last_visited: DateTime<Utc>,
    pub first_visited: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl Default for HistoryEntry {
    fn default() -> Self {
        HistoryEntry {
            url: String::new(),
            title: String::new(),
            visit_count: 1,
            last_visited: Utc::now(),
            first_visited: Utc::now(),
            tags: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HistoryManager {
    pub entries: VecDeque<HistoryEntry>,
    pub max_entries: usize,
    pub history_file_path: String,
}

impl HistoryManager {
    pub fn new(history_file_path: String, max_entries: usize) -> Self {
        let mut history = HistoryManager {
            entries: VecDeque::new(),
            max_entries,
            history_file_path,
        };

        // Load existing history from file if it exists
        history.load_from_file();

        history
    }

    pub fn add_entry(&mut self, url: String, title: String, timestamp: Instant) {
        // Check if URL already exists in history
        if let Some(pos) = self.entries.iter().position(|entry| entry.url == url) {
            // Update existing entry
            let mut entry = self.entries.remove(pos).unwrap();
            entry.visit_count += 1;
            entry.last_visited = Utc::now();
            if !title.is_empty() && entry.title.is_empty() {
                entry.title = title;
            }
            self.entries.push_front(entry);
        } else {
            // Add new entry
            let entry = HistoryEntry {
                url,
                title: if title.is_empty() { "Untitled".to_string() } else { title },
                visit_count: 1,
                last_visited: Utc::now(),
                first_visited: Utc::now(),
                tags: Vec::new(),
            };
            self.entries.push_front(entry);
        }

        // Trim to max entries if needed
        while self.entries.len() > self.max_entries {
            self.entries.pop_back();
        }

        // Save to file
        let _ = self.save_to_file();
    }

    pub fn get_recent(&self, count: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().take(count).collect()
    }

    pub fn get_today(&self) -> Vec<&HistoryEntry> {
        let today = Utc::now().date_naive();
        self.entries.iter()
            .filter(|entry| entry.last_visited.naive_utc().date() >= today)
            .collect()
    }

    pub fn get_yesterday(&self) -> Vec<&HistoryEntry> {
        let today = Utc::now().date_naive();
        let yesterday = today.pred_opt().unwrap_or(today);
        self.entries.iter()
            .filter(|entry| {
                let entry_date = entry.last_visited.naive_utc().date();
                entry_date == yesterday
            })
            .collect()
    }

    pub fn get_this_week(&self) -> Vec<&HistoryEntry> {
        let today = Utc::now();
        let week_start = today - chrono::Duration::days(7);
        self.entries.iter()
            .filter(|entry| entry.last_visited >= week_start)
            .collect()
    }

    pub fn get_last_month(&self) -> Vec<&HistoryEntry> {
        let today = Utc::now();
        let month_start = today - chrono::Duration::days(30);
        self.entries.iter()
            .filter(|entry| entry.last_visited >= month_start)
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries.iter()
            .filter(|entry| {
                entry.url.to_lowercase().contains(&query_lower) ||
                entry.title.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    pub fn get_most_visited(&self, count: usize) -> Vec<&HistoryEntry> {
        let mut sorted_entries: Vec<&HistoryEntry> = self.entries.iter().collect();
        sorted_entries.sort_by(|a, b| b.visit_count.cmp(&a.visit_count));
        sorted_entries.truncate(count);
        sorted_entries
    }

    pub fn remove_url(&mut self, url: &str) -> bool {
        let initial_len = self.entries.len();
        self.entries.retain(|entry| entry.url != url);
        let removed = self.entries.len() != initial_len;

        if removed {
            let _ = self.save_to_file();
        }

        removed
    }

    pub fn clear_history(&mut self) {
        self.entries.clear();
        let _ = self.save_to_file();
    }

    pub fn save_to_file(&self) -> Result<(), String> {
        // Create directory if it doesn't exist
        if let Some(parent) = Path::new(&self.history_file_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create history directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| format!("Failed to serialize history: {}", e))?;

        fs::write(&self.history_file_path, json)
            .map_err(|e| format!("Failed to write history file: {}", e))?;

        Ok(())
    }

    pub fn load_from_file(&mut self) -> Result<(), String> {
        if !Path::new(&self.history_file_path).exists() {
            // File doesn't exist yet, that's OK
            return Ok(());
        }

        let content = fs::read_to_string(&self.history_file_path)
            .map_err(|e| format!("Failed to read history file: {}", e))?;

        let loaded_entries: Vec<HistoryEntry> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse history file: {}", e))?;

        self.entries = VecDeque::from(loaded_entries);

        Ok(())
    }

    pub fn get_stats(&self) -> HistoryStats {
        let total_visits: u32 = self.entries.iter().map(|e| e.visit_count).sum();
        let unique_urls = self.entries.len();

        HistoryStats {
            total_entries: self.entries.len(),
            total_visits,
            unique_urls,
            oldest_entry: self.entries.back().map(|e| e.first_visited),
            newest_entry: self.entries.front().map(|e| e.last_visited),
        }
    }

    pub fn export_to_file(&self, path: &str) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| format!("Failed to serialize history: {}", e))?;

        fs::write(path, json)
            .map_err(|e| format!("Failed to write export file: {}", e))?;

        Ok(())
    }

    pub fn import_from_file(&mut self, path: &str) -> Result<(), String> {
        if !Path::new(path).exists() {
            return Err("Import file does not exist".to_string());
        }

        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read import file: {}", e))?;

        let imported_entries: Vec<HistoryEntry> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse import file: {}", e))?;

        // Add imported entries to current history (avoiding duplicates)
        for entry in imported_entries {
            if !self.entries.iter().any(|e| e.url == entry.url) {
                self.entries.push_front(entry);
            }
        }

        // Trim to max entries
        while self.entries.len() > self.max_entries {
            self.entries.pop_back();
        }

        let _ = self.save_to_file();
        Ok(())
    }

    pub fn get_urls_with_prefix(&self, prefix: &str) -> Vec<&HistoryEntry> {
        self.entries.iter()
            .filter(|entry| entry.url.starts_with(prefix))
            .collect()
    }

    pub fn get_entries_by_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&HistoryEntry> {
        self.entries.iter()
            .filter(|entry| entry.last_visited >= start && entry.last_visited <= end)
            .collect()
    }

    pub fn get_unique_domains(&self) -> Vec<String> {
        let mut domains = std::collections::HashSet::new();
        for entry in &self.entries {
            if let Some(domain) = self.extract_domain(&entry.url) {
                domains.insert(domain);
            }
        }
        domains.into_iter().collect()
    }

    fn extract_domain(&self, url: &str) -> Option<String> {
        // Simple domain extraction - in a real implementation, we'd use a proper URL parser
        if url.starts_with("http://") || url.starts_with("https://") {
            let start = if url.starts_with("http://") { 7 } else { 8 }; // Length of "http://" or "https://"
            if let Some(end) = url[start..].find('/') {
                Some(url[start..start + end].to_string())
            } else {
                Some(url[start..].to_string())
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct HistoryStats {
    pub total_entries: usize,
    pub total_visits: u32,
    pub unique_urls: usize,
    pub oldest_entry: Option<DateTime<Utc>>,
    pub newest_entry: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_history_manager_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        assert_eq!(history_manager.entries.len(), 0);
    }

    #[test]
    fn test_add_entry() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        
        history_manager.add_entry("http://example.com".to_string(), "Example".to_string(), Instant::now());
        assert_eq!(history_manager.entries.len(), 1);
        assert_eq!(history_manager.entries[0].url, "http://example.com");
        assert_eq!(history_manager.entries[0].title, "Example");
    }

    #[test]
    fn test_duplicate_entry_updates_count() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        
        // Add the same URL twice
        history_manager.add_entry("http://example.com".to_string(), "Example".to_string(), Instant::now());
        history_manager.add_entry("http://example.com".to_string(), "Example Updated".to_string(), Instant::now());
        
        assert_eq!(history_manager.entries.len(), 1);
        assert_eq!(history_manager.entries[0].visit_count, 2);
        // Title should remain unchanged if new title is empty, but updated if it's different
        assert_eq!(history_manager.entries[0].title, "Example");
    }

    #[test]
    fn test_max_entries_limit() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 3);
        
        history_manager.add_entry("http://site1.com".to_string(), "Site 1".to_string(), Instant::now());
        history_manager.add_entry("http://site2.com".to_string(), "Site 2".to_string(), Instant::now());
        history_manager.add_entry("http://site3.com".to_string(), "Site 3".to_string(), Instant::now());
        history_manager.add_entry("http://site4.com".to_string(), "Site 4".to_string(), Instant::now());
        
        assert_eq!(history_manager.entries.len(), 3);
        // The most recent entry should be first
        assert_eq!(history_manager.entries[0].url, "http://site4.com");
    }

    #[test]
    fn test_get_recent() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        
        history_manager.add_entry("http://site1.com".to_string(), "Site 1".to_string(), Instant::now());
        history_manager.add_entry("http://site2.com".to_string(), "Site 2".to_string(), Instant::now());
        history_manager.add_entry("http://site3.com".to_string(), "Site 3".to_string(), Instant::now());
        
        let recent = history_manager.get_recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].url, "http://site3.com");
        assert_eq!(recent[1].url, "http://site2.com");
    }

    #[test]
    fn test_search() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        
        history_manager.add_entry("http://example.com".to_string(), "Example Site".to_string(), Instant::now());
        history_manager.add_entry("http://test.com".to_string(), "Test Site".to_string(), Instant::now());
        
        let results = history_manager.search("example");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "http://example.com");
    }

    #[test]
    fn test_save_and_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        
        history_manager.add_entry("http://example.com".to_string(), "Example".to_string(), Instant::now());
        history_manager.save_to_file().unwrap();
        
        let mut history_manager2 = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        history_manager2.load_from_file().unwrap();
        
        assert_eq!(history_manager2.entries.len(), 1);
        assert_eq!(history_manager2.entries[0].url, "http://example.com");
    }

    #[test]
    fn test_remove_url() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        
        history_manager.add_entry("http://example.com".to_string(), "Example".to_string(), Instant::now());
        history_manager.add_entry("http://test.com".to_string(), "Test".to_string(), Instant::now());
        
        assert_eq!(history_manager.entries.len(), 2);
        
        let removed = history_manager.remove_url("http://example.com");
        assert!(removed);
        assert_eq!(history_manager.entries.len(), 1);
        assert_eq!(history_manager.entries[0].url, "http://test.com");
    }

    #[test]
    fn test_clear_history() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        
        history_manager.add_entry("http://example.com".to_string(), "Example".to_string(), Instant::now());
        history_manager.add_entry("http://test.com".to_string(), "Test".to_string(), Instant::now());
        
        assert_eq!(history_manager.entries.len(), 2);
        
        history_manager.clear_history();
        assert_eq!(history_manager.entries.len(), 0);
    }

    #[test]
    fn test_get_most_visited() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        
        // Add entries with different visit counts
        history_manager.add_entry("http://least_visited.com".to_string(), "Least Visited".to_string(), Instant::now());
        history_manager.add_entry("http://most_visited.com".to_string(), "Most Visited".to_string(), Instant::now());
        history_manager.add_entry("http://most_visited.com".to_string(), "Most Visited".to_string(), Instant::now());
        history_manager.add_entry("http://most_visited.com".to_string(), "Most Visited".to_string(), Instant::now());
        
        let most_visited = history_manager.get_most_visited(1);
        assert_eq!(most_visited.len(), 1);
        assert_eq!(most_visited[0].url, "http://most_visited.com");
        assert_eq!(most_visited[0].visit_count, 3);
    }

    #[test]
    fn test_get_today() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        
        history_manager.add_entry("http://today.com".to_string(), "Today".to_string(), Instant::now());
        
        let today_entries = history_manager.get_today();
        assert!(!today_entries.is_empty());
    }

    #[test]
    fn test_get_unique_domains() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut history_manager = HistoryManager::new(temp_file.path().to_str().unwrap().to_string(), 100);
        
        history_manager.add_entry("http://example.com/page1".to_string(), "Page 1".to_string(), Instant::now());
        history_manager.add_entry("http://example.com/page2".to_string(), "Page 2".to_string(), Instant::now());
        history_manager.add_entry("http://test.org/index.html".to_string(), "Index".to_string(), Instant::now());
        
        let domains = history_manager.get_unique_domains();
        assert_eq!(domains.len(), 2);
        assert!(domains.contains(&"example.com".to_string()));
        assert!(domains.contains(&"test.org".to_string()));
    }
}