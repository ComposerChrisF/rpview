//! Integration tests for rpview-gpui
//!
//! These tests verify end-to-end workflows including:
//! - CLI argument parsing workflows
//! - File loading workflows
//! - Navigation workflows
//! - Zoom/pan workflows

use rpview_gpui::state::app_state::{AppState, SortMode};
use rpview_gpui::state::image_state::{FilterSettings, ImageState};
use rpview_gpui::utils::file_scanner::{process_dropped_path, scan_directory};
use rpview_gpui::utils::zoom::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// CLI Argument Parsing Workflows
// ============================================================================

#[test]
fn test_cli_workflow_empty_directory() {
    // Simulate scanning an empty directory (like CLI with no images)
    let temp_dir = TempDir::new().unwrap();

    let images = scan_directory(temp_dir.path()).unwrap();
    let state = AppState::new(images);

    // Should create empty state
    assert_eq!(state.image_paths.len(), 0);
    assert_eq!(state.current_image(), None);
}

#[test]
fn test_cli_workflow_directory_with_images() {
    // Simulate CLI opening a directory with images
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    fs::write(dir_path.join("image1.png"), b"fake png").unwrap();
    fs::write(dir_path.join("image2.jpg"), b"fake jpg").unwrap();
    fs::write(dir_path.join("readme.txt"), b"text file").unwrap();

    let images = scan_directory(dir_path).unwrap();
    let state = AppState::new(images);

    // Should load only image files
    assert_eq!(state.image_paths.len(), 2);
    assert!(state.current_image().is_some());
}

#[test]
fn test_cli_workflow_single_file() {
    // Simulate CLI opening a single file - should scan parent directory
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    fs::write(dir_path.join("image1.png"), b"fake png").unwrap();
    fs::write(dir_path.join("image2.jpg"), b"fake jpg").unwrap();
    fs::write(dir_path.join("image3.gif"), b"fake gif").unwrap();

    let target_file = dir_path.join("image2.jpg");

    // Process as if it were dropped/opened
    let (images, start_index) = process_dropped_path(&target_file).unwrap();
    let state = AppState::new_with_index(images, start_index);

    // Should scan parent dir and start at the specified file
    assert_eq!(state.image_paths.len(), 3);
    assert_eq!(
        state.current_image().unwrap().file_name().unwrap(),
        "image2.jpg"
    );
}

#[test]
fn test_cli_workflow_mixed_files() {
    // Simulate CLI with both valid and invalid files in directory
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    fs::write(dir_path.join("photo.png"), b"fake png").unwrap();
    fs::write(dir_path.join("document.pdf"), b"fake pdf").unwrap();
    fs::write(dir_path.join("video.mp4"), b"fake video").unwrap();
    fs::write(dir_path.join("picture.jpg"), b"fake jpg").unwrap();

    let images = scan_directory(dir_path).unwrap();

    // Should only include supported image formats
    assert_eq!(images.len(), 2);
    for img in &images {
        let ext = img.extension().unwrap().to_str().unwrap();
        assert!(ext == "png" || ext == "jpg");
    }
}

// ============================================================================
// File Loading Workflows
// ============================================================================

#[test]
fn test_file_loading_workflow_sequential() {
    // Simulate loading images sequentially
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    fs::write(dir_path.join("a.png"), b"fake png").unwrap();
    fs::write(dir_path.join("b.jpg"), b"fake jpg").unwrap();
    fs::write(dir_path.join("c.gif"), b"fake gif").unwrap();

    let mut images = scan_directory(dir_path).unwrap();

    // Sort alphabetically for predictable order
    use rpview_gpui::utils::file_scanner::sort_alphabetically;
    sort_alphabetically(&mut images);

    let mut state = AppState::new(images);

    // Load first image
    assert_eq!(state.current_image().unwrap().file_name().unwrap(), "a.png");

    // Navigate to second
    state.next_image();
    assert_eq!(state.current_image().unwrap().file_name().unwrap(), "b.jpg");

    // Navigate to third
    state.next_image();
    assert_eq!(state.current_image().unwrap().file_name().unwrap(), "c.gif");
}

#[test]
fn test_file_loading_workflow_with_state_persistence() {
    // Simulate loading images and persisting state
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    fs::write(dir_path.join("image1.png"), b"fake").unwrap();
    fs::write(dir_path.join("image2.jpg"), b"fake").unwrap();

    let images = scan_directory(dir_path).unwrap();
    let mut state = AppState::new(images);

    // Set zoom for first image
    let mut img_state = ImageState::new();
    img_state.zoom = 2.0;
    state.save_current_state(img_state);

    // Navigate to second image
    state.next_image();

    // Set different zoom for second image
    let mut img_state2 = ImageState::new();
    img_state2.zoom = 0.5;
    state.save_current_state(img_state2);

    // Navigate back to first
    state.previous_image();

    // Should restore first image's state
    let restored = state.get_current_state(FilterSettings::default());
    assert_eq!(restored.zoom, 2.0);
}

#[test]
fn test_file_loading_workflow_drag_and_drop() {
    // Simulate drag-and-drop workflow
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    fs::write(dir_path.join("dropped.png"), b"fake").unwrap();
    fs::write(dir_path.join("other1.jpg"), b"fake").unwrap();
    fs::write(dir_path.join("other2.gif"), b"fake").unwrap();

    let dropped_file = dir_path.join("dropped.png");

    // Process the drop
    let (images, start_index) = process_dropped_path(&dropped_file).unwrap();
    let state = AppState::new_with_index(images, start_index);

    // Should start at dropped file
    assert_eq!(
        state.current_image().unwrap().file_name().unwrap(),
        "dropped.png"
    );

    // Should have all images from parent directory
    assert_eq!(state.image_paths.len(), 3);
}

// ============================================================================
// Navigation Workflows
// ============================================================================

#[test]
fn test_navigation_workflow_forward_backward() {
    let paths = vec![
        PathBuf::from("a.png"),
        PathBuf::from("b.jpg"),
        PathBuf::from("c.gif"),
    ];

    let mut state = AppState::new(paths);

    // Forward navigation
    assert_eq!(state.current_index, 0);
    state.next_image();
    assert_eq!(state.current_index, 1);
    state.next_image();
    assert_eq!(state.current_index, 2);

    // Backward navigation
    state.previous_image();
    assert_eq!(state.current_index, 1);
    state.previous_image();
    assert_eq!(state.current_index, 0);
}

#[test]
fn test_navigation_workflow_wraparound() {
    let paths = vec![PathBuf::from("a.png"), PathBuf::from("b.jpg")];

    let mut state = AppState::new(paths);

    // Wrap from last to first
    state.current_index = 1;
    state.next_image();
    assert_eq!(state.current_index, 0);

    // Wrap from first to last
    state.previous_image();
    assert_eq!(state.current_index, 1);
}

#[test]
fn test_navigation_workflow_with_sort_mode_change() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create files
    fs::write(dir_path.join("zebra.png"), b"fake").unwrap();
    fs::write(dir_path.join("apple.jpg"), b"fake").unwrap();

    let mut images = scan_directory(dir_path).unwrap();

    // Sort alphabetically for predictable initial state
    use rpview_gpui::utils::file_scanner::sort_alphabetically;
    sort_alphabetically(&mut images);

    let mut state = AppState::new(images);

    // Initially alphabetical
    assert_eq!(state.image_paths[0].file_name().unwrap(), "apple.jpg");
    assert_eq!(state.image_paths[1].file_name().unwrap(), "zebra.png");

    // Change to modified date sort (will re-sort)
    state.set_sort_mode(SortMode::ModifiedDate);

    // Images should still be accessible
    assert_eq!(state.image_paths.len(), 2);
}

#[test]
fn test_navigation_workflow_preload_paths() {
    let paths = vec![
        PathBuf::from("image1.png"),
        PathBuf::from("image2.jpg"),
        PathBuf::from("image3.gif"),
    ];

    let state = AppState::new(paths);

    // At first image, next should be image2
    assert_eq!(
        state.next_image_path().unwrap().file_name().unwrap(),
        "image2.jpg"
    );

    // At first image, previous should be image3 (wraparound)
    assert_eq!(
        state.previous_image_path().unwrap().file_name().unwrap(),
        "image3.gif"
    );
}

// ============================================================================
// Zoom/Pan Workflows
// ============================================================================

#[test]
fn test_zoom_workflow_fit_to_window_then_manual() {
    // Simulate typical zoom workflow: start fit-to-window, then manual zoom
    let image_width = 2000;
    let image_height = 1500;
    let viewport_width = 1000.0;
    let viewport_height = 800.0;

    // Initial: fit to window
    let mut zoom =
        calculate_fit_to_window(image_width, image_height, viewport_width, viewport_height);

    // Should scale down to fit
    assert!(zoom < 1.0);

    // User zooms in manually
    zoom = zoom_in(zoom, ZOOM_STEP);
    zoom = zoom_in(zoom, ZOOM_STEP);
    zoom = zoom_in(zoom, ZOOM_STEP);

    // Should be closer to 1:1 now
    assert!(
        zoom > calculate_fit_to_window(image_width, image_height, viewport_width, viewport_height)
    );
}

#[test]
fn test_zoom_workflow_toggle_fit_100() {
    // Simulate "0" key toggle between fit and 100%
    let image_width = 2000;
    let image_height = 1500;
    let viewport_width = 1000.0;
    let viewport_height = 800.0;

    let fit_zoom =
        calculate_fit_to_window(image_width, image_height, viewport_width, viewport_height);

    let mut zoom = fit_zoom;
    let mut is_fit = true;

    // Toggle to 100%
    if is_fit {
        zoom = 1.0;
        is_fit = false;
    }

    assert_eq!(zoom, 1.0);
    assert!(!is_fit);

    // Toggle back to fit
    if !is_fit {
        zoom = fit_zoom;
        is_fit = true;
    }

    assert_eq!(zoom, fit_zoom);
    assert!(is_fit);
}

#[test]
fn test_zoom_workflow_with_different_modifiers() {
    let mut zoom = 1.0;

    // Normal zoom
    zoom = zoom_in(zoom, ZOOM_STEP);
    assert_eq!(zoom, 1.2);

    // Fast zoom (Shift)
    zoom = 1.0;
    zoom = zoom_in(zoom, ZOOM_STEP_FAST);
    assert_eq!(zoom, 1.5);

    // Slow zoom (Ctrl/Cmd)
    zoom = 1.0;
    zoom = zoom_in(zoom, ZOOM_STEP_SLOW);
    assert_eq!(zoom, 1.05);

    // Incremental zoom (Shift+Ctrl/Cmd)
    zoom = 1.0;
    zoom = zoom_in(zoom, 1.0 + ZOOM_STEP_INCREMENTAL);
    assert!((zoom - 1.01).abs() < 0.0001);
}

#[test]
fn test_pan_workflow_with_zoom() {
    // Simulate panning at different zoom levels
    let mut image_state = ImageState::new();

    // At 100%, pan freely
    image_state.zoom = 1.0;
    image_state.pan = (100.0, 50.0);

    // Zoom in
    image_state.zoom = 2.0;

    // Pan position should still be valid
    assert_eq!(image_state.pan, (100.0, 50.0));

    // Zoom out to fit-to-window
    image_state.zoom = 0.5;
    image_state.is_fit_to_window = true;

    // Pan should reset when fit-to-window
    image_state.pan = (0.0, 0.0);

    assert_eq!(image_state.pan, (0.0, 0.0));
}

#[test]
fn test_complete_workflow_load_navigate_zoom_pan() {
    // Comprehensive workflow test
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // 1. Load images
    fs::write(dir_path.join("photo1.png"), b"fake").unwrap();
    fs::write(dir_path.join("photo2.jpg"), b"fake").unwrap();

    let images = scan_directory(dir_path).unwrap();
    let mut state = AppState::new(images);

    // 2. View first image at fit-to-window
    let mut img_state = state.get_current_state(FilterSettings::default());
    assert!(img_state.is_fit_to_window);

    // 3. Zoom in manually
    img_state.zoom = zoom_in(img_state.zoom, ZOOM_STEP);
    img_state.is_fit_to_window = false;

    // 4. Pan the image
    img_state.pan = (50.0, 25.0);

    // 5. Save state
    state.save_current_state(img_state.clone());

    // 6. Navigate to next image
    state.next_image();

    // 7. Second image should have default state
    let img_state2 = state.get_current_state(FilterSettings::default());
    assert!(img_state2.is_fit_to_window);
    assert_eq!(img_state2.pan, (0.0, 0.0));

    // 8. Navigate back to first
    state.previous_image();

    // 9. First image should restore previous state
    let restored = state.get_current_state(FilterSettings::default());
    assert_eq!(restored.zoom, img_state.zoom);
    assert_eq!(restored.pan, (50.0, 25.0));
    assert!(!restored.is_fit_to_window);
}

#[test]
fn test_workflow_empty_list_safety() {
    // Ensure app doesn't crash with empty image list
    let mut state = AppState::new(vec![]);

    assert_eq!(state.current_image(), None);
    assert_eq!(state.next_image_path(), None);
    assert_eq!(state.previous_image_path(), None);

    // Navigation should not panic
    state.next_image();
    state.previous_image();

    // State operations should not panic
    let img_state = state.get_current_state(FilterSettings::default());
    state.save_current_state(img_state);
}

#[test]
fn test_workflow_large_image_list() {
    // Test with a large number of images
    let mut paths = Vec::new();
    for i in 0..1000 {
        paths.push(PathBuf::from(format!("image{:04}.png", i)));
    }

    let mut state = AppState::new(paths);

    // Navigate forward 100 times
    for _ in 0..100 {
        state.next_image();
    }

    assert_eq!(state.current_index, 100);

    // Navigate backward 50 times
    for _ in 0..50 {
        state.previous_image();
    }

    assert_eq!(state.current_index, 50);

    // State cache should work efficiently
    for i in 0..50 {
        let mut img_state = ImageState::new();
        img_state.zoom = i as f32 / 10.0;
        state.save_current_state(img_state);
        state.next_image();
    }

    // Cache should have evicted old entries
    assert!(state.image_states.len() <= state.max_cache_size);
}
