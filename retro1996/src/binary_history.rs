use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visit_time: DateTime<Utc>,
    pub visit_count: u32,
    pub last_visit_time: DateTime<Utc>,
    pub redirect_source: Option<String>,
    pub redirect_destination: Option<String>,
    pub referrer: Option<String>,
    pub is_typical: bool,
    pub typed_count: u32,
    pub transition_type: TransitionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionType {
    Link,
    Typed,
    AutoBookmark,
    ManualSubframe,
    AutoSubframe,
    FormSubmit,
    Reload,
    Keyword,
    KeywordGenerated,
}

pub struct BinaryHistoryManager {
    pub entries: VecDeque<HistoryEntry>,
    pub max_entries: usize,
    pub file_path: String,
    pub dirty: bool,
}

impl HistoryEntry {
    pub fn new(url: String, title: String) -> Self {
        let now = Utc::now();
        HistoryEntry {
            url,
            title,
            visit_time: now,
            visit_count: 1,
            last_visit_time: now,
            redirect_source: None,
            redirect_destination: None,
            referrer: None,
            is_typical: true,
            typed_count: 0,
            transition_type: TransitionType::Typed,
        }
    }

    pub fn increment_visit(&mut self) {
        self.visit_count += 1;
        self.last_visit_time = Utc::now();
    }

    pub fn mark_as_typed(&mut self) {
        self.typed_count += 1;
    }
}

impl From<&str> for TransitionType {
    fn from(s: &str) -> Self {
        match s {
            "link" => TransitionType::Link,
            "typed" => TransitionType::Typed,
            "auto_bookmark" => TransitionType::AutoBookmark,
            "manual_subframe" => TransitionType::ManualSubframe,
            "auto_subframe" => TransitionType::AutoSubframe,
            "form_submit" => TransitionType::FormSubmit,
            "reload" => TransitionType::Reload,
            "keyword" => TransitionType::Keyword,
            "keyword_generated" => TransitionType::KeywordGenerated,
            _ => TransitionType::Typed,
        }
    }
}

impl ToString for TransitionType {
    fn to_string(&self) -> String {
        match self {
            TransitionType::Link => "link".to_string(),
            TransitionType::Typed => "typed".to_string(),
            TransitionType::AutoBookmark => "auto_bookmark".to_string(),
            TransitionType::ManualSubframe => "manual_subframe".to_string(),
            TransitionType::AutoSubframe => "auto_subframe".to_string(),
            TransitionType::FormSubmit => "form_submit".to_string(),
            TransitionType::Reload => "reload".to_string(),
            TransitionType::Keyword => "keyword".to_string(),
            TransitionType::KeywordGenerated => "keyword_generated".to_string(),
        }
    }
}

impl BinaryHistoryManager {
    pub fn new(file_path: String, max_entries: usize) -> Self {
        let mut history = BinaryHistoryManager {
            entries: VecDeque::new(),
            max_entries,
            file_path,
            dirty: false,
        };
        
        if history.load_from_binary_file().is_err() {
        }
        
        history
    }

    pub fn add_entry(&mut self, url: String, title: String) {
        let now = Utc::now();
        if let Some(pos) = self.entries.iter().position(|entry| entry.url == url) {
            let mut entry = self.entries.remove(pos).unwrap();
            entry.increment_visit();
            entry.last_visit_time = now;
            if entry.title.is_empty() && !title.is_empty() {
                entry.title = title;
            }
            self.entries.push_front(entry);
        } else {
            let entry = HistoryEntry {
                url,
                title: if title.is_empty() { "Untitled".to_string() } else { title },
                visit_time: now,
                visit_count: 1,
                last_visit_time: now,
                redirect_source: None,
                redirect_destination: None,
                referrer: None,
                is_typical: true,
                typed_count: 0,
                transition_type: TransitionType::Typed,
            };
            self.entries.push_front(entry);
        }

        while self.entries.len() > self.max_entries {
            self.entries.pop_back();
        }
        
        self.dirty = true;
    }

    pub fn add_entry_with_referrer(&mut self, url: String, title: String, referrer: Option<String>) {
        let now = Utc::now();
        if let Some(pos) = self.entries.iter().position(|entry| entry.url == url) {
            let mut entry = self.entries.remove(pos).unwrap();
            entry.increment_visit();
            entry.last_visit_time = now;
            entry.referrer = referrer;
            if entry.title.is_empty() && !title.is_empty() {
                entry.title = title;
            }
            self.entries.push_front(entry);
        } else {
            let entry = HistoryEntry {
                url,
                title: if title.is_empty() { "Untitled".to_string() } else { title },
                visit_time: now,
                visit_count: 1,
                last_visit_time: now,
                redirect_source: None,
                redirect_destination: None,
                referrer,
                is_typical: true,
                typed_count: 0,
                transition_type: TransitionType::Typed,
            };
            self.entries.push_front(entry);
        }

        while self.entries.len() > self.max_entries {
            self.entries.pop_back();
        }
        
        self.dirty = true;
    }

    pub fn add_redirect_entry(&mut self, source_url: String, dest_url: String) {
        if let Some(pos) = self.entries.iter().position(|entry| entry.url == source_url) {
            let mut entry = self.entries.remove(pos).unwrap();
            entry.redirect_destination = Some(dest_url.clone());
            self.entries.push_front(entry);
        } else {
            let mut entry = HistoryEntry::new(source_url.clone(), String::new());
            entry.redirect_destination = Some(dest_url.clone());
            self.entries.push_front(entry);
        }

        if let Some(pos) = self.entries.iter().position(|entry| entry.url == dest_url) {
            let mut entry = self.entries.remove(pos).unwrap();
            entry.redirect_source = Some(source_url);
            self.entries.push_front(entry);
        } else {
            let mut entry = HistoryEntry::new(dest_url, String::new());
            entry.redirect_source = Some(source_url);
            self.entries.push_front(entry);
        }
        
        self.dirty = true;
    }

    pub fn mark_as_typed(&mut self, url: &str) {
        if let Some(pos) = self.entries.iter().position(|entry| entry.url == url) {
            let mut entry = self.entries.remove(pos).unwrap();
            entry.mark_as_typed();
            entry.transition_type = TransitionType::Typed;
            self.entries.push_front(entry);
            self.dirty = true;
        }
    }

    pub fn get_entry(&self, url: &str) -> Option<&HistoryEntry> {
        self.entries.iter().find(|entry| entry.url == url)
    }

    pub fn get_recent_entries(&self, count: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().take(count).collect()
    }

    pub fn get_entries_by_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&HistoryEntry> {
        self.entries.iter()
            .filter(|entry| entry.visit_time >= start && entry.visit_time <= end)
            .collect()
    }

    pub fn search_entries(&self, query: &str) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries.iter()
            .filter(|entry| {
                entry.url.to_lowercase().contains(&query_lower) ||
                entry.title.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    pub fn get_top_visited(&self, count: usize) -> Vec<&HistoryEntry> {
        let mut entries: Vec<&HistoryEntry> = self.entries.iter().collect();
        entries.sort_by(|a, b| b.visit_count.cmp(&a.visit_count));
        entries.truncate(count);
        entries
    }

    pub fn remove_entry(&mut self, url: &str) -> bool {
        let initial_len = self.entries.len();
        self.entries.retain(|entry| entry.url != url);
        let removed = initial_len != self.entries.len();
        
        if removed {
            self.dirty = true;
        }
        
        removed
    }

    pub fn clear_history(&mut self) {
        self.entries.clear();
        self.dirty = true;
    }

    pub fn get_urls(&self) -> Vec<String> {
        self.entries.iter().map(|entry| entry.url.clone()).collect()
    }

    pub fn get_total_entries(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get_most_recently_visited(&self) -> Option<&HistoryEntry> {
        self.entries.front()
    }

    pub fn get_least_recently_visited(&self) -> Option<&HistoryEntry> {
        self.entries.back()
    }

    pub fn get_urls_by_prefix(&self, prefix: &str) -> Vec<String> {
        self.entries.iter()
            .filter(|entry| entry.url.starts_with(prefix))
            .map(|entry| entry.url.clone())
            .collect()
    }

    pub fn save_to_binary_file(&self) -> Result<(), String> {
        if !self.dirty {
            return Ok(());
        }

        let dir = std::path::Path::new(&self.file_path).parent()
            .ok_or_else(|| "Invalid file path".to_string())?;
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create history directory: {}", e))?;

        let mut file = File::create(&self.file_path)
            .map_err(|e| format!("Failed to create history file: {}", e))?;

        for entry in &self.entries {
            write_string(&mut file, &entry.url)
                .map_err(|e| format!("Failed to write URL: {}", e))?;
            write_string(&mut file, &entry.title)
                .map_err(|e| format!("Failed to write title: {}", e))?;
            write_datetime(&mut file, entry.visit_time)
                .map_err(|e| format!("Failed to write visit time: {}", e))?;
            file.write_all(&entry.visit_count.to_le_bytes())
                .map_err(|e| format!("Failed to write visit count: {}", e))?;
            write_datetime(&mut file, entry.last_visit_time)
                .map_err(|e| format!("Failed to write last visit time: {}", e))?;
            write_optional_string(&mut file, &entry.redirect_source)
                .map_err(|e| format!("Failed to write redirect source: {}", e))?;
            write_optional_string(&mut file, &entry.redirect_destination)
                .map_err(|e| format!("Failed to write redirect destination: {}", e))?;
            write_optional_string(&mut file, &entry.referrer)
                .map_err(|e| format!("Failed to write referrer: {}", e))?;
            file.write_all(&[if entry.is_typical { 1 } else { 0 }])
                .map_err(|e| format!("Failed to write is_typical: {}", e))?;
            file.write_all(&entry.typed_count.to_le_bytes())
                .map_err(|e| format!("Failed to write typed count: {}", e))?;
            write_string(&mut file, &entry.transition_type.to_string())
                .map_err(|e| format!("Failed to write transition type: {}", e))?;
        }

        Ok(())
    }

    pub fn load_from_binary_file(&mut self) -> Result<(), String> {
        if !Path::new(&self.file_path).exists() {
            return Ok(());
        }

        let mut file = File::open(&self.file_path)
            .map_err(|e| format!("Failed to open history file: {}", e))?;

        self.entries.clear();

        loop {
            let url = match read_string(&mut file) {
                Ok(url) => url,
                Err(_) => break, 
            };
            
            let title = read_string(&mut file)
                .map_err(|e| format!("Failed to read title: {}", e))?;
            let visit_time = read_datetime(&mut file)
                .map_err(|e| format!("Failed to read visit time: {}", e))?;
            let mut visit_count_bytes = [0u8; 4];
            file.read_exact(&mut visit_count_bytes)
                .map_err(|e| format!("Failed to read visit count: {}", e))?;
            let visit_count = u32::from_le_bytes(visit_count_bytes);
            let last_visit_time = read_datetime(&mut file)
                .map_err(|e| format!("Failed to read last visit time: {}", e))?;
            let redirect_source = read_optional_string(&mut file)
                .map_err(|e| format!("Failed to read redirect source: {}", e))?;
            let redirect_destination = read_optional_string(&mut file)
                .map_err(|e| format!("Failed to read redirect destination: {}", e))?;
            let referrer = read_optional_string(&mut file)
                .map_err(|e| format!("Failed to read referrer: {}", e))?;
            
            let mut is_typical_byte = [0u8; 1];
            file.read_exact(&mut is_typical_byte)
                .map_err(|e| format!("Failed to read is_typical: {}", e))?;
            let is_typical = is_typical_byte[0] == 1;
            
            let mut typed_count_bytes = [0u8; 4];
            file.read_exact(&mut typed_count_bytes)
                .map_err(|e| format!("Failed to read typed count: {}", e))?;
            let typed_count = u32::from_le_bytes(typed_count_bytes);
            
            let transition_type_str = read_string(&mut file)
                .map_err(|e| format!("Failed to read transition type: {}", e))?;
            let transition_type = TransitionType::from(transition_type_str.as_str());

            let entry = HistoryEntry {
                url,
                title,
                visit_time,
                visit_count,
                last_visit_time,
                redirect_source,
                redirect_destination,
                referrer,
                is_typical,
                typed_count,
                transition_type,
            };

            self.entries.push_back(entry);
        }

        Ok(())
    }

    pub fn gc(&mut self) {
        while self.entries.len() > self.max_entries {
            self.entries.pop_back();
        }
        self.dirty = true;
    }
}

fn write_string<W: Write>(writer: &mut W, s: &str) -> std::io::Result<()> {
    let bytes = s.as_bytes();
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(bytes)?;
    Ok(())
}

fn read_string<R: Read>(reader: &mut R) -> std::io::Result<String> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    
    let mut buffer = vec![0u8; len];
    reader.read_exact(&mut buffer)?;
    
    String::from_utf8(buffer)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid UTF-8"))
}

fn write_optional_string<W: Write>(writer: &mut W, opt: &Option<String>) -> std::io::Result<()> {
    match opt {
        Some(s) => {
            writer.write_all(&[1])?;
            write_string(writer, s)
        }
        None => {
            writer.write_all(&[0])?;
            Ok(())
        }
    }
}

fn read_optional_string<R: Read>(reader: &mut R) -> std::io::Result<Option<String>> {
    let mut present_byte = [0u8; 1];
    reader.read_exact(&mut present_byte)?;
    
    if present_byte[0] == 1 {
        read_string(reader).map(Some)
    } else {
        Ok(None)
    }
}

fn write_datetime<W: Write>(writer: &mut W, dt: DateTime<Utc>) -> std::io::Result<()> {
    let timestamp = dt.timestamp();
    writer.write_all(&timestamp.to_le_bytes())?;
    Ok(())
}

fn read_datetime<R: Read>(reader: &mut R) -> std::io::Result<DateTime<Utc>> {
    let mut timestamp_bytes = [0u8; 8];
    reader.read_exact(&mut timestamp_bytes)?;
    let timestamp = i64::from_le_bytes(timestamp_bytes);
    DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid timestamp"))
}

impl Drop for BinaryHistoryManager {
    fn drop(&mut self) {
        let _ = self.save_to_binary_file();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_add_and_get_entry() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.dat").to_str().unwrap().to_string();
        let mut history = BinaryHistoryManager::new(history_file, 100);
        
        history.add_entry("http://example.com".to_string(), "Example Site".to_string());
        
        let entry = history.get_entry("http://example.com").unwrap();
        assert_eq!(entry.url, "http://example.com");
        assert_eq!(entry.title, "Example Site");
        assert_eq!(entry.visit_count, 1);
    }

    #[test]
    fn test_multiple_visits() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.dat").to_str().unwrap().to_string();
        let mut history = BinaryHistoryManager::new(history_file, 100);
        
        history.add_entry("http://example.com".to_string(), "Example Site".to_string());
        history.add_entry("http://example.com".to_string(), "Example Site".to_string());
        
        let entry = history.get_entry("http://example.com").unwrap();
        assert_eq!(entry.visit_count, 2);
    }

    #[test]
    fn test_binary_history_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.dat").to_str().unwrap().to_string();
        
        let mut history = BinaryHistoryManager::new(history_file.clone(), 100);
        history.add_entry("http://example.com".to_string(), "Example Site".to_string());
        history.add_entry("http://test.com".to_string(), "Test Site".to_string());
        
        assert!(history.save_to_binary_file().is_ok());
        
        let mut history2 = BinaryHistoryManager::new(history_file, 100);
        assert!(history2.load_from_binary_file().is_ok());
        
        assert_eq!(history2.entries.len(), 2);
        assert_eq!(history2.entries[0].url, "http://test.com");
        assert_eq!(history2.entries[1].url, "http://example.com");
    }

    #[test]
    fn test_history_search() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.dat").to_str().unwrap().to_string();
        let mut history = BinaryHistoryManager::new(history_file, 100);
        
        history.add_entry("http://example.com".to_string(), "Example Site".to_string());
        history.add_entry("http://test.com".to_string(), "Test Site".to_string());
        
        let results = history.search_entries("example");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "http://example.com");
    }

    #[test]
    fn test_history_clear() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.dat").to_str().unwrap().to_string();
        let mut history = BinaryHistoryManager::new(history_file, 100);
        
        history.add_entry("http://example.com".to_string(), "Example Site".to_string());
        assert_eq!(history.entries.len(), 1);
        
        history.clear_history();
        assert_eq!(history.entries.len(), 0);
    }
}