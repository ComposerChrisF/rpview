use std::time::Instant;

/// Per-image state that is cached and persisted
#[derive(Debug, Clone)]
pub struct ImageState {
    /// Current zoom level (1.0 = 100%, 0.1 = 10%, 20.0 = 2000%)
    pub zoom: f32,
    
    /// Pan position (x, y) in pixels
    pub pan: (f32, f32),
    
    /// Whether the image is at fit-to-window size
    pub is_fit_to_window: bool,
    
    /// Last time this state was accessed (for LRU cache)
    pub last_accessed: Instant,
    
    /// Filter settings (brightness, contrast, gamma)
    pub filters: FilterSettings,
    
    /// Whether filters are currently enabled
    pub filters_enabled: bool,
    
    /// Animation state (if applicable)
    pub animation: Option<AnimationState>,
}

impl ImageState {
    /// Create a new ImageState with default values
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            pan: (0.0, 0.0),
            is_fit_to_window: true,
            last_accessed: Instant::now(),
            filters: FilterSettings::default(),
            filters_enabled: true,
            animation: None,
        }
    }
    
    /// Update the last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

impl Default for ImageState {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter settings for image processing
#[derive(Debug, Clone, Copy)]
pub struct FilterSettings {
    /// Brightness adjustment (-100 to +100)
    pub brightness: i32,
    
    /// Contrast adjustment (-100 to +100)
    pub contrast: i32,
    
    /// Gamma correction (0.1 to 10.0)
    pub gamma: f32,
}

impl Default for FilterSettings {
    fn default() -> Self {
        Self {
            brightness: 0,
            contrast: 0,
            gamma: 1.0,
        }
    }
}

/// Animation state for animated images (GIF, WEBP)
#[derive(Debug, Clone)]
pub struct AnimationState {
    /// Current frame index
    pub current_frame: usize,
    
    /// Whether animation is playing
    pub is_playing: bool,
    
    /// Total number of frames
    pub frame_count: usize,
    
    /// Frame durations in milliseconds
    pub frame_durations: Vec<u32>,
}

impl AnimationState {
    pub fn new(frame_count: usize, frame_durations: Vec<u32>) -> Self {
        Self {
            current_frame: 0,
            is_playing: true,
            frame_count,
            frame_durations,
        }
    }
}
