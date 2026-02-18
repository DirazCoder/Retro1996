use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub size: usize,
    pub mime_type: String,
    pub expires: Option<u64>,
    pub last_accessed: u64,
    pub access_count: u64,
    pub file_path: PathBuf,
}

impl CacheEntry {
    pub fn new(data: Vec<u8>, mime_type: String, file_path: PathBuf) -> Self {
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
            last_accessed: timestamp,
            access_count: 1,
            file_path,
        }
    }

    pub fn with_ttl(data: Vec<u8>, mime_type: String, file_path: PathBuf, ttl_seconds: u64) -> Self {
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
            last_accessed: timestamp,
            access_count: 1,
            file_path,
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

    pub fn update_access(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        self.last_accessed = current_time;
        self.access_count = self.access_count.saturating_add(1);
    }
}

#[derive(Debug, Clone)]
pub struct DiskCacheConfig {
    pub cache_dir: PathBuf,
    pub max_size: usize,
    pub max_files: usize,
    pub default_ttl: Option<u64>,
    pub cleanup_threshold: f32,
    pub eviction_policy: EvictionPolicy,
    pub enable_windows_optimization: bool,
    pub enable_gpu_accelerated_serialization: bool,
    pub enable_high_dpi_awareness: bool,
    pub enable_background_gc: bool,
    pub gc_interval_seconds: u64,
    pub cache_format_version: u32,
}

#[derive(Debug, Clone)]
pub enum EvictionPolicy {
    LRU,
    FIFO,
    LFU,
    Hybrid,
}

impl Default for DiskCacheConfig {
    fn default() -> Self {
        DiskCacheConfig {
            cache_dir: PathBuf::from("./cache"),
            max_size: 100 * 1024 * 1024, 
            max_files: 1000,
            default_ttl: Some(3600), 
            cleanup_threshold: 0.9,
            eviction_policy: EvictionPolicy::LRU,
            enable_windows_optimization: true,
            enable_gpu_accelerated_serialization: false, // Disabled for 1996 authenticity
            enable_high_dpi_awareness: true,
            enable_background_gc: true,
            gc_interval_seconds: 300, // 5 minutes
            cache_format_version: 1,
        }
    }
}

#[derive(Debug)]
pub struct WindowsFileSystem {
    cache_dir: PathBuf,
    file_handles: Arc<Mutex<HashMap<String, std::fs::File>>>,
    ntfs_optimizations: bool,
}

impl WindowsFileSystem {
    pub fn new(cache_dir: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        
        Ok(WindowsFileSystem {
            cache_dir,
            file_handles: Arc::new(Mutex::new(HashMap::new())),
            ntfs_optimizations: true,
        })
    }

    pub fn write_file(&self, key: &str, data: &[u8]) -> io::Result<PathBuf> {
        let file_path = self.get_file_path(key);
        
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt;
            if self.ntfs_optimizations {
                let file = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .attributes(0x80) // FILE_ATTRIBUTE_NORMAL
                    .open(&file_path)?;
                
                let mut writer = BufWriter::new(file);
                writer.write_all(data)?;
                writer.flush()?;
                return Ok(file_path);
            }
        }
        
        fs::write(&file_path, data)?;
        Ok(file_path)
    }

    pub fn read_file(&self, key: &str) -> io::Result<Vec<u8>> {
        let file_path = self.get_file_path(key);
        
        if self.ntfs_optimizations {
            let file = fs::File::open(&file_path)?;
            let mut reader = BufReader::new(file);
            let mut data = Vec::new();
            reader.read_to_end(&mut data)?;
            Ok(data)
        } else {
            fs::read(&file_path)
        }
    }

    pub fn remove_file(&self, key: &str) -> io::Result<()> {
        let file_path = self.get_file_path(key);
        fs::remove_file(file_path)
    }

    pub fn file_exists(&self, key: &str) -> bool {
        let file_path = self.get_file_path(key);
        file_path.exists()
    }

    fn get_file_path(&self, key: &str) -> PathBuf {
        let hash = self.hash_key(key);
        self.cache_dir.join(format!("{}.cache", hash))
    }

    fn hash_key(&self, key: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug)]
pub struct GpuSerializer {
    compression_enabled: bool,
    gpu_available: bool,
}

impl GpuSerializer {
    pub fn new() -> Self {
        GpuSerializer {
            compression_enabled: false, // Disabled for 1996 authenticity
            gpu_available: false,
        }
    }

    pub fn serialize(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        if self.gpu_available && self.compression_enabled {
            // GPU-accelerated compression would go here
            Ok(data.to_vec())
        } else {
            Ok(data.to_vec())
        }
    }

    pub fn deserialize(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        if self.gpu_available && self.compression_enabled {
            // GPU-accelerated decompression would go here
            Ok(data.to_vec())
        } else {
            Ok(data.to_vec())
        }
    }
}

#[derive(Debug)]
pub struct Cache1996Format {
    version: u32,
    created_date: u64,
    last_modified: u64,
}

impl Cache1996Format {
    pub fn new() -> Self {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        
        Cache1996Format {
            version: 1,
            created_date: current_time,
            last_modified: current_time,
        }
    }

    pub fn update_modified(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        self.last_modified = current_time;
    }

    pub fn serialize_header(&self) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend_from_slice(&self.version.to_le_bytes());
        header.extend_from_slice(&self.created_date.to_le_bytes());
        header.extend_from_slice(&self.last_modified.to_le_bytes());
        header
    }

    pub fn deserialize_header(data: &[u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }
        
        let version = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let created_date = u64::from_le_bytes([data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11]]);
        let last_modified = u64::from_le_bytes([data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19]]);
        
        Some(Cache1996Format {
            version,
            created_date,
            last_modified,
        })
    }
}

#[derive(Debug)]
pub struct DpiFileManager {
    current_dpi: f32,
    cache_size_adjustment: f32,
}

impl DpiFileManager {
    pub fn new() -> Self {
        DpiFileManager {
            current_dpi: 96.0, // Standard DPI
            cache_size_adjustment: 1.0,
        }
    }

    pub fn update_dpi(&mut self, dpi: f32) {
        self.current_dpi = dpi;
        self.cache_size_adjustment = dpi / 96.0;
    }

    pub fn adjust_cache_size(&self, base_size: usize) -> usize {
        (base_size as f32 * self.cache_size_adjustment) as usize
    }
}

#[derive(Debug)]
pub struct BackgroundGcManager {
    gc_sender: Sender<()>,
    gc_thread: Option<thread::JoinHandle<()>>,
    running: Arc<AtomicU64>,
}

impl BackgroundGcManager {
    pub fn new(gc_interval: u64) -> Self {
        let (tx, rx) = channel();
        let running = Arc::new(AtomicU64::new(1));
        let running_clone = running.clone();
        
        let gc_thread = thread::spawn(move || {
            while running_clone.load(Ordering::SeqCst) == 1 {
                let _ = rx.recv_timeout(Duration::from_secs(gc_interval));
                // Continue loop - GC triggered by timeout or sender
            }
        });

        BackgroundGcManager {
            gc_sender: tx,
            gc_thread: Some(gc_thread),
            running,
        }
    }

    pub fn trigger_gc(&self) {
        let _ = self.gc_sender.send(());
    }

    pub fn stop(&mut self) {
        self.running.store(0, Ordering::SeqCst);
        let _ = self.gc_sender.send(());
        if let Some(thread) = self.gc_thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Debug)]
pub struct DiskCache {
    config: DiskCacheConfig,
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    access_order: Arc<Mutex<Vec<String>>>,
    current_size: Arc<AtomicU64>,
    file_system: WindowsFileSystem,
    serializer: GpuSerializer,
    cache_format: Cache1996Format,
    dpi_manager: DpiFileManager,
    background_gc: Option<BackgroundGcManager>,
    stats: Arc<RwLock<DiskCacheStats>>,
    last_gc_time: Arc<AtomicU64>,
}

impl DiskCache {
    pub fn new(config: DiskCacheConfig) -> io::Result<Self> {
        let file_system = WindowsFileSystem::new(config.cache_dir.clone())?;
        
        let mut cache = DiskCache {
            config: config.clone(),
            entries: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(Mutex::new(Vec::new())),
            current_size: Arc::new(AtomicU64::new(0)),
            file_system,
            serializer: GpuSerializer::new(),
            cache_format: Cache1996Format::new(),
            dpi_manager: DpiFileManager::new(),
            background_gc: None,
            stats: Arc::new(RwLock::new(DiskCacheStats::default())),
            last_gc_time: Arc::new(AtomicU64::new(0)),
        };
        
        if config.enable_background_gc {
            cache.background_gc = Some(BackgroundGcManager::new(config.gc_interval_seconds));
        }
        
        cache.load_existing_files()?;
        Ok(cache)
    }

    pub fn with_default_config() -> io::Result<Self> {
        DiskCache::new(DiskCacheConfig::default())
    }

    fn load_existing_files(&mut self) -> io::Result<()> {
        if let Ok(entries) = fs::read_dir(&self.config.cache_dir) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if let Some(key) = self.extract_key_from_filename(file_name) {
                        if let Ok(metadata) = entry.metadata() {
                            let file_size = metadata.len() as usize;
                            let timestamp = metadata.modified()
                                .unwrap_or(SystemTime::now())
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or(Duration::from_secs(0))
                                .as_secs();
                            
                            let mime_type = self.infer_mime_type(file_name);
                            let file_path = entry.path();
                            
                            let entry = CacheEntry {
                                data: Vec::new(), 
                                timestamp,
                                size: file_size,
                                mime_type,
                                expires: None,
                                last_accessed: timestamp,
                                access_count: 1,
                                file_path: file_path.clone(),
                            };
                            
                            let mut entries = self.entries.write().unwrap();
                            entries.insert(key, entry);
                            self.current_size.fetch_add(file_size as u64, Ordering::SeqCst);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn extract_key_from_filename(&self, filename: &str) -> Option<String> {
        filename.split('.').next().map(|s| s.to_string())
    }

    fn infer_mime_type(&self, filename: &str) -> String {
        if filename.ends_with(".html") || filename.ends_with(".htm") {
            "text/html".to_string()
        } else if filename.ends_with(".css") {
            "text/css".to_string()
        } else if filename.ends_with(".js") {
            "application/javascript".to_string()
        } else if filename.ends_with(".png") {
            "image/png".to_string()
        } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            "image/jpeg".to_string()
        } else if filename.ends_with(".gif") {
            "image/gif".to_string()
        } else {
            "application/octet-stream".to_string()
        }
    }

    pub fn insert(&mut self, key: String, data: Vec<u8>, mime_type: String) -> io::Result<bool> {
        let entries = self.entries.read().unwrap();
        if entries.len() >= self.config.max_files {
            drop(entries);
            self.evict_entries()?;
        }

        let file_path = self.file_system.get_file_path(&key);
        let entry = CacheEntry::new(data, mime_type, file_path.clone());
        
        let old_entry = {
            let mut entries = self.entries.write().unwrap();
            entries.insert(key.clone(), entry)
        };

        if let Some(old_entry) = old_entry {
            self.current_size.fetch_sub(old_entry.size as u64, Ordering::SeqCst);
            let _ = self.file_system.remove_file(&key);
        }

        let serialized_data = self.serializer.serialize(&self.entries.read().unwrap()[&key].data)?;
        self.file_system.write_file(&key, &serialized_data)?;
        
        let new_size = self.entries.read().unwrap()[&key].data.len();
        self.current_size.fetch_add(new_size as u64, Ordering::SeqCst);
        self.update_access_order(&key);
        self.cache_format.update_modified();

        if self.should_cleanup() {
            self.cleanup_expired()?;
        }

        Ok(true)
    }

    pub fn insert_with_ttl(&mut self, key: String, data: Vec<u8>, mime_type: String, ttl_seconds: u64) -> io::Result<bool> {
        let entries = self.entries.read().unwrap();
        if entries.len() >= self.config.max_files {
            drop(entries);
            self.evict_entries()?;
        }

        let file_path = self.file_system.get_file_path(&key);
        let entry = CacheEntry::with_ttl(data, mime_type, file_path.clone(), ttl_seconds);
        
        let old_entry = {
            let mut entries = self.entries.write().unwrap();
            entries.insert(key.clone(), entry)
        };

        if let Some(old_entry) = old_entry {
            self.current_size.fetch_sub(old_entry.size as u64, Ordering::SeqCst);
            let _ = self.file_system.remove_file(&key);
        }

        let serialized_data = self.serializer.serialize(&self.entries.read().unwrap()[&key].data)?;
        self.file_system.write_file(&key, &serialized_data)?;
        
        let new_size = self.entries.read().unwrap()[&key].data.len();
        self.current_size.fetch_add(new_size as u64, Ordering::SeqCst);
        self.update_access_order(&key);
        self.cache_format.update_modified();

        if self.should_cleanup() {
            self.cleanup_expired()?;
        }

        Ok(true)
    }

    pub fn get(&self, key: &str) -> io::Result<Option<Vec<u8>>> {
        {
            let entries = self.entries.read().unwrap();
            if let Some(entry) = entries.get(key) {
                if entry.is_expired() {
                    drop(entries);
                    self.remove(key)?;
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        }
        
        // Update access with write lock
        {
            let mut entries = self.entries.write().unwrap();
            if let Some(entry) = entries.get_mut(key) {
                entry.update_access();
            }
        }
        
        let entries = self.entries.read().unwrap();
        if let Some(entry) = entries.get(key) {
            if let Ok(data) = self.file_system.read_file(key) {
                let deserialized_data = self.serializer.deserialize(&data)?;
                self.update_access_order(key);
                Ok(Some(deserialized_data))
            } else {
                drop(entries);
                self.entries.write().unwrap().remove(key);
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        let entries = self.entries.read().unwrap();
        if let Some(entry) = entries.get(key) {
            !entry.is_expired()
        } else {
            false
        }
    }

    pub fn remove(&self, key: &str) -> io::Result<Option<Vec<u8>>> {
        let old_entry = {
            let mut entries = self.entries.write().unwrap();
            entries.remove(key)
        };

        if let Some(entry) = old_entry {
            self.current_size.fetch_sub(entry.size as u64, Ordering::SeqCst);
            self.access_order.lock().unwrap().retain(|k| k != key);
            self.file_system.remove_file(key)?;
            Ok(Some(entry.data))
        } else {
            Ok(None)
        }
    }

    pub fn clear(&self) -> io::Result<()> {
        let keys: Vec<String> = {
            let entries = self.entries.read().unwrap();
            entries.keys().cloned().collect()
        };

        for key in &keys {
            let _ = self.file_system.remove_file(key);
        }
        
        let mut entries = self.entries.write().unwrap();
        entries.clear();
        self.access_order.lock().unwrap().clear();
        self.current_size.store(0, Ordering::SeqCst);
        Ok(())
    }

    pub fn size(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.len()
    }

    pub fn disk_usage(&self) -> usize {
        self.current_size.load(Ordering::SeqCst) as usize
    }

    pub fn max_disk_usage(&self) -> usize {
        self.config.max_size
    }

    pub fn keys(&self) -> Vec<String> {
        let entries = self.entries.read().unwrap();
        entries.keys().cloned().collect()
    }

    pub fn get_mime_type(&self, key: &str) -> Option<String> {
        let entries = self.entries.read().unwrap();
        entries.get(key).map(|entry| entry.mime_type.clone())
    }

    pub fn get_entry_age(&self, key: &str) -> Option<u64> {
        let entries = self.entries.read().unwrap();
        entries.get(key).map(|entry| entry.age())
    }

    pub fn get_entry_size(&self, key: &str) -> Option<usize> {
        let entries = self.entries.read().unwrap();
        entries.get(key).map(|entry| entry.data.len())
    }

    pub fn evict_entries(&self) -> io::Result<()> {
        match self.config.eviction_policy {
            EvictionPolicy::LRU => self.evict_lru()?,
            EvictionPolicy::FIFO => self.evict_fifo()?,
            EvictionPolicy::LFU => self.evict_lfu()?,
            EvictionPolicy::Hybrid => self.evict_hybrid()?,
        }
        Ok(())
    }

    fn evict_lru(&self) -> io::Result<()> {
        let mut access_order = self.access_order.lock().unwrap();
        while self.entries.read().unwrap().len() >= self.config.max_files && !access_order.is_empty() {
            let oldest_key = access_order.remove(0);
            if let Some(entry) = {
                let mut entries = self.entries.write().unwrap();
                entries.remove(&oldest_key)
            } {
                self.current_size.fetch_sub(entry.size as u64, Ordering::SeqCst);
                self.file_system.remove_file(&oldest_key)?;
            }
        }
        Ok(())
    }

    fn evict_fifo(&self) -> io::Result<()> {
        let mut access_order = self.access_order.lock().unwrap();
        while self.entries.read().unwrap().len() >= self.config.max_files && !access_order.is_empty() {
            let oldest_key = access_order.remove(0);
            if let Some(entry) = {
                let mut entries = self.entries.write().unwrap();
                entries.remove(&oldest_key)
            } {
                self.current_size.fetch_sub(entry.size as u64, Ordering::SeqCst);
                self.file_system.remove_file(&oldest_key)?;
            }
        }
        Ok(())
    }

    fn evict_lfu(&self) -> io::Result<()> {
        let entries = self.entries.read().unwrap();
        let mut least_frequent_key = None;
        let mut min_accesses = u64::MAX;

        for (key, entry) in entries.iter() {
            if entry.access_count < min_accesses {
                min_accesses = entry.access_count;
                least_frequent_key = Some(key.clone());
            }
        }

        if let Some(key) = least_frequent_key {
            drop(entries);
            if let Some(entry) = {
                let mut entries = self.entries.write().unwrap();
                entries.remove(key.as_str())
            } {
                self.current_size.fetch_sub(entry.size as u64, Ordering::SeqCst);
                self.access_order.lock().unwrap().retain(|k| k != &key);
                self.file_system.remove_file(&key)?;
            }
        }
        Ok(())
    }

    fn evict_hybrid(&self) -> io::Result<()> {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() - 300; // 5 minutes ago

        // Collect owned data to avoid borrow conflicts
        let candidates: Vec<(String, u64)> = {
            let entries = self.entries.read().unwrap();
            entries
                .iter()
                .filter(|(_, entry)| entry.last_accessed < cutoff_time)
                .map(|(key, entry)| (key.clone(), entry.access_count))
                .collect()
        };

        if candidates.is_empty() {
            self.evict_lru()?;
            return Ok(());
        }

        // Find the entry with the lowest access count
        let mut min_key = None;
        let mut min_access = u64::MAX;
        for (key, access_count) in candidates {
            if access_count < min_access {
                min_access = access_count;
                min_key = Some(key);
            }
        }
        
        if let Some(key) = min_key {
            if let Some(entry) = {
                let mut entries = self.entries.write().unwrap();
                entries.remove(&key)
            } {
                self.current_size.fetch_sub(entry.size as u64, Ordering::SeqCst);
                self.access_order.lock().unwrap().retain(|k| k != &key);
                self.file_system.remove_file(&key)?;
            }
        }
        Ok(())
    }

    fn update_access_order(&self, key: &str) {
        let mut access_order = self.access_order.lock().unwrap();
        access_order.retain(|k| k != key);
        access_order.push(key.to_string());
    }

    fn should_cleanup(&self) -> bool {
        let current_size = self.current_size.load(Ordering::SeqCst) as usize;
        let max_size = self.config.max_size;
        let threshold = (max_size as f32 * self.config.cleanup_threshold) as usize;
        
        current_size > threshold || self.entries.read().unwrap().len() as f32 > (self.config.max_files as f32 * self.config.cleanup_threshold)
    }

    fn cleanup_expired(&self) -> io::Result<()> {
        let mut expired_keys = Vec::new();
        {
            let entries = self.entries.read().unwrap();
            for (key, entry) in entries.iter() {
                if entry.is_expired() {
                    expired_keys.push(key.clone());
                }
            }
        }

        for key in expired_keys {
            let _ = self.remove(&key);
        }
        Ok(())
    }

    pub fn resize(&self, new_max_size: usize, new_max_files: usize) -> io::Result<()> {
        let mut config = self.config.clone();
        config.max_size = new_max_size;
        config.max_files = new_max_files;

        while self.current_size.load(Ordering::SeqCst) as usize > config.max_size || self.entries.read().unwrap().len() > config.max_files {
            self.evict_entries()?;
        }
        Ok(())
    }

    pub fn set_eviction_policy(&mut self, policy: EvictionPolicy) {
        self.config.eviction_policy = policy;
    }

    pub fn stats(&self) -> DiskCacheStats {
        let total_size = self.disk_usage();
        let num_entries = self.size();
        let hit_rate = {
            let stats = self.stats.read().unwrap();
            stats.hit_rate
        };
        let miss_rate = 1.0 - hit_rate;

        DiskCacheStats {
            total_size,
            num_entries,
            hit_rate,
            miss_rate,
            max_size: self.max_disk_usage(),
            total_hits: {
                let stats = self.stats.read().unwrap();
                stats.total_hits
            },
            total_misses: {
                let stats = self.stats.read().unwrap();
                stats.total_misses
            },
            total_inserts: {
                let stats = self.stats.read().unwrap();
                stats.total_inserts
            },
            total_removals: {
                let stats = self.stats.read().unwrap();
                stats.total_removals
            },
            total_evictions: {
                let stats = self.stats.read().unwrap();
                stats.total_evictions
            },
            total_expired: {
                let stats = self.stats.read().unwrap();
                stats.total_expired
            },
            total_cleared: {
                let stats = self.stats.read().unwrap();
                stats.total_cleared
            },
            last_access_time: {
                let stats = self.stats.read().unwrap();
                stats.last_access_time
            },
            gc_duration: 0,
            total_gc_runs: 0,
        }
    }

    pub fn gc(&self) -> io::Result<()> {
        let start_time = Instant::now();
        self.cleanup_expired()?;
        self.evict_entries()?;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        self.last_gc_time.store(current_time, Ordering::SeqCst);
        
        let mut stats = self.stats.write().unwrap();
        stats.gc_duration = start_time.elapsed().as_millis() as u64;
        stats.total_gc_runs += 1;
        Ok(())
    }

    pub fn set_dpi(&mut self, dpi: f32) {
        self.dpi_manager.update_dpi(dpi);
        let adjusted_size = self.dpi_manager.adjust_cache_size(self.config.max_size);
        let _ = self.resize(adjusted_size, self.config.max_files);
    }

    pub fn trigger_background_gc(&self) {
        if let Some(gc_manager) = &self.background_gc {
            gc_manager.trigger_gc();
        }
    }

    pub fn stop_background_gc(&mut self) {
        if let Some(gc_manager) = &mut self.background_gc {
            gc_manager.stop();
        }
    }

    pub fn get_cache_format_version(&self) -> u32 {
        self.cache_format.version
    }

    pub fn get_cache_created_date(&self) -> u64 {
        self.cache_format.created_date
    }

    pub fn get_cache_last_modified(&self) -> u64 {
        self.cache_format.last_modified
    }
}

#[derive(Debug, Default)]
pub struct DiskCacheStats {
    pub total_size: usize,
    pub num_entries: usize,
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub max_size: usize,
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_inserts: u64,
    pub total_removals: u64,
    pub total_evictions: u64,
    pub total_expired: u64,
    pub total_cleared: u64,
    pub last_access_time: u64,
    pub gc_duration: u64,
    pub total_gc_runs: u64,
}

impl Default for DiskCache {
    fn default() -> Self {
        Self::with_default_config().unwrap_or_else(|_| {
            DiskCache {
                config: DiskCacheConfig::default(),
                entries: Arc::new(RwLock::new(HashMap::new())),
                access_order: Arc::new(Mutex::new(Vec::new())),
                current_size: Arc::new(AtomicU64::new(0)),
                file_system: WindowsFileSystem::new(PathBuf::from("./cache")).unwrap(),
                serializer: GpuSerializer::new(),
                cache_format: Cache1996Format::new(),
                dpi_manager: DpiFileManager::new(),
                background_gc: None,
                stats: Arc::new(RwLock::new(DiskCacheStats::default())),
                last_gc_time: Arc::new(AtomicU64::new(0)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_disk_cache_insert_and_get() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = DiskCacheConfig::default();
        config.cache_dir = temp_dir.path().to_path_buf();
        let cache = DiskCache::new(config)?;
        
        let data = vec![1, 2, 3, 4, 5];
        cache.insert("test_key".to_string(), data.clone(), "application/octet-stream".to_string())?;
        
        let retrieved = cache.get("test_key")?.unwrap();
        assert_eq!(retrieved, data);
        Ok(())
    }

    #[test]
    fn test_disk_cache_with_ttl() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = DiskCacheConfig::default();
        config.cache_dir = temp_dir.path().to_path_buf();
        let cache = DiskCache::new(config)?;
        
        let data = vec![1, 2, 3, 4, 5];
        cache.insert_with_ttl("expiring_key".to_string(), data.clone(), "application/octet-stream".to_string(), 1)?; 
        
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        let retrieved = cache.get("expiring_key")?;
        assert!(retrieved.is_none());
        Ok(())
    }

    #[test]
    fn test_disk_cache_eviction() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = DiskCacheConfig::default();
        config.cache_dir = temp_dir.path().to_path_buf();
        config.max_files = 2;
        let cache = DiskCache::new(config)?;
        
        cache.insert("key1".to_string(), vec![1], "application/octet-stream".to_string())?;
        cache.insert("key2".to_string(), vec![2], "application/octet-stream".to_string())?;
        cache.insert("key3".to_string(), vec![3], "application/octet-stream".to_string())?;
        
        assert!(cache.size() <= 2);
        assert!(cache.contains("key3")); 
        Ok(())
    }

    #[test]
    fn test_disk_cache_contains() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = DiskCacheConfig::default();
        config.cache_dir = temp_dir.path().to_path_buf();
        let cache = DiskCache::new(config)?;
        
        let data = vec![1, 2, 3, 4, 5];
        cache.insert("test_key".to_string(), data, "application/octet-stream".to_string())?;
        
        assert!(cache.contains("test_key"));
        assert!(!cache.contains("nonexistent_key"));
        Ok(())
    }

    #[test]
    fn test_disk_cache_clear() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = DiskCacheConfig::default();
        config.cache_dir = temp_dir.path().to_path_buf();
        let cache = DiskCache::new(config)?;
        
        cache.insert("key1".to_string(), vec![1], "application/octet-stream".to_string())?;
        cache.insert("key2".to_string(), vec![2], "application/octet-stream".to_string())?;
        
        assert_eq!(cache.size(), 2);
        
        cache.clear()?;
        
        assert_eq!(cache.size(), 0);
        Ok(())
    }

    #[test]
    fn test_windows_file_system() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_system = WindowsFileSystem::new(temp_dir.path().to_path_buf())?;
        
        let data = vec![1, 2, 3, 4, 5];
        let file_path = file_system.write_file("test", &data)?;
        
        assert!(file_system.file_exists("test"));
        
        let retrieved = file_system.read_file("test")?;
        assert_eq!(retrieved, data);
        
        file_system.remove_file("test")?;
        assert!(!file_system.file_exists("test"));
        
        Ok(())
    }

    #[test]
    fn test_cache_1996_format() {
        let format = Cache1996Format::new();
        assert_eq!(format.version, 1);
        
        let header = format.serialize_header();
        assert_eq!(header.len(), 20);
        
        let deserialized = Cache1996Format::deserialize_header(&header).unwrap();
        assert_eq!(deserialized.version, format.version);
        assert_eq!(deserialized.created_date, format.created_date);
        assert_eq!(deserialized.last_modified, format.last_modified);
    }

    #[test]
    fn test_dpi_file_manager() {
        let mut dpi_manager = DpiFileManager::new();
        assert_eq!(dpi_manager.current_dpi, 96.0);
        
        dpi_manager.update_dpi(192.0);
        assert_eq!(dpi_manager.current_dpi, 192.0);
        
        let adjusted_size = dpi_manager.adjust_cache_size(1000);
        assert_eq!(adjusted_size, 2000);
    }
}