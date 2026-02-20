use rpview_gpui::utils::file_scanner::{
    is_supported_image, process_dropped_path, scan_directory, sort_alphabetically,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn test_is_supported_image_all_formats() {
    // PNG
    assert!(is_supported_image(Path::new("test.png")));
    assert!(is_supported_image(Path::new("test.PNG")));

    // JPEG
    assert!(is_supported_image(Path::new("test.jpg")));
    assert!(is_supported_image(Path::new("test.JPG")));
    assert!(is_supported_image(Path::new("test.jpeg")));
    assert!(is_supported_image(Path::new("test.JPEG")));

    // BMP
    assert!(is_supported_image(Path::new("test.bmp")));
    assert!(is_supported_image(Path::new("test.BMP")));

    // GIF
    assert!(is_supported_image(Path::new("test.gif")));
    assert!(is_supported_image(Path::new("test.GIF")));

    // TIFF
    assert!(is_supported_image(Path::new("test.tiff")));
    assert!(is_supported_image(Path::new("test.tif")));
    assert!(is_supported_image(Path::new("test.TIFF")));
    assert!(is_supported_image(Path::new("test.TIF")));

    // ICO
    assert!(is_supported_image(Path::new("test.ico")));
    assert!(is_supported_image(Path::new("test.ICO")));

    // WEBP
    assert!(is_supported_image(Path::new("test.webp")));
    assert!(is_supported_image(Path::new("test.WEBP")));
}

#[test]
fn test_is_supported_image_unsupported_formats() {
    assert!(!is_supported_image(Path::new("test.txt")));
    assert!(!is_supported_image(Path::new("test.pdf")));
    assert!(!is_supported_image(Path::new("test.doc")));
    assert!(!is_supported_image(Path::new("test.mp4")));
    assert!(is_supported_image(Path::new("test.svg")));
    assert!(is_supported_image(Path::new("test.SVG")));
    assert!(!is_supported_image(Path::new("test")));
    assert!(!is_supported_image(Path::new("test.")));
}

#[test]
fn test_scan_directory_with_images() {
    // Create a temporary directory with test images
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create some test files
    fs::write(dir_path.join("image1.png"), b"fake png").unwrap();
    fs::write(dir_path.join("image2.jpg"), b"fake jpg").unwrap();
    fs::write(dir_path.join("image3.gif"), b"fake gif").unwrap();
    fs::write(dir_path.join("not_an_image.txt"), b"text file").unwrap();

    // Scan the directory
    let images = scan_directory(dir_path).unwrap();

    // Should find 3 images
    assert_eq!(images.len(), 3);

    // All should be image files
    for path in &images {
        assert!(is_supported_image(path));
    }
}

#[test]
fn test_scan_directory_empty() {
    // Create an empty temporary directory
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Scan should return empty list
    let images = scan_directory(dir_path).unwrap();
    assert_eq!(images.len(), 0);
}

#[test]
fn test_scan_directory_no_images() {
    // Create a temporary directory with only non-image files
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    fs::write(dir_path.join("file1.txt"), b"text").unwrap();
    fs::write(dir_path.join("file2.pdf"), b"pdf").unwrap();

    // Should return empty list
    let images = scan_directory(dir_path).unwrap();
    assert_eq!(images.len(), 0);
}

#[test]
fn test_scan_directory_ignores_subdirectories() {
    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create image in main directory
    fs::write(dir_path.join("image1.png"), b"fake png").unwrap();

    // Create subdirectory with image
    let sub_dir = dir_path.join("subdir");
    fs::create_dir(&sub_dir).unwrap();
    fs::write(sub_dir.join("image2.png"), b"fake png").unwrap();

    // Scan should only find image1.png (non-recursive)
    let images = scan_directory(dir_path).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].file_name().unwrap(), "image1.png");
}

#[test]
fn test_sort_alphabetically() {
    let mut paths = vec![
        PathBuf::from("zebra.png"),
        PathBuf::from("Apple.jpg"),
        PathBuf::from("banana.gif"),
    ];

    sort_alphabetically(&mut paths);

    assert_eq!(paths[0].file_name().unwrap(), "Apple.jpg");
    assert_eq!(paths[1].file_name().unwrap(), "banana.gif");
    assert_eq!(paths[2].file_name().unwrap(), "zebra.png");
}

#[test]
fn test_sort_alphabetically_case_insensitive() {
    let mut paths = vec![
        PathBuf::from("ZEBRA.png"),
        PathBuf::from("apple.jpg"),
        PathBuf::from("BANANA.gif"),
        PathBuf::from("Cherry.bmp"),
    ];

    sort_alphabetically(&mut paths);

    assert_eq!(paths[0].file_name().unwrap(), "apple.jpg");
    assert_eq!(paths[1].file_name().unwrap(), "BANANA.gif");
    assert_eq!(paths[2].file_name().unwrap(), "Cherry.bmp");
    assert_eq!(paths[3].file_name().unwrap(), "ZEBRA.png");
}

#[test]
fn test_process_dropped_path_file() {
    // Create a temporary directory with test images
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    fs::write(dir_path.join("image1.png"), b"fake png").unwrap();
    fs::write(dir_path.join("image2.jpg"), b"fake jpg").unwrap();
    fs::write(dir_path.join("image3.gif"), b"fake gif").unwrap();

    let dropped_file = dir_path.join("image2.jpg");

    // Process the dropped file
    let (all_images, start_index) = process_dropped_path(&dropped_file).unwrap();

    // Should find all 3 images
    assert_eq!(all_images.len(), 3);

    // Should start at image2.jpg
    assert_eq!(all_images[start_index].file_name().unwrap(), "image2.jpg");
}

#[test]
fn test_process_dropped_path_directory() {
    // Create a temporary directory with test images
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    fs::write(dir_path.join("image1.png"), b"fake png").unwrap();
    fs::write(dir_path.join("image2.jpg"), b"fake jpg").unwrap();

    // Process the dropped directory
    let (all_images, start_index) = process_dropped_path(dir_path).unwrap();

    // Should find 2 images
    assert_eq!(all_images.len(), 2);

    // Should start at index 0
    assert_eq!(start_index, 0);
}

#[test]
fn test_process_dropped_path_nonexistent() {
    let nonexistent = PathBuf::from("/nonexistent/path/image.png");

    // Should return FileNotFound error
    let result = process_dropped_path(&nonexistent);
    assert!(result.is_err());
}

#[test]
fn test_process_dropped_path_unsupported_format() {
    // Create a temporary directory with a non-image file
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    let text_file = dir_path.join("document.txt");
    fs::write(&text_file, b"text content").unwrap();

    // Should return InvalidFormat error
    let result = process_dropped_path(&text_file);
    assert!(result.is_err());
}

#[test]
fn test_process_dropped_path_sorts_alphabetically() {
    // Create a temporary directory with unsorted images
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    fs::write(dir_path.join("zebra.png"), b"fake png").unwrap();
    fs::write(dir_path.join("apple.jpg"), b"fake jpg").unwrap();
    fs::write(dir_path.join("banana.gif"), b"fake gif").unwrap();

    let dropped_file = dir_path.join("zebra.png");

    // Process the dropped file
    let (all_images, start_index) = process_dropped_path(&dropped_file).unwrap();

    // Should be sorted alphabetically
    assert_eq!(all_images[0].file_name().unwrap(), "apple.jpg");
    assert_eq!(all_images[1].file_name().unwrap(), "banana.gif");
    assert_eq!(all_images[2].file_name().unwrap(), "zebra.png");

    // zebra.png should be at index 2
    assert_eq!(start_index, 2);
}
