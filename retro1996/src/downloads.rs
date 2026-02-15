use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use url::Url;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed(String),
    Canceled,
}

#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub destination_path: String,
    pub total_size: Option<u64>,
    pub downloaded_size: u64,
    pub status: DownloadStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub download_speed: f64, 
    pub time_remaining: Option<Duration>,
    pub content_type: Option<String>,
    pub auto_open_after_download: bool,
}

impl DownloadItem {
    pub fn progress(&self) -> f64 {
        if let Some(total) = self.total_size {
            if total == 0 {
                1.0
            } else {
                self.downloaded_size as f64 / total as f64
            }
        } else {
            0.0
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.status, DownloadStatus::Completed)
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, DownloadStatus::Downloading | DownloadStatus::Paused)
    }
}

pub struct DownloadManager {
    downloads: Arc<Mutex<HashMap<String, DownloadItem>>>,
    http_client: Client,
    max_concurrent_downloads: usize,
    active_downloads: Arc<Mutex<Vec<String>>>,
    history_file_path: String,
}

impl DownloadManager {
    pub fn new(max_concurrent_downloads: usize, history_file_path: String) -> Self {
        let mut manager = DownloadManager {
            downloads: Arc::new(Mutex::new(HashMap::new())),
            http_client: Client::new(),
            max_concurrent_downloads,
            active_downloads: Arc::new(Mutex::new(Vec::new())),
            history_file_path,
        };
        
        manager.load_history_from_file();
        manager
    }

    pub fn add_download(&self, url: String, destination_path: String) -> String {
        let id = self.generate_id();
        let filename = self.extract_filename(&url);
        
        let download_item = DownloadItem {
            id: id.clone(),
            url,
            filename,
            destination_path,
            total_size: None,
            downloaded_size: 0,
            status: DownloadStatus::Queued,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            download_speed: 0.0,
            time_remaining: None,
            content_type: None,
            auto_open_after_download: false,
        };

        let mut downloads = self.downloads.lock().unwrap();
        downloads.insert(id.clone(), download_item);
        self.save_history_to_file();
        id
    }

    pub fn start_download(&self, id: &str) -> Result<(), String> {
        let mut downloads = self.downloads.lock().unwrap();
        let mut download = downloads.get_mut(id).ok_or("Download not found")?.clone();
        drop(downloads);

        if matches!(download.status, DownloadStatus::Downloading | DownloadStatus::Completed) {
            return Err("Download already in progress or completed".to_string());
        }

        let active_downloads = self.active_downloads.clone();
        let downloads_ref = self.downloads.clone();
        let max_concurrent = self.max_concurrent_downloads;

        std::thread::spawn(move || {
            let mut active_lock = active_downloads.lock().unwrap();
            if active_lock.len() >= max_concurrent {
                let mut downloads = downloads_ref.lock().unwrap();
                if let Some(item) = downloads.get_mut(&id) {
                    item.status = DownloadStatus::Queued;
                }
                return;
            }
            active_lock.push(id.to_string());
            drop(active_lock);

            let result = DownloadManager::perform_download(id, downloads_ref.clone());
            
            let mut active_lock = active_downloads.lock().unwrap();
            active_lock.retain(|x| x != id);
        });

        Ok(())
    }

    fn perform_download(id: &str, downloads_ref: Arc<Mutex<HashMap<String, DownloadItem>>>) -> Result<(), String> {
        let (url, destination_path) = {
            let downloads = downloads_ref.lock().unwrap();
            let download = downloads.get(id).ok_or("Download not found")?;
            (download.url.clone(), download.destination_path.clone())
        };

        let response = Client::new()
            .get(&url)
            .send()
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            let mut downloads = downloads_ref.lock().unwrap();
            if let Some(download) = downloads.get_mut(id) {
                download.status = DownloadStatus::Failed(format!("HTTP {}", response.status()));
                download.error_message = Some(format!("HTTP {}", response.status()));
            }
            return Err(format!("HTTP {}", response.status()));
        }

        let total_size = response.content_length();
        let content_type = response.headers().get("content-type")
            .and_then(|v| v.to_str().ok()).map(|s| s.to_string());

        {
            let mut downloads = downloads_ref.lock().unwrap();
            if let Some(download) = downloads.get_mut(id) {
                download.total_size = total_size;
                download.content_type = content_type;
                download.status = DownloadStatus::Downloading;
                download.started_at = Some(Utc::now());
            }
        }

        let mut file = File::create(&destination_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut downloaded: u64 = 0;
        let mut buffer = [0; 8192];
        let mut start_time = Instant::now();
        let mut last_update = Instant::now();

        let mut response_bytes = response.bytes().map_err(|e| format!("Failed to read response: {}", e))?;

        while !response_bytes.is_empty() {
            let chunk_size = std::cmp::min(buffer.len(), response_bytes.len());
            buffer[..chunk_size].copy_from_slice(&response_bytes[..chunk_size]);
            
            file.write_all(&buffer[..chunk_size])
                .map_err(|e| format!("Failed to write to file: {}", e))?;

            downloaded += chunk_size as u64;

            if last_update.elapsed() >= Duration::from_millis(500) {
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 { downloaded as f64 / elapsed } else { 0.0 };

                let time_remaining = if speed > 0.0 {
                    total_size.map(|total| Duration::from_secs(((total - downloaded) as f64 / speed) as u64))
                } else {
                    None
                };

                let mut downloads = downloads_ref.lock().unwrap();
                if let Some(download) = downloads.get_mut(id) {
                    download.downloaded_size = downloaded;
                    download.download_speed = speed;
                    download.time_remaining = time_remaining;
                }
                last_update = Instant::now();
            }

            response_bytes = response_bytes[chunk_size..].to_vec().into();
        }

        {
            let mut downloads = downloads_ref.lock().unwrap();
            if let Some(download) = downloads.get_mut(id) {
                download.downloaded_size = downloaded;
                download.completed_at = Some(Utc::now());
                download.status = DownloadStatus::Completed;
            }
        }

        Ok(())
    }

    pub fn pause_download(&self, id: &str) -> Result<(), String> {
        let mut downloads = self.downloads.lock().unwrap();
        let download = downloads.get_mut(id).ok_or("Download not found")?;
        
        if matches!(download.status, DownloadStatus::Downloading) {
            download.status = DownloadStatus::Paused;
            Ok(())
        } else {
            Err("Download is not currently downloading".to_string())
        }
    }

    pub fn resume_download(&self, id: &str) -> Result<(), String> {
        let mut downloads = self.downloads.lock().unwrap();
        let download = downloads.get_mut(id).ok_or("Download not found")?;
        
        if matches!(download.status, DownloadStatus::Paused) {
            download.status = DownloadStatus::Queued;
            self.start_download(id)
        } else {
            Err("Download is not paused".to_string())
        }
    }

    pub fn cancel_download(&self, id: &str) -> Result<(), String> {
        let mut downloads = self.downloads.lock().unwrap();
        let download = downloads.get_mut(id).ok_or("Download not found")?;
        
        download.status = DownloadStatus::Canceled;
        Ok(())
    }

    pub fn remove_download(&self, id: &str) -> Result<(), String> {
        let mut downloads = self.downloads.lock().unwrap();
        downloads.remove(id).ok_or("Download not found")?;
        self.save_history_to_file();
        Ok(())
    }

    pub fn get_download(&self, id: &str) -> Option<DownloadItem> {
        let downloads = self.downloads.lock().unwrap();
        downloads.get(id).cloned()
    }

    pub fn get_all_downloads(&self) -> Vec<DownloadItem> {
        let downloads = self.downloads.lock().unwrap();
        downloads.values().cloned().collect()
    }

    pub fn get_active_downloads(&self) -> Vec<DownloadItem> {
        let downloads = self.downloads.lock().unwrap();
        downloads.values()
            .filter(|d| d.is_active())
            .cloned()
            .collect()
    }

    pub fn get_completed_downloads(&self) -> Vec<DownloadItem> {
        let downloads = self.downloads.lock().unwrap();
        downloads.values()
            .filter(|d| d.is_complete())
            .cloned()
            .collect()
    }

    pub fn get_queued_downloads(&self) -> Vec<DownloadItem> {
        let downloads = self.downloads.lock().unwrap();
        downloads.values()
            .filter(|d| matches!(d.status, DownloadStatus::Queued))
            .cloned()
            .collect()
    }

    pub fn set_auto_open_after_download(&self, id: &str, auto_open: bool) -> Result<(), String> {
        let mut downloads = self.downloads.lock().unwrap();
        let download = downloads.get_mut(id).ok_or("Download not found")?;
        download.auto_open_after_download = auto_open;
        Ok(())
    }

    pub fn clear_completed_downloads(&self) {
        let mut downloads = self.downloads.lock().unwrap();
        downloads.retain(|_, download| !matches!(download.status, DownloadStatus::Completed));
        self.save_history_to_file();
    }

    pub fn get_total_downloaded(&self) -> u64 {
        let downloads = self.downloads.lock().unwrap();
        downloads.values().map(|d| d.downloaded_size).sum()
    }

    pub fn get_total_pending_downloads(&self) -> usize {
        let downloads = self.downloads.lock().unwrap();
        downloads.values().filter(|d| matches!(d.status, DownloadStatus::Queued)).count()
    }

    pub fn get_total_active_downloads(&self) -> usize {
        let downloads = self.downloads.lock().unwrap();
        downloads.values().filter(|d| d.is_active()).count()
    }

    fn generate_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_micros();
        format!("dl_{:x}", now)
    }

    fn extract_filename(&self, url: &str) -> String {
        let parsed_url = Url::parse(url).unwrap_or_else(|_| Url::parse("http://example.com").unwrap());
        let path_segments: Vec<&str> = parsed_url.path_segments()
            .map(|c| c.collect())
            .unwrap_or_default();
        
        if let Some(filename) = path_segments.last() {
            if !filename.is_empty() {
                return filename.to_string();
            }
        }
        
        format!("download_{}", self.generate_id())
    }

    fn save_history_to_file(&self) {
        let downloads = self.downloads.lock().unwrap();
        if let Ok(mut file) = File::create(&self.history_file_path) {
            for download in downloads.values() {
                let line = format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    download.id,
                    download.url,
                    download.filename,
                    download.destination_path,
                    download.total_size.unwrap_or(0),
                    download.downloaded_size,
                    match download.status {
                        DownloadStatus::Queued => "Queued",
                        DownloadStatus::Downloading => "Downloading",
                        DownloadStatus::Paused => "Paused",
                        DownloadStatus::Completed => "Completed",
                        DownloadStatus::Failed(_) => "Failed",
                        DownloadStatus::Canceled => "Canceled",
                    },
                    download.created_at.to_rfc3339(),
                    download.started_at.map(|t| t.to_rfc3339()).unwrap_or_else(|| "null".to_string()),
                    download.completed_at.map(|t| t.to_rfc3339()).unwrap_or_else(|| "null".to_string()),
                    download.content_type.as_deref().unwrap_or("")
                );
                let _ = file.write_all(line.as_bytes());
            }
        }
    }

    fn load_history_from_file(&mut self) {
        if !Path::new(&self.history_file_path).exists() {
            return;
        }

        if let Ok(contents) = std::fs::read_to_string(&self.history_file_path) {
            let mut downloads = self.downloads.lock().unwrap();
            for line in contents.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 11 {
                    let status = match parts[6] {
                        "Queued" => DownloadStatus::Queued,
                        "Downloading" => DownloadStatus::Downloading,
                        "Paused" => DownloadStatus::Paused,
                        "Completed" => DownloadStatus::Completed,
                        "Failed" => DownloadStatus::Failed("Previously failed".to_string()),
                        "Canceled" => DownloadStatus::Canceled,
                        _ => DownloadStatus::Queued,
                    };

                    let download = DownloadItem {
                        id: parts[0].to_string(),
                        url: parts[1].to_string(),
                        filename: parts[2].to_string(),
                        destination_path: parts[3].to_string(),
                        total_size: if parts[4] == "0" { None } else { Some(parts[4].parse().unwrap_or(0)) },
                        downloaded_size: parts[5].parse().unwrap_or(0),
                        status,
                        created_at: DateTime::parse_from_rfc3339(parts[7]).unwrap().into(),
                        started_at: if parts[8] == "null" { None } else { Some(DateTime::parse_from_rfc3339(parts[8]).unwrap().into()) },
                        completed_at: if parts[9] == "null" { None } else { Some(DateTime::parse_from_rfc3339(parts[9]).unwrap().into()) },
                        error_message: None,
                        download_speed: 0.0,
                        time_remaining: None,
                        content_type: if parts[10].is_empty() { None } else { Some(parts[10].to_string()) },
                        auto_open_after_download: false,
                    };

                    downloads.insert(download.id.clone(), download);
                }
            }
        }
    }
}