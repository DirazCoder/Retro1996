use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub size: usize,
    pub mime_type: String,
    pub expires: Option<u64>,
}

impl CacheEntry {
    pub fn new(data: Vec<u8>, mime_type: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        
        CacheEntry {
            data,
            timestamp,
            size: 0,
            mime_type,
            expires: None,
        }
    }

    pub fn with_ttl(data: Vec<u8>, mime_type: String, ttl_seconds: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        
        CacheEntry {
            data,
            timestamp,
            size: 0,
            mime_type,
            expires: Some(timestamp + ttl_seconds),
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.expires {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs();
            current_time > expiry
        } else {
            false
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

#[derive(Debug, Clone)]
pub struct BinaryCacheConfig {
    pub max_size: usize,
    pub max_entries: usize,
    pub default_ttl: Option<u64>,
    pub cleanup_threshold: f32,
    pub eviction_policy: EvictionPolicy,
}

#[derive(Debug, Clone)]
pub enum EvictionPolicy {
    LRU,
    FIFO,
    LFU,
}

impl Default for BinaryCacheConfig {
    fn default() -> Self {
        BinaryCacheConfig {
            max_size: 50 * 1024 * 1024, 
            max_entries: 1000,
            default_ttl: Some(3600), 
            cleanup_threshold: 0.9,
            eviction_policy: EvictionPolicy::LRU,
        }
    }
}

#[derive(Debug)]
pub struct BinaryCache {
    entries: HashMap<String, CacheEntry>,
    access_order: Vec<String>,
    config: BinaryCacheConfig,
    current_size: usize,
}

impl BinaryCache {
    pub fn new(config: BinaryCacheConfig) -> Self {
        BinaryCache {
            entries: HashMap::new(),
            access_order: Vec::new(),
            config,
            current_size: 0,
        }
    }

    pub fn with_default_config() -> Self {
        BinaryCache::new(BinaryCacheConfig::default())
    }

    pub fn insert(&mut self, key: String, data: Vec<u8>, mime_type: String) -> bool {
        if self.entries.len() >= self.config.max_entries {
            self.evict_entries();
        }

        let entry = CacheEntry::new(data, mime_type);
        let old_entry = self.entries.insert(key.clone(), entry);

        if let Some(old_entry) = old_entry {
            self.current_size -= old_entry.size;
        }

        self.current_size += self.entries[&key].data.len();
        self.update_access_order(&key);

        if self.should_cleanup() {
            self.cleanup_expired();
        }

        true
    }

    pub fn insert_with_ttl(&mut self, key: String, data: Vec<u8>, mime_type: String, ttl_seconds: u64) -> bool {
        if self.entries.len() >= self.config.max_entries {
            self.evict_entries();
        }

        let entry = CacheEntry::with_ttl(data, mime_type, ttl_seconds);
        let old_entry = self.entries.insert(key.clone(), entry);

        if let Some(old_entry) = old_entry {
            self.current_size -= old_entry.size;
        }

        self.current_size += self.entries[&key].data.len();
        self.update_access_order(&key);

        if self.should_cleanup() {
            self.cleanup_expired();
        }

        true
    }

    pub fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.entries.get(key) {
            if entry.is_expired() {
                self.remove(key);
                return None;
            }
            
            let data = entry.data.clone();
            self.update_access_order(key);
            Some(data)
        } else {
            None
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        if let Some(entry) = self.entries.get(key) {
            !entry.is_expired()
        } else {
            false
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.entries.remove(key) {
            self.current_size -= entry.size;
            self.access_order.retain(|k| k != key);
            Some(entry.data)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
        self.current_size = 0;
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn memory_usage(&self) -> usize {
        self.current_size
    }

    pub fn max_memory_usage(&self) -> usize {
        self.config.max_size
    }

    pub fn keys(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    pub fn get_mime_type(&self, key: &str) -> Option<String> {
        self.entries.get(key).map(|entry| entry.mime_type.clone())
    }

    pub fn get_entry_age(&self, key: &str) -> Option<u64> {
        self.entries.get(key).map(|entry| entry.age())
    }

    pub fn get_entry_size(&self, key: &str) -> Option<usize> {
        self.entries.get(key).map(|entry| entry.data.len())
    }

    fn evict_entries(&mut self) {
        match self.config.eviction_policy {
            EvictionPolicy::LRU => self.evict_lru(),
            EvictionPolicy::FIFO => self.evict_fifo(),
            EvictionPolicy::LFU => self.evict_lfu(),
        }
    }

    fn evict_lru(&mut self) {
        while self.entries.len() >= self.config.max_entries && !self.access_order.is_empty() {
            let oldest_key = self.access_order.remove(0);
            if let Some(entry) = self.entries.remove(&oldest_key) {
                self.current_size -= entry.size;
            }
        }
    }

    fn evict_fifo(&mut self) {
        while self.entries.len() >= self.config.max_entries && !self.access_order.is_empty() {
            let oldest_key = self.access_order.remove(0);
            if let Some(entry) = self.entries.remove(&oldest_key) {
                self.current_size -= entry.size;
            }
        }
    }

    fn evict_lfu(&mut self) {
        let mut least_frequent_key = None;
        let mut min_accesses = usize::MAX;

        for (key, _) in &self.entries {
            let accesses = self.access_order.iter().filter(|k| k == &key).count();
            if accesses < min_accesses {
                min_accesses = accesses;
                least_frequent_key = Some(key.clone());
            }
        }

        if let Some(key) = least_frequent_key {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_size -= entry.size;
            }
            self.access_order.retain(|k| k != &key);
        }
    }

    fn update_access_order(&mut self, key: &str) {
        self.access_order.retain(|k| k != key);
        self.access_order.push(key.to_string());
    }

    fn should_cleanup(&self) -> bool {
        let current_size = self.current_size;
        let max_size = self.config.max_size;
        let threshold = (max_size as f32 * self.config.cleanup_threshold) as usize;
        
        current_size > threshold || self.entries.len() as f32 > (self.config.max_entries as f32 * self.config.cleanup_threshold)
    }

    fn cleanup_expired(&mut self) {
        let mut expired_keys = Vec::new();
        for (key, entry) in &self.entries {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        for key in expired_keys {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_size -= entry.size;
            }
            self.access_order.retain(|k| k != &key);
        }
    }

    pub fn resize(&mut self, new_max_size: usize, new_max_entries: usize) {
        self.config.max_size = new_max_size;
        self.config.max_entries = new_max_entries;

        while self.current_size > self.config.max_size || self.entries.len() > self.config.max_entries {
            self.evict_entries();
        }
    }

    pub fn set_eviction_policy(&mut self, policy: EvictionPolicy) {
        self.config.eviction_policy = policy;
    }

    pub fn stats(&self) -> CacheStats {
        let total_size = self.memory_usage();
        let num_entries = self.size();
        let hit_rate = 0.0; 
        let miss_rate = 0.0; 

        CacheStats {
            total_size,
            num_entries,
            hit_rate,
            miss_rate,
            max_size: self.max_memory_usage(),
        }
    }

    pub fn gc(&mut self) {
        self.cleanup_expired();
        self.evict_entries();
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_size: usize,
    pub num_entries: usize,
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub max_size: usize,
}

impl Default for BinaryCache {
    fn default() -> Self {
        Self::with_default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = BinaryCache::with_default_config();
        let data = vec![1, 2, 3, 4, 5];
        cache.insert("test_key".to_string(), data.clone(), "application/octet-stream".to_string());
        
        let retrieved = cache.get("test_key").unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_cache_with_ttl() {
        let mut cache = BinaryCache::with_default_config();
        let data = vec![1, 2, 3, 4, 5];
        cache.insert_with_ttl("expiring_key".to_string(), data.clone(), "application/octet-stream".to_string(), 1); 
        
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        let retrieved = cache.get("expiring_key");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let mut config = BinaryCacheConfig::default();
        config.max_entries = 2;
        let mut cache = BinaryCache::new(config);
        
        cache.insert("key1".to_string(), vec![1], "application/octet-stream".to_string());
        cache.insert("key2".to_string(), vec![2], "application/octet-stream".to_string());
        cache.insert("key3".to_string(), vec![3], "application/octet-stream".to_string());
        
        assert!(cache.size() <= 2);
        assert!(cache.contains("key3")); 
    }

    #[test]
    fn test_cache_contains() {
        let mut cache = BinaryCache::with_default_config();
        let data = vec![1, 2, 3, 4, 5];
        cache.insert("test_key".to_string(), data, "application/octet-stream".to_string());
        
        assert!(cache.contains("test_key"));
        assert!(!cache.contains("nonexistent_key"));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = BinaryCache::with_default_config();
        cache.insert("key1".to_string(), vec![1], "application/octet-stream".to_string());
        cache.insert("key2".to_string(), vec![2], "application/octet-stream".to_string());
        
        assert_eq!(cache.size(), 2);
        
        cache.clear();
        
        assert_eq!(cache.size(), 0);
    }
}