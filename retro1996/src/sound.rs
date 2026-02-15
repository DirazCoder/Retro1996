use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AudioTrack {
    pub id: String,
    pub src: String,
    pub loop_count: Option<u32>,  // None = infinite, Some(0) = no repeat, Some(n) = repeat n times
    pub volume: f32,             // 0.0 to 1.0
    pub autoplay: bool,
    pub preload: PreloadType,
    pub is_playing: bool,
    pub is_muted: bool,
}

#[derive(Debug, Clone)]
pub enum PreloadType {
    None,
    Metadata,
    Auto,
}

#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub is_available: bool,
}

#[derive(Debug)]
pub enum AudioError {
    DeviceUnavailable,
    FileNotFound(String),
    UnsupportedFormat(String),
    IoError(String),
    DecodeError(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::DeviceUnavailable => write!(f, "Audio device unavailable"),
            AudioError::FileNotFound(path) => write!(f, "Audio file not found: {}", path),
            AudioError::UnsupportedFormat(format) => write!(f, "Unsupported audio format: {}", format),
            AudioError::IoError(msg) => write!(f, "IO Error: {}", msg),
            AudioError::DecodeError(msg) => write!(f, "Decode Error: {}", msg),
        }
    }
}

impl std::error::Error for AudioError {}

pub struct AudioEngine {
    pub tracks: HashMap<String, AudioTrack>,
    pub device: AudioDevice,
    pub master_volume: f32,
    pub is_muted: bool,
    pub bg_sound_enabled: bool,  // For BGSOUND compatibility
    pub current_bg_sound: Option<String>,
}

impl AudioEngine {
    pub fn new() -> Self {
        AudioEngine {
            tracks: HashMap::new(),
            device: AudioDevice {
                sample_rate: 22050,  // Standard for 1996 era
                channels: 1,         // Mono was common
                bits_per_sample: 8,  // 8-bit was typical for early web audio
                is_available: true,  // In a real implementation, this would check for audio devices
            },
            master_volume: 0.8,
            is_muted: false,
            bg_sound_enabled: true,
            current_bg_sound: None,
        }
    }

    pub fn load_audio(&mut self, id: String, src: String) -> Result<(), AudioError> {
        // Check if file exists
        if !Path::new(&src).exists() {
            return Err(AudioError::FileNotFound(src));
        }

        // Determine format and validate
        let format = self.get_audio_format(&src)?;
        
        if !self.is_supported_format(&format) {
            return Err(AudioError::UnsupportedFormat(format));
        }

        let track = AudioTrack {
            id: id.clone(),
            src,
            loop_count: None,  // Default to infinite for background sounds
            volume: 0.5,
            autoplay: false,
            preload: PreloadType::Auto,
            is_playing: false,
            is_muted: false,
        };

        self.tracks.insert(id, track);
        Ok(())
    }

    fn get_audio_format(&self, file_path: &str) -> Result<String, AudioError> {
        let path = Path::new(file_path);
        let extension = path.extension()
            .ok_or_else(|| AudioError::UnsupportedFormat("No file extension".to_string()))?
            .to_str()
            .ok_or_else(|| AudioError::UnsupportedFormat("Invalid file extension".to_string()))?
            .to_lowercase();

        Ok(extension)
    }

    fn is_supported_format(&self, format: &str) -> bool {
        matches!(format.as_ref(),
            "wav" | "au" | "snd" | "mid" | "midi" | "rmi" | "aiff" | "aif" | "aifc"
        )
    }

    pub fn play(&mut self, id: &str) -> Result<(), AudioError> {
        if !self.device.is_available {
            return Err(AudioError::DeviceUnavailable);
        }

        if let Some(track) = self.tracks.get_mut(id) {
            if self.is_muted || track.is_muted {
                return Ok(()); // Silently succeed if muted
            }

            // In a real implementation, this would play the audio
            track.is_playing = true;
            
            // Set as current background sound if it's a BGSOUND
            if id.starts_with("bg_") {
                self.current_bg_sound = Some(id.to_string());
            }
            
            println!("Playing audio: {} from {}", id, track.src);
            Ok(())
        } else {
            Err(AudioError::FileNotFound(id.to_string()))
        }
    }

    pub fn pause(&mut self, id: &str) -> Result<(), AudioError> {
        if let Some(track) = self.tracks.get_mut(id) {
            track.is_playing = false;
            println!("Paused audio: {}", id);
            Ok(())
        } else {
            Err(AudioError::FileNotFound(id.to_string()))
        }
    }

    pub fn stop(&mut self, id: &str) -> Result<(), AudioError> {
        if let Some(track) = self.tracks.get_mut(id) {
            track.is_playing = false;
            println!("Stopped audio: {}", id);
            Ok(())
        } else {
            Err(AudioError::FileNotFound(id.to_string()))
        }
    }

    pub fn set_volume(&mut self, id: &str, volume: f32) -> Result<(), AudioError> {
        if let Some(track) = self.tracks.get_mut(id) {
            track.volume = volume.clamp(0.0, 1.0);
            Ok(())
        } else {
            Err(AudioError::FileNotFound(id.to_string()))
        }
    }

    pub fn set_loop(&mut self, id: &str, loop_count: Option<u32>) -> Result<(), AudioError> {
        if let Some(track) = self.tracks.get_mut(id) {
            track.loop_count = loop_count;
            Ok(())
        } else {
            Err(AudioError::FileNotFound(id.to_string()))
        }
    }

    pub fn play_bg_sound(&mut self, src: String) -> Result<(), AudioError> {
        if !self.bg_sound_enabled {
            return Ok(()); // BGSOUND is disabled
        }

        // Create a special background sound track
        let id = format!("bg_{}", self.generate_track_id());
        self.load_audio(id.clone(), src)?;
        
        // Set loop to infinite for background sound
        self.set_loop(&id, None)?;
        
        // Play it
        self.play(&id)?;
        
        Ok(())
    }

    pub fn stop_bg_sound(&mut self) -> Result<(), AudioError> {
        if let Some(bg_id) = self.current_bg_sound.take() {
            self.stop(&bg_id)?;
            Ok(())
        } else {
            Err(AudioError::DeviceUnavailable) // No background sound playing
        }
    }

    pub fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    pub fn get_track_info(&self, id: &str) -> Option<&AudioTrack> {
        self.tracks.get(id)
    }

    pub fn get_playing_tracks(&self) -> Vec<&AudioTrack> {
        self.tracks.values()
            .filter(|track| track.is_playing)
            .collect()
    }

    pub fn enable_bg_sound(&mut self) {
        self.bg_sound_enabled = true;
    }

    pub fn disable_bg_sound(&mut self) {
        self.bg_sound_enabled = false;
        let _ = self.stop_bg_sound(); // Stop any currently playing background sound
    }

    fn generate_track_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();
        format!("{}", now)
    }

    pub fn cleanup_stopped_tracks(&mut self) {
        // In a real implementation, this would free memory for stopped tracks
        // For now, we'll just keep all tracks
    }

    pub fn get_supported_formats(&self) -> Vec<&str> {
        vec!["wav", "au", "snd", "mid", "midi", "rmi", "aiff", "aif", "aifc"]
    }

    pub fn is_playing_bg_sound(&self) -> bool {
        self.current_bg_sound.is_some()
    }
}

// BGSOUND compatibility layer
pub struct BgSoundEmulator {
    pub audio_engine: AudioEngine,
    pub is_enabled: bool,
}

impl BgSoundEmulator {
    pub fn new() -> Self {
        BgSoundEmulator {
            audio_engine: AudioEngine::new(),
            is_enabled: true,
        }
    }

    pub fn play_background_sound(&mut self, src: String, loop_count: Option<u32>) -> Result<(), AudioError> {
        if !self.is_enabled {
            return Ok(());
        }

        // Load and play the background sound
        let id = format!("bg_{}", self.audio_engine.generate_track_id());
        self.audio_engine.load_audio(id.clone(), src)?;
        
        if let Some(loop_count) = loop_count {
            self.audio_engine.set_loop(&id, Some(loop_count))?;
        } else {
            // Default to infinite loop for BGSOUND
            self.audio_engine.set_loop(&id, None)?;
        }
        
        self.audio_engine.play(&id)?;
        self.audio_engine.current_bg_sound = Some(id);
        
        Ok(())
    }

    pub fn stop_background_sound(&mut self) -> Result<(), AudioError> {
        if let Some(bg_id) = self.audio_engine.current_bg_sound.clone() {
            self.audio_engine.stop(&bg_id)?;
            self.audio_engine.current_bg_sound = None;
            Ok(())
        } else {
            Err(AudioError::DeviceUnavailable)
        }
    }

    pub fn is_background_sound_playing(&self) -> bool {
        self.audio_engine.is_playing_bg_sound()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.is_enabled = enabled;
        if !enabled {
            let _ = self.stop_background_sound(); // Stop any playing background sound
        }
    }
}

// Audio utility functions for 1996 compatibility
pub struct AudioUtils;

impl AudioUtils {
    pub fn is_audio_file(path: &str) -> bool {
        let path_lower = path.to_lowercase();
        path_lower.ends_with(".wav") || 
        path_lower.ends_with(".au") || 
        path_lower.ends_with(".snd") || 
        path_lower.ends_with(".mid") || 
        path_lower.ends_with(".midi") || 
        path_lower.ends_with(".rmi") || 
        path_lower.ends_with(".aiff") || 
        path_lower.ends_with(".aif") || 
        path_lower.ends_with(".aifc")
    }

    pub fn get_file_size(path: &str) -> Option<u64> {
        if Path::new(path).exists() {
            fs::metadata(path).ok().map(|m| m.len())
        } else {
            None
        }
    }

    pub fn is_small_file(path: &str, max_size_kb: u64) -> bool {
        if let Some(size) = Self::get_file_size(path) {
            size <= max_size_kb * 1024
        } else {
            false
        }
    }

    pub fn format_duration(seconds: u32) -> String {
        let mins = seconds / 60;
        let secs = seconds % 60;
        format!("{}:{:02}", mins, secs)
    }
}

// Audio mixer for combining multiple audio sources
pub struct AudioMixer {
    pub tracks: Vec<AudioTrack>,
    pub master_volume: f32,
    pub is_processing: bool,
}

impl AudioMixer {
    pub fn new() -> Self {
        AudioMixer {
            tracks: Vec::new(),
            master_volume: 1.0,
            is_processing: false,
        }
    }

    pub fn add_track(&mut self, track: AudioTrack) {
        self.tracks.push(track);
    }

    pub fn remove_track(&mut self, id: &str) -> bool {
        let initial_len = self.tracks.len();
        self.tracks.retain(|t| t.id != id);
        self.tracks.len() != initial_len
    }

    pub fn mix_audio(&mut self) {
        // In a real implementation, this would mix the audio tracks
        // For now, we'll just simulate the mixing process
        self.is_processing = true;
        println!("Mixing {} audio tracks", self.tracks.len());
        self.is_processing = false;
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_detection() {
        let engine = AudioEngine::new();
        assert!(engine.is_supported_format("wav"));
        assert!(engine.is_supported_format("mid"));
        assert!(!engine.is_supported_format("mp3"));
    }

    #[test]
    fn test_is_audio_file() {
        assert!(AudioUtils::is_audio_file("test.wav"));
        assert!(AudioUtils::is_audio_file("music.mid"));
        assert!(!AudioUtils::is_audio_file("document.txt"));
    }
}