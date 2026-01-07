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

#[cfg(test)]
mod tests {
    use super::*;

    // Test constants
    const DEFAULT_CACHE_SIZE: usize = 1000;
    const SMALL_CACHE_SIZE: usize = 2;

    #[test]
    fn test_sort_mode_default() {
        // Arrange & Act
        let mode = SortMode::default();

        // Assert - default should be Alphabetical (via derive(Default) with #[default])
        assert_eq!(mode, SortMode::Alphabetical);
    }

    #[test]
    fn test_sort_mode_equality() {
        // Arrange & Act & Assert
        assert_eq!(SortMode::Alphabetical, SortMode::Alphabetical);
        assert_eq!(SortMode::ModifiedDate, SortMode::ModifiedDate);
        assert_ne!(SortMode::Alphabetical, SortMode::ModifiedDate);
    }

    #[test]
    fn test_sort_mode_clone() {
        // Arrange
        let mode = SortMode::ModifiedDate;

        // Act
        let cloned = mode.clone();

        // Assert
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_sort_mode_copy() {
        // Arrange
        let mode = SortMode::Alphabetical;

        // Act - Copy trait allows direct assignment
        let copied: SortMode = mode;

        // Assert - both should be equal and mode still valid (Copy)
        assert_eq!(mode, copied);
        assert_eq!(mode, SortMode::Alphabetical);
    }

    #[test]
    fn test_app_state_new_uses_default_sort_mode() {
        // Arrange & Act
        let state = AppState::new(Vec::new());

        // Assert
        assert_eq!(state.sort_mode, SortMode::default());
        assert_eq!(state.sort_mode, SortMode::Alphabetical);
    }

    #[test]
    fn test_app_state_new_with_settings() {
        // Arrange
        let paths = vec![
            PathBuf::from("zebra.png"),
            PathBuf::from("apple.jpg"),
        ];

        // Act
        let state = AppState::new_with_settings(paths, 0, SortMode::Alphabetical, 500);

        // Assert
        assert_eq!(state.max_cache_size, 500);
        assert_eq!(state.sort_mode, SortMode::Alphabetical);
        // Images should be sorted alphabetically
        assert_eq!(
            state.image_paths[0].file_name().unwrap().to_string_lossy(),
            "apple.jpg"
        );
    }

    #[test]
    fn test_app_state_new_with_settings_modified_date() {
        // Arrange
        let paths = vec![PathBuf::from("test.png")];

        // Act
        let state = AppState::new_with_settings(paths, 0, SortMode::ModifiedDate, DEFAULT_CACHE_SIZE);

        // Assert
        assert_eq!(state.sort_mode, SortMode::ModifiedDate);
    }

    #[test]
    fn test_set_sort_mode_same_mode_no_resort() {
        // Arrange
        let paths = vec![
            PathBuf::from("zebra.png"),
            PathBuf::from("apple.jpg"),
        ];
        let mut state = AppState::new(paths.clone());

        // Act - set to same mode (Alphabetical is default)
        state.set_sort_mode(SortMode::Alphabetical);

        // Assert - order should be unchanged from initial (not sorted on new())
        assert_eq!(state.image_paths[0], paths[0]);
        assert_eq!(state.image_paths[1], paths[1]);
    }

    #[test]
    fn test_set_sort_mode_different_mode_triggers_sort() {
        // Arrange
        let paths = vec![
            PathBuf::from("zebra.png"),
            PathBuf::from("apple.jpg"),
        ];
        let mut state = AppState::new(paths);

        // Act - change to ModifiedDate, then back to Alphabetical
        state.set_sort_mode(SortMode::ModifiedDate);
        state.set_sort_mode(SortMode::Alphabetical);

        // Assert - should now be sorted alphabetically
        assert_eq!(
            state.image_paths[0].file_name().unwrap().to_string_lossy(),
            "apple.jpg"
        );
    }

    #[test]
    fn test_next_image_with_wrap_true() {
        // Arrange
        let paths = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
        let mut state = AppState::new(paths);
        state.current_index = 1; // Last image

        // Act
        state.next_image_with_wrap(true);

        // Assert - should wrap to first
        assert_eq!(state.current_index, 0);
    }

    #[test]
    fn test_next_image_with_wrap_false() {
        // Arrange
        let paths = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
        let mut state = AppState::new(paths);
        state.current_index = 1; // Last image

        // Act
        state.next_image_with_wrap(false);

        // Assert - should stay at last
        assert_eq!(state.current_index, 1);
    }

    #[test]
    fn test_previous_image_with_wrap_true() {
        // Arrange
        let paths = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
        let mut state = AppState::new(paths);
        // current_index is 0 (first image)

        // Act
        state.previous_image_with_wrap(true);

        // Assert - should wrap to last
        assert_eq!(state.current_index, 1);
    }

    #[test]
    fn test_previous_image_with_wrap_false() {
        // Arrange
        let paths = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
        let mut state = AppState::new(paths);
        // current_index is 0 (first image)

        // Act
        state.previous_image_with_wrap(false);

        // Assert - should stay at first
        assert_eq!(state.current_index, 0);
    }

    #[test]
    fn test_get_current_state_creates_new_with_defaults() {
        // Arrange
        let paths = vec![PathBuf::from("test.png")];
        let mut state = AppState::new(paths);
        let custom_filters = FilterSettings {
            brightness: 10.0,
            contrast: 20.0,
            gamma: 1.5,
        };

        // Act
        let image_state = state.get_current_state(custom_filters);

        // Assert - new state should use provided default filters
        assert_eq!(image_state.filters.brightness, 10.0);
        assert_eq!(image_state.filters.contrast, 20.0);
        assert_eq!(image_state.filters.gamma, 1.5);
    }

    #[test]
    fn test_get_current_state_returns_cached() {
        // Arrange
        let paths = vec![PathBuf::from("test.png")];
        let mut state = AppState::new(paths);

        // Save a custom state
        let mut custom_state = ImageState::new();
        custom_state.zoom = 2.5;
        state.save_current_state(custom_state);

        // Act - get state with different defaults (should return cached)
        let image_state = state.get_current_state(FilterSettings::default());

        // Assert - should return cached state, not new one
        assert_eq!(image_state.zoom, 2.5);
    }

    #[test]
    fn test_get_current_state_empty_list() {
        // Arrange
        let mut state = AppState::new(Vec::new());
        let custom_filters = FilterSettings {
            brightness: 5.0,
            contrast: 5.0,
            gamma: 1.2,
        };

        // Act
        let image_state = state.get_current_state(custom_filters);

        // Assert - should return default state with provided filters
        assert_eq!(image_state.filters.brightness, 5.0);
        assert_eq!(image_state.zoom, 1.0);
    }

    #[test]
    fn test_save_current_state_evicts_on_cache_full() {
        // Arrange
        let mut state = AppState::new(vec![PathBuf::from("current.png")]);
        state.max_cache_size = SMALL_CACHE_SIZE;

        // Fill the cache to capacity
        for i in 0..SMALL_CACHE_SIZE {
            let path = PathBuf::from(format!("image{}.png", i));
            let img_state = ImageState::new();
            state.image_states.insert(path, img_state);
        }

        // Act - save state for current image (should trigger eviction)
        let new_state = ImageState::new();
        state.save_current_state(new_state);

        // Assert - cache should not exceed max_cache_size
        assert!(state.image_states.len() <= state.max_cache_size);
    }

    #[test]
    fn test_save_current_state_empty_list_no_panic() {
        // Arrange
        let mut state = AppState::new(Vec::new());

        // Act - should not panic
        state.save_current_state(ImageState::new());

        // Assert - cache should be empty (no current image to save for)
        assert!(state.image_states.is_empty());
    }

    #[test]
    fn test_app_state_default_cache_size() {
        // Arrange & Act
        let state = AppState::default();

        // Assert
        assert_eq!(state.max_cache_size, DEFAULT_CACHE_SIZE);
    }
}
