use std::collections::HashMap;
use std::path::PathBuf;
use super::image_state::{ImageState, FilterSettings};

/// Sort mode for image list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    /// Alphabetical (case-insensitive)
    #[default]
    Alphabetical,

    /// Modified date (newest first)
    ModifiedDate,
}

/// Application-wide state
#[derive(Debug, Clone)]
pub struct AppState {
    /// List of image file paths
    pub image_paths: Vec<PathBuf>,
    
    /// Current image index
    pub current_index: usize,
    
    /// Current sort mode
    pub sort_mode: SortMode,
    
    /// Cache of per-image states (LRU cache with max 1000 items)
    pub image_states: HashMap<PathBuf, ImageState>,
    
    /// Maximum cache size
    pub max_cache_size: usize,
}

impl AppState {
    /// Create a new AppState
    pub fn new(image_paths: Vec<PathBuf>) -> Self {
        Self::new_with_index(image_paths, 0)
    }
    
    /// Create a new AppState with a specific starting index
    pub fn new_with_index(image_paths: Vec<PathBuf>, start_index: usize) -> Self {
        let current_index = if start_index < image_paths.len() {
            start_index
        } else {
            0
        };
        
        Self {
            image_paths,
            current_index,
            sort_mode: SortMode::default(),
            image_states: HashMap::new(),
            max_cache_size: 1000,
        }
    }
    
    /// Create a new AppState with settings
    pub fn new_with_settings(
        image_paths: Vec<PathBuf>, 
        start_index: usize,
        default_sort_mode: SortMode,
        cache_size: usize,
    ) -> Self {
        let current_index = if start_index < image_paths.len() {
            start_index
        } else {
            0
        };
        
        let mut state = Self {
            image_paths,
            current_index,
            sort_mode: default_sort_mode,
            image_states: HashMap::new(),
            max_cache_size: cache_size,
        };
        
        // Sort images according to the default sort mode
        state.sort_images();
        
        state
    }
    
    /// Get the current image path
    pub fn current_image(&self) -> Option<&PathBuf> {
        self.image_paths.get(self.current_index)
    }
    
    /// Get the next image path (for preloading)
    pub fn next_image_path(&self) -> Option<&PathBuf> {
        if self.image_paths.is_empty() {
            return None;
        }
        let next_index = (self.current_index + 1) % self.image_paths.len();
        self.image_paths.get(next_index)
    }
    
    /// Get the previous image path (for preloading)
    pub fn previous_image_path(&self) -> Option<&PathBuf> {
        if self.image_paths.is_empty() {
            return None;
        }
        let prev_index = if self.current_index == 0 {
            self.image_paths.len() - 1
        } else {
            self.current_index - 1
        };
        self.image_paths.get(prev_index)
    }
    
    /// Navigate to the next image
    #[allow(dead_code)]
    pub fn next_image(&mut self) {
        self.next_image_with_wrap(true);
    }
    
    /// Navigate to the next image with optional wrapping
    pub fn next_image_with_wrap(&mut self, wrap: bool) {
        if !self.image_paths.is_empty() {
            if self.current_index + 1 < self.image_paths.len() {
                self.current_index += 1;
            } else if wrap {
                self.current_index = 0;
            }
            // If not wrapping and at end, stay at current position
        }
    }
    
    /// Navigate to the previous image
    #[allow(dead_code)]
    pub fn previous_image(&mut self) {
        self.previous_image_with_wrap(true);
    }
    
    /// Navigate to the previous image with optional wrapping
    pub fn previous_image_with_wrap(&mut self, wrap: bool) {
        if !self.image_paths.is_empty() {
            if self.current_index > 0 {
                self.current_index -= 1;
            } else if wrap {
                self.current_index = self.image_paths.len() - 1;
            }
            // If not wrapping and at start, stay at current position
        }
    }
    
    /// Get the state for the current image, creating a default if it doesn't exist
    pub fn get_current_state(&mut self, default_filters: FilterSettings) -> ImageState {
        if let Some(path) = self.current_image() {
            self.image_states
                .entry(path.clone())
                .or_insert_with(|| ImageState::new_with_filter_defaults(default_filters))
                .clone()
        } else {
            ImageState::new_with_filter_defaults(default_filters)
        }
    }
    
    /// Save the state for the current image
    pub fn save_current_state(&mut self, state: ImageState) {
        if let Some(path) = self.current_image().cloned() {
            // Evict old entries if cache is too large
            if self.image_states.len() >= self.max_cache_size {
                self.evict_oldest_state();
            }
            
            self.image_states.insert(path, state);
        }
    }
    
    /// Evict the oldest (least recently accessed) state from cache
    fn evict_oldest_state(&mut self) {
        if let Some(oldest_path) = self.image_states
            .iter()
            .min_by_key(|(_, state)| state.last_accessed)
            .map(|(path, _)| path.clone())
        {
            self.image_states.remove(&oldest_path);
        }
    }
    
    /// Set the sort mode and re-sort the image list
    pub fn set_sort_mode(&mut self, mode: SortMode) {
        if self.sort_mode != mode {
            self.sort_mode = mode;
            self.sort_images();
        }
    }
    
    /// Sort the image list according to the current sort mode
    fn sort_images(&mut self) {
        match self.sort_mode {
            SortMode::Alphabetical => {
                self.image_paths.sort_by(|a, b| {
                    a.to_string_lossy()
                        .to_lowercase()
                        .cmp(&b.to_string_lossy().to_lowercase())
                });
            }
            SortMode::ModifiedDate => {
                self.image_paths.sort_by(|a, b| {
                    let a_modified = std::fs::metadata(a)
                        .and_then(|m| m.modified())
                        .ok();
                    let b_modified = std::fs::metadata(b)
                        .and_then(|m| m.modified())
                        .ok();
                    
                    // Newest first, so reverse the comparison
                    b_modified.cmp(&a_modified)
                });
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
