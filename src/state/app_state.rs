use std::collections::HashMap;
use std::path::PathBuf;
use super::image_state::ImageState;

/// Sort mode for image list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    /// Alphabetical (case-insensitive)
    Alphabetical,
    
    /// Modified date (newest first)
    ModifiedDate,
}

impl Default for SortMode {
    fn default() -> Self {
        SortMode::Alphabetical
    }
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
        Self {
            image_paths,
            current_index: 0,
            sort_mode: SortMode::default(),
            image_states: HashMap::new(),
            max_cache_size: 1000,
        }
    }
    
    /// Get the current image path
    pub fn current_image(&self) -> Option<&PathBuf> {
        self.image_paths.get(self.current_index)
    }
    
    /// Navigate to the next image
    pub fn next_image(&mut self) {
        if !self.image_paths.is_empty() {
            self.current_index = (self.current_index + 1) % self.image_paths.len();
        }
    }
    
    /// Navigate to the previous image
    pub fn previous_image(&mut self) {
        if !self.image_paths.is_empty() {
            if self.current_index == 0 {
                self.current_index = self.image_paths.len() - 1;
            } else {
                self.current_index -= 1;
            }
        }
    }
    
    /// Get the state for the current image, creating a default if it doesn't exist
    pub fn get_current_state(&mut self) -> ImageState {
        if let Some(path) = self.current_image() {
            self.image_states
                .entry(path.clone())
                .or_insert_with(ImageState::new)
                .clone()
        } else {
            ImageState::new()
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
