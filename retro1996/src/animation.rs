use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum AnimationType {
    Gif,
    Marquee,
    Blink,
    Other,
}

#[derive(Debug, Clone)]
pub struct AnimationFrame {
    pub pixels: Vec<u8>,
    pub duration: Duration,
    pub disposal_method: DisposalMethod,
    pub transparent_color_index: Option<u8>,
}

#[derive(Debug, Clone)]
pub enum DisposalMethod {
    Leave,
    RestoreBackground,
    RestorePrevious,
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub animation_type: AnimationType,
    pub frames: Vec<AnimationFrame>,
    pub current_frame: usize,
    pub last_update: Instant,
    pub playing: bool,
    pub loop_count: Option<u16>,
    pub current_loop: u16,
    pub width: u32,
    pub height: u32,
    pub background_color_index: Option<u8>,
    pub palette: Vec<(u8, u8, u8)>, 
}

impl Animation {
    pub fn new(animation_type: AnimationType, width: u32, height: u32) -> Self {
        Animation {
            animation_type,
            frames: Vec::new(),
            current_frame: 0,
            last_update: Instant::now(),
            playing: false,
            loop_count: None,
            current_loop: 0,
            width,
            height,
            background_color_index: None,
            palette: vec![(0, 0, 0); 256],
        }
    }

    pub fn add_frame(&mut self, frame: AnimationFrame) {
        self.frames.push(frame);
    }

    pub fn start(&mut self) {
        self.playing = true;
        self.last_update = Instant::now();
        self.current_frame = 0;
        self.current_loop = 0;
    }

    pub fn stop(&mut self) {
        self.playing = false;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn resume(&mut self) {
        self.playing = true;
        self.last_update = Instant::now();
    }

    pub fn update(&mut self) -> bool {
        if !self.playing || self.frames.is_empty() {
            return false;
        }

        let elapsed = self.last_update.elapsed();
        let current_frame_duration = self.frames[self.current_frame].duration;

        if elapsed >= current_frame_duration {
            self.last_update = Instant::now();
            
            self.current_frame += 1;
            
            if self.current_frame >= self.frames.len() {
                self.current_frame = 0;
                self.current_loop += 1;
                
                if let Some(max_loops) = self.loop_count {
                    if self.current_loop >= max_loops {
                        self.playing = false;
                        return false;
                    }
                }
            }
            
            true
        } else {
            false
        }
    }

    pub fn get_current_frame(&self) -> Option<&AnimationFrame> {
        if self.current_frame < self.frames.len() {
            Some(&self.frames[self.current_frame])
        } else {
            None
        }
    }

    pub fn get_current_frame_image(&self) -> Option<Vec<u8>> {
        if let Some(frame) = self.get_current_frame() {
            Some(frame.pixels.clone())
        } else {
            None
        }
    }

    pub fn set_loop_count(&mut self, count: Option<u16>) {
        self.loop_count = count;
    }

    pub fn is_finished(&self) -> bool {
        !self.playing && self.current_loop > 0
    }

    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.current_loop = 0;
        self.playing = false;
        self.last_update = Instant::now();
    }

    pub fn get_progress(&self) -> f32 {
        if self.frames.is_empty() {
            0.0
        } else {
            self.current_frame as f32 / self.frames.len() as f32
        }
    }

    pub fn get_total_duration(&self) -> Duration {
        let total_per_loop: Duration = self.frames.iter()
            .map(|frame| frame.duration)
            .sum();
        
        match self.loop_count {
            Some(count) => total_per_loop * count as u32,
            None => total_per_loop, 
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarqueeAnimation {
    pub text: String,
    pub direction: MarqueeDirection,
    pub speed: u32,
    pub position: i32,
    pub width: u32,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub enum MarqueeDirection {
    Left,
    Right,
    Up,
    Down,
}

impl MarqueeAnimation {
    pub fn new(text: String, direction: MarqueeDirection, speed: u32, width: u32) -> Self {
        MarqueeAnimation {
            text,
            direction,
            speed,
            position: 0,
            width,
            visible: true,
        }
    }

    pub fn update(&mut self) {
        match self.direction {
            MarqueeDirection::Left => self.position -= self.speed as i32,
            MarqueeDirection::Right => self.position += self.speed as i32,
            MarqueeDirection::Up => self.position -= self.speed as i32,
            MarqueeDirection::Down => self.position += self.speed as i32,
        }
    }

    pub fn reset(&mut self) {
        match self.direction {
            MarqueeDirection::Left | MarqueeDirection::Right => self.position = self.width as i32,
            MarqueeDirection::Up | MarqueeDirection::Down => self.position = self.width as i32,
        }
    }

    pub fn get_display_text(&self) -> String {
        self.text.clone()
    }
}

#[derive(Debug, Clone)]
pub struct BlinkAnimation {
    pub visible: bool,
    pub blink_speed: Duration,
    pub last_toggle: Instant,
    pub enabled: bool,
}

impl BlinkAnimation {
    pub fn new(speed: Duration) -> Self {
        BlinkAnimation {
            visible: true,
            blink_speed: speed,
            last_toggle: Instant::now(),
            enabled: true,
        }
    }

    pub fn update(&mut self) -> bool {
        if !self.enabled {
            return false;
        }

        if self.last_toggle.elapsed() >= self.blink_speed {
            self.visible = !self.visible;
            self.last_toggle = Instant::now();
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.visible = true;
        self.last_toggle = Instant::now();
    }
}

#[derive(Debug, Clone)]
pub struct AnimationManager {
    pub animations: HashMap<String, Animation>,
    pub marquee_animations: HashMap<String, MarqueeAnimation>,
    pub blink_animations: HashMap<String, BlinkAnimation>,
    pub active: bool,
}

impl AnimationManager {
    pub fn new() -> Self {
        AnimationManager {
            animations: HashMap::new(),
            marquee_animations: HashMap::new(),
            blink_animations: HashMap::new(),
            active: true,
        }
    }

    pub fn add_animation(&mut self, id: String, animation: Animation) {
        self.animations.insert(id, animation);
    }

    pub fn add_marquee_animation(&mut self, id: String, animation: MarqueeAnimation) {
        self.marquee_animations.insert(id, animation);
    }

    pub fn add_blink_animation(&mut self, id: String, animation: BlinkAnimation) {
        self.blink_animations.insert(id, animation);
    }

    pub fn update_all(&mut self) {
        if !self.active {
            return;
        }

        let mut animations_to_remove = Vec::new();
        
        for (id, animation) in self.animations.iter_mut() {
            if animation.update() {
                if animation.is_finished() {
                    animations_to_remove.push(id.clone());
                }
            }
        }
        
        for id in animations_to_remove {
            self.animations.remove(&id);
        }

        for animation in self.marquee_animations.values_mut() {
            animation.update();
        }

        for animation in self.blink_animations.values_mut() {
            animation.update();
        }
    }

    pub fn get_animation(&self, id: &str) -> Option<&Animation> {
        self.animations.get(id)
    }

    pub fn get_animation_mut(&mut self, id: &str) -> Option<&mut Animation> {
        self.animations.get_mut(id)
    }

    pub fn remove_animation(&mut self, id: &str) -> Option<Animation> {
        self.animations.remove(id)
    }

    pub fn start_animation(&mut self, id: &str) {
        if let Some(animation) = self.animations.get_mut(id) {
            animation.start();
        }
    }

    pub fn stop_animation(&mut self, id: &str) {
        if let Some(animation) = self.animations.get_mut(id) {
            animation.stop();
        }
    }

    pub fn get_marquee_animation(&self, id: &str) -> Option<&MarqueeAnimation> {
        self.marquee_animations.get(id)
    }

    pub fn get_blink_animation(&self, id: &str) -> Option<&BlinkAnimation> {
        self.blink_animations.get(id)
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn clear_all(&mut self) {
        self.animations.clear();
        self.marquee_animations.clear();
        self.blink_animations.clear();
    }

    pub fn is_animation_playing(&self, id: &str) -> bool {
        if let Some(animation) = self.animations.get(id) {
            animation.playing
        } else {
            false
        }
    }
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}