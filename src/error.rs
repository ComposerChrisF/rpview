use std::fmt;
use std::io;
use std::path::PathBuf;

/// Application-wide error type
#[derive(Debug)]
pub enum AppError {
    /// I/O error
    Io(io::Error),
    /// File not found
    FileNotFound(PathBuf),
    /// Invalid file format
    InvalidFormat(PathBuf, String),
    /// No images found in directory
    NoImagesFound(PathBuf),
    /// Permission denied
    PermissionDenied(PathBuf),
    /// Image loading error
    ImageLoadError(PathBuf, String),
    /// Generic error with message
    Generic(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(err) => write!(f, "I/O error: {}", err),
            AppError::FileNotFound(path) => write!(f, "File not found: {}", path.display()),
            AppError::InvalidFormat(path, msg) => {
                write!(f, "Invalid format for {}: {}", path.display(), msg)
            }
            AppError::NoImagesFound(path) => {
                write!(f, "No images found in directory: {}", path.display())
            }
            AppError::PermissionDenied(path) => {
                write!(f, "Permission denied: {}", path.display())
            }
            AppError::ImageLoadError(path, msg) => {
                write!(f, "Failed to load image {}: {}", path.display(), msg)
            }
            AppError::Generic(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => AppError::FileNotFound(PathBuf::new()),
            io::ErrorKind::PermissionDenied => AppError::PermissionDenied(PathBuf::new()),
            _ => AppError::Io(err),
        }
    }
}

/// Result type for application operations
pub type AppResult<T> = Result<T, AppError>;
