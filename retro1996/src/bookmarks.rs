use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Bookmark {
    pub id: String,
    pub name: String,
    pub url: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub date_added: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
    pub parent_folder: Option<String>, 
    pub is_folder: bool,
    pub favorite: bool,
    pub click_count: u32,
    pub last_clicked: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BookmarkFolder {
    pub id: String,
    pub name: String,
    pub description: String,
    pub date_added: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
    pub parent_folder: Option<String>,
    pub expanded: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BookmarkManager {
    pub bookmarks: HashMap<String, Bookmark>,
    pub folders: HashMap<String, BookmarkFolder>,
    pub root_folder_id: String,
    pub save_path: String,
}

impl Bookmark {
    pub fn new(name: String, url: String, description: String, parent_folder: Option<String>) -> Self {
        let now = Utc::now();
        Bookmark {
            id: generate_id(),
            name,
            url,
            description,
            keywords: Vec::new(),
            date_added: now,
            date_modified: now,
            parent_folder,
            is_folder: false,
            favorite: false,
            click_count: 0,
            last_clicked: None,
        }
    }

    pub fn new_folder(name: String, description: String, parent_folder: Option<String>) -> Self {
        let now = Utc::now();
        Bookmark {
            id: generate_id(),
            name,
            url: String::new(),
            description,
            keywords: Vec::new(),
            date_added: now,
            date_modified: now,
            parent_folder,
            is_folder: true,
            favorite: false,
            click_count: 0,
            last_clicked: None,
        }
    }

    pub fn increment_click(&mut self) {
        self.click_count += 1;
        self.last_clicked = Some(Utc::now());
        self.date_modified = Utc::now();
    }

    pub fn add_keyword(&mut self, keyword: String) {
        if !self.keywords.contains(&keyword) {
            self.keywords.push(keyword);
            self.date_modified = Utc::now();
        }
    }

    pub fn remove_keyword(&mut self, keyword: &str) {
        self.keywords.retain(|k| k != keyword);
        self.date_modified = Utc::now();
    }

    pub fn toggle_favorite(&mut self) {
        self.favorite = !self.favorite;
        self.date_modified = Utc::now();
    }
}

impl BookmarkFolder {
    pub fn new(name: String, description: String, parent_folder: Option<String>) -> Self {
        let now = Utc::now();
        BookmarkFolder {
            id: generate_id(),
            name,
            description,
            date_added: now,
            date_modified: now,
            parent_folder,
            expanded: false,
        }
    }

    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
        self.date_modified = Utc::now();
    }
}

impl BookmarkManager {
    pub fn new(save_path: String) -> Self {
        let root_id = generate_id();
        let mut manager = BookmarkManager {
            bookmarks: HashMap::new(),
            folders: HashMap::new(),
            root_folder_id: root_id,
            save_path,
        };
        manager.create_default_folders();
        manager.load_from_disk();
        manager
    }

    fn create_default_folders(&mut self) {
        let root_id = self.root_folder_id.clone();
        
        let folder_bookmarks = BookmarkFolder {
            id: "folder_bookmarks".to_string(),
            name: "Bookmarks".to_string(),
            description: "Main bookmarks folder".to_string(),
            date_added: Utc::now(),
            date_modified: Utc::now(),
            parent_folder: Some(root_id.clone()),
            expanded: true,
        };
        
        let folder_favorites = BookmarkFolder {
            id: "folder_favorites".to_string(),
            name: "Favorites".to_string(),
            description: "Favorite bookmarks".to_string(),
            date_added: Utc::now(),
            date_modified: Utc::now(),
            parent_folder: Some(root_id.clone()),
            expanded: true,
        };
        
        let folder_history = BookmarkFolder {
            id: "folder_history".to_string(),
            name: "History".to_string(),
            description: "Historical bookmarks".to_string(),
            date_added: Utc::now(),
            date_modified: Utc::now(),
            parent_folder: Some(root_id.clone()),
            expanded: false,
        };
        
        self.folders.insert("folder_bookmarks".to_string(), folder_bookmarks);
        self.folders.insert("folder_favorites".to_string(), folder_favorites);
        self.folders.insert("folder_history".to_string(), folder_history);
        
        let root_folder = BookmarkFolder {
            id: root_id,
            name: "Root".to_string(),
            description: "Root bookmark folder".to_string(),
            date_added: Utc::now(),
            date_modified: Utc::now(),
            parent_folder: None,
            expanded: true,
        };
        
        self.folders.insert(self.root_folder_id.clone(), root_folder);
    }

    pub fn add_bookmark(&mut self, name: String, url: String, description: String, parent_folder: Option<String>) -> String {
        let bookmark = Bookmark::new(name, url, description, parent_folder);
        let id = bookmark.id.clone();
        self.bookmarks.insert(id.clone(), bookmark);
        self.save_to_disk();
        id
    }

    pub fn add_folder(&mut self, name: String, description: String, parent_folder: Option<String>) -> String {
        let folder = Bookmark::new_folder(name, description, parent_folder);
        let id = folder.id.clone();
        self.bookmarks.insert(id.clone(), folder);
        self.save_to_disk();
        id
    }

    pub fn remove_bookmark(&mut self, id: &str) -> bool {
        if self.bookmarks.remove(id).is_some() {
            self.save_to_disk();
            true
        } else {
            false
        }
    }

    pub fn get_bookmark(&self, id: &str) -> Option<&Bookmark> {
        self.bookmarks.get(id)
    }

    pub fn get_bookmark_mut(&mut self, id: &str) -> Option<&mut Bookmark> {
        self.bookmarks.get_mut(id)
    }

    pub fn get_folder(&self, id: &str) -> Option<&BookmarkFolder> {
        if let Some(bookmark) = self.bookmarks.get(id) {
            if bookmark.is_folder {
                let folder = BookmarkFolder {
                    id: bookmark.id.clone(),
                    name: bookmark.name.clone(),
                    description: bookmark.description.clone(),
                    date_added: bookmark.date_added,
                    date_modified: bookmark.date_modified,
                    parent_folder: bookmark.parent_folder.clone(),
                    expanded: false,
                };
                return Some(folder.clone());
            }
        }
        None
    }

    pub fn get_children_of_folder(&self, folder_id: &str) -> Vec<&Bookmark> {
        self.bookmarks
            .values()
            .filter(|bookmark| bookmark.parent_folder.as_deref() == Some(folder_id))
            .collect()
    }

    pub fn move_bookmark(&mut self, id: &str, new_parent_folder: Option<String>) -> bool {
        if let Some(bookmark) = self.bookmarks.get_mut(id) {
            bookmark.parent_folder = new_parent_folder;
            bookmark.date_modified = Utc::now();
            self.save_to_disk();
            true
        } else {
            false
        }
    }

    pub fn search_bookmarks(&self, query: &str) -> Vec<&Bookmark> {
        let query_lower = query.to_lowercase();
        self.bookmarks
            .values()
            .filter(|bookmark| 
                bookmark.name.to_lowercase().contains(&query_lower) ||
                bookmark.url.to_lowercase().contains(&query_lower) ||
                bookmark.description.to_lowercase().contains(&query_lower) ||
                bookmark.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower))
            )
            .collect()
    }

    pub fn get_all_bookmarks(&self) -> Vec<&Bookmark> {
        self.bookmarks.values().collect()
    }

    pub fn get_top_level_bookmarks(&self) -> Vec<&Bookmark> {
        self.bookmarks
            .values()
            .filter(|bookmark| bookmark.parent_folder.is_none() || bookmark.parent_folder.as_deref() == Some(&self.root_folder_id))
            .collect()
    }

    pub fn get_most_clicked(&self, count: usize) -> Vec<&Bookmark> {
        let mut bookmarks: Vec<&Bookmark> = self.bookmarks.values().collect();
        bookmarks.sort_by(|a, b| b.click_count.cmp(&a.click_count));
        bookmarks.truncate(count);
        bookmarks
    }

    pub fn get_recently_added(&self, count: usize) -> Vec<&Bookmark> {
        let mut bookmarks: Vec<&Bookmark> = self.bookmarks.values().collect();
        bookmarks.sort_by(|a, b| b.date_added.cmp(&a.date_added));
        bookmarks.truncate(count);
        bookmarks
    }

    pub fn get_favorites(&self) -> Vec<&Bookmark> {
        self.bookmarks
            .values()
            .filter(|bookmark| bookmark.favorite)
            .collect()
    }

    pub fn toggle_favorite(&mut self, id: &str) -> bool {
        if let Some(mut bookmark) = self.bookmarks.get_mut(id) {
            bookmark.toggle_favorite();
            self.save_to_disk();
            true
        } else {
            false
        }
    }

    pub fn increment_click_count(&mut self, id: &str) -> bool {
        if let Some(mut bookmark) = self.bookmarks.get_mut(id) {
            bookmark.increment_click();
            self.save_to_disk();
            true
        } else {
            false
        }
    }

    pub fn add_keyword(&mut self, id: &str, keyword: String) -> bool {
        if let Some(mut bookmark) = self.bookmarks.get_mut(id) {
            bookmark.add_keyword(keyword);
            self.save_to_disk();
            true
        } else {
            false
        }
    }

    pub fn remove_keyword(&mut self, id: &str, keyword: &str) -> bool {
        if let Some(mut bookmark) = self.bookmarks.get_mut(id) {
            bookmark.remove_keyword(keyword);
            self.save_to_disk();
            true
        } else {
            false
        }
    }

    pub fn export_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn import_from_file(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(path).exists() {
            return Ok(());
        }
        
        let contents = fs::read_to_string(path)?;
        let imported: BookmarkManager = serde_json::from_str(&contents)?;
        
        self.bookmarks = imported.bookmarks;
        self.folders = imported.folders;
        self.root_folder_id = imported.root_folder_id;
        
        Ok(())
    }

    pub fn save_to_disk(&mut self) {
        let _ = self.export_to_file(&self.save_path);
    }

    pub fn load_from_disk(&mut self) {
        let save_path = self.save_path.clone();
        let _ = self.import_from_file(&save_path);
    }

    pub fn get_bookmark_count(&self) -> usize {
        self.bookmarks.len()
    }

    pub fn get_folder_count(&self) -> usize {
        self.folders.len()
    }

    pub fn get_total_clicks(&self) -> u32 {
        self.bookmarks.values().map(|b| b.click_count).sum()
    }

    pub fn clear_all(&mut self) {
        self.bookmarks.clear();
        self.folders.clear();
        self.create_default_folders();
        self.save_to_disk();
    }

    pub fn rebuild_indices(&mut self) {
        for bookmark in self.bookmarks.values_mut() {
            if bookmark.parent_folder.is_none() {
                bookmark.parent_folder = Some(self.root_folder_id.clone());
            }
        }
    }

    pub fn get_bookmarks_by_keyword(&self, keyword: &str) -> Vec<&Bookmark> {
        self.bookmarks
            .values()
            .filter(|bookmark| bookmark.keywords.contains(&keyword.to_string()))
            .collect()
    }

    pub fn get_bookmarks_by_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&Bookmark> {
        self.bookmarks
            .values()
            .filter(|bookmark| bookmark.date_added >= start && bookmark.date_added <= end)
            .collect()
    }

    pub fn get_bookmarks_modified_after(&self, since: DateTime<Utc>) -> Vec<&Bookmark> {
        self.bookmarks
            .values()
            .filter(|bookmark| bookmark.date_modified >= since)
            .collect()
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_micros();
    format!("bm_{:x}", now)
}

impl Default for BookmarkManager {
    fn default() -> Self {
        BookmarkManager::new("bookmarks.json".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bookmark() {
        let mut manager = BookmarkManager::new("test_bookmarks.json".to_string());
        let id = manager.add_bookmark(
            "Test Site".to_string(),
            "http://test.com".to_string(),
            "A test site".to_string(),
            None
        );
        
        let bookmark = manager.get_bookmark(&id).unwrap();
        assert_eq!(bookmark.name, "Test Site");
        assert_eq!(bookmark.url, "http://test.com");
        assert_eq!(bookmark.description, "A test site");
    }

    #[test]
    fn test_add_and_remove_bookmark() {
        let mut manager = BookmarkManager::new("test_bookmarks.json".to_string());
        let id = manager.add_bookmark(
            "Test Site".to_string(),
            "http://test.com".to_string(),
            "A test site".to_string(),
            None
        );
        
        assert!(manager.get_bookmark(&id).is_some());
        
        assert!(manager.remove_bookmark(&id));
        assert!(manager.get_bookmark(&id).is_none());
    }

    #[test]
    fn test_search_bookmarks() {
        let mut manager = BookmarkManager::new("test_bookmarks.json".to_string());
        manager.add_bookmark(
            "Example Site".to_string(),
            "http://example.com".to_string(),
            "An example site".to_string(),
            None
        );
        
        let results = manager.search_bookmarks("example");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Example Site");
    }

    #[test]
    fn test_increment_click() {
        let mut manager = BookmarkManager::new("test_bookmarks.json".to_string());
        let id = manager.add_bookmark(
            "Test Site".to_string(),
            "http://test.com".to_string(),
            "A test site".to_string(),
            None
        );
        
        assert_eq!(manager.get_bookmark(&id).unwrap().click_count, 0);
        
        manager.increment_click_count(&id);
        assert_eq!(manager.get_bookmark(&id).unwrap().click_count, 1);
        
        manager.increment_click_count(&id);
        assert_eq!(manager.get_bookmark(&id).unwrap().click_count, 2);
    }

    #[test]
    fn test_toggle_favorite() {
        let mut manager = BookmarkManager::new("test_bookmarks.json".to_string());
        let id = manager.add_bookmark(
            "Test Site".to_string(),
            "http://test.com".to_string(),
            "A test site".to_string(),
            None
        );
        
        assert!(!manager.get_bookmark(&id).unwrap().favorite);
        
        manager.toggle_favorite(&id);
        assert!(manager.get_bookmark(&id).unwrap().favorite);
        
        manager.toggle_favorite(&id);
        assert!(!manager.get_bookmark(&id).unwrap().favorite);
    }

    #[test]
    fn test_add_keyword() {
        let mut manager = BookmarkManager::new("test_bookmarks.json".to_string());
        let id = manager.add_bookmark(
            "Test Site".to_string(),
            "http://test.com".to_string(),
            "A test site".to_string(),
            None
        );
        
        assert_eq!(manager.get_bookmark(&id).unwrap().keywords.len(), 0);
        
        manager.add_keyword(&id, "technology".to_string());
        assert_eq!(manager.get_bookmark(&id).unwrap().keywords.len(), 1);
        assert!(manager.get_bookmark(&id).unwrap().keywords.contains(&"technology".to_string()));
    }
}