use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::path::Path;
use std::fs;
use serde::{Deserialize, Serialize};
use crate::binary_cache::{BinaryCache, CacheEntry, BinaryCacheConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResource {
    pub url: String,
    pub content: Vec<u8>,
    pub content_type: String,
    pub last_modified: u64,
    pub expires: Option<u64>,
    pub etag: Option<String>,
    pub size: usize,
    pub timestamp: u64,
}

impl CachedResource {
    pub fn new(url: String, content: Vec<u8>, content_type: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        
        CachedResource {
            url,
            content,
            content_type,
            last_modified: timestamp,
            expires: None,
            etag: None,
            size: 0,
            timestamp,
        }
    }

    pub fn with_expires(url: String, content: Vec<u8>, content_type: String, ttl_seconds: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        
        CachedResource {
            url,
            content,
            content_type,
            last_modified: timestamp,
            expires: Some(timestamp + ttl_seconds),
            etag: None,
            size: 0,
            timestamp,
        }
    }

    pub fn is_valid(&self) -> bool {
        if let Some(expires) = self.expires {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs();
            current_time < expires
        } else {
            true
        }
    }

    pub fn age(&self) -> u64 {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        current_time - self.timestamp
    }
}

#[derive(Debug)]
pub struct DiskCache {
    cache_dir: String,
    max_size: usize,
    current_size: usize,
    entries: HashMap<String, CachedResource>,
    lru_order: VecDeque<String>,
}

impl DiskCache {
    pub fn new(cache_dir: String, max_size: usize) -> Self {
        let mut cache = DiskCache {
            cache_dir: cache_dir.clone(),
            max_size,
            current_size: 0,
            entries: HashMap::new(),
            lru_order: VecDeque::new(),
        };
        cache.load_from_disk();
        cache
    }

    fn load_from_disk(&mut self) {
        if !Path::new(&self.cache_dir).exists() {
            return;
        }

        let paths = fs::read_dir(&self.cache_dir);
        if let Ok(paths) = paths {
            for path in paths {
                if let Ok(entry) = path {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("cache") {
                        if let Ok(contents) = fs::read(&path) {
        if let Ok(resource) = bincode::deserialize::<CachedResource>(&contents) {
            let url_hash = hash_url(&resource.url);
            let content_len = resource.content.len();
            self.entries.insert(url_hash.clone(), resource);
            self.lru_order.push_back(url_hash);
            self.current_size += content_len;
                            }
                        }
                    }
                }
            }
        }
    }

    fn save_to_disk(&self, url_hash: &str, resource: &CachedResource) -> Result<(), std::io::Error> {
        if !Path::new(&self.cache_dir).exists() {
            fs::create_dir_all(&self.cache_dir)?;
        }

        let cache_path = format!("{}/{}.cache", self.cache_dir, url_hash);
        let serialized = bincode::serialize(resource)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        fs::write(cache_path, serialized)?;
        Ok(())
    }

    fn remove_from_disk(&self, url_hash: &str) -> Result<(), std::io::Error> {
        let cache_path = format!("{}/{}.cache", self.cache_dir, url_hash);
        if Path::new(&cache_path).exists() {
            fs::remove_file(cache_path)?;
        }
        Ok(())
    }

    pub fn insert(&mut self, url: String, content: Vec<u8>, content_type: String) -> bool {
        let url_hash = hash_url(&url);
        let resource = CachedResource::new(url, content, content_type);
        let old_resource = self.entries.insert(url_hash.clone(), resource);

        if let Some(old_resource) = old_resource {
            self.current_size -= old_resource.content.len();
            let _ = self.remove_from_disk(&url_hash);
        }

        let resource = &self.entries[&url_hash];
        self.current_size += resource.content.len();

        if let Some(pos) = self.lru_order.iter().position(|x| x == &url_hash) {
            self.lru_order.remove(pos);
        }
        self.lru_order.push_back(url_hash.clone());

        if let Err(_) = self.save_to_disk(&url_hash, &self.entries[&url_hash]) {
        }

        if self.current_size > self.max_size {
            self.evict_lru();
        }

        true
    }

    pub fn get(&mut self, url: &str) -> Option<CachedResource> {
        let url_hash = hash_url(url);
        
        if let Some(resource) = self.entries.get(&url_hash) {
            if resource.is_valid() {
                if let Some(pos) = self.lru_order.iter().position(|x| x == &url_hash) {
                    self.lru_order.remove(pos);
                }
                self.lru_order.push_back(url_hash.clone());
                
                Some(resource.clone())
            } else {
                self.remove(url);
                None
            }
        } else {
            None
        }
    }

    pub fn contains(&self, url: &str) -> bool {
        let url_hash = hash_url(url);
        if let Some(resource) = self.entries.get(&url_hash) {
            resource.is_valid()
        } else {
            false
        }
    }

    pub fn remove(&mut self, url: &str) -> bool {
        let url_hash = hash_url(url);
        if let Some(resource) = self.entries.remove(&url_hash) {
            self.current_size -= resource.content.len();
            self.lru_order.retain(|k| k != &url_hash);
            let _ = self.remove_from_disk(&url_hash);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru_order.clear();
        self.current_size = 0;
        
        if Path::new(&self.cache_dir).exists() {
            let _ = fs::remove_dir_all(&self.cache_dir);
        }
    }

    fn evict_lru(&mut self) {
        while self.current_size > self.max_size && !self.lru_order.is_empty() {
            if let Some(oldest_key) = self.lru_order.pop_front() {
                if let Some(resource) = self.entries.remove(&oldest_key) {
                    self.current_size -= resource.content.len();
                    let _ = self.remove_from_disk(&oldest_key);
                }
            }
        }
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn memory_usage(&self) -> usize {
        self.current_size
    }

    pub fn get_urls(&self) -> Vec<String> {
        self.entries.values().map(|r| r.url.clone()).collect()
    }

    pub fn get_resource_age(&self, url: &str) -> Option<u64> {
        let url_hash = hash_url(url);
        self.entries.get(&url_hash).map(|r| r.age())
    }

    pub fn get_resource_size(&self, url: &str) -> Option<usize> {
        let url_hash = hash_url(url);
        self.entries.get(&url_hash).map(|r| r.content.len())
    }

    pub fn resize(&mut self, new_max_size: usize) {
        self.max_size = new_max_size;
        while self.current_size > self.max_size && !self.lru_order.is_empty() {
            self.evict_lru();
        }
    }

    pub fn gc(&mut self) {
        let mut expired_keys = Vec::new();
        for (key, resource) in &self.entries {
            if !resource.is_valid() {
                expired_keys.push(key.clone());
            }
        }

        for key in expired_keys {
            if let Some(resource) = self.entries.remove(&key) {
                self.current_size -= resource.content.len();
                self.lru_order.retain(|k| k != &key);
                let _ = self.remove_from_disk(&key);
            }
        }
    }
}

#[derive(Debug)]
pub struct HybridCache {
    pub memory_cache: BinaryCache,
    pub disk_cache: DiskCache,
    pub cache_policy: CachePolicy,
}

#[derive(Debug, Clone)]
pub enum CachePolicy {
    MemoryFirst,
    DiskFirst,
    Both,
}

#[derive(Debug)]
pub enum CacheError {
    Io(std::io::Error),
    Serialization(String),
    NotFound,
    InvalidResource,
}

impl From<std::io::Error> for CacheError {
    fn from(err: std::io::Error) -> Self {
        CacheError::Io(err)
    }
}

impl HybridCache {
    pub fn new(memory_config: BinaryCacheConfig, disk_cache_dir: String, disk_max_size: usize) -> Self {
        HybridCache {
            memory_cache: BinaryCache::new(memory_config),
            disk_cache: DiskCache::new(disk_cache_dir, disk_max_size),
            cache_policy: CachePolicy::MemoryFirst,
        }
    }

    pub fn insert(&mut self, url: String, content: Vec<u8>, content_type: String) -> Result<(), CacheError> {
        match self.cache_policy {
            CachePolicy::MemoryFirst => {
                self.memory_cache.insert(url.clone(), content.clone(), content_type.clone());
                self.disk_cache.insert(url, content, content_type);
            },
            CachePolicy::DiskFirst => {
                self.disk_cache.insert(url.clone(), content.clone(), content_type.clone());
                self.memory_cache.insert(url, content, content_type);
            },
            CachePolicy::Both => {
                self.memory_cache.insert(url.clone(), content.clone(), content_type.clone());
                self.disk_cache.insert(url, content, content_type);
            },
        }
        Ok(())
    }

    pub fn get(&mut self, url: &str) -> Result<Option<CachedResource>, CacheError> {
        match self.cache_policy {
            CachePolicy::MemoryFirst => {
                if let Some(resource) = self.memory_cache.get(url) {
                    return Ok(Some(CachedResource::new(
                        url.to_string(),
                        resource,
                        self.memory_cache.get_mime_type(url).unwrap_or("application/octet-stream".to_string())
                    )));
                }
                
                if let Some(resource) = self.disk_cache.get(url) {
                    if resource.is_valid() {
                        self.memory_cache.insert(
                            url.to_string(),
                            resource.content.clone(),
                            resource.content_type.clone()
                        );
                        return Ok(Some(resource));
                    }
                }
            },
            CachePolicy::DiskFirst => {
                if let Some(resource) = self.disk_cache.get(url) {
                    if resource.is_valid() {
                        self.memory_cache.insert(
                            url.to_string(),
                            resource.content.clone(),
                            resource.content_type.clone()
                        );
                        return Ok(Some(resource));
                    }
                }
                
                if let Some(resource) = self.memory_cache.get(url) {
                    return Ok(Some(CachedResource::new(
                        url.to_string(),
                        resource,
                        self.memory_cache.get_mime_type(url).unwrap_or("application/octet-stream".to_string())
                    )));
                }
            },
            CachePolicy::Both => {
                if let Some(resource) = self.memory_cache.get(url) {
                    return Ok(Some(CachedResource::new(
                        url.to_string(),
                        resource,
                        self.memory_cache.get_mime_type(url).unwrap_or("application/octet-stream".to_string())
                    )));
                }
                
                if let Some(resource) = self.disk_cache.get(url) {
                    if resource.is_valid() {
                        self.memory_cache.insert(
                            url.to_string(),
                            resource.content.clone(),
                            resource.content_type.clone()
                        );
                        return Ok(Some(resource));
                    }
                }
            },
        }
        
        Ok(None)
    }

    pub fn contains(&self, url: &str) -> bool {
        match self.cache_policy {
            CachePolicy::MemoryFirst | CachePolicy::Both => {
                self.memory_cache.contains(url) || self.disk_cache.contains(url)
            },
            CachePolicy::DiskFirst => {
                self.disk_cache.contains(url) || self.memory_cache.contains(url)
            },
        }
    }

    pub fn remove(&mut self, url: &str) -> Result<bool, CacheError> {
        let mem_removed = self.memory_cache.remove(url).is_some();
        let disk_removed = self.disk_cache.remove(url);
        Ok(mem_removed || disk_removed)
    }

    pub fn clear(&mut self) {
        self.memory_cache.clear();
        self.disk_cache.clear();
    }

    pub fn size(&self) -> usize {
        self.memory_cache.size() + self.disk_cache.size()
    }

    pub fn memory_usage(&self) -> usize {
        self.memory_cache.memory_usage() + self.disk_cache.memory_usage()
    }

    pub fn set_policy(&mut self, policy: CachePolicy) {
        self.cache_policy = policy;
    }

    pub fn gc(&mut self) {
        self.memory_cache.gc();
        self.disk_cache.gc();
    }

    pub fn get_urls(&self) -> Vec<String> {
        let mut urls = self.memory_cache.keys();
        urls.extend(self.disk_cache.get_urls());
        urls.sort();
        urls.dedup();
        urls
    }

    pub fn get_resource_age(&self, url: &str) -> Option<u64> {
        if self.memory_cache.contains(url) {
            self.memory_cache.get_entry_age(url)
        } else {
            self.disk_cache.get_resource_age(url)
        }
    }

    pub fn get_resource_size(&self, url: &str) -> Option<usize> {
        if self.memory_cache.contains(url) {
            self.memory_cache.get_entry_size(url)
        } else {
            self.disk_cache.get_resource_size(url)
        }
    }

    pub fn resize(&mut self, memory_max_size: usize, disk_max_size: usize) {
        self.memory_cache.resize(
            memory_max_size,
            self.memory_cache.size() // Keep the same entry limit
        );
        self.disk_cache.resize(disk_max_size);
    }
}

fn hash_url(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

impl Default for HybridCache {
    fn default() -> Self {
        let memory_config = BinaryCacheConfig {
            max_size: 10 * 1024 * 1024, 
            max_entries: 100,
            default_ttl: Some(3600),
            cleanup_threshold: 0.9,
            eviction_policy: crate::binary_cache::EvictionPolicy::LRU,
        };
        
        HybridCache::new(memory_config, "cache".to_string(), 100 * 1024 * 1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_cache_insert_and_get() {
        let mut cache = DiskCache::new("test_cache_dir".to_string(), 1024 * 1024);
        let content = vec![1, 2, 3, 4, 5];
        cache.insert("http://example.com".to_string(), content.clone(), "text/html".to_string());
        
        let retrieved = cache.get("http://example.com").unwrap();
        assert_eq!(retrieved.content, content);
    }

    #[test]
    fn test_hybrid_cache_insert_and_get() {
        let mut cache = HybridCache::default();
        let content = vec![1, 2, 3, 4, 5];
        cache.insert("http://example.com".to_string(), content.clone(), "text/html".to_string()).unwrap();
        
        let retrieved = cache.get("http://example.com").unwrap().unwrap();
        assert_eq!(retrieved.content, content);
    }

    #[test]
    fn test_cache_contains() {
        let mut cache = HybridCache::default();
        let content = vec![1, 2, 3, 4, 5];
        cache.insert("http://example.com".to_string(), content, "text/html".to_string()).unwrap();
        
        assert!(cache.contains("http://example.com"));
        assert!(!cache.contains("http://nonexistent.com"));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = HybridCache::default();
        cache.insert("http://example1.com".to_string(), vec![1], "text/html".to_string()).unwrap();
        cache.insert("http://example2.com".to_string(), vec![2], "text/html".to_string()).unwrap();
        
        assert!(cache.contains("http://example1.com"));
        assert!(cache.contains("http://example2.com"));
        
        cache.clear();
        
        assert!(!cache.contains("http://example1.com"));
        assert!(!cache.contains("http://example2.com"));
    }
}