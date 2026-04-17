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

    /// Whether user has chosen to override the size limit for this image
    pub override_size_limit: bool,
}

impl ImageState {
    /// Create a new ImageState with default values
    pub fn new() -> Self {
        Self::new_with_filter_defaults(FilterSettings::default())
    }

    /// Create a new ImageState with custom default filters
    pub fn new_with_filter_defaults(default_filters: FilterSettings) -> Self {
        Self {
            zoom: 1.0,
            pan: (0.0, 0.0),
            is_fit_to_window: true,
            last_accessed: Instant::now(),
            filters: default_filters,
            filters_enabled: true,
            animation: None,
            override_size_limit: false,
        }
    }
}

impl Default for ImageState {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter settings for image processing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterSettings {
    /// Brightness adjustment (-100.0 to +100.0)
    pub brightness: f32,

    /// Contrast adjustment (-100.0 to +100.0)
    pub contrast: f32,

    /// Gamma correction (0.1 to 10.0)
    pub gamma: f32,
}

impl Default for FilterSettings {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 0.0,
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

    /// Whether the next frame has been preloaded and is ready to display
    pub next_frame_ready: bool,
}

impl AnimationState {
    pub fn new(frame_count: usize, frame_durations: Vec<u32>) -> Self {
        Self {
            current_frame: 0,
            is_playing: true,
            frame_count,
            frame_durations,
            next_frame_ready: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- ImageState defaults --------------------------------------------------

    #[test]
    fn image_state_new_has_correct_defaults() {
        let state = ImageState::new();
        assert_eq!(state.zoom, 1.0);
        assert_eq!(state.pan, (0.0, 0.0));
        assert!(state.is_fit_to_window);
        assert!(state.filters_enabled);
        assert!(state.animation.is_none());
        assert!(!state.override_size_limit);
    }

    #[test]
    fn image_state_default_matches_new() {
        let from_new = ImageState::new();
        let from_default = ImageState::default();
        assert_eq!(from_new.zoom, from_default.zoom);
        assert_eq!(from_new.pan, from_default.pan);
        assert_eq!(from_new.is_fit_to_window, from_default.is_fit_to_window);
        assert_eq!(from_new.filters_enabled, from_default.filters_enabled);
        assert_eq!(from_new.filters, from_default.filters);
    }

    #[test]
    fn image_state_with_custom_filters() {
        let custom = FilterSettings {
            brightness: 25.0,
            contrast: -10.0,
            gamma: 2.2,
        };
        let state = ImageState::new_with_filter_defaults(custom);
        assert_eq!(state.filters.brightness, 25.0);
        assert_eq!(state.filters.contrast, -10.0);
        assert_eq!(state.filters.gamma, 2.2);
        // Other fields should still have defaults
        assert_eq!(state.zoom, 1.0);
        assert!(state.is_fit_to_window);
    }

    // -- FilterSettings defaults ----------------------------------------------

    #[test]
    fn filter_settings_default_is_neutral() {
        let f = FilterSettings::default();
        assert_eq!(f.brightness, 0.0);
        assert_eq!(f.contrast, 0.0);
        assert_eq!(f.gamma, 1.0);
    }

    #[test]
    fn filter_settings_partial_eq() {
        let a = FilterSettings { brightness: 1.0, contrast: 2.0, gamma: 3.0 };
        let b = FilterSettings { brightness: 1.0, contrast: 2.0, gamma: 3.0 };
        let c = FilterSettings { brightness: 1.0, contrast: 2.0, gamma: 3.1 };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // -- AnimationState -------------------------------------------------------

    #[test]
    fn animation_state_new_starts_at_frame_zero() {
        let durations = vec![100, 200, 150];
        let anim = AnimationState::new(3, durations.clone());
        assert_eq!(anim.current_frame, 0);
        assert!(anim.is_playing);
        assert_eq!(anim.frame_count, 3);
        assert_eq!(anim.frame_durations, durations);
        assert!(!anim.next_frame_ready);
    }

    #[test]
    fn animation_state_single_frame() {
        let anim = AnimationState::new(1, vec![0]);
        assert_eq!(anim.frame_count, 1);
        assert_eq!(anim.frame_durations.len(), 1);
    }

    #[test]
    fn animation_state_empty_durations() {
        let anim = AnimationState::new(0, vec![]);
        assert_eq!(anim.frame_count, 0);
        assert!(anim.frame_durations.is_empty());
    }
}
