use std::io;
use std::path::PathBuf;

/// Application-wide error type
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("File not found: {}", .0.display())]
    FileNotFound(PathBuf),
    #[error("Invalid format for {}: {}", .0.display(), .1)]
    InvalidFormat(PathBuf, String),
    #[error("No images found in directory: {}", .0.display())]
    NoImagesFound(PathBuf),
    #[error("Permission denied: {}", .0.display())]
    PermissionDenied(PathBuf),
    #[error("Failed to load image {}: {}", .0.display(), .1)]
    ImageLoadError(PathBuf, String),
    #[error("{0}")]
    Generic(String),
}

/// Result type for application operations
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::path::PathBuf;

    // -- Display formatting ---------------------------------------------------

    #[test]
    fn display_io_error() {
        let inner = io::Error::new(io::ErrorKind::BrokenPipe, "pipe broke");
        let err = AppError::Io(inner);
        let msg = err.to_string();
        assert!(msg.starts_with("I/O error:"), "got: {msg}");
        assert!(msg.contains("pipe broke"), "got: {msg}");
    }

    #[test]
    fn display_file_not_found() {
        let err = AppError::FileNotFound(PathBuf::from("/tmp/missing.png"));
        assert_eq!(err.to_string(), "File not found: /tmp/missing.png");
    }

    #[test]
    fn display_invalid_format() {
        let err = AppError::InvalidFormat(
            PathBuf::from("photo.heic"),
            "HEIC not supported".into(),
        );
        let msg = err.to_string();
        assert!(msg.contains("photo.heic"), "got: {msg}");
        assert!(msg.contains("HEIC not supported"), "got: {msg}");
    }

    #[test]
    fn display_no_images_found() {
        let err = AppError::NoImagesFound(PathBuf::from("/empty"));
        assert!(err.to_string().contains("/empty"));
    }

    #[test]
    fn display_permission_denied() {
        let err = AppError::PermissionDenied(PathBuf::from("/secret"));
        assert!(err.to_string().contains("/secret"));
    }

    #[test]
    fn display_image_load_error() {
        let err = AppError::ImageLoadError(
            PathBuf::from("bad.png"),
            "corrupt header".into(),
        );
        let msg = err.to_string();
        assert!(msg.contains("bad.png"), "got: {msg}");
        assert!(msg.contains("corrupt header"), "got: {msg}");
    }

    #[test]
    fn display_generic() {
        let err = AppError::Generic("something went wrong".into());
        assert_eq!(err.to_string(), "something went wrong");
    }

    // -- Error::source --------------------------------------------------------

    #[test]
    fn source_returns_inner_for_io_variant() {
        let inner = io::Error::new(io::ErrorKind::NotFound, "gone");
        let err = AppError::Io(inner);
        let source = std::error::Error::source(&err);
        assert!(source.is_some());
    }

    #[test]
    fn source_returns_none_for_non_io_variants() {
        let variants: Vec<AppError> = vec![
            AppError::FileNotFound(PathBuf::from("x")),
            AppError::InvalidFormat(PathBuf::from("x"), "y".into()),
            AppError::NoImagesFound(PathBuf::from("x")),
            AppError::PermissionDenied(PathBuf::from("x")),
            AppError::ImageLoadError(PathBuf::from("x"), "y".into()),
            AppError::Generic("z".into()),
        ];
        for v in &variants {
            assert!(
                std::error::Error::source(v).is_none(),
                "expected None for {:?}",
                v
            );
        }
    }

    // -- From<io::Error> ------------------------------------------------------

    #[test]
    fn from_io_error_wraps_in_io_variant() {
        let io_err = io::Error::new(io::ErrorKind::TimedOut, "timeout");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
    }
}
